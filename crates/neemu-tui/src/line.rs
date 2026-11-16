//! Non-TTY line mode (DESIGN D2): the same execute path as the
//! full-screen TUI, driven by one command per line, so the TUI is
//! scriptable and testable from a pipe.
//!
//! Commands: `status`, `tools`, `works`, `select <index>`,
//! `workflow <index>`, `args <json>`, `mode plan|auto`, `stage`,
//! `approve`, `abort`, `run`, `review`, `media <path>`,
//! `candidate approve <index>`, `log`, `fleet`, `refresh`, `quit`.

use std::io::{BufRead, Write};

use serde_json::Value;

use crate::app::App;
use crate::media::{detect_protocol, half_block_ansi, kitty_escape, Protocol};
use crate::panels::{fleet_panel, overview_panel};

/// Run line mode over generic I/O until `quit` or EOF.
pub fn run_line_mode<R: BufRead, W: Write>(
    app: &mut App,
    input: &mut R,
    output: &mut W,
) -> std::io::Result<()> {
    let protocol = detect_protocol(|k| std::env::var(k).ok());
    let mut line = String::new();
    loop {
        line.clear();
        let read = input.read_line(&mut line)?;
        if read == 0 {
            break;
        }
        let command = line.trim();
        if command.is_empty() {
            continue;
        }
        let (head, rest) = match command.split_once(' ') {
            Some((h, r)) => (h, r.trim()),
            None => (command, ""),
        };
        match head {
            "quit" | "exit" => break,
            "status" => writeln!(output, "{}", status_text(app))?,
            "tools" => writeln!(output, "{}", tools_text(app))?,
            "works" => writeln!(output, "{}", works_text(app))?,
            "select" => writeln!(output, "{}", select_work(app, rest))?,
            "workflow" => writeln!(output, "{}", select_workflow(app, rest))?,
            "args" => {
                app.workflows.extra_args_text = rest.to_string();
                writeln!(output, "extra args set: {rest}")?;
            }
            "mode" => writeln!(output, "{}", set_mode(app, rest))?,
            "stage" => match app.stage_selected() {
                Ok(staged) => writeln!(
                    output,
                    "staged {} {} — approve to execute, abort to drop",
                    staged.tool, staged.args
                )?,
                Err(e) => writeln!(output, "stage error: {e}")?,
            },
            "approve" => match app.approve_staged() {
                Ok(value) => writeln!(output, "result: {value}")?,
                Err(e) => writeln!(output, "approve error: {e}")?,
            },
            "abort" => {
                app.abort_staged();
                writeln!(output, "aborted (no call was made)")?;
            }
            "run" => match app.stage_or_run() {
                Ok(value) => writeln!(output, "result: {value}")?,
                Err(e) => writeln!(output, "run error: {e}")?,
            },
            "review" => writeln!(output, "{}", review_text(app))?,
            "media" => media_command(app, rest, protocol, output)?,
            "candidate" => writeln!(output, "{}", candidate_command(app, rest))?,
            "log" => {
                for entry in app.log.lines() {
                    writeln!(output, "{entry}")?;
                }
            }
            "fleet" => writeln!(output, "{}", fleet_text(app))?,
            "refresh" => {
                app.refresh();
                writeln!(output, "refreshed")?;
            }
            "help" => writeln!(output, "{}", HELP)?,
            other => writeln!(output, "unknown command: {other} (try `help`)")?,
        }
        output.flush()?;
    }
    Ok(())
}

/// The line-mode command list.
pub const HELP: &str = "commands: status tools works select <i> workflow <i> args <json> \
mode plan|auto stage approve abort run review media <path> candidate approve <i> log fleet refresh quit";

fn status_text(app: &App) -> String {
    match &app.status_cache {
        Ok(value) => value.to_string(),
        Err(error) => format!("{{\"server_state\": \"unknown\", \"error\": \"{error}\"}}"),
    }
}

fn tools_text(app: &mut App) -> String {
    // tools go through the facade via a fresh status-independent path:
    // the App owns the facade, so line mode reaches it through a
    // staged helper — here we reuse the cached tool count plus the
    // workflow table, and name the full list source honestly.
    format!(
        "workflow tools: {} (full tools/list is served by the Overview status tool_count={})",
        crate::screens::WORKFLOWS
            .iter()
            .map(|(_, t)| *t)
            .collect::<Vec<_>>()
            .join(", "),
        app.status_cache
            .as_ref()
            .ok()
            .and_then(|v| v.get("tool_count").cloned())
            .unwrap_or(Value::Null)
    )
}

fn works_text(app: &App) -> String {
    let mut out = String::new();
    for (idx, work) in app.workflows.works.iter().enumerate() {
        out.push_str(&format!(
            "{idx}: {} [{}] {}\n",
            work.get("work_id").and_then(Value::as_str).unwrap_or("?"),
            work.get("determination")
                .and_then(Value::as_str)
                .unwrap_or("?"),
            work.get("title").and_then(Value::as_str).unwrap_or(""),
        ));
    }
    if out.is_empty() {
        out.push_str("(no works scanned)\n");
    }
    out.trim_end().to_string()
}

fn select_work(app: &mut App, rest: &str) -> String {
    match rest.parse::<usize>() {
        Ok(idx) if idx < app.workflows.works.len() => {
            app.workflows.selected_work = idx;
            format!("selected work {idx}")
        }
        _ => format!("invalid work index: {rest}"),
    }
}

fn select_workflow(app: &mut App, rest: &str) -> String {
    match rest.parse::<usize>() {
        Ok(idx) if idx < crate::screens::WORKFLOWS.len() => {
            app.workflows.selected_workflow = idx;
            format!("selected workflow {}", crate::screens::WORKFLOWS[idx].1)
        }
        _ => format!("invalid workflow index: {rest}"),
    }
}

