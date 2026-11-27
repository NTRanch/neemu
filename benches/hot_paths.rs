//! Hot-path benchmarks for the `neemu` MCP server: cold start to discover, tools/list latency, and `neemu_list_records` gate-refusal latency over one warm stdio session (full profile, scratch registry home). Hand-rolled on std::time; run with `cargo bench --bench hot_paths`.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// Spawns for the cold-start workload (bounded, named).
const COLD_STARTS: usize = 10;
/// Warmup requests before each measured loop.
const WARMUP: usize = 20;
/// Measured requests per warm workload (bounded, named).
const MEASURED: usize = 200;
/// Per-response read deadline.
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(20);
/// The one protocol era this fleet speaks (D-113).
const PROTOCOL_VERSION: &str = "2026-07-28";

struct Session {
    child: Child,
    stdin: ChildStdin,
    lines: mpsc::Receiver<String>,
    next_id: u64,
}

impl Session {
    fn spawn(command: &mut Command) -> Session {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn server");
        let stdin = child.stdin.take().expect("server stdin");
        let stdout = child.stdout.take().expect("server stdout");
        let (tx, rx) = mpsc::channel::<String>();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        if tx.send(line.clone()).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        Session {
            child,
            stdin,
            lines: rx,
            next_id: 0,
        }
    }

    fn request(&mut self, method: &str, extra_params: Value) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        let mut params = json!({
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": PROTOCOL_VERSION,
                "io.modelcontextprotocol/clientInfo": {"name": "hot-paths-bench", "version": "0"},
                "io.modelcontextprotocol/clientCapabilities": {}
            }
        });
        if let (Some(dst), Some(src)) = (params.as_object_mut(), extra_params.as_object()) {
            for (key, value) in src {
                dst.insert(key.clone(), value.clone());
            }
        }
        let frame = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        let mut line = serde_json::to_string(&frame).expect("encode frame");
        line.push('\n');
        self.stdin.write_all(line.as_bytes()).expect("write frame");
        self.stdin.flush().expect("flush frame");
        let deadline = Instant::now() + RESPONSE_TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            assert!(!remaining.is_zero(), "timed out awaiting id {id}");
            let raw = self.lines.recv_timeout(remaining).expect("server output");
            let value: Value = serde_json::from_str(&raw).expect("frame is JSON");
            if value["id"] == json!(id) {
                return value;
            }
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn report(name: &str, total: Duration, iterations: usize) {
    let per = total.as_secs_f64() / iterations as f64;
    println!(
        "{name}: {iterations} iters, total {:.1} ms, {:.1} us/op, {:.0} ops/s",
        total.as_secs_f64() * 1e3,
        per * 1e6,
        1.0 / per,
    );
}

fn scratch_dir() -> PathBuf {
    std::env::temp_dir().join(format!("neemu-bench-{}", std::process::id()))
}

fn setup_scratch() {
    std::fs::create_dir_all(scratch_dir()).expect("mkdir scratch home");
}

fn make_command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_neemu"));
    command
        .arg("--home")
        .arg(scratch_dir())
        .arg("--profile")
        .arg("full");
    command
}

fn main() {
    setup_scratch();
    // Cold start: spawn -> discover response, per spawn.
    let started = Instant::now();
    for _ in 0..COLD_STARTS {
        let mut session = Session::spawn(&mut make_command());
        let discover = session.request("server/discover", json!({}));
        assert!(
            discover["result"].is_object(),
            "discover failed: {discover}"
        );
    }
    report("cold_start_discover", started.elapsed(), COLD_STARTS);

    // Warm session workloads.
    let mut session = Session::spawn(&mut make_command());
    let discover = session.request("server/discover", json!({}));
    assert!(
        discover["result"].is_object(),
        "discover failed: {discover}"
    );
    for _ in 0..WARMUP {
        let _ = session.request("tools/list", json!({}));
        let _ = session.request(
            "tools/call",
            json!({"name": "neemu_list_records", "arguments": json!({"work_id": "work_nope"})}),
        );
    }
    let started = Instant::now();
    for _ in 0..MEASURED {
        let list = session.request("tools/list", json!({}));
        assert!(
            list["result"]["tools"].is_array(),
            "tools/list failed: {list}"
        );
    }
    report("tools_list", started.elapsed(), MEASURED);

    let started = Instant::now();
    for _ in 0..MEASURED {
        let call = session.request(
            "tools/call",
            json!({"name": "neemu_list_records", "arguments": json!({"work_id": "work_nope"})}),
        );
        assert!(call["result"].is_object(), "tools/call failed: {call}");
    }
    report("tool_call_neemu_list_records", started.elapsed(), MEASURED);
    let _ = std::fs::remove_dir_all(scratch_dir());
}
