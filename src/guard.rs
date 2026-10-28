//! Era guard: the byte-level front door of the server.
//!
//! rmcp's service layer enforces the modern era at the message level
//! (discover negotiation, per-request `_meta` validation) once the
//! handler pins `supported_protocol_versions` to 2026-07-28 — but it
//! silently drops lines it cannot parse, and it will still *answer*
//! an `initialize` request (with the unsupported-version refusal)
//! only by failing the whole serve. This guard sits in front of the
//! SDK on stdio and owns the framing contract, reproducing the
//! hand-rolled baseline's exact shapes wherever the baseline had one
//! (measured in the WS2-W5 continuation baseline sessions, not
//! assumed — the fleet template's W1 §3b rule):
//!
//! - malformed JSON → `-32700` with the baseline's own message
//!   shape, `parse error: {serde detail}` (id null)
//! - oversized line → `-32700 "message exceeds size limit"` with
//!   the remainder drained (the fleet 4 MiB cap; the baseline had
//!   no cap — itemized in the gate report; the server stays alive)
//! - `initialize` (any version) → the designed modern-only refusal,
//!   `-32022` naming the supported versions; the connection and the
//!   process stay up, so a probe can continue with modern frames
//! - unknown methods → `-32601 "method not found: {method}"` (the
//!   baseline's exact message); a frame with an `id` but no string
//!   `method` earns the same error with an empty name, exactly as
//!   the baseline answered it. Only `server/discover`, `ping`,
//!   `tools/list`, and `tools/call` frames are forwarded. A
//!   non-object request is answered `-32600` (the fleet framing
//!   contract — the baseline dropped those silently; itemized)
//! - notifications (no `id`) are dropped silently and UNEXECUTED —
//!   the designed fleet behavior. The baseline executed
//!   notification-shaped `tools/list`/`tools/call` frames and
//!   answered them with id null; that delta is itemized in the
//!   gate report (the same class as lupin's W1 exception)
//!
//! Forwarded frames pass through except for shape normalization
//! that reproduces the baseline's handler-level behavior inside the
//! SDK's typed envelope:
//!
//! - a missing/non-object `params` becomes `{}` (the baseline
//!   ignored `params` shape on discover/list/ping entirely, and a
//!   params-less call degraded to an empty name + `{}` arguments)
//! - a missing or non-string tool `name` becomes `""`, so the
//!   handler itself reproduces the baseline's protocol error
//!   `-32602 "unknown tool: "` — the baseline's nameless-call
//!   answer was a protocol error, not an in-band result, so no
//!   guard-side answer is needed (contrast periscope)
//! - a present-but-non-object `arguments` is answered by the guard
//!   with the baseline's exact protocol error, `-32602 "arguments
//!   must be a JSON object"` (the SDK's typed request cannot
//!   represent it, and the baseline's answer was this error)

use std::sync::Arc;

use serde_json::{json, Value};
use tokio::io::{
    AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader, DuplexStream, ReadHalf, Stdin, Stdout,
    WriteHalf,
};
use tokio::sync::Mutex;

/// The one protocol version this server speaks.
pub const MODERN_VERSION: &str = "2026-07-28";
/// Maximum bytes in one JSON-RPC line (the fleet cap).
const MAX_LINE_BYTES: usize = 4 * 1024 * 1024;

const PARSE_ERROR: i32 = -32700;
const INVALID_REQUEST: i32 = -32600;
const METHOD_NOT_FOUND: i32 = -32601;
const INVALID_PARAMS: i32 = -32602;
const UNSUPPORTED_PROTOCOL_VERSION: i32 = -32022;

/// Stdout shared by the guard (its own error frames) and the pump
/// (SDK responses); one line is written per lock, never interleaved.
pub type SharedStdout = Arc<Mutex<Stdout>>;

/// Response-ordering state shared by the guard and the pump: the
/// ids of forwarded requests whose SDK response has not yet been
/// written to stdout. The baseline answered strictly in request
/// order; the guard therefore never writes one of its own frames
/// while an earlier forwarded request is unanswered (without this,
/// a guard frame — answered instantly — overtakes the SDK's first
/// response, which pays the service bootstrap latency).
pub struct Ordering {
    outstanding: Mutex<Vec<Value>>,
    drained: tokio::sync::Notify,
}

