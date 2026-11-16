//! The application state machine and the shared execute path.
//!
//! Both front ends (full-screen ratatui, line mode) drive this same
//! `App`: staging, approval, execution, event logging, and trigger
//! evaluation live here exactly once. Execution is synchronous over
//! the facade; `RunState::Running` exists only while a call is on
//! the stack, and every failure path lands in `Failed`/`Unknown` —
//! a dead server can never leave the UI showing a stuck "running".

use serde_json::{json, Value};

use crate::events::{summarise_response, EventLog, RunEvent};
use crate::facade::{FacadeError, NeemuFacade};
use crate::media::{view_frame_with, Protocol};
use crate::screens::{
    classify_records, MediaState, Mode, ReviewState, RunState, Screen, StagedCall, WorkflowState,
    WORKFLOWS,
};
use crate::triggers::{default_rules, Engine};

/// The whole TUI state over one facade.
pub struct App {
    facade: Box<dyn NeemuFacade>,
    /// Active screen.
    pub screen: Screen,
    /// Workflow-screen state.
    pub workflows: WorkflowState,
    /// Review-screen state.
    pub review: ReviewState,
    /// Media-screen state.
    pub media: MediaState,
    /// Shared event stream (Log screen + trigger engine input).
    pub log: EventLog,
    /// Trigger engine (rules, candidates, audit).
    pub engine: Engine,
    /// Last status result, cached for the Overview.
    pub status_cache: Result<Value, String>,
    next_run_id: u64,
    /// Set by the `q` key / `quit` command.
    pub should_quit: bool,
}

impl App {
    /// Build the app over a facade and load the default rules.
    pub fn new(facade: Box<dyn NeemuFacade>) -> App {
        App::with_rules(facade, default_rules())
    }

    /// Build with explicit rules (tests, `--triggers` overrides).
    pub fn with_rules(facade: Box<dyn NeemuFacade>, rules: Vec<crate::triggers::Rule>) -> App {
        let mut app = App {
            facade,
            screen: Screen::Overview,
            workflows: WorkflowState::default(),
            review: ReviewState::default(),
            media: MediaState::default(),
            log: EventLog::default(),
            engine: Engine::new(rules),
            status_cache: Err("not loaded yet".to_string()),
            next_run_id: 0,
            should_quit: false,
        };
        app.refresh();
        app
    }

    /// Reload status + works from the facade. A facade failure is
    /// cached as the honest `Err` the Overview renders as UNKNOWN.
    pub fn refresh(&mut self) {
        self.status_cache = self.facade.status().map_err(|e| e.to_string());
        match self.facade.works() {
            Ok(Value::Array(works)) => {
                self.workflows.works = works;
                if self.workflows.selected_work >= self.workflows.works.len() {
                    self.workflows.selected_work = 0;
                }
            }
            Ok(other) => {
                self.log.note(format!(
                    "works scan returned a malformed payload ({}); list left unchanged",
                    type_name(&other)
                ));
            }
            Err(error) => {
                self.log.note(format!("works scan unavailable: {error}"));
            }
        }
    }

    /// Stage the selected workflow for the selected work. In plan
    /// mode this is where execution stops until approval; the exact
    /// tool + args are what the operator sees.
    pub fn stage_selected(&mut self) -> Result<StagedCall, String> {
        let work_id = self
            .workflows
            .selected_work_id()
            .ok_or_else(|| "no work selected (registry scan is empty)".to_string())?;
        let extra: Value = serde_json::from_str(&self.workflows.extra_args_text)
            .map_err(|e| format!("extra args are not valid JSON: {e}"))?;
        if !extra.is_object() {
            return Err("extra args must be a JSON object".to_string());
        }
        let mut args = json!({"work_id": work_id});
        if let (Value::Object(base), Value::Object(more)) = (&mut args, &extra) {
            for (key, value) in more {
                base.insert(key.clone(), value.clone());
            }
        }
        let staged = StagedCall {
            tool: self.workflows.selected_tool().to_string(),
            args,
        };
        self.workflows.staged = Some(staged.clone());
        self.workflows.run_state = RunState::Staged;
        self.log.note(format!(
            "staged {} {} (mode={})",
            staged.tool,
            staged.args,
            self.workflows.mode.name()
        ));
        Ok(staged)
    }

