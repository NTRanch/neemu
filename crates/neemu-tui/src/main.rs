//! neemu-tui binary: parse args, build the real facade, run.
//!
//! Facade choice: the library facade by default (in-process, the
//! server's own dispatch). `--stdio-bin <path>` selects the MCP
//! stdio client fallback against a spawned server binary. Line mode
//! is forced by `--line` or selected automatically when stdin is
//! not a TTY (the phlow precedent: same execute path, scriptable).

#![forbid(unsafe_code)]

use std::io::{BufReader, IsTerminal, Write};
use std::path::PathBuf;

use crossterm::event::{self, Event};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Terminal;

use neemu_tui::app::App;
use neemu_tui::facade::{LibFacade, NeemuFacade, StdioFacade};
use neemu_tui::line::run_line_mode;
use neemu_tui::media::{detect_protocol, Protocol};
use neemu_tui::panels::{
    fleet_panel, log_panel, media_header, overview_panel, review_panel, tab_bar, workflows_panel,
};
use neemu_tui::screens::Screen;

/// Resolve the registry home exactly like the server binary:
/// `--home`, else `$NEEMU_HOME`, else `$XDG_DATA_HOME/neemu`, else
/// `~/.local/share/neemu`.
fn resolve_home(flag: Option<String>) -> PathBuf {
    if let Some(home) = flag {
        return PathBuf::from(home);
    }
    if let Ok(home) = std::env::var("NEEMU_HOME") {
        if !home.is_empty() {
            return PathBuf::from(home);
        }
    }
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("neemu");
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".local/share/neemu")
}

struct Args {
    home: Option<String>,
    stdio_bin: Option<String>,
    line: bool,
    triggers: Option<String>,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        home: None,
        stdio_bin: None,
        line: false,
        triggers: None,
    };
    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--home" => args.home = Some(iter.next().ok_or("--home needs a value")?),
            "--stdio-bin" => args.stdio_bin = Some(iter.next().ok_or("--stdio-bin needs a value")?),
            "--line" => args.line = true,
            "--triggers" => args.triggers = Some(iter.next().ok_or("--triggers needs a value")?),
            "--help" | "-h" => {
                println!(
                    "neemu-tui [--home <dir>] [--stdio-bin <neemu binary>] [--line] \
                     [--triggers <triggers.toml>]\n{}",
                    neemu_tui::line::HELP
                );
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(args)
}

fn build_app(args: &Args) -> Result<App, String> {
    let home = resolve_home(args.home.clone());
    let facade: Box<dyn NeemuFacade> = match &args.stdio_bin {
        Some(bin) => {
            Box::new(StdioFacade::spawn(bin, &home).map_err(|e| format!("stdio facade: {e}"))?)
        }
        None => Box::new(LibFacade::open(home).map_err(|e| format!("lib facade: {e}"))?),
    };
    let rules = match &args.triggers {
        Some(path) => {
            let text =
                std::fs::read_to_string(path).map_err(|e| format!("read triggers {path}: {e}"))?;
            let (rules, errors) = neemu_tui::triggers::parse_rules(&text);
            for error in &errors {
                eprintln!("triggers: {error}");
            }
            rules
        }
        None => neemu_tui::triggers::default_rules(),
    };
    Ok(App::with_rules(facade, rules))
}

fn main() {
    let args = match parse_args() {
        Ok(args) => args,
        Err(error) => {
            eprintln!("neemu-tui: {error}");
            std::process::exit(2);
        }
    };
    let mut app = match build_app(&args) {
        Ok(app) => app,
        Err(error) => {
            eprintln!("neemu-tui: {error}");
            std::process::exit(1);
        }
    };
    let stdin_is_tty = std::io::stdin().is_terminal();
    if args.line || !stdin_is_tty {
        let stdin = std::io::stdin();
        let mut input = BufReader::new(stdin);
        let mut output = std::io::stdout();
        if let Err(error) = run_line_mode(&mut app, &mut input, &mut output) {
            eprintln!("neemu-tui: line mode: {error}");
            std::process::exit(1);
        }
        return;
    }
    if let Err(error) = run_fullscreen(&mut app) {
        eprintln!("neemu-tui: {error}");
        std::process::exit(1);
    }
}

