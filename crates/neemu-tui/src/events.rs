//! The typed run-event stream. The Log screen renders it and the
//! trigger engine consumes the same values — one stream, two readers
//! (DESIGN D2/D4). Progress entries carry fields copied from the
//! server's own response (record counts, `partial` continuations);
//! the TUI never invents percentages.

use std::collections::VecDeque;

use serde_json::Value;

use crate::{truncate_chars, LOG_ENTRIES_MAX, LOG_LINE_CHARS_MAX};

/// One event on a workflow run.
#[derive(Debug, Clone, PartialEq)]
pub enum RunEvent {
    /// A staged call started executing.
    Started {
        /// Monotonic run id within this TUI session.
        run_id: u64,
        /// MCP tool name being called.
        tool: String,
    },
    /// The server answered an intermediate step; `detail` is a
    /// summary built only from fields of that response.
    Progress {
        /// Run this progress belongs to.
        run_id: u64,
        /// Human-readable, server-derived summary line.
        summary: String,
    },
    /// The run completed; carries the trigger-layer event fields.
    Done {
        /// Run id.
        run_id: u64,
        /// MCP tool that completed.
        tool: String,
        /// Work the run operated on, when known.
        work_id: Option<String>,
    },
    /// The run failed; `error` is the typed facade error text.
    Failed {
        /// Run id.
        run_id: u64,
        /// MCP tool that failed.
        tool: String,
        /// Error text.
        error: String,
    },
    /// A free-form log line (staging, approvals, trigger notes).
    Note(String),
}

impl RunEvent {
    /// Render one bounded log line.
    pub fn line(&self) -> String {
        let raw = match self {
            RunEvent::Started { run_id, tool } => format!("[run {run_id}] started {tool}"),
            RunEvent::Progress { run_id, summary } => {
                format!("[run {run_id}] progress: {summary}")
            }
            RunEvent::Done { run_id, tool, .. } => format!("[run {run_id}] done {tool}"),
            RunEvent::Failed {
                run_id,
                tool,
                error,
            } => {
                format!("[run {run_id}] FAILED {tool}: {error}")
            }
            RunEvent::Note(text) => text.clone(),
        };
        truncate_chars(&raw, LOG_LINE_CHARS_MAX)
    }
}

/// A typed event for the trigger engine (DESIGN D4): a name plus
/// named fields; predicates read only these fields.
#[derive(Debug, Clone, PartialEq)]
pub struct TriggerEvent {
    /// Event name, e.g. `extraction.completed`.
    pub name: String,
    /// Named fields the rule predicates may read.
    pub fields: Value,
    /// Trigger depth of the call that produced this event (0 = a
    /// human-started run). Rules never fire beyond their max_depth.
    pub depth: u32,
}

/// Bounded event log: rendered lines plus the trigger-event queue.
#[derive(Debug, Default)]
pub struct EventLog {
    lines: VecDeque<String>,
    pending_trigger_events: VecDeque<TriggerEvent>,
}

impl EventLog {
    /// Append one run event; `Done` also enqueues the matching
    /// trigger event (`extraction.completed`).
    pub fn push(&mut self, event: RunEvent) {
        if let RunEvent::Done { tool, work_id, .. } = &event {
            self.pending_trigger_events.push_back(TriggerEvent {
                name: "extraction.completed".to_string(),
                fields: serde_json::json!({
                    "tool": tool,
                    "work_id": work_id,
                }),
                depth: 0,
            });
        }
        self.lines.push_back(event.line());
        while self.lines.len() > LOG_ENTRIES_MAX {
            self.lines.pop_front();
        }
    }

    /// Append a plain note line.
    pub fn note(&mut self, text: impl Into<String>) {
        self.lines
            .push_back(truncate_chars(&text.into(), LOG_LINE_CHARS_MAX));
        while self.lines.len() > LOG_ENTRIES_MAX {
            self.lines.pop_front();
        }
    }

    /// All retained lines, oldest first.
    pub fn lines(&self) -> Vec<String> {
        self.lines.iter().cloned().collect()
    }

    /// Drain queued trigger events for the engine.
    pub fn drain_trigger_events(&mut self) -> Vec<TriggerEvent> {
        self.pending_trigger_events.drain(..).collect()
    }
}

/// Summarise a server response using only its own fields: array
/// lengths per key, the `partial` continuation flag, and a minted
/// `record_id`/`bundle_id` when present. This is the "progress from
/// the server's own reporting" the Log screen shows.
pub fn summarise_response(tool: &str, response: &Value) -> String {
    let mut parts = vec![format!("{tool} answered")];
    if let Value::Object(map) = response {
        let mut keys: Vec<&String> = map.keys().collect();
        keys.sort();
        for key in keys {
            if let Value::Array(items) = &map[key] {
                parts.push(format!("{key}={}", items.len()));
            }
        }
        if let Some(total) = map.get("total") {
            parts.push(format!("total={total}"));
        }
        if map.get("partial").and_then(Value::as_bool) == Some(true) {
            parts.push("partial=true (continuation available)".to_string());
        }
        for id_key in ["record_id", "bundle_id", "cue_id"] {
            if let Some(id) = map.get(id_key).and_then(Value::as_str) {
                parts.push(format!("{id_key}={id}"));
            }
        }
    } else {
        parts.push("(non-object response)".to_string());
    }
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn summary_uses_only_response_fields() {
        let text = summarise_response(
            "neemu_extract_faces",
            &json!({"records": [1, 2, 3], "partial": true, "record_id": "r-1"}),
        );
        assert!(text.contains("records=3"), "{text}");
        assert!(text.contains("partial=true"), "{text}");
        assert!(text.contains("record_id=r-1"), "{text}");
    }

    #[test]
    fn done_enqueues_extraction_completed() {
        let mut log = EventLog::default();
        log.push(RunEvent::Done {
            run_id: 1,
            tool: "neemu_extract_faces".to_string(),
            work_id: Some("work-a".to_string()),
        });
        let events = log.drain_trigger_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].name, "extraction.completed");
        assert_eq!(events[0].fields["tool"], "neemu_extract_faces");
    }

    #[test]
    fn log_is_bounded() {
        let mut log = EventLog::default();
        for i in 0..(LOG_ENTRIES_MAX + 50) {
            log.note(format!("line {i}"));
        }
        assert_eq!(log.lines().len(), LOG_ENTRIES_MAX);
    }
}