    /// Execute a call through the facade, emitting the run's events
    /// and feeding completions to the trigger engine. This is the
    /// single execute path both modes share.
    pub fn execute(&mut self, staged: StagedCall) -> Result<Value, FacadeError> {
        self.next_run_id += 1;
        let run_id = self.next_run_id;
        self.workflows.run_state = RunState::Running;
        self.log.push(RunEvent::Started {
            run_id,
            tool: staged.tool.clone(),
        });
        let work_id = staged
            .args
            .get("work_id")
            .and_then(Value::as_str)
            .map(str::to_string);
        match self.facade.call(&staged.tool, &staged.args) {
            Ok(response) => {
                if !response.is_object() && !response.is_array() {
                    // Malformed tool result: the call returned, but
                    // not in a shape the UI may present as success
                    // data. Record it as a failed run, honestly.
                    let error = FacadeError::Protocol(format!(
                        "malformed result from {}: {}",
                        staged.tool,
                        type_name(&response)
                    ));
                    self.workflows.run_state = RunState::Failed(error.to_string());
                    self.log.push(RunEvent::Failed {
                        run_id,
                        tool: staged.tool.clone(),
                        error: error.to_string(),
                    });
                    return Err(error);
                }
                self.log.push(RunEvent::Progress {
                    run_id,
                    summary: summarise_response(&staged.tool, &response),
                });
                self.workflows.run_state = RunState::Done;
                self.log.push(RunEvent::Done {
                    run_id,
                    tool: staged.tool.clone(),
                    work_id,
                });
                self.pump_triggers();
                Ok(response)
            }
            Err(error) => {
                self.workflows.run_state = match &error {
                    FacadeError::Unavailable(_) => RunState::Unknown(error.to_string()),
                    _ => RunState::Failed(error.to_string()),
                };
                self.log.push(RunEvent::Failed {
                    run_id,
                    tool: staged.tool.clone(),
                    error: error.to_string(),
                });
                Err(error)
            }
        }
    }

    /// The Enter key / `run` command: plan mode stages, auto mode
    /// stages and executes in one motion.
    pub fn stage_or_run(&mut self) -> Result<Value, String> {
        let staged = self.stage_selected()?;
        if self.workflows.mode == Mode::Plan {
            return Ok(json!({"staged": true, "tool": staged.tool, "args": staged.args}));
        }
        self.workflows.staged = None;
        self.execute(staged).map_err(|e| e.to_string())
    }

    /// Approve the staged call (plan mode). No staged call, no call.
    pub fn approve_staged(&mut self) -> Result<Value, String> {
        let staged = self
            .workflows
            .staged
            .take()
            .ok_or_else(|| "nothing is staged".to_string())?;
        self.log.note(format!("approved {}", staged.tool));
        self.execute(staged).map_err(|e| e.to_string())
    }

    /// Abort the staged call. The facade is never touched.
    pub fn abort_staged(&mut self) {
        if let Some(staged) = self.workflows.staged.take() {
            self.log.note(format!("aborted staged {}", staged.tool));
            self.workflows.run_state = RunState::Idle;
        }
    }

    /// Feed queued trigger events to the engine; log new candidates.
    pub fn pump_triggers(&mut self) {
        for event in self.log.drain_trigger_events() {
            for candidate in self.engine.evaluate(&event) {
                self.log.note(format!(
                    "trigger candidate: {} → {}.{} (mode={}, not fired — rung 1)",
                    candidate.rule_id, candidate.target_mcp, candidate.target_tool, candidate.mode
                ));
            }
        }
    }

    /// Record a human approval of one candidate. Audited; the call
    /// itself is NOT placed (no torori transport in rung 1).
    pub fn approve_candidate(&mut self, index: usize) -> Result<(), String> {
        let candidate = self
            .engine
            .candidates
            .get_mut(index)
            .ok_or_else(|| format!("no candidate at index {index}"))?;
        candidate.approved = true;
        let line = format!(
            "candidate approved by operator: {} → {}.{} — recorded only; \
             torori_bundle_ingest is not called in rung 1 (bundle assembly seam)",
            candidate.rule_id, candidate.target_mcp, candidate.target_tool
        );
        self.log.note(line);
        Ok(())
    }

