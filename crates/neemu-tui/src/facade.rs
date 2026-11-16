//! The narrow facade the UI programs against (DESIGN D2).
//!
//! Three implementations:
//! - [`LibFacade`] — the real one for rung 1: drives the `neemu`
//!   library crate in-process (`Neemu::open` + `tools::call_tool`),
//!   which is exactly the dispatch the MCP server itself serves.
//! - [`StdioFacade`] — the MCP stdio client fallback: spawns the
//!   member's own server binary and speaks the modern-era protocol
//!   (`server/discover`, `tools/list`, `tools/call`). Server death
//!   surfaces as a typed [`FacadeError::Unavailable`], never a hang
//!   disguised as a running call.
//! - [`FakeFacade`] — in-memory, scripted, records every call; used
//!   by every test.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{json, Value};

use crate::{RECORDS_MAX, WORKS_MAX};

/// Typed facade failures. `Unavailable` means the server side cannot
/// be reached or died; the UI renders that as unknown/failed, never
/// as a stuck "running".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FacadeError {
    /// The server (or its process) is gone or unreachable.
    Unavailable(String),
    /// The server answered with a typed refusal/error payload.
    Server(String),
    /// The peer violated the protocol shape.
    Protocol(String),
    /// Local I/O failed before any server contact.
    Io(String),
}

impl std::fmt::Display for FacadeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FacadeError::Unavailable(m) => write!(f, "unavailable: {m}"),
            FacadeError::Server(m) => write!(f, "server error: {m}"),
            FacadeError::Protocol(m) => write!(f, "protocol error: {m}"),
            FacadeError::Io(m) => write!(f, "io error: {m}"),
        }
    }
}

impl std::error::Error for FacadeError {}

/// The operations the screens need. All results are JSON values in
/// the server's own shapes; the UI validates before it renders.
pub trait NeemuFacade {
    /// Member name/version, registry root, work counts by
    /// determination, tool count.
    fn status(&mut self) -> Result<Value, FacadeError>;
    /// The member's own MCP tool definitions (`tools/list` payload).
    fn tools(&mut self) -> Result<Value, FacadeError>;
    /// Bounded registry scan: one summary object per work.
    fn works(&mut self) -> Result<Value, FacadeError>;
    /// All stored records for one work (bounded by `RECORDS_MAX`).
    fn records(&mut self, work_id: &str) -> Result<Value, FacadeError>;
    /// Dispatch one tool call by name with exact arguments.
    fn call(&mut self, tool: &str, args: &Value) -> Result<Value, FacadeError>;
}

/// Summarise one work record JSON into the Overview/Workflows shape.
/// Defensive: unknown or missing fields stay visibly unknown.
fn summarise_work(work: &Value) -> Value {
    let determination = work
        .get("determination")
        .and_then(Value::as_str)
        .or_else(|| work.get("screen_determination").and_then(Value::as_str))
        .unwrap_or("unknown");
    json!({
        "work_id": work.get("work_id").cloned().unwrap_or(Value::Null),
        "title": work.get("title").cloned().unwrap_or(Value::Null),
        "determination": determination,
        "tier": work.get("tier").cloned().unwrap_or(Value::Null),
    })
}

/// Count tools in a definitions payload, accepting both shapes in
/// play: the lib's bare array and the wire's `{tools: [...]}`.
pub fn count_tools(definitions: &Value) -> usize {
    if let Some(array) = definitions.as_array() {
        return array.len();
    }
    definitions
        .get("tools")
        .and_then(Value::as_array)
        .map_or(0, Vec::len)
}

/// Count works by determination from a works array.
pub fn count_by_determination(works: &[Value]) -> Value {
    let mut cleared = 0usize;
    let mut excluded = 0usize;
    let mut other = 0usize;
    for work in works {
        match work.get("determination").and_then(Value::as_str) {
            Some("cleared") => cleared += 1,
            Some("excluded") => excluded += 1,
            _ => other += 1,
        }
    }
    json!({ "total": works.len(), "cleared": cleared, "excluded": excluded, "other": other })
}

// ---------------------------------------------------------------- LibFacade

/// Real facade over the `neemu` library crate.
pub struct LibFacade {
    neemu: neemu::Neemu,
}

impl LibFacade {
    /// Open (creating if needed) the registry at `root`, mirroring
    /// the server binary's own home resolution done by the caller.
    pub fn open(root: PathBuf) -> Result<LibFacade, FacadeError> {
        let neemu = neemu::Neemu::open(root).map_err(|e| FacadeError::Io(e.to_string()))?;
        Ok(LibFacade { neemu })
    }

