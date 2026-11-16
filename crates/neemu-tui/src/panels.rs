//! Pure widget constructors (DESIGN D2): each function builds a
//! ratatui widget from plain data, with named truncation bounds.
//! No I/O, no facade access — rendering is tested through
//! `TestBackend` without a terminal.

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use serde_json::Value;

use crate::screens::{Mode, ReviewState, RunState, WorkflowState, WORKFLOWS};
use crate::triggers::{Candidate, Rule};
use crate::{truncate_chars, ARGS_PREVIEW_CHARS_MAX, RECORD_LINE_CHARS_MAX};

/// Maximum lines shown in the Log panel tail.
pub const LOG_TAIL_LINES_MAX: usize = 200;
/// Maximum rules shown in the Fleet panel.
pub const RULES_SHOWN_MAX: usize = 32;
/// Maximum audit lines shown in the Fleet panel tail.
pub const AUDIT_TAIL_LINES_MAX: usize = 20;

/// The tab bar line for all screens.
pub fn tab_bar(active: crate::screens::Screen) -> Paragraph<'static> {
    let mut spans = Vec::new();
    for screen in crate::screens::Screen::ALL {
        let style = if screen == active {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::default().fg(Color::Gray)
        };
        spans.push(Span::styled(format!(" {} ", screen.name()), style));
        spans.push(Span::raw(" "));
    }
    Paragraph::new(Line::from(spans))
}

/// Overview: member identity, queue state (works by determination),
/// trigger count, and the honest server-state line.
pub fn overview_panel(
    status: &Result<Value, String>,
    works: &[Value],
    rule_count: usize,
    last_lines: &[String],
) -> Paragraph<'static> {
    let mut lines: Vec<Line> = Vec::new();
    match status {
        Ok(value) => {
            lines.push(Line::from(format!(
                "member: {} {}",
                value.get("member").and_then(Value::as_str).unwrap_or("?"),
                value.get("version").and_then(Value::as_str).unwrap_or("?"),
            )));
            lines.push(Line::from(format!(
                "transport: {}",
                value
                    .get("transport")
                    .and_then(Value::as_str)
                    .unwrap_or("?")
            )));
            lines.push(Line::from(format!(
                "tools: {}",
                value
                    .get("tool_count")
                    .map_or("?".to_string(), |n| n.to_string())
            )));
            if let Some(w) = value.get("works") {
                lines.push(Line::from(format!(
                    "extraction queue: {} works — {} cleared, {} excluded, {} other/pending",
                    w.get("total").map_or("?".to_string(), |n| n.to_string()),
                    w.get("cleared").map_or("?".to_string(), |n| n.to_string()),
                    w.get("excluded").map_or("?".to_string(), |n| n.to_string()),
                    w.get("other").map_or("?".to_string(), |n| n.to_string()),
                )));
            }
            lines.push(Line::from("server state: reachable".to_string()));
        }
        Err(error) => {
            lines.push(Line::from("member: neemu".to_string()));
            lines.push(Line::from(format!(
                "server state: UNKNOWN — {}",
                truncate_chars(error, RECORD_LINE_CHARS_MAX)
            )));
            lines.push(Line::from(
                "queue state is not guessed while the server is unreachable",
            ));
        }
    }
    lines.push(Line::from(format!("works scanned: {}", works.len())));
    lines.push(Line::from(format!("trigger rules declared: {rule_count}")));
    lines.push(Line::from(""));
    lines.push(Line::from("last events:"));
    for line in last_lines.iter().rev().take(5).rev() {
        lines.push(Line::from(truncate_chars(line, RECORD_LINE_CHARS_MAX)));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(
        "keys: Tab next screen · ↑/↓ select · Enter stage/run · a approve · x abort · m mode · q quit",
    ));
    Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Overview"))
        .wrap(Wrap { trim: false })
}

/// Workflows: target picker (works), workflow list, mode, the staged
/// call preview, and the run state — the plan-mode contract surface.
pub fn workflows_panel(state: &WorkflowState) -> List<'static> {
    let mut items: Vec<ListItem> = Vec::new();
    items.push(ListItem::new(format!(
        "mode: {}  (m toggles)  provider: n/a — neemu routes no model calls",
        state.mode.name()
    )));
    items.push(ListItem::new("— target work —".to_string()));
    if state.works.is_empty() {
        items.push(ListItem::new("  (no works scanned)".to_string()));
    }
    for (idx, work) in state.works.iter().enumerate() {
        let marker = if idx == state.selected_work { ">" } else { " " };
        items.push(ListItem::new(format!(
            "{marker} {} [{}] {}",
            work.get("work_id").and_then(Value::as_str).unwrap_or("?"),
            work.get("determination")
                .and_then(Value::as_str)
                .unwrap_or("?"),
            work.get("title").and_then(Value::as_str).unwrap_or(""),
        )));
    }
    items.push(ListItem::new("— workflow —".to_string()));
    for (idx, (label, tool)) in WORKFLOWS.iter().enumerate() {
        let marker = if idx == state.selected_workflow {
            ">"
        } else {
            " "
        };
        items.push(ListItem::new(format!("{marker} {label} ({tool})")));
    }
    items.push(ListItem::new(format!(
        "run state: {}",
        match &state.run_state {
            RunState::Idle => "idle".to_string(),
            RunState::Staged => "staged — waiting for approval".to_string(),
            RunState::Running => "running".to_string(),
            RunState::Done => "done".to_string(),
            RunState::Failed(e) => format!("FAILED — {}", truncate_chars(e, RECORD_LINE_CHARS_MAX)),
            RunState::Unknown(e) =>
                format!("UNKNOWN — {}", truncate_chars(e, RECORD_LINE_CHARS_MAX)),
        }
    )));
    if let Some(staged) = &state.staged {
        items.push(ListItem::new(format!(
            "staged: {} {}",
            staged.tool,
            truncate_chars(&staged.args.to_string(), ARGS_PREVIEW_CHARS_MAX)
        )));
        items.push(ListItem::new(
            "press a to approve and execute, x to abort (nothing has been called yet)".to_string(),
        ));
    }
    List::new(items).block(Block::default().borders(Borders::ALL).title("Workflows"))
}