fn set_mode(app: &mut App, rest: &str) -> String {
    match rest {
        "plan" => {
            app.workflows.mode = crate::screens::Mode::Plan;
            "mode: plan".to_string()
        }
        "auto" => {
            app.workflows.mode = crate::screens::Mode::Auto;
            "mode: auto".to_string()
        }
        other => format!("unknown mode: {other} (plan|auto)"),
    }
}

fn review_text(app: &mut App) -> String {
    if let Err(error) = app.load_review() {
        return format!("review error: {error}");
    }
    let review = &app.review;
    format!(
        "work={} faces={} dialogue={} acts={} cues={}{}",
        review.work_id.as_deref().unwrap_or("?"),
        review.faces.len(),
        review.dialogue.len(),
        review.acts.len(),
        review.cues.len(),
        review
            .error
            .as_ref()
            .map(|e| format!(" error={e}"))
            .unwrap_or_default(),
    )
}

fn media_command<W: Write>(
    app: &mut App,
    path: &str,
    protocol: Protocol,
    output: &mut W,
) -> std::io::Result<()> {
    if path.is_empty() {
        writeln!(output, "media error: usage: media <path>")?;
        return Ok(());
    }
    app.open_media(path, protocol);
    match (&app.media.view, &app.media.error) {
        (Some(view), _) => {
            writeln!(output, "{}", view.label)?;
            if let Some(grid) = &view.grid {
                write!(output, "{}", half_block_ansi(grid))?;
            }
            if let Some(bytes) = view.kitty_bytes {
                // Kitty passthrough: transmit the file bytes to the
                // terminal. Read-only — the file is never rewritten.
                match std::fs::read(path) {
                    Ok(raw) if raw.len() == bytes => {
                        write!(output, "{}", kitty_escape(&raw))?;
                    }
                    Ok(_) => writeln!(output, "media error: file changed while viewing")?,
                    Err(e) => writeln!(output, "media error: {e}")?,
                }
            }
        }
        (None, Some(error)) => writeln!(output, "media error: {error}")?,
        (None, None) => writeln!(output, "media: nothing to show")?,
    }
    Ok(())
}

fn candidate_command(app: &mut App, rest: &str) -> String {
    let mut parts = rest.split_whitespace();
    match (parts.next(), parts.next()) {
        (Some("approve"), Some(idx)) => match idx.parse::<usize>() {
            Ok(idx) => match app.approve_candidate(idx) {
                Ok(()) => format!("candidate {idx} approved (recorded; not fired in rung 1)"),
                Err(e) => format!("candidate error: {e}"),
            },
            Err(_) => format!("invalid candidate index: {idx}"),
        },
        _ => "usage: candidate approve <index>".to_string(),
    }
}

fn fleet_text(app: &App) -> String {
    // Render the fleet panel to text via a TestBackend-free path:
    // summarise the same data the panel shows.
    let mut out = String::new();
    for rule in &app.engine.rules {
        out.push_str(&format!(
            "rule {} [{}] {} → {}.{} mode={} lane={} label={}\n",
            rule.id,
            if rule.enabled { "on" } else { "off" },
            rule.on_event,
            rule.target_mcp,
            rule.target_tool,
            rule.mode,
            rule.origin_lane,
            rule.content_label,
        ));
    }
    out.push_str(&format!("candidates: {}\n", app.engine.candidates.len()));
    out.push_str("live firing: not wired in rung 1\n");
    out.trim_end().to_string()
}

/// Render a panel widget to plain text for tests and debugging.
pub fn widget_to_text<W: ratatui::widgets::Widget>(widget: W, width: u16, height: u16) -> String {
    let backend = ratatui::backend::TestBackend::new(width, height);
    let mut terminal = ratatui::Terminal::new(backend).expect("test backend");
    terminal
        .draw(|frame| widget.render(frame.area(), frame.buffer_mut()))
        .expect("draw");
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect()
}

/// Overview as text (used by the smoke test).
pub fn overview_text(app: &App) -> String {
    widget_to_text(
        overview_panel(
            &app.status_cache,
            &app.workflows.works,
            app.engine.rules.len(),
            &app.log.lines(),
        ),
        100,
        24,
    )
}

/// Fleet panel as text (used by the smoke test).
pub fn fleet_panel_text(app: &App) -> String {
    widget_to_text(
        fleet_panel(
            &app.engine.rules,
            &app.engine.candidates,
            &app.engine.audit,
            "neemu is the only fleet member this console drives in rung 1",
        ),
        120,
        30,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facade::FakeFacade;
    use std::io::Cursor;

    #[test]
    fn line_mode_answers_status_and_quits() {
        let mut app = App::new(Box::new(FakeFacade::seeded()));
        let mut input = Cursor::new(b"status\nworks\nquit\n".to_vec());
        let mut output: Vec<u8> = Vec::new();
        run_line_mode(&mut app, &mut input, &mut output).expect("line mode");
        let text = String::from_utf8(output).expect("utf8");
        assert!(
            text.contains("\"member\": \"neemu\"") || text.contains("neemu"),
            "{text}"
        );
        assert!(text.contains("work-a"), "{text}");
    }

    #[test]
    fn line_mode_plan_then_approve_executes() {
        let mut app = App::new(Box::new(FakeFacade::seeded()));
        let mut input = Cursor::new(b"stage\napprove\nlog\nquit\n".to_vec());
        let mut output: Vec<u8> = Vec::new();
        run_line_mode(&mut app, &mut input, &mut output).expect("line mode");
        let text = String::from_utf8(output).expect("utf8");
        assert!(text.contains("staged neemu_extract_faces"), "{text}");
        assert!(text.contains("done neemu_extract_faces"), "{text}");
    }
}