    fn scan_works(&self) -> Vec<Value> {
        let works_dir = self.neemu.root().join("works");
        let mut summaries = Vec::new();
        let Ok(entries) = std::fs::read_dir(&works_dir) else {
            return summaries;
        };
        let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        paths.sort();
        for path in paths {
            if summaries.len() >= WORKS_MAX {
                break;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(&path) {
                if let Ok(work) = serde_json::from_str::<Value>(&text) {
                    summaries.push(summarise_work(&work));
                }
            }
        }
        summaries
    }
}

impl NeemuFacade for LibFacade {
    fn status(&mut self) -> Result<Value, FacadeError> {
        let works = self.scan_works();
        let counts = count_by_determination(&works);
        let tool_count = count_tools(&neemu::tools::tool_definitions());
        Ok(json!({
            "member": "neemu",
            "version": env!("CARGO_PKG_VERSION"),
            "registry_root": self.neemu.root().display().to_string(),
            "transport": "lib (in-process, server dispatch)",
            "tool_count": tool_count,
            "works": counts,
        }))
    }

    fn tools(&mut self) -> Result<Value, FacadeError> {
        Ok(neemu::tools::tool_definitions())
    }

    fn works(&mut self) -> Result<Value, FacadeError> {
        Ok(Value::Array(self.scan_works()))
    }

    fn records(&mut self, work_id: &str) -> Result<Value, FacadeError> {
        let mut all = self.neemu.all_records(work_id);
        all.truncate(RECORDS_MAX);
        Ok(Value::Array(all))
    }

    fn call(&mut self, tool: &str, args: &Value) -> Result<Value, FacadeError> {
        neemu::tools::call_tool(&self.neemu, tool, args).map_err(|e| match e {
            neemu::CallError::Gate(tool_error) => {
                FacadeError::Server(tool_error.to_json().to_string())
            }
            neemu::CallError::Args(message) => FacadeError::Server(message),
        })
    }
}

// -------------------------------------------------------------- StdioFacade

/// MCP stdio client fallback: spawns the member's server binary.
/// One request in flight at a time; a closed pipe (server death) is
/// a typed `Unavailable`, which is what keeps the queue honest.
pub struct StdioFacade {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

/// Maximum bytes in one stdio frame line.
pub const FRAME_BYTES_MAX: usize = 1024 * 1024;

fn modern_meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": {"name": "neemu-tui", "version": env!("CARGO_PKG_VERSION")},
        "io.modelcontextprotocol/clientCapabilities": {},
    })
}

impl StdioFacade {
    /// Spawn `bin` with `--home <home>` and run `server/discover`.
    /// The console requests the server's `full` tool profile (its
    /// default `core` profile serves only the spine tools); the
    /// profile is the server's own scoping, named in `status`.
    pub fn spawn(bin: &str, home: &std::path::Path) -> Result<StdioFacade, FacadeError> {
        let mut child = Command::new(bin)
            .arg("--home")
            .arg(home)
            .arg("--profile")
            .arg("full")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| FacadeError::Unavailable(format!("spawn {bin}: {e}")))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| FacadeError::Protocol("child stdin missing".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| FacadeError::Protocol("child stdout missing".to_string()))?;
        let mut facade = StdioFacade {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            next_id: 0,
        };
        facade.request("server/discover", json!({}))?;
        Ok(facade)
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value, FacadeError> {
        self.next_id += 1;
        let id = self.next_id;
        let mut params = params;
        if let Value::Object(map) = &mut params {
            map.insert("_meta".to_string(), modern_meta());
        }
        let frame = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        let line = frame.to_string();
        self.stdin
            .write_all(line.as_bytes())
            .and_then(|()| self.stdin.write_all(b"\n"))
            .and_then(|()| self.stdin.flush())
            .map_err(|e| FacadeError::Unavailable(format!("write to server: {e}")))?;
        loop {
            let mut buf = String::new();
            let read = self
                .stdout
                .read_line(&mut buf)
                .map_err(|e| FacadeError::Unavailable(format!("read from server: {e}")))?;
            if read == 0 {
                return Err(FacadeError::Unavailable(
                    "server closed the connection (died or exited)".to_string(),
                ));
            }
            if buf.len() > FRAME_BYTES_MAX {
                return Err(FacadeError::Protocol("oversized frame".to_string()));
            }
            let Ok(value) = serde_json::from_str::<Value>(buf.trim()) else {
                continue; // not a JSON frame; keep waiting for our id
            };
            if value.get("id").and_then(Value::as_u64) != Some(id) {
                continue; // notification or another frame; keep waiting
            }
            if let Some(error) = value.get("error") {
                return Err(FacadeError::Server(error.to_string()));
            }
            return value
                .get("result")
                .cloned()
                .ok_or_else(|| FacadeError::Protocol("response without result".to_string()));
        }
    }
}

impl NeemuFacade for StdioFacade {
    fn status(&mut self) -> Result<Value, FacadeError> {
        let tools = self.tools()?;
        let count = count_tools(&tools);
        Ok(json!({
            "member": "neemu",
            "version": env!("CARGO_PKG_VERSION"),
            "transport": "mcp-stdio (spawned server, profile=full)",
            "tool_count": count,
            "works": {"total": 0, "cleared": 0, "excluded": 0, "other": 0,
                      "note": "work counts come from the lib facade; stdio status reports tools only"},
        }))
    }