impl Ordering {
    pub fn new() -> Arc<Self> {
        Arc::new(Ordering {
            outstanding: Mutex::new(Vec::new()),
            drained: tokio::sync::Notify::new(),
        })
    }

    async fn track(&self, id: Value) {
        self.outstanding.lock().await.push(id);
    }

    async fn complete(&self, id: &Value) {
        {
            let mut queue = self.outstanding.lock().await;
            if let Some(pos) = queue.iter().position(|q| q == id) {
                queue.remove(pos);
            }
        }
        self.drained.notify_waiters();
    }

    /// Wait until no forwarded request is unanswered. Bounded: if
    /// the SDK died mid-request its response will never come, and
    /// the guard must go on answering framing errors regardless.
    async fn wait_drained(&self) {
        let deadline = std::time::Duration::from_secs(5);
        let _ = tokio::time::timeout(deadline, async {
            loop {
                let notified = self.drained.notified();
                if self.outstanding.lock().await.is_empty() {
                    return;
                }
                notified.await;
            }
        })
        .await;
    }
}

fn error_frame(id: Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

/// The designed refusal of legacy-era contact (CHECKLIST 1.8 shape:
/// the SDK's own `unsupported_protocol_version` payload).
fn unsupported_version_frame(id: Value, requested: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": UNSUPPORTED_PROTOCOL_VERSION,
            "message": "Unsupported protocol version",
            "data": {"requested": requested, "supported": [MODERN_VERSION]}
        }
    })
}

/// What the guard does with one inbound line.
#[derive(Debug, PartialEq)]
enum Action {
    /// Forward this (possibly shape-normalized) frame to the SDK.
    Forward(Value),
    /// Answer it here; the SDK never sees it.
    Respond(Value),
    /// Drop silently (blank line, notification).
    Drop,
}

/// Classify one line. Pure: unit-tested without any transport.
fn classify(line: &[u8]) -> Action {
    let text = String::from_utf8_lossy(line);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Action::Drop;
    }
    let value: Value = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(error) => {
            // The baseline's exact shape: the serde detail rides
            // along ("parse error: expected value at line 1 …").
            return Action::Respond(error_frame(
                Value::Null,
                PARSE_ERROR,
                &format!("parse error: {error}"),
            ));
        }
    };
    let Some(object) = value.as_object() else {
        return Action::Respond(error_frame(
            Value::Null,
            INVALID_REQUEST,
            "request must be a JSON object",
        ));
    };
    // No `id` key: a notification. Dropped silently and unexecuted
    // (the designed fleet behavior; the baseline executed these —
    // itemized in the gate report).
    if !object.contains_key("id") {
        return Action::Drop;
    }
    let id = object.get("id").cloned().unwrap_or(Value::Null);
    // No string method: the baseline answered `-32601` with an
    // empty method name (its `unwrap_or("")` dispatch fell through
    // to the unknown-method arm whenever an id was present).
    let Some(method) = object.get("method").and_then(Value::as_str) else {
        return Action::Respond(error_frame(id, METHOD_NOT_FOUND, "method not found: "));
    };
    match method {
        "initialize" => {
            let requested = object
                .get("params")
                .and_then(|p| p.get("protocolVersion"))
                .and_then(Value::as_str)
                .map_or(Value::Null, |v| Value::String(v.to_string()));
            Action::Respond(unsupported_version_frame(id, requested))
        }
        "server/discover" | "ping" | "tools/list" => Action::Forward(normalize_params(value)),
        "tools/call" => classify_call(value, id),
        _ => Action::Respond(error_frame(
            id,
            METHOD_NOT_FOUND,
            &format!("method not found: {method}"),
        )),
    }
}

