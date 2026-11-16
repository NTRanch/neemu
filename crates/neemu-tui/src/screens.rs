//! Screen state types (DESIGN D2). Invalid states are kept
//! unrepresentable: a run is exactly one of idle / staged / running /
//! done / failed / unknown, and the queue view distinguishes a dead
//! server (`Unknown`) from a live one.

use serde_json::Value;

use crate::media::MediaView;

/// The six screens, in tab order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// Member status + extraction queue state.
    Overview,
    /// Screening/extraction runs over a chosen work.
    Workflows,
    /// Extracted faces / dialogue / acts for the selected work.
    Review,
    /// Face-frame viewer (DESIGN D5 ladder rung 1).
    Media,
    /// Bounded live event log.
    Log,
    /// Fleet members, trigger rules, candidates, audit tail.
    Fleet,
}

impl Screen {
    /// All screens in tab order.
    pub const ALL: [Screen; 6] = [
        Screen::Overview,
        Screen::Workflows,
        Screen::Review,
        Screen::Media,
        Screen::Log,
        Screen::Fleet,
    ];

    /// Display name.
    pub fn name(self) -> &'static str {
        match self {
            Screen::Overview => "Overview",
            Screen::Workflows => "Workflows",
            Screen::Review => "Review",
            Screen::Media => "Media",
            Screen::Log => "Log",
            Screen::Fleet => "Fleet",
        }
    }

    /// Next screen (wraps).
    pub fn next(self) -> Screen {
        let idx = Screen::ALL.iter().position(|s| *s == self).unwrap_or(0);
        Screen::ALL[(idx + 1) % Screen::ALL.len()]
    }
}

/// Run control mode: plan stages a call for approval; auto executes
/// declared operations directly (DESIGN D2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Stage the exact tool + args; nothing runs before approval.
    Plan,
    /// Execute declared operations directly.
    Auto,
}

impl Mode {
    /// Display name.
    pub fn name(self) -> &'static str {
        match self {
            Mode::Plan => "plan",
            Mode::Auto => "auto",
        }
    }
}

/// State of the current (or last) run. `Unknown` is the honest state
/// when the server cannot be reached — never rendered as running.
#[derive(Debug, Clone, PartialEq)]
pub enum RunState {
    /// No run yet.
    Idle,
    /// A call is staged, waiting for approval (plan mode).
    Staged,
    /// A call is executing (only ever set while `call` is on the
    /// stack; a failure moves it to `Failed`, never leaves it here).
    Running,
    /// The last run completed.
    Done,
    /// The last run failed with this error.
    Failed(String),
    /// Server state could not be determined.
    Unknown(String),
}

/// One staged tool call: exact tool + exact args, shown before any
/// approval can execute it.
#[derive(Debug, Clone, PartialEq)]
pub struct StagedCall {
    /// MCP tool name.
    pub tool: String,
    /// Exact arguments that will be sent on approval.
    pub args: Value,
}

/// The member's workflow list: (label, tool) pairs. These are
/// neemu's own primary operations — screening/extraction over a
/// chosen work, plus the voice-cue emit that feeds the trigger.
pub const WORKFLOWS: [(&str, &str); 7] = [
    ("Screen work", "neemu_screen_work"),
    ("Segment scene", "neemu_segment_scene"),
    ("Extract faces", "neemu_extract_faces"),
    ("Extract dialogue", "neemu_extract_dialogue"),
    ("Extract acts", "neemu_extract_acts"),
    ("Understand scene", "neemu_understand_scene"),
    ("Emit torori cue", "neemu_emit_torori_cue"),
];

/// Workflow-screen state.
#[derive(Debug, Clone)]
pub struct WorkflowState {
    /// Works from the last registry scan (summary objects).
    pub works: Vec<Value>,
    /// Selected work index into `works`.
    pub selected_work: usize,
    /// Selected workflow index into `WORKFLOWS`.
    pub selected_workflow: usize,
    /// Extra args JSON text merged over `{work_id}` at staging time.
    pub extra_args_text: String,
    /// Plan/auto run control.
    pub mode: Mode,
    /// The staged call, when one waits for approval.
    pub staged: Option<StagedCall>,
    /// Current run state.
    pub run_state: RunState,
}