/// Review: classified extraction lists for the selected work.
pub fn review_panel(state: &ReviewState) -> Paragraph<'static> {
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(format!(
        "work: {}",
        state.work_id.as_deref().unwrap_or("(none selected)")
    )));
    if let Some(error) = &state.error {
        lines.push(Line::from(format!(
            "records unavailable: {}",
            truncate_chars(error, RECORD_LINE_CHARS_MAX)
        )));
    }
    let section = |lines: &mut Vec<Line>, title: &str, items: &[String]| {
        lines.push(Line::from(format!("— {title} ({}) —", items.len())));
        if items.is_empty() {
            lines.push(Line::from("  (none)"));
        }
        for item in items.iter().take(50) {
            lines.push(Line::from(format!("  {item}")));
        }
    };
    section(&mut lines, "faces", &state.faces);
    section(&mut lines, "dialogue", &state.dialogue);
    section(&mut lines, "acts", &state.acts);
    section(&mut lines, "torori voice cues (data rows)", &state.cues);
    lines.push(Line::from(
        "select a face with ↑/↓ on Workflows' work, then open Media (Tab) for its frame",
    ));
    Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Review"))
        .wrap(Wrap { trim: false })
}

/// Media panel header: the protocol+path label is always the first
/// line (DESIGN D5 labelling); the grid itself is drawn into the
/// buffer by the app because half-block cells are per-cell colours.
pub fn media_header(
    label: &str,
    kitty_bytes: Option<usize>,
    error: Option<&str>,
) -> Paragraph<'static> {
    let mut lines = vec![Line::from(truncate_chars(label, RECORD_LINE_CHARS_MAX))];
    if let Some(bytes) = kitty_bytes {
        lines.push(Line::from(format!(
            "kitty passthrough: {bytes} bytes sent to the terminal by the front end; \
             the TUI buffer shows this label only"
        )));
    }
    if let Some(error) = error {
        lines.push(Line::from(format!(
            "media error: {}",
            truncate_chars(error, RECORD_LINE_CHARS_MAX)
        )));
    }
    lines.push(Line::from(
        "viewing is read-only — the file is never rewritten",
    ));
    Paragraph::new(lines).wrap(Wrap { trim: false })
}

/// Log panel: bounded tail of the shared event stream.
pub fn log_panel(lines: &[String]) -> List<'static> {
    let tail: Vec<ListItem> = lines
        .iter()
        .rev()
        .take(LOG_TAIL_LINES_MAX)
        .rev()
        .map(|line| ListItem::new(line.clone()))
        .collect();
    List::new(tail).block(Block::default().borders(Borders::ALL).title("Log / Live"))
}

/// Fleet panel: declared rules with enable state, pending
/// candidates, and the audit tail. Nothing fires that is not listed.
pub fn fleet_panel(
    rules: &[Rule],
    candidates: &[Candidate],
    audit: &[String],
    status_note: &str,
) -> Paragraph<'static> {
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(format!("fleet note: {status_note}")));
    lines.push(Line::from("— trigger rules (declared in triggers.toml) —"));
    if rules.is_empty() {
        lines.push(Line::from("  (no rules declared)"));
    }
    for rule in rules.iter().take(RULES_SHOWN_MAX) {
        lines.push(Line::from(format!(
            "  [{}] {} on {} → {}.{} mode={} lane={} label={}",
            if rule.enabled { "on" } else { "off" },
            rule.id,
            rule.on_event,
            rule.target_mcp,
            rule.target_tool,
            rule.mode,
            rule.origin_lane,
            rule.content_label,
        )));
        if !rule.note.is_empty() {
            lines.push(Line::from(format!(
                "      note: {}",
                truncate_chars(&rule.note, RECORD_LINE_CHARS_MAX)
            )));
        }
    }
    lines.push(Line::from("— handoff candidates —"));
    if candidates.is_empty() {
        lines.push(Line::from("  (none)"));
    }
    for candidate in candidates.iter().take(20) {
        lines.push(Line::from(format!(
            "  {} → {}.{} mode={} approved={} key={}…",
            candidate.rule_id,
            candidate.target_mcp,
            candidate.target_tool,
            candidate.mode,
            candidate.approved,
            &candidate.key[..candidate.key.len().min(12)]
        )));
    }
    lines.push(Line::from(
        "live firing: NOT wired in rung 1 — candidates are plan-mode records, \
         approval is audited, no torori call is placed",
    ));
    lines.push(Line::from("— trigger audit (tail) —"));
    for line in audit.iter().rev().take(AUDIT_TAIL_LINES_MAX).rev() {
        lines.push(Line::from(format!(
            "  {}",
            truncate_chars(line, RECORD_LINE_CHARS_MAX)
        )));
    }
    Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Fleet / MCP"))
        .wrap(Wrap { trim: false })
}

/// Mode is re-exported for panel callers that switch on it.
pub fn mode_label(mode: Mode) -> &'static str {
    mode.name()
}
