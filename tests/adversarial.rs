//! Adversarial suite (the adversarial half of the estate 50/50 split):
//! the S1 gate under attack — extraction without screening or
//! pre-flight, tier breaches, blocklist evasion attempts, malformed
//! and oversized inputs. Every case expects a typed refusal or an
//! invalid-arguments failure, never a silent success.

mod common;

use common::*;
use serde_json::{json, Value};
use std::fs;

#[test]
fn extract_faces_without_preflight_refused() {
    let rig = rig("adv-nopf-faces", 2);
    let (work_id, _preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    let kind = call_kind(
        &rig.neemu,
        "neemu_extract_faces",
        json!({
            "work_id": work_id,
            "page_start": 0,
            "page_end": 1,
            "observations": [face_observation(0)],
        }),
    );
    assert_eq!(kind, "refusal_not_screened");
}

#[test]
fn extract_dialogue_without_preflight_refused() {
    let rig = rig("adv-nopf-dlg", 2);
    let (work_id, _preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    let kind = call_kind(
        &rig.neemu,
        "neemu_extract_dialogue",
        json!({
            "work_id": work_id,
            "page_start": 0,
            "page_end": 1,
            "lines": [{ "page": 0, "speaker": "lead", "text": "Deeper" }],
        }),
    );
    assert_eq!(kind, "refusal_not_screened");
}

#[test]
fn segment_without_preflight_refused() {
    let rig = rig("adv-nopf-seg", 2);
    let (work_id, _preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    let kind = call_kind(
        &rig.neemu,
        "neemu_segment_scene",
        json!({ "work_id": work_id }),
    );
    assert_eq!(kind, "refusal_not_screened");
}

#[test]
fn extract_on_unscreened_work_refused() {
    let rig = rig("adv-unscreened", 2);
    let (work_id, preflight_id) = register_and_validate(&rig, "Moonlit Wife", "A");
    let kind = call_kind(
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
    assert_eq!(kind, "screen_refusal");
}

#[test]
fn extract_on_excluded_work_refused() {
    let rig = rig("adv-excluded", 2);
    let (work_id, preflight_id) = register_and_validate(&rig, "Schoolgirl Diary", "A");
    let screen = call(
        &rig.neemu,
        "neemu_screen_work",
        json!({
            "work_id": work_id,
            "evidence": {
                "adult_basis": [{ "kind": "age_statement", "detail": "claims adults" }]
            }
        }),
    );
    assert_eq!(screen["determination"], json!("excluded"));
    let kind = call_kind(
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
    assert_eq!(kind, "screen_refusal");
}

#[test]
fn preflight_from_another_work_refused() {
    let rig = rig("adv-crosspf", 2);
    let (_work_a, preflight_a) = cleared_work(&rig, "Moonlit Wife", "A");
    // A second, distinct work in the same registry.
    let media_b = rig.root.join("media-b");
    let media_b_dir = write_pages(&media_b, 2, 640, 960);
    let work_b = call(
        &rig.neemu,
        "neemu_register_work",
        json!({
            "title": "Other Work",
            "source": { "kind": "owner_supplied", "ref": "fixture" },
            "media_dir": media_b_dir,
            "tier_claim": "A",
        }),
    );
    let work_b_id = work_b["work_id"].as_str().unwrap().to_string();
    let preflight_b = call(
        &rig.neemu,
        "neemu_validate_ingest",
        json!({ "work_id": work_b_id }),
    );
    assert_eq!(preflight_b["passed"], json!(true));
    screen_cleared(&rig.neemu, &work_b_id);
    let kind = call_kind(
        &rig.neemu,
        "neemu_extract_faces",
        json!({
            "work_id": work_b_id,
            "preflight_id": preflight_a,
            "page_start": 0,
            "page_end": 1,
            "observations": [face_observation(0)],
        }),
    );
    assert_eq!(kind, "refusal_not_screened");
}

#[test]
fn unknown_preflight_id_refused() {
    let rig = rig("adv-unknownpf", 2);
    let (work_id, _preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    let kind = call_kind(
        &rig.neemu,
        "neemu_extract_faces",
        json!({
            "work_id": work_id,
            "preflight_id": "preflight_00000000000000000000000000000000",
            "page_start": 0,
            "page_end": 1,
            "observations": [face_observation(0)],
        }),
    );
    assert_eq!(kind, "refusal_not_screened");
}

#[test]
fn tampered_media_fails_preflight_and_extraction() {
    let rig = rig("adv-tamper", 2);
    let (work_id, _good_preflight) = cleared_work(&rig, "Moonlit Wife", "A");
    // Tamper with a page after registration + the good preflight.
    let page = format!("{}/page-001.png", rig.media_dir);
    fs::write(&page, png_bytes(800, 1200, 99)).unwrap();
    let preflight = call(
        &rig.neemu,
        "neemu_validate_ingest",
        json!({ "work_id": work_id }),
    );
    assert_eq!(preflight["passed"], json!(false));
    assert_eq!(preflight["checks"]["hashes_match"], json!(false));
    let bad_id = preflight["record_id"].as_str().unwrap().to_string();
    let kind = call_kind(
        &rig.neemu,
        "neemu_extract_faces",
        json!({
            "work_id": work_id,
            "preflight_id": bad_id,
            "page_start": 0,
            "page_end": 1,
            "observations": [face_observation(0)],
        }),
    );
    assert_eq!(kind, "refusal_not_screened");
}

#[test]
fn tier_a_into_training_bundle_refused() {
    let rig = rig("adv-tiertrain", 2);
    let (work_id, _preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    let error = match neemu::tools::call_tool(
        &rig.neemu,
        "neemu_export",
        &json!({ "work_id": work_id, "bundle_kind": "training" }),
    ) {
        Err(neemu::CallError::Gate(error)) => error,
        other => panic!("expected tier refusal, got {other:?}"),
    };
    assert_eq!(error.kind, "tier_refusal");
    assert_eq!(error.details["blocking_works"][0], json!(work_id));
}

#[test]
fn register_tier_t_without_evidence_refused() {
    let rig = rig("adv-tier-noev", 2);
    let kind = call_kind(
        &rig.neemu,
        "neemu_register_work",
        json!({
            "title": "Unlicensed Work",
            "source": { "kind": "census", "ref": "fixture" },
            "media_dir": rig.media_dir,
            "tier_claim": "T",
        }),
    );
    assert_eq!(kind, "tier_refusal");
}

#[test]
fn blocklist_title_excludes_despite_adult_evidence() {
    let rig = rig("adv-blocklist", 2);
    let (work_id, _) = register_and_validate(&rig, "Schoolgirl Diary", "A");
    let screen = call(
        &rig.neemu,
        "neemu_screen_work",
        json!({
            "work_id": work_id,
            "evidence": {
                "adult_basis": [{ "kind": "age_statement", "detail": "states adults" }]
            }
        }),
    );
    assert_eq!(screen["determination"], json!("excluded"));
    assert_eq!(screen["blocklist_hit"], json!("schoolgirl"));
    let reasons = screen["reasons"].as_array().unwrap();
    assert!(reasons
        .iter()
        .any(|r| r.as_str().unwrap().contains("blocklist term: schoolgirl")));
}

#[test]
fn school_coded_flag_excludes_with_clean_title() {
    // The S1 gate case from SPEC §7: school-coded evidence excludes
    // even when the title passes the blocklist clean.
    let rig = rig("adv-schoolflag", 2);
    let (work_id, _) = register_and_validate(&rig, "Moonlit Wife", "A");
    let screen = call(
        &rig.neemu,
        "neemu_screen_work",
        json!({
            "work_id": work_id,
            "evidence": {
                "adult_basis": [{ "kind": "series_context", "detail": "adult cast claimed" }],
                "school_coded": true
            }
        }),
    );
    assert_eq!(screen["determination"], json!("excluded"));
    assert_eq!(screen["blocklist_hit"], Value::Null);
    let reasons = screen["reasons"].as_array().unwrap();
    assert!(reasons
        .iter()
        .any(|r| r.as_str().unwrap().contains("school_coded")));
}

#[test]
fn empty_adult_basis_excludes() {
    let rig = rig("adv-noevidence", 2);
    let (work_id, _) = register_and_validate(&rig, "Moonlit Wife", "A");
    let screen = call(
        &rig.neemu,
        "neemu_screen_work",
        json!({ "work_id": work_id, "evidence": {} }),
    );
    assert_eq!(screen["determination"], json!("excluded"));
    let reasons = screen["reasons"].as_array().unwrap();
    assert!(reasons
        .iter()
        .any(|r| r.as_str().unwrap().contains("insufficient_evidence")));
}

#[test]
fn age_readable_flag_excludes() {
    let rig = rig("adv-ageflag", 2);
    let (work_id, _) = register_and_validate(&rig, "Moonlit Wife", "A");
    let screen = call(
        &rig.neemu,
        "neemu_screen_work",
        json!({
            "work_id": work_id,
            "evidence": {
                "adult_basis": [{ "kind": "age_statement", "detail": "states adults" }],
                "age_readable_under_18": true
            }
        }),
    );
    assert_eq!(screen["determination"], json!("excluded"));
}

#[test]
fn too_many_pages_register_bound_exceeded() {
    let root = temp_dir("adv-toomany");
    let media = root.join("media");
    let media_dir = write_pages(&media, neemu::MAX_MEDIA_FILES + 1, 100, 100);
    let neemu = neemu::Neemu::open(root.join("registry")).unwrap();
    let kind = call_kind(
        &neemu,
        "neemu_register_work",
        json!({
            "title": "Huge Work",
            "source": { "kind": "owner_supplied", "ref": "fixture" },
            "media_dir": media_dir,
            "tier_claim": "A",
        }),
    );
    assert_eq!(kind, "bound_exceeded");
}

#[test]
fn too_many_observations_bound_exceeded() {
    let rig = rig("adv-tooobs", 2);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    let observations: Vec<Value> = (0..=neemu::MAX_OBSERVATIONS_PER_CALL)
        .map(|_| face_observation(0))
        .collect();
    let kind = call_kind(
        &rig.neemu,
        "neemu_extract_faces",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "page_start": 0,
            "page_end": 1,
            "observations": observations,
        }),
    );
    assert_eq!(kind, "bound_exceeded");
}

#[test]
fn oversized_dialogue_text_is_args_error() {
    let rig = rig("adv-bigtext", 2);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    let big = "x".repeat(neemu::MAX_TEXT_CHARS + 1);
    let message = call_args_err(
        &rig.neemu,
        "neemu_extract_dialogue",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "page_start": 0,
            "page_end": 1,
            "lines": [{ "page": 0, "speaker": "lead", "text": big }],
        }),
    );
    assert!(message.contains("exceeds"));
}

#[test]
fn unknown_f_code_is_args_error() {
    let rig = rig("adv-fcode", 2);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    let mut observation = face_observation(0);
    observation["f_code"] = json!("F-99");
    call_args_err(
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
}

#[test]
fn null_f_code_without_candidate_is_args_error() {
    // New states are first-class (SPEC §3.1) — but a null f_code with
    // no candidate block would silently drop the observation.
    let rig = rig("adv-nocand", 2);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    let mut observation = face_observation(0);
    observation["f_code"] = Value::Null;
    let message = call_args_err(
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
    assert!(message.contains("new_candidate"));
}

#[test]
fn incomplete_channels_is_args_error() {
    let rig = rig("adv-channels", 2);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    let mut observation = face_observation(0);
    observation["channels"]
        .as_object_mut()
        .unwrap()
        .remove("flush");
    call_args_err(
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
}

#[test]
fn model_estimate_fer_lineage_on_tier_t_refused() {
    let rig = rig("adv-fer", 2);
    let (work_id, preflight_id) = cleared_work(&rig, "Licensed Work", "T");
    let mut observation = face_observation(0);
    observation["basis"] = json!("model-estimate");
    observation["model_lineage"] = json!({
        "code_license": "MIT",
        "weights_license": "not stated",
        "training_data_licenses": ["non-commercial"],
        "trained_on": ["AffectNet"],
    });
    let kind = call_kind(
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
    assert_eq!(kind, "tier_refusal");
}

#[test]
fn model_estimate_without_lineage_is_args_error() {
    let rig = rig("adv-nolineage", 2);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    let mut observation = face_observation(0);
    observation["basis"] = json!("model-estimate");
    call_args_err(
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
}

#[test]
fn unknown_work_refused_everywhere() {
    let rig = rig("adv-unknownwork", 2);
    for (name, args) in [
        ("neemu_validate_ingest", json!({ "work_id": "work_nope" })),
        (
            "neemu_screen_work",
            json!({ "work_id": "work_nope", "evidence": {} }),
        ),
        ("neemu_list_records", json!({ "work_id": "work_nope" })),
        (
            "neemu_export",
            json!({ "work_id": "work_nope", "bundle_kind": "analysis" }),
        ),
        (
            "neemu_segment_scene",
            json!({ "work_id": "work_nope", "preflight_id": "preflight_x" }),
        ),
    ] {
        let kind = call_kind(&rig.neemu, name, args);
        assert_eq!(kind, "screen_refusal", "tool {name}");
    }
}

#[test]
fn excluded_work_list_faces_refused() {
    let rig = rig("adv-listexcl", 2);
    let (work_id, _) = register_and_validate(&rig, "Schoolgirl Diary", "A");
    call(
        &rig.neemu,
        "neemu_screen_work",
        json!({ "work_id": work_id, "evidence": {} }),
    );
    let kind = call_kind(
        &rig.neemu,
        "neemu_list_records",
        json!({ "work_id": work_id, "kind": "face_observation" }),
    );
    assert_eq!(kind, "screen_refusal");
    // …while the gate trail itself stays inspectable.
    let screens = call(
        &rig.neemu,
        "neemu_list_records",
        json!({ "work_id": work_id, "kind": "screen" }),
    );
    assert_eq!(screens["total"], json!(1));
}

#[test]
fn export_on_excluded_work_refused() {
    let rig = rig("adv-exportexcl", 2);
    let (work_id, _) = register_and_validate(&rig, "Schoolgirl Diary", "A");
    call(
        &rig.neemu,
        "neemu_screen_work",
        json!({ "work_id": work_id, "evidence": {} }),
    );
    let kind = call_kind(
        &rig.neemu,
        "neemu_export",
        json!({ "work_id": work_id, "bundle_kind": "analysis" }),
    );
    assert_eq!(kind, "screen_refusal");
}

#[test]
fn unknown_tool_is_args_error() {
    let rig = rig("adv-unknowntool", 1);
    call_args_err(&rig.neemu, "neemu_extract_acts", json!({}));
}

#[test]
fn empty_media_dir_is_media_unreadable() {
    let root = temp_dir("adv-emptydir");
    let media = root.join("media");
    fs::create_dir_all(&media).unwrap();
    let neemu = neemu::Neemu::open(root.join("registry")).unwrap();
    let kind = call_kind(
        &neemu,
        "neemu_register_work",
        json!({
            "title": "Empty Work",
            "source": { "kind": "owner_supplied", "ref": "fixture" },
            "media_dir": media.to_string_lossy(),
            "tier_claim": "A",
        }),
    );
    assert_eq!(kind, "media_unreadable");
}

#[test]
fn dialogue_face_ref_unknown_is_args_error() {
    let rig = rig("adv-badfaceref", 2);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    call_args_err(
        &rig.neemu,
        "neemu_extract_dialogue",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "page_start": 0,
            "page_end": 1,
            "lines": [
                { "page": 0, "speaker": "lead", "text": "Deeper", "face_ref": "face_00000000000000000000000000000000" }
            ],
        }),
    );
}

#[test]
fn range_beyond_work_is_args_error() {
    let rig = rig("adv-range", 2);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    call_args_err(
        &rig.neemu,
        "neemu_extract_faces",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "page_start": 0,
            "page_end": 9,
            "observations": [],
        }),
    );
}
