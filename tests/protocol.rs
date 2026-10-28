//! MCP front-end tests: the compiled server driven over stdio.
//!
//! The framing moved to rmcp behind the era guard in 0.2.0 (WS2 of
//! the MCP standardization program), so these tests drive the real
//! binary as a subprocess — the same surface a client sees.
//! Responses are correlated by JSON-RPC id, never positionally:
//! rmcp dispatches pipelined requests concurrently and may answer
//! out of order (legal JSON-RPC; the hand-rolled baseline answered
//! strictly sequentially).
//!
//! Validation: discover negotiation, tools/list shape (including
//! the D-147 `neemu_export` annotation honesty fix on the wire),
//! a gate-refusal round trip. Adversarial: legacy contact,
//! malformed lines, oversized lines, nameless calls, non-object
//! arguments, notification-shaped calls — the server must answer
//! in the baseline's shapes (or the fleet's designed ones) and
//! stay alive.

mod common;

use common::{png_bytes, temp_dir};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

struct Client {
    child: tokio::process::Child,
    pending: HashMap<JsonValue, JsonValue>,
    receiver: mpsc::UnboundedReceiver<JsonValue>,
    stdin: tokio::process::ChildStdin,
}

fn modern_meta() -> JsonValue {
    json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": {"name": "neemu-test", "version": "test"},
        "io.modelcontextprotocol/clientCapabilities": {},
    })
}

async fn spawn_server(home: &std::path::Path) -> Client {
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_neemu"))
        .arg("--home")
        .arg(home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .expect("spawn neemu");
    let stdin = child.stdin.take().expect("stdin");
    let stdout = child.stdout.take().expect("stdout");
    let (sender, receiver) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if let Ok(value) = serde_json::from_str::<JsonValue>(&line) {
                let _ = sender.send(value);
            }
        }
    });
    Client {
        child,
        pending: HashMap::new(),
        receiver,
        stdin,
    }
}

impl Client {
    async fn send(&mut self, message: &JsonValue) {
        let mut text = message.to_string();
        text.push('\n');
        self.stdin.write_all(text.as_bytes()).await.expect("send");
        self.stdin.flush().await.expect("flush");
    }

    async fn send_raw(&mut self, text: &str) {
        self.stdin
            .write_all(text.as_bytes())
            .await
            .expect("send raw");
        self.stdin.flush().await.expect("flush raw");
    }

    /// Read responses until the one carrying `id` arrives; any
    /// other response is buffered for its own waiter.
    async fn receive_for(&mut self, id: &JsonValue) -> JsonValue {
        if let Some(response) = self.pending.remove(id) {
            return response;
        }
        loop {
            let response =
                tokio::time::timeout(std::time::Duration::from_secs(30), self.receiver.recv())
                    .await
                    .expect("response within deadline")
                    .expect("a response line");
            if response.get("id") == Some(id) {
                return response;
            }
            if let Some(other) = response.get("id").cloned() {
                self.pending.insert(other, response);
            }
        }
    }

    async fn discover(&mut self, id: u64) -> JsonValue {
        self.send(
            &json!({"jsonrpc": "2.0", "id": id, "method": "server/discover",
                    "params": {"_meta": modern_meta()}}),
        )
        .await;
        self.receive_for(&json!(id)).await
    }

    async fn list_tools(&mut self, id: u64) -> JsonValue {
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": "tools/list",
                    "params": {"_meta": modern_meta()}}))
            .await;
        self.receive_for(&json!(id)).await
    }

    async fn call(&mut self, id: u64, name: &str, arguments: JsonValue) -> JsonValue {
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": "tools/call",
                    "params": {"name": name, "arguments": arguments, "_meta": modern_meta()}}))
            .await;
        self.receive_for(&json!(id)).await
    }

    async fn shutdown(mut self) {
        drop(self.stdin);
        let _ = tokio::time::timeout(std::time::Duration::from_secs(10), self.child.wait()).await;
    }
}