/// Classify a `tools/call` frame, reproducing the baseline's
/// degradation in the shapes the SDK's typed envelope cannot
/// represent, and the baseline's exact protocol error for the one
/// shape it must answer itself (non-object `arguments`).
fn classify_call(mut value: Value, id: Value) -> Action {
    let params = params_map(&mut value);
    // Missing/non-string name: forward with "" — the handler runs
    // the baseline's dispatch, which answers -32602 "unknown
    // tool: " for the empty name, byte-identical to the baseline.
    let name_ok = params.get("name").is_some_and(Value::is_string);
    if !name_ok {
        params.insert("name".to_string(), Value::String(String::new()));
    }
    // Present-but-non-object arguments: the baseline's arg_object
    // refused with this exact protocol error before any dispatch.
    if let Some(arguments) = params.get("arguments") {
        if !arguments.is_object() {
            return Action::Respond(error_frame(
                id,
                INVALID_PARAMS,
                "arguments must be a JSON object",
            ));
        }
    }
    Action::Forward(value)
}

/// The baseline ignored `params` on discover/list/ping entirely; a
/// missing or non-object value is normalized to `{}` so the SDK's
/// typed envelope always parses.
fn normalize_params(mut value: Value) -> Value {
    let object = value.as_object_mut().expect("classified object");
    match object.get("params") {
        Some(p) if p.is_object() => {}
        _ => {
            object.insert("params".to_string(), json!({}));
        }
    }
    value
}

/// Ensure `params` exists as an object and return the frame's
/// params map for further normalization.
fn params_map(value: &mut Value) -> &mut serde_json::Map<String, Value> {
    let object = value.as_object_mut().expect("classified object");
    match object.get("params") {
        Some(p) if p.is_object() => {}
        _ => {
            object.insert("params".to_string(), json!({}));
        }
    }
    object
        .get_mut("params")
        .and_then(Value::as_object_mut)
        .expect("params normalized to an object")
}

async fn write_frame(stdout: &SharedStdout, frame: &Value) {
    let mut line = serde_json::to_string(frame).unwrap_or_default();
    line.push('\n');
    let mut out = stdout.lock().await;
    let _ = out.write_all(line.as_bytes()).await;
    let _ = out.flush().await;
}

async fn write_bytes(stdout: &SharedStdout, bytes: &[u8]) {
    let mut out = stdout.lock().await;
    let _ = out.write_all(bytes).await;
    let _ = out.flush().await;
}

/// One inbound line, bounded: the error trips when the content
/// (excluding the newline) reaches [`MAX_LINE_BYTES`]; an oversized
/// line's remainder is drained.
enum LineOutcome {
    Line(Vec<u8>),
    Oversized,
    Eof,
}

async fn next_line<R: AsyncBufRead + Unpin>(reader: &mut R) -> std::io::Result<LineOutcome> {
    let mut out: Vec<u8> = Vec::new();
    let mut content_len = 0usize;
    let mut oversized = false;
    loop {
        let chunk = reader.fill_buf().await?;
        if chunk.is_empty() {
            return Ok(if oversized {
                LineOutcome::Oversized
            } else if out.is_empty() && content_len == 0 {
                LineOutcome::Eof
            } else {
                LineOutcome::Line(out)
            });
        }
        match chunk.iter().position(|&b| b == b'\n') {
            Some(pos) => {
                content_len += pos;
                if content_len >= MAX_LINE_BYTES {
                    oversized = true;
                }
                if !oversized {
                    out.extend_from_slice(&chunk[..pos]);
                }
                reader.consume(pos + 1);
                return Ok(if oversized {
                    LineOutcome::Oversized
                } else {
                    LineOutcome::Line(out)
                });
            }
            None => {
                let len = chunk.len();
                content_len += len;
                if content_len >= MAX_LINE_BYTES {
                    oversized = true;
                    out.clear();
                } else {
                    out.extend_from_slice(chunk);
                }
                reader.consume(len);
            }
        }
    }
}

/// Run the guard until stdin reaches EOF, then shut the duplex
/// write side down: a split write half signals end-of-input to the
/// SDK service only through an explicit shutdown, never by being
/// dropped (both halves share ownership of the stream).
pub async fn run(
    stdin: Stdin,
    mut to_server: WriteHalf<DuplexStream>,
    stdout: SharedStdout,
    ordering: Arc<Ordering>,
) -> std::io::Result<()> {
    let result = forward_loop(stdin, &mut to_server, &stdout, &ordering).await;
    let _ = to_server.shutdown().await;
    result
}