    fn tools(&mut self) -> Result<Value, FacadeError> {
        self.request("tools/list", json!({}))
    }

    fn works(&mut self) -> Result<Value, FacadeError> {
        Err(FacadeError::Protocol(
            "works scan needs the lib facade (the server has no list-works tool)".to_string(),
        ))
    }

    fn records(&mut self, work_id: &str) -> Result<Value, FacadeError> {
        let result = self.call(
            "neemu_list_records",
            &json!({"work_id": work_id, "limit": 100}),
        )?;
        Ok(result
            .get("records")
            .cloned()
            .unwrap_or(Value::Array(vec![])))
    }

    fn call(&mut self, tool: &str, args: &Value) -> Result<Value, FacadeError> {
        let result = self.request(
            "tools/call",
            json!({"name": tool, "arguments": args.clone()}),
        )?;
        // MCP CallToolResult wraps text content; unwrap one text block
        // carrying JSON when present, else return the raw result.
        if let Some(content) = result.get("content").and_then(Value::as_array) {
            if let Some(first) = content.first() {
                if let Some(text) = first.get("text").and_then(Value::as_str) {
                    if let Ok(parsed) = serde_json::from_str::<Value>(text) {
                        if result.get("isError").and_then(Value::as_bool) == Some(true) {
                            return Err(FacadeError::Server(text.to_string()));
                        }
                        return Ok(parsed);
                    }
                }
            }
        }
        Ok(result)
    }
}

impl Drop for StdioFacade {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// --------------------------------------------------------------- FakeFacade

/// In-memory scripted facade for tests. Records every call so plan
/// mode can *prove* no call happened before approval.
#[derive(Default)]
pub struct FakeFacade {
    /// Value returned by `status` (or an error when `status_error`).
    pub status_value: Value,
    /// When set, `status` returns this error instead.
    pub status_error: Option<FacadeError>,
    /// Value returned by `tools`.
    pub tools_value: Value,
    /// Value returned by `works`.
    pub works_value: Value,
    /// Value returned by `records`.
    pub records_value: Value,
    /// Scripted results for `call`, consumed in order; when the
    /// script runs out, calls succeed with `{"ok": true}`.
    pub call_script: VecDeque<Result<Value, FacadeError>>,
    /// Every `(tool, args)` passed to `call`, in order.
    pub calls: Vec<(String, Value)>,
    /// Shared mirror of `calls` so a test can keep a handle on the
    /// log after the fake is boxed into the app (the direct
    /// plan-mode no-call proof reads this).
    pub calls_log: std::sync::Arc<std::sync::Mutex<Vec<(String, Value)>>>,
}

impl FakeFacade {
    /// A fake preloaded with a small, well-formed registry picture.
    pub fn seeded() -> FakeFacade {
        FakeFacade {
            status_value: json!({
                "member": "neemu", "version": "1.0.1", "transport": "fake",
                "tool_count": 15,
                "works": {"total": 2, "cleared": 1, "excluded": 1, "other": 0},
            }),
            tools_value: neemu::tools::tool_definitions(),
            works_value: json!([
                {"work_id": "work-a", "title": "Alpha", "determination": "cleared", "tier": "T"},
                {"work_id": "work-b", "title": "Beta", "determination": "excluded", "tier": "A"},
            ]),
            records_value: json!([
                {"record_id": "rec-face-1", "record_kind": "face_observation",
                 "character_slot": "lead", "page": 3},
                {"record_id": "rec-line-1", "record_kind": "dialogue_line",
                 "speaker": "lead", "text": "We leave at dawn."},
                {"record_id": "rec-act-1", "record_kind": "act_record",
                 "act": "departure"},
                {"record_id": "rec-cue-1", "record_kind": "torori_cue",
                 "origin_lane": "ntranch", "content_label": "nsfw"},
            ]),
            ..FakeFacade::default()
        }
    }
}

impl NeemuFacade for FakeFacade {
    fn status(&mut self) -> Result<Value, FacadeError> {
        if let Some(error) = &self.status_error {
            return Err(error.clone());
        }
        Ok(self.status_value.clone())
    }

    fn tools(&mut self) -> Result<Value, FacadeError> {
        Ok(self.tools_value.clone())
    }

    fn works(&mut self) -> Result<Value, FacadeError> {
        Ok(self.works_value.clone())
    }

    fn records(&mut self, _work_id: &str) -> Result<Value, FacadeError> {
        Ok(self.records_value.clone())
    }

    fn call(&mut self, tool: &str, args: &Value) -> Result<Value, FacadeError> {
        self.calls.push((tool.to_string(), args.clone()));
        if let Ok(mut log) = self.calls_log.lock() {
            log.push((tool.to_string(), args.clone()));
        }
        self.call_script
            .pop_front()
            .unwrap_or_else(|| Ok(json!({"ok": true})))
    }
}
