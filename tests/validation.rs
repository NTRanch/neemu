//! Validation suite (the validation half of the estate 50/50 split):
//! happy-path behaviour of the S1 surface, determinism, idempotency,
//! and the tools/list contract shape.

mod common;

use common::*;
use neemu::tools::{tool_definitions, TOOL_NAMES};
use serde_json::{json, Value};
use std::fs;

#[test]
fn register_happy_tier_a() {
    let rig = rig("reg-a", 3);
    let work = register(&rig, "Moonlit Wife", "A");
    assert!(work["work_id"].as_str().unwrap().starts_with("work_"));
    assert_eq!(work["tier"], json!("A"));
    assert_eq!(work["file_count"], json!(3));
    assert_eq!(work["idempotent_replay"], json!(false));
}

#[test]
fn register_idempotent_replay() {
    let rig = rig("reg-idem", 2);
    let first = register(&rig, "Moonlit Wife", "A");
    let second = register(&rig, "Moonlit Wife", "A");
    assert_eq!(first["work_id"], second["work_id"]);
    assert_eq!(second["idempotent_replay"], json!(true));
}

#[test]
fn register_tier_t_with_evidence() {
    let rig = rig("reg-t", 2);
    let work = register(&rig, "Licensed Work", "T");
    assert_eq!(work["tier"], json!("T"));
}

#[test]
fn validate_ingest_passes_and_is_deterministic() {
    let rig = rig("val-det", 3);
    let work = register(&rig, "Moonlit Wife", "A");
    let work_id = work["work_id"].as_str().unwrap().to_string();
    let first = call(
        &rig.neemu,
        "neemu_validate_ingest",
        json!({ "work_id": work_id }),
    );
    let second = call(
        &rig.neemu,
        "neemu_validate_ingest",
        json!({ "work_id": work_id }),
    );
    assert_eq!(first["passed"], json!(true));
    assert_eq!(first["screen_result"]["passed"], json!(true));
    assert_eq!(first["screen_result"]["blocklist_hit"], Value::Null);
    assert_eq!(first["record_id"], second["record_id"]);
    assert_eq!(first["checks"]["hashes_match"], json!(true));
}

#[test]
fn screen_clears_with_adult_evidence() {
    let rig = rig("scr-clear", 2);
    let (work_id, _) = register_and_validate(&rig, "Moonlit Wife", "A");
    let record = screen_cleared(&rig.neemu, &work_id);
    assert_eq!(record["reasons"], json!([]));
    assert_eq!(record["blocklist_hit"], Value::Null);
    let listed = call(
        &rig.neemu,
        "neemu_list_records",
        json!({ "work_id": work_id, "kind": "screen" }),
    );
    assert_eq!(listed["total"], json!(1));
}

#[test]
fn segment_scenes_by_max_pages() {
    let rig = rig("seg-max", 6);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    let result = call(
        &rig.neemu,
        "neemu_segment_scene",
        json!({ "work_id": work_id, "preflight_id": preflight_id, "max_pages_per_scene": 4 }),
    );
    assert_eq!(result["total_scenes"], json!(2));
    assert_eq!(result["scenes"][0]["page_start"], json!(0));
    assert_eq!(result["scenes"][0]["page_end"], json!(3));
    assert_eq!(result["scenes"][1]["page_start"], json!(4));
    assert_eq!(result["scenes"][1]["page_end"], json!(5));
    assert_eq!(result["partial"], json!(false));
}

#[test]
fn segment_scene_ids_stable_across_calls() {
    let rig = rig("seg-stable", 4);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    let args = json!({ "work_id": work_id, "preflight_id": preflight_id });
    let first = call(&rig.neemu, "neemu_segment_scene", args.clone());
    let second = call(&rig.neemu, "neemu_segment_scene", args);
    assert_eq!(first["scenes"], second["scenes"]);
}

#[test]
fn segment_boundary_on_dimension_change() {
    let root = temp_dir("seg-dims");
    let media = root.join("media");
    fs::create_dir_all(&media).unwrap();
    fs::write(media.join("page-001.png"), png_bytes(800, 1200, 1)).unwrap();
    fs::write(media.join("page-002.png"), png_bytes(800, 1200, 2)).unwrap();
    fs::write(media.join("page-003.png"), png_bytes(1024, 768, 3)).unwrap();
    let rig = Rig {
        neemu: neemu::Neemu::open(root.join("registry")).unwrap(),
        root,
        media_dir: media.to_string_lossy().to_string(),
    };
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    let result = call(
        &rig.neemu,
        "neemu_segment_scene",
        json!({ "work_id": work_id, "preflight_id": preflight_id }),
    );
    assert_eq!(result["total_scenes"], json!(2));
    assert_eq!(result["scenes"][1]["page_start"], json!(2));
    assert_eq!(result["scenes"][1]["width"], json!(1024));
}