fn run_fullscreen(app: &mut App) -> std::io::Result<()> {
    let protocol = detect_protocol(|k| std::env::var(k).ok());
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = event_loop(app, &mut terminal, protocol);
    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    result
}

fn event_loop(
    app: &mut App,
    terminal: &mut Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    protocol: Protocol,
) -> std::io::Result<()> {
    let mut kitty_sent_for: Option<String> = None;
    loop {
        terminal.draw(|frame| draw(app, frame))?;
        // Kitty passthrough happens outside the ratatui buffer: the
        // escape is written to the real terminal once per path.
        if app.screen == Screen::Media {
            if let (Some(view), Some(path)) = (&app.media.view, &app.media.path) {
                if view.kitty_bytes.is_some() && kitty_sent_for.as_deref() != Some(path) {
                    if let Ok(raw) = std::fs::read(path) {
                        let mut out = std::io::stdout();
                        let _ = write!(out, "{}", neemu_tui::media::kitty_escape(&raw));
                        let _ = out.flush();
                    }
                    kitty_sent_for = Some(path.clone());
                }
            }
        }
        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key, protocol);
            }
        }
        if app.should_quit {
            return Ok(());
        }
    }
}

fn draw(app: &App, frame: &mut ratatui::Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(frame.area());
    frame.render_widget(tab_bar(app.screen), chunks[0]);
    let area = chunks[1];
    match app.screen {
        Screen::Overview => frame.render_widget(
            overview_panel(
                &app.status_cache,
                &app.workflows.works,
                app.engine.rules.len(),
                &app.log.lines(),
            ),
            area,
        ),
        Screen::Workflows => frame.render_widget(workflows_panel(&app.workflows), area),
        Screen::Review => frame.render_widget(review_panel(&app.review), area),
        Screen::Media => draw_media(app, frame, area),
        Screen::Log => frame.render_widget(log_panel(&app.log.lines()), area),
        Screen::Fleet => frame.render_widget(
            fleet_panel(
                &app.engine.rules,
                &app.engine.candidates,
                &app.engine.audit,
                "neemu is the only fleet member this console drives in rung 1; \
                 torori appears as the declared trigger target",
            ),
            area,
        ),
    }
}

fn draw_media(app: &App, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);
    let label = app
        .media
        .view
        .as_ref()
        .map(|v| v.label.clone())
        .unwrap_or_else(|| {
            app.media
                .path
                .clone()
                .unwrap_or_else(|| "(no frame selected)".to_string())
        });
    let kitty_bytes = app.media.view.as_ref().and_then(|v| v.kitty_bytes);
    frame.render_widget(
        media_header(&label, kitty_bytes, app.media.error.as_deref()),
        chunks[0],
    );
    if let Some(view) = &app.media.view {
        if let Some(grid) = &view.grid {
            let buffer = frame.buffer_mut();
            for (row_idx, line) in grid.iter().enumerate() {
                for (col_idx, cell) in line.iter().enumerate() {
                    let x = chunks[1].x + col_idx as u16;
                    let y = chunks[1].y + row_idx as u16;
                    if x < chunks[1].right() && y < chunks[1].bottom() {
                        buffer[(x, y)]
                            .set_symbol("▀")
                            .set_fg(ratatui::style::Color::Rgb(
                                cell.fg[0], cell.fg[1], cell.fg[2],
                            ))
                            .set_bg(ratatui::style::Color::Rgb(
                                cell.bg[0], cell.bg[1], cell.bg[2],
                            ));
                    }
                }
            }
        }
    }
}