impl Default for WorkflowState {
    fn default() -> Self {
        WorkflowState {
            works: Vec::new(),
            selected_work: 0,
            selected_workflow: 2, // Extract faces: the primary op.
            extra_args_text: "{}".to_string(),
            mode: Mode::Plan,
            staged: None,
            run_state: RunState::Idle,
        }
    }
}

impl WorkflowState {
    /// The selected work's id, when the scan produced one.
    pub fn selected_work_id(&self) -> Option<String> {
        self.works
            .get(self.selected_work)
            .and_then(|w| w.get("work_id"))
            .and_then(Value::as_str)
            .map(str::to_string)
    }

    /// The selected workflow's tool name.
    pub fn selected_tool(&self) -> &'static str {
        WORKFLOWS
            .get(self.selected_workflow)
            .map_or("neemu_extract_faces", |(_, tool)| tool)
    }
}

/// Review-screen state: classified record summaries for one work.
#[derive(Debug, Clone, Default)]
pub struct ReviewState {
    /// Work these records belong to.
    pub work_id: Option<String>,
    /// `face_observation` summaries.
    pub faces: Vec<String>,
    /// `dialogue_line` summaries.
    pub dialogue: Vec<String>,
    /// `act_record` summaries.
    pub acts: Vec<String>,
    /// `torori_cue` summaries.
    pub cues: Vec<String>,
    /// Selected face index (Media opens this face's frame).
    pub selected_face: usize,
    /// Frame paths parallel to `faces` (from record fields, when a
    /// record carries one; `None` = the record names no frame file).
    pub face_frames: Vec<Option<String>>,
    /// Load error, shown instead of lists when the read failed.
    pub error: Option<String>,
}

/// Media-screen state.
#[derive(Debug, Clone, Default)]
pub struct MediaState {
    /// Path being viewed.
    pub path: Option<String>,
    /// The loaded view (label + grid / kitty byte count).
    pub view: Option<MediaView>,
    /// Load/render error, shown honestly instead of a picture.
    pub error: Option<String>,
}

/// Classify stored records into Review lists. Malformed records
/// (non-objects, missing kinds) are skipped and counted, never
/// panicked on — the server owns the shapes, the UI stays honest.
pub fn classify_records(records: &[Value]) -> ReviewState {
    let mut state = ReviewState::default();
    for record in records {
        let Some(kind) = record.get("record_kind").and_then(Value::as_str) else {
            continue;
        };
        let id = record
            .get("record_id")
            .and_then(Value::as_str)
            .unwrap_or("<no id>");
        match kind {
            "face_observation" => {
                let slot = record
                    .get("character_slot")
                    .and_then(Value::as_str)
                    .unwrap_or("?");
                let page = record
                    .get("page")
                    .map_or("?".to_string(), |p| p.to_string());
                state.faces.push(crate::truncate_chars(
                    &format!("{id} slot={slot} page={page}"),
                    crate::RECORD_LINE_CHARS_MAX,
                ));
                let frame = record
                    .get("frame_path")
                    .or_else(|| record.get("image_path"))
                    .and_then(Value::as_str)
                    .map(str::to_string);
                state.face_frames.push(frame);
            }
            "dialogue_line" => {
                let speaker = record.get("speaker").and_then(Value::as_str).unwrap_or("?");
                let text = record.get("text").and_then(Value::as_str).unwrap_or("");
                state.dialogue.push(crate::truncate_chars(
                    &format!("{id} {speaker}: {text}"),
                    crate::RECORD_LINE_CHARS_MAX,
                ));
            }
            "act_record" => {
                let act = record.get("act").and_then(Value::as_str).unwrap_or("?");
                state.acts.push(crate::truncate_chars(
                    &format!("{id} act={act}"),
                    crate::RECORD_LINE_CHARS_MAX,
                ));
            }
            "torori_cue" => {
                state.cues.push(crate::truncate_chars(
                    &format!("{id} (voice cue — data row, never played)"),
                    crate::RECORD_LINE_CHARS_MAX,
                ));
            }
            _ => {}
        }
    }
    state
}