#[test]
fn segment_emits_file_metric_metadata() {
    let rig = rig("seg-meta", 3);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    let result = call(
        &rig.neemu,
        "neemu_segment_scene",
        json!({ "work_id": work_id, "preflight_id": preflight_id }),
    );
    assert_eq!(result["metadata"]["basis"], json!("file-metric"));
    assert_eq!(result["metadata"]["media_kind"], json!("page-sequence"));
    assert_eq!(result["metadata"]["page_count"], json!(3));
    assert_eq!(result["metadata"]["formats"]["png"], json!(3));
}

#[test]
fn extract_faces_grid_read_mints_records() {
    let rig = rig("faces-grid", 3);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    let result = call(
        &rig.neemu,
        "neemu_extract_faces",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "page_start": 0,
            "page_end": 2,
            "observations": [face_observation(0), face_observation(1)],
        }),
    );
    assert_eq!(result["count"], json!(2));
    assert_eq!(result["partial"], json!(false));
    let listed = call(
        &rig.neemu,
        "neemu_list_records",
        json!({ "work_id": work_id, "kind": "face_observation" }),
    );
    assert_eq!(listed["total"], json!(2));
}

#[test]
fn extract_faces_new_candidate_accepted() {
    let rig = rig("faces-new", 2);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    let mut observation = face_observation(0);
    observation["f_code"] = Value::Null;
    observation["new_candidate"] = json!({
        "channels": observation["channels"].clone(),
        "beats": ["induction", "deepening"],
        "proposed_code": "NEW-001",
    });
    let result = call(
        &rig.neemu,
        "neemu_extract_faces",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "page_start": 0,
            "page_end": 1,
            "observations": [observation],
        }),
    );
    assert_eq!(result["count"], json!(1));
}

#[test]
fn extract_faces_idempotent() {
    let rig = rig("faces-idem", 2);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    let args = json!({
        "work_id": work_id,
        "preflight_id": preflight_id,
        "page_start": 0,
        "page_end": 1,
        "observations": [face_observation(0)],
    });
    let first = call(&rig.neemu, "neemu_extract_faces", args.clone());
    let second = call(&rig.neemu, "neemu_extract_faces", args);
    assert_eq!(first["record_ids"], second["record_ids"]);
    let listed = call(
        &rig.neemu,
        "neemu_list_records",
        json!({ "work_id": work_id, "kind": "face_observation" }),
    );
    assert_eq!(listed["total"], json!(1));
}

#[test]
fn extract_dialogue_mints_word_nesting() {
    let rig = rig("dlg-nest", 2);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    let result = call(
        &rig.neemu,
        "neemu_extract_dialogue",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "page_start": 0,
            "page_end": 1,
            "lines": [
                { "page": 0, "speaker": "lead", "text": "Look into my eyes now", "beat": "induction" }
            ],
        }),
    );
    assert_eq!(result["count"], json!(1));
    let record_id = result["record_ids"][0].as_str().unwrap().to_string();
    let stored = rig
        .neemu
        .load_record("records", &work_id, &record_id)
        .expect("stored dialogue record");
    assert_eq!(stored["words"].as_array().unwrap().len(), 5);
    assert_eq!(stored["words"][0]["word"], json!("Look"));
    assert!(stored["known_limitations"]
        .as_str()
        .unwrap()
        .contains("page-image lane"));
}

#[test]
fn extract_dialogue_face_ref_links() {
    let rig = rig("dlg-ref", 2);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    let faces = call(
        &rig.neemu,
        "neemu_extract_faces",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "page_start": 0,
            "page_end": 1,
            "observations": [face_observation(0)],
        }),
    );
    let face_id = faces["record_ids"][0].as_str().unwrap().to_string();
    let dialogue = call(
        &rig.neemu,
        "neemu_extract_dialogue",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "page_start": 0,
            "page_end": 1,
            "lines": [
                { "page": 0, "speaker": "lead", "text": "Deeper now", "face_ref": face_id }
            ],
        }),
    );
    assert_eq!(dialogue["count"], json!(1));
}

#[test]
fn export_analysis_tier_a_ok() {
    let rig = rig("exp-a", 2);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    call(
        &rig.neemu,
        "neemu_extract_faces",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "page_start": 0,
            "page_end": 1,
            "observations": [face_observation(0)],
        }),
    );
    let exported = call(
        &rig.neemu,
        "neemu_export",
        json!({ "work_id": work_id, "bundle_kind": "analysis" }),
    );
    assert_eq!(exported["record_count"], json!(1));
    assert_eq!(exported["tier"], json!("A"));
    let path = exported["path"].as_str().unwrap();
    assert!(fs::metadata(path).is_ok(), "bundle file exists at {path}");
}

#[test]
fn export_training_tier_t_ok() {
    let rig = rig("exp-t", 2);
    let (work_id, _preflight_id) = cleared_work(&rig, "Licensed Work", "T");
    let exported = call(
        &rig.neemu,
        "neemu_export",
        json!({ "work_id": work_id, "bundle_kind": "training" }),
    );
    assert_eq!(exported["tier"], json!("T"));
    assert_eq!(exported["bundle_kind"], json!("training"));
}