async fn forward_loop(
    stdin: Stdin,
    to_server: &mut WriteHalf<DuplexStream>,
    stdout: &SharedStdout,
    ordering: &Ordering,
) -> std::io::Result<()> {
    let mut reader = BufReader::new(stdin);
    loop {
        match next_line(&mut reader).await? {
            LineOutcome::Eof => return Ok(()),
            LineOutcome::Oversized => {
                ordering.wait_drained().await;
                write_frame(
                    stdout,
                    &error_frame(Value::Null, PARSE_ERROR, "message exceeds size limit"),
                )
                .await;
            }
            LineOutcome::Line(bytes) => match classify(&bytes) {
                Action::Drop => {}
                Action::Respond(frame) => {
                    ordering.wait_drained().await;
                    write_frame(stdout, &frame).await;
                }
                Action::Forward(frame) => {
                    if let Some(id) = frame.get("id").cloned() {
                        ordering.track(id).await;
                    }
                    let mut line = serde_json::to_string(&frame).unwrap_or_default();
                    line.push('\n');
                    to_server.write_all(line.as_bytes()).await?;
                    to_server.flush().await?;
                }
            },
        }
    }
}

/// Copy SDK responses from the duplex to stdout, one line at a time,
/// marking each answered request id complete for the guard's
/// ordering wait.
pub async fn pump(
    mut from_server: ReadHalf<DuplexStream>,
    stdout: SharedStdout,
    ordering: Arc<Ordering>,
) {
    let mut reader = BufReader::new(&mut from_server);
    let mut line: Vec<u8> = Vec::new();
    loop {
        line.clear();
        match reader.read_until(b'\n', &mut line).await {
            Ok(0) | Err(_) => return,
            Ok(_) => {
                write_bytes(&stdout, &line).await;
                if let Ok(frame) = serde_json::from_slice::<Value>(&line) {
                    if let Some(id) = frame.get("id") {
                        ordering.complete(id).await;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classify_str(s: &str) -> Action {
        classify(s.as_bytes())
    }

    fn classify_json(frame: &Value) -> Action {
        classify(serde_json::to_string(frame).unwrap().as_bytes())
    }

    fn modern_meta() -> Value {
        json!({
            "io.modelcontextprotocol/protocolVersion": "2026-07-28",
            "io.modelcontextprotocol/clientInfo": {"name": "test", "version": "1"},
            "io.modelcontextprotocol/clientCapabilities": {},
        })
    }

    fn modern_call(id: u64, params: Value) -> Value {
        let mut params = params;
        params
            .as_object_mut()
            .expect("object params")
            .insert("_meta".to_string(), modern_meta());
        json!({"jsonrpc": "2.0", "id": id, "method": "tools/call", "params": params})
    }

    #[test]
    fn blank_lines_and_notifications_drop() {
        assert_eq!(classify_str("  \n"), Action::Drop);
        assert_eq!(
            classify_str("{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}"),
            Action::Drop
        );
        // A notification-shaped initialize is a notification.
        assert_eq!(
            classify_str("{\"jsonrpc\":\"2.0\",\"method\":\"initialize\",\"params\":{}}"),
            Action::Drop
        );
        // A notification-shaped tools/call drops unexecuted — the
        // fleet rule (the baseline executed it; see module docs).
        assert_eq!(
            classify_str(
                "{\"jsonrpc\":\"2.0\",\"method\":\"tools/call\",\"params\":{\"name\":\"neemu_register_work\",\"arguments\":{}}}"
            ),
            Action::Drop
        );
        // A notification-shaped tools/list drops too (the baseline
        // answered it with id null).
        assert_eq!(
            classify_str("{\"jsonrpc\":\"2.0\",\"method\":\"tools/list\"}"),
            Action::Drop
        );
    }

    #[test]
    fn framing_errors_match_the_baseline() {
        match classify_str("this is not json") {
            Action::Respond(f) => {
                assert_eq!(f["error"]["code"], PARSE_ERROR);
                // The baseline's shape carries the serde detail.
                let message = f["error"]["message"].as_str().expect("message");
                assert!(
                    message.starts_with("parse error: "),
                    "baseline parse shape: {message}"
                );
                assert_eq!(f["id"], Value::Null);
            }
            other => panic!("expected respond, got {other:?}"),
        }
        match classify_str("[1,2,3]") {
            Action::Respond(f) => assert_eq!(f["error"]["code"], INVALID_REQUEST),
            other => panic!("expected respond, got {other:?}"),
        }
    }

    #[test]
    fn initialize_is_refused_with_supported_versions() {
        let frame = "{\"id\":7,\"jsonrpc\":\"2.0\",\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2025-06-18\",\"clientInfo\":{\"name\":\"x\"}}}";
        match classify_str(frame) {
            Action::Respond(f) => {
                assert_eq!(f["id"], 7);
                assert_eq!(f["error"]["code"], UNSUPPORTED_PROTOCOL_VERSION);
                assert_eq!(f["error"]["data"]["requested"], "2025-06-18");
                assert_eq!(f["error"]["data"]["supported"], json!(["2026-07-28"]));
            }
            other => panic!("expected respond, got {other:?}"),
        }
    }

    #[test]
    fn unknown_and_missing_methods_carry_the_baseline_text() {
        let frame = "{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"resources/list\",\"params\":{}}";
        match classify_str(frame) {
            Action::Respond(f) => {
                assert_eq!(f["error"]["code"], METHOD_NOT_FOUND);
                assert_eq!(f["error"]["message"], "method not found: resources/list");
            }
            other => panic!("expected respond, got {other:?}"),
        }
        // Id present, no method: the baseline answered -32601 with
        // an empty name (it did NOT drop response-shaped frames).
        match classify_str("{\"id\":9,\"jsonrpc\":\"2.0\",\"result\":{}}") {
            Action::Respond(f) => {
                assert_eq!(f["error"]["code"], METHOD_NOT_FOUND);
                assert_eq!(f["error"]["message"], "method not found: ");
            }
            other => panic!("expected respond, got {other:?}"),
        }
    }

    #[test]
    fn nameless_calls_forward_with_empty_name() {
        // The handler reproduces the baseline's -32602 "unknown
        // tool: " — the guard only normalizes the name.
        for frame in [
            modern_call(1, json!({"arguments": {}})),
            modern_call(2, json!({"name": 42, "arguments": {}})),
            json!({"id": 3, "method": "tools/call", "params": {}}),
        ] {
            match classify_json(&frame) {
                Action::Forward(f) => assert_eq!(f["params"]["name"], ""),
                other => panic!("expected forward for {frame}, got {other:?}"),
            }
        }
    }

    #[test]
    fn non_object_arguments_get_the_baseline_protocol_error() {
        for frame in [
            modern_call(
                2,
                json!({"name": "neemu_list_records", "arguments": "nope"}),
            ),
            modern_call(3, json!({"name": "neemu_list_records", "arguments": []})),
        ] {
            match classify_json(&frame) {
                Action::Respond(f) => {
                    assert_eq!(f["error"]["code"], INVALID_PARAMS);
                    assert_eq!(f["error"]["message"], "arguments must be a JSON object");
                }
                other => panic!("expected respond for {frame}, got {other:?}"),
            }
        }
    }

    #[test]
    fn malformed_calls_normalize_to_baseline_degradation() {
        // Missing arguments forward untouched — the handler
        // defaults to {} and the domain's own errors answer.
        match classify_json(&modern_call(2, json!({"name": "neemu_list_records"}))) {
            Action::Forward(f) => assert!(f["params"].get("arguments").is_none()),
            other => panic!("expected forward, got {other:?}"),
        }
        // Non-object params on tools/list normalize to {}.
        match classify_str("{\"id\":9,\"method\":\"tools/list\",\"params\":\"x\"}") {
            Action::Forward(f) => assert_eq!(f["params"], json!({})),
            other => panic!("expected forward, got {other:?}"),
        }
    }

    #[test]
    fn valid_modern_frames_forward() {
        for frame in [
            json!({"id": 1, "method": "server/discover", "params": {"_meta": modern_meta()}}),
            json!({"id": 2, "method": "tools/list", "params": {"_meta": modern_meta()}}),
            modern_call(3, json!({"name": "neemu_list_records", "arguments": {}})),
            json!({"id": 4, "method": "ping"}),
        ] {
            match classify_json(&frame) {
                Action::Forward(_) => {}
                other => panic!("expected forward for {frame}, got {other:?}"),
            }
        }
    }
}