    /// Load Review records for the selected work.
    pub fn load_review(&mut self) -> Result<(), String> {
        let work_id = self
            .workflows
            .selected_work_id()
            .ok_or_else(|| "no work selected".to_string())?;
        match self.facade.records(&work_id) {
            Ok(Value::Array(records)) => {
                self.review = classify_records(&records);
                self.review.work_id = Some(work_id);
                Ok(())
            }
            Ok(other) => {
                self.review = ReviewState {
                    work_id: Some(work_id),
                    error: Some(format!("malformed records payload ({})", type_name(&other))),
                    ..ReviewState::default()
                };
                Ok(())
            }
            Err(error) => {
                self.review = ReviewState {
                    work_id: Some(work_id),
                    error: Some(error.to_string()),
                    ..ReviewState::default()
                };
                Ok(())
            }
        }
    }

    /// Open the Media screen on a path (from a face record's frame
    /// field or an explicit operator path).
    pub fn open_media(&mut self, path: &str, protocol: Protocol) {
        self.media.path = Some(path.to_string());
        match view_frame_with(std::path::Path::new(path), 64, 24, protocol) {
            Ok(view) => {
                self.media.view = Some(view);
                self.media.error = None;
            }
            Err(error) => {
                self.media.view = None;
                self.media.error = Some(error.to_string());
            }
        }
        self.screen = Screen::Media;
    }

    /// Open Media on the selected face's frame, when it names one.
    pub fn open_selected_face_media(&mut self, protocol: Protocol) {
        match self
            .review
            .face_frames
            .get(self.review.selected_face)
            .cloned()
            .flatten()
        {
            Some(path) => self.open_media(&path, protocol),
            None => {
                self.media.path = None;
                self.media.view = None;
                self.media.error = Some(
                    "selected face record names no frame file (frame_path absent)".to_string(),
                );
                self.screen = Screen::Media;
            }
        }
    }