#[tokio::test]
async fn discover_and_tools_list_shape() {
    let home = temp_dir("protocol-list");
    let mut client = spawn_server(&home).await;
    let discover = client.discover(1).await;
    assert!(discover.get("result").is_some(), "discover: {discover}");
    let list = client.list_tools(2).await;
    let tools = list["result"]["tools"].as_array().expect("tools array");
    assert_eq!(tools.len(), 15);
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(names, neemu::tools::TOOL_NAMES);
    for tool in tools {
        assert!(tool.get("outputSchema").is_some(), "tool {tool}");
        assert!(tool["annotations"].get("readOnlyHint").is_some());
    }
    let export = tools
        .iter()
        .find(|t| t["name"] == "neemu_export")
        .expect("export");
    assert_eq!(
        export["annotations"]["readOnlyHint"],
        json!(false),
        "neemu_export writes its bundle; readOnlyHint must be false"
    );
    let listed = tools
        .iter()
        .find(|t| t["name"] == "neemu_list_records")
        .expect("list_records");
    assert_eq!(listed["annotations"]["readOnlyHint"], json!(true));
    client.shutdown().await;
}

#[tokio::test]
async fn legacy_initialize_is_refused_and_the_server_lives() {
    let home = temp_dir("protocol-legacy");
    let mut client = spawn_server(&home).await;
    for (id, version) in [(1, "2025-06-18"), (2, "2024-11-05"), (3, "2025-03-26")] {
        client
            .send(&json!({"jsonrpc": "2.0", "id": id, "method": "initialize",
                        "params": {"protocolVersion": version, "capabilities": {},
                                   "clientInfo": {"name": "legacy", "version": "0"}}}))
            .await;
        let refusal = client.receive_for(&json!(id)).await;
        assert_eq!(
            refusal["error"]["code"],
            json!(-32022),
            "refusal: {refusal}"
        );
        assert_eq!(refusal["error"]["data"]["requested"], json!(version));
        assert_eq!(refusal["error"]["data"]["supported"], json!(["2026-07-28"]));
    }
    // Even a modern-stamped initialize is refused: the era has no
    // initialize.
    client
        .send(&json!({"jsonrpc": "2.0", "id": 4, "method": "initialize",
                    "params": {"protocolVersion": "2026-07-28", "_meta": modern_meta()}}))
        .await;
    let refusal = client.receive_for(&json!(4)).await;
    assert_eq!(refusal["error"]["code"], json!(-32022));
    // The server answers normally afterwards.
    let list = client.list_tools(5).await;
    assert_eq!(list["result"]["tools"].as_array().unwrap().len(), 15);
    client.shutdown().await;
}

#[tokio::test]
async fn gate_refusal_round_trip_and_args_errors() {
    let home = temp_dir("protocol-calls");
    let mut client = spawn_server(&home).await;
    client.discover(1).await;
    // Gate refusal: in-band, typed (SPEC §5) — not a protocol error.
    let refusal = client
        .call(2, "neemu_list_records", json!({"work_id": "work_nope"}))
        .await;
    assert_eq!(refusal["result"]["isError"], json!(true));
    let text = refusal["result"]["content"][0]["text"].as_str().unwrap();
    let payload: JsonValue = serde_json::from_str(text).unwrap();
    assert_eq!(payload["error"]["kind"], json!("screen_refusal"));
    // Unknown tool: the baseline's protocol error, -32602.
    let unknown = client.call(3, "neemu_nope", json!({})).await;
    assert_eq!(unknown["error"]["code"], json!(-32602));
    assert_eq!(
        unknown["error"]["message"],
        json!("unknown tool: neemu_nope")
    );
    // Missing required field: the domain's own -32602.
    let missing = client.call(4, "neemu_list_records", json!({})).await;
    assert_eq!(missing["error"]["code"], json!(-32602));
    assert_eq!(
        missing["error"]["message"],
        json!("missing required field 'work_id'")
    );
    client.shutdown().await;
}

#[tokio::test]
async fn nameless_and_non_object_arguments_match_the_baseline() {
    let home = temp_dir("protocol-shapes");
    let mut client = spawn_server(&home).await;
    client.discover(1).await;
    // Missing name → forwarded as "" → the baseline's -32602.
    client
        .send(&json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
                    "params": {"arguments": {}, "_meta": modern_meta()}}))
        .await;
    let nameless = client.receive_for(&json!(2)).await;
    assert_eq!(nameless["error"]["code"], json!(-32602));
    assert_eq!(nameless["error"]["message"], json!("unknown tool: "));
    // Non-object arguments → the baseline's exact protocol error.
    client
        .send(&json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call",
                    "params": {"name": "neemu_list_records", "arguments": "nope",
                               "_meta": modern_meta()}}))
        .await;
    let bad_args = client.receive_for(&json!(3)).await;
    assert_eq!(bad_args["error"]["code"], json!(-32602));
    assert_eq!(
        bad_args["error"]["message"],
        json!("arguments must be a JSON object")
    );
    client.shutdown().await;
}