#[test]
fn export_selected_records_subset() {
    let rig = rig("exp-sub", 3);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    let faces = call(
        &rig.neemu,
        "neemu_extract_faces",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "page_start": 0,
            "page_end": 2,
            "observations": [face_observation(0), face_observation(1)],
        }),
    );
    let one = faces["record_ids"][0].clone();
    let exported = call(
        &rig.neemu,
        "neemu_export",
        json!({ "work_id": work_id, "bundle_kind": "analysis", "record_ids": [one] }),
    );
    assert_eq!(exported["record_count"], json!(1));
}

#[test]
fn list_records_pagination() {
    let rig = rig("list-page", 3);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    call(
        &rig.neemu,
        "neemu_extract_faces",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "page_start": 0,
            "page_end": 2,
            "observations": [face_observation(0), face_observation(1), face_observation(2)],
        }),
    );
    let page1 = call(
        &rig.neemu,
        "neemu_list_records",
        json!({ "work_id": work_id, "kind": "face_observation", "limit": 2 }),
    );
    assert_eq!(page1["total"], json!(3));
    assert_eq!(page1["records"].as_array().unwrap().len(), 2);
    assert_eq!(page1["partial"], json!(true));
    let next = page1["next_offset"].as_u64().unwrap();
    let page2 = call(
        &rig.neemu,
        "neemu_list_records",
        json!({ "work_id": work_id, "kind": "face_observation", "limit": 2, "offset": next }),
    );
    assert_eq!(page2["records"].as_array().unwrap().len(), 1);
    assert_eq!(page2["partial"], json!(false));
}

#[test]
fn tools_list_surface_contract() {
    let tools = tool_definitions();
    let tools = tools.as_array().expect("tools array");
    assert_eq!(tools.len(), 15);
    let mut names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    names.sort();
    let mut expected: Vec<&str> = TOOL_NAMES.to_vec();
    expected.sort();
    assert_eq!(names, expected);
    for tool in tools {
        let schema = &tool["inputSchema"];
        assert_eq!(schema["type"], json!("object"), "tool {}", tool["name"]);
        assert!(
            schema.get("properties").is_some(),
            "tool {} lacks explicit properties",
            tool["name"]
        );
        assert!(
            tool.get("outputSchema").is_some(),
            "tool {} lacks outputSchema",
            tool["name"]
        );
        let annotations = &tool["annotations"];
        for hint in [
            "readOnlyHint",
            "destructiveHint",
            "idempotentHint",
            "openWorldHint",
        ] {
            assert!(
                annotations.get(hint).is_some(),
                "tool {} lacks annotation {hint}",
                tool["name"]
            );
        }
        assert_eq!(annotations["destructiveHint"], json!(false));
    }
}

#[test]
fn stdio_discover_and_call() {
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Command, Stdio};

    let root = temp_dir("stdio");
    let registry = root.join("registry");
    let media = root.join("media");
    let media_dir = write_pages(&media, 2, 800, 1200);
    let mut child = Command::new(env!("CARGO_BIN_EXE_neemu"))
        .arg("--home")
        .arg(&registry)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn neemu");
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut send = |line: &str| {
        writeln!(stdin, "{line}").unwrap();
        stdin.flush().unwrap();
    };
    let read_json = |stdout: &mut BufReader<std::process::ChildStdout>| -> Value {
        let mut line = String::new();
        stdout.read_line(&mut line).unwrap();
        serde_json::from_str(&line).expect("json response")
    };
    // Modern era (0.2.0, WS2): sessions open with server/discover
    // and every request carries its protocol version in `_meta`;
    // there is no initialize handshake.
    const META: &str = r#""_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"validation","version":"0"},"io.modelcontextprotocol/clientCapabilities":{}}"#;
    send(&format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{{{META}}}}}"#
    ));
    let discover = read_json(&mut stdout);
    assert!(discover.get("result").is_some(), "discover: {discover}");
    send(&format!(
        r#"{{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{{{META}}}}}"#
    ));
    let list = read_json(&mut stdout);
    assert_eq!(list["result"]["tools"].as_array().unwrap().len(), 15);
    let register = format!(
        r#"{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{"name":"neemu_register_work","arguments":{{"title":"Moonlit Wife","source":{{"kind":"owner_supplied","ref":"stdio fixture"}},"media_dir":{},"tier_claim":"A"}},{META}}}}}"#,
        serde_json::to_string(&media_dir).unwrap()
    );
    send(&register);
    let registered = read_json(&mut stdout);
    assert_eq!(registered["result"]["isError"], json!(false));
    let text = registered["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed["work_id"].as_str().unwrap().starts_with("work_"));
    drop(stdin);
    child.wait().unwrap();
}