    /// Handle one key in the full-screen front end. Pure state
    /// transitions plus the shared execute path — testable headless.
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent, protocol: Protocol) {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Tab => self.screen = self.screen.next(),
            KeyCode::Char(c) if ('1'..='6').contains(&c) => {
                let idx = (c as usize) - ('1' as usize);
                self.screen = Screen::ALL[idx];
                if self.screen == Screen::Review {
                    let _ = self.load_review();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::Char('m') => {
                self.workflows.mode = match self.workflows.mode {
                    Mode::Plan => Mode::Auto,
                    Mode::Auto => Mode::Plan,
                };
                self.log
                    .note(format!("mode → {}", self.workflows.mode.name()));
            }
            KeyCode::Enter => {
                if self.screen == Screen::Workflows {
                    let _ = self.stage_or_run();
                } else if self.screen == Screen::Review {
                    self.open_selected_face_media(protocol);
                }
            }
            KeyCode::Char('a') => {
                let _ = self.approve_staged();
            }
            KeyCode::Char('x') => self.abort_staged(),
            KeyCode::Char('r') => self.refresh(),
            _ => {}
        }
    }

    fn move_selection(&mut self, delta: i32) {
        let shift = |idx: usize, len: usize| -> usize {
            if len == 0 {
                return 0;
            }
            (idx as i64 + i64::from(delta)).rem_euclid(len as i64) as usize
        };
        match self.screen {
            Screen::Workflows => {
                // One step moves both pickers (work and workflow) —
                // simple and predictable; line mode offers exact
                // index selection for precision.
                self.workflows.selected_work =
                    shift(self.workflows.selected_work, self.workflows.works.len());
                self.workflows.selected_workflow =
                    shift(self.workflows.selected_workflow, WORKFLOWS.len());
            }
            Screen::Review => {
                self.review.selected_face =
                    shift(self.review.selected_face, self.review.faces.len());
            }
            _ => {}
        }
    }
}

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facade::{FacadeError, FakeFacade};
    use serde_json::json;

    fn app_with(fake: FakeFacade) -> App {
        App::new(Box::new(fake))
    }

    #[test]
    fn plan_mode_stages_without_calling() {
        // Direct proof: keep the fake's shared call log, box the
        // fake into the app, stage + abort — the log must stay empty.
        let fake = FakeFacade::seeded();
        let calls_log = std::sync::Arc::clone(&fake.calls_log);
        let mut app = app_with(fake);
        let staged = app.stage_selected().expect("stage");
        assert_eq!(staged.tool, "neemu_extract_faces");
        assert_eq!(staged.args["work_id"], "work-a");
        assert_eq!(app.workflows.run_state, RunState::Staged);
        assert!(calls_log.lock().expect("log lock").is_empty());
        app.abort_staged();
        assert_eq!(app.workflows.run_state, RunState::Idle);
        assert!(calls_log.lock().expect("log lock").is_empty());
    }

    #[test]
    fn approve_executes_exactly_once() {
        let fake = FakeFacade::seeded();
        let calls_log = std::sync::Arc::clone(&fake.calls_log);
        let mut app = app_with(fake);
        let _ = app.stage_or_run().expect("stage");
        assert!(calls_log.lock().expect("log lock").is_empty());
        let result = app.approve_staged().expect("approve executes");
        assert_eq!(result["ok"], true);
        assert_eq!(app.workflows.run_state, RunState::Done);
        let calls = calls_log.lock().expect("log lock");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "neemu_extract_faces");
        assert_eq!(calls[0].1["work_id"], "work-a");
    }

    #[test]
    fn auto_mode_executes_without_approval() {
        let mut app = app_with(FakeFacade::seeded());
        app.workflows.mode = Mode::Auto;
        let result = app.stage_or_run().expect("auto runs");
        assert_eq!(result["ok"], true);
        assert_eq!(app.workflows.run_state, RunState::Done);
    }

    #[test]
    fn server_death_mid_run_shows_failed_never_stuck_running() {
        let mut fake = FakeFacade::seeded();
        fake.call_script
            .push_back(Err(FacadeError::Unavailable("server died".to_string())));
        let mut app = app_with(fake);
        app.workflows.mode = Mode::Auto;
        let result = app.stage_or_run();
        assert!(result.is_err());
        assert!(matches!(app.workflows.run_state, RunState::Unknown(_)));
        assert!(app
            .log
            .lines()
            .iter()
            .any(|l| l.contains("FAILED neemu_extract_faces")));
    }

    #[test]
    fn malformed_result_is_a_failed_run_not_success() {
        let mut fake = FakeFacade::seeded();
        fake.call_script.push_back(Ok(json!("not an object")));
        let mut app = app_with(fake);
        app.workflows.mode = Mode::Auto;
        assert!(app.stage_or_run().is_err());
        assert!(matches!(app.workflows.run_state, RunState::Failed(_)));
    }

    #[test]
    fn status_failure_renders_unknown_not_running() {
        let mut fake = FakeFacade::seeded();
        fake.status_error = Some(FacadeError::Unavailable("gone".to_string()));
        let app = app_with(fake);
        assert!(app.status_cache.is_err());
    }

    #[test]
    fn cue_completion_produces_torori_candidate() {
        let mut fake = FakeFacade::seeded();
        fake.call_script.push_back(Ok(
            json!({"record_id": "rec-cue-9", "record_kind": "torori_cue"}),
        ));
        let mut app = app_with(fake);
        app.workflows.selected_workflow = 6; // Emit torori cue
        app.workflows.mode = Mode::Auto;
        let _ = app.stage_or_run().expect("cue emit runs");
        assert_eq!(app.engine.candidates.len(), 1);
        assert_eq!(app.engine.candidates[0].target_tool, "torori_bundle_ingest");
        assert!(!app.engine.candidates[0].approved);
        app.approve_candidate(0).expect("approve records");
        assert!(app.engine.candidates[0].approved);
    }

    #[test]
    fn review_classifies_seeded_records() {
        let mut app = app_with(FakeFacade::seeded());
        app.load_review().expect("review loads");
        assert_eq!(app.review.faces.len(), 1);
        assert_eq!(app.review.dialogue.len(), 1);
        assert_eq!(app.review.acts.len(), 1);
        assert_eq!(app.review.cues.len(), 1);
    }

    #[test]
    fn malformed_records_payload_is_an_error_row_not_a_panic() {
        let mut fake = FakeFacade::seeded();
        fake.records_value = json!({"unexpected": true});
        let mut app = app_with(fake);
        app.load_review().expect("handled");
        assert!(app.review.error.is_some());
        assert!(app.review.faces.is_empty());
    }
}