#[tokio::test]
async fn framing_battery_and_server_survival() {
    let home = temp_dir("protocol-framing");
    let mut client = spawn_server(&home).await;
    // Malformed JSON: -32700 carrying the baseline's serde detail.
    client.send_raw("this is not json\n").await;
    let parse = client.receive_for(&JsonValue::Null).await;
    assert_eq!(parse["error"]["code"], json!(-32700));
    let message = parse["error"]["message"].as_str().unwrap();
    assert!(message.starts_with("parse error: "), "message: {message}");
    // Non-object frame: the fleet's -32600.
    client.send_raw("[1,2,3]\n").await;
    let non_object = client.receive_for(&JsonValue::Null).await;
    assert_eq!(non_object["error"]["code"], json!(-32600));
    // Unknown method names itself.
    client
        .send(&json!({"jsonrpc": "2.0", "id": 5, "method": "bogus/method"}))
        .await;
    let unknown = client.receive_for(&json!(5)).await;
    assert_eq!(unknown["error"]["code"], json!(-32601));
    assert_eq!(
        unknown["error"]["message"],
        json!("method not found: bogus/method")
    );
    // Oversize line: drained, -32700 size error, server alive.
    let big = "a".repeat(5 * 1024 * 1024);
    client.send_raw(&format!("{big}\n")).await;
    let oversize = client.receive_for(&JsonValue::Null).await;
    assert_eq!(oversize["error"]["code"], json!(-32700));
    assert_eq!(
        oversize["error"]["message"],
        json!("message exceeds size limit")
    );
    let list = client.list_tools(6).await;
    assert_eq!(list["result"]["tools"].as_array().unwrap().len(), 15);
    client.shutdown().await;
}

#[tokio::test]
async fn notification_shaped_call_drops_unexecuted() {
    let root = temp_dir("protocol-notif");
    let home = root.join("registry");
    let media = root.join("media");
    std::fs::create_dir_all(&media).unwrap();
    std::fs::write(media.join("page-001.png"), png_bytes(800, 1200, 3)).unwrap();
    let mut client = spawn_server(&home).await;
    // A notification-shaped register: no id. The fleet rule drops
    // it unexecuted (the baseline executed it and answered with id
    // null — the gate report itemizes the delta).
    client
        .send(&json!({"jsonrpc": "2.0", "method": "tools/call",
                    "params": {"name": "neemu_register_work",
                               "arguments": {"title": "Notification Wife",
                                             "source": {"kind": "owner_supplied", "ref": "test"},
                                             "media_dir": media.to_string_lossy(),
                                             "tier_claim": "A"}}}))
        .await;
    let list = client.list_tools(1).await;
    assert_eq!(list["result"]["tools"].as_array().unwrap().len(), 15);
    // No id-null response may have arrived for the notification.
    assert!(
        client.pending.is_empty(),
        "unexpected extra responses: {:?}",
        client.pending.keys().collect::<Vec<_>>()
    );
    let works = home.join("works");
    let minted: Vec<_> = if works.exists() {
        std::fs::read_dir(&works).unwrap().collect()
    } else {
        Vec::new()
    };
    assert!(minted.is_empty(), "notification must not mint a work");
    client.shutdown().await;
}

#[tokio::test]
async fn pipelined_startup_answers_in_request_order() {
    let home = temp_dir("protocol-ordering");
    let mut client = spawn_server(&home).await;
    // One write: discover (SDK, slow first response), initialize
    // (guard refusal), unknown method (guard refusal). Without the
    // Ordering drain the guard's instant answers overtake id 0.
    let mut batch = String::new();
    for frame in [
        json!({"jsonrpc": "2.0", "id": 0, "method": "server/discover",
               "params": {"_meta": modern_meta()}}),
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
               "params": {"protocolVersion": "2025-06-18"}}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "bogus/method"}),
    ] {
        batch.push_str(&frame.to_string());
        batch.push('\n');
    }
    client.send_raw(&batch).await;
    let mut order = Vec::new();
    for _ in 0..3 {
        let response =
            tokio::time::timeout(std::time::Duration::from_secs(30), client.receiver.recv())
                .await
                .expect("response within deadline")
                .expect("a response line");
        order.push(response["id"].as_u64().unwrap());
    }
    assert_eq!(order, vec![0, 1, 2]);
    client.shutdown().await;
}
