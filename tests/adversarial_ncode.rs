//! N-code adversarial suite (D-326 follow-up): the resolution is
//! fail-closed. Unknown or malformed N-codes are refused with the
//! same shape as unknown F-codes; N-05 can never be obtained as a
//! full canon entry; ratified codes cannot re-enter disguised as
//! new candidates; and an unratified code enters ONLY as a
//! candidate, never as canon.

mod common;

use common::*;
use neemu::resolve_face_code;
use serde_json::{json, Value};

/// A grid-read face observation carrying the given code.
fn obs_with_code(page: usize, code: &str) -> Value {
    let mut observation = face_observation(page);
    observation["f_code"] = json!(code);
    observation
}

/// Call extract_faces with one observation and expect an
/// invalid-arguments failure; returns the message.
fn faces_args_err(rig: &Rig, work_id: &str, preflight_id: &str, observation: Value) -> String {
    call_args_err(
        &rig.neemu,
        "neemu_extract_faces",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "page_start": 0,
            "page_end": 0,
            "observations": [observation],
        }),
    )
}

#[test]
fn unknown_n_codes_refused_with_unknown_f_shape() {
    let rig = rig("adv-ncode-unknown", 2);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    for code in ["N-07", "N-99"] {
        let message = faces_args_err(&rig, &work_id, &preflight_id, obs_with_code(0, code));
        assert!(
            message.contains(&format!("unknown f_code {code}")),
            "{code} must be refused as an unknown f_code: {message}"
        );
    }
    // The refusal shape is the F path's own: an unknown F-code
    // produces the same message form.
    let message = faces_args_err(&rig, &work_id, &preflight_id, obs_with_code(0, "F-99"));
    assert!(
        message.contains("unknown f_code F-99"),
        "unknown F shape: {message}"
    );
    assert!(resolve_face_code("N-07").is_none());
    assert!(resolve_face_code("N-99").is_none());
}

#[test]
fn malformed_n_codes_refused() {
    let rig = rig("adv-ncode-malformed", 2);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    // No normalisation is applied to codes: padding, case, and
    // width are exact, so near-miss spellings fail closed rather
    // than resolving by resemblance.
    for code in ["N-5", "n-03", "N-003", "N03", "NN-03"] {
        let message = faces_args_err(&rig, &work_id, &preflight_id, obs_with_code(0, code));
        assert!(
            message.contains(&format!("unknown f_code {code}")),
            "{code} must not resolve: {message}"
        );
        assert!(resolve_face_code(code).is_none(), "{code}");
    }
}

#[test]
fn n05_never_resolves_as_a_full_entry() {
    let resolution = resolve_face_code("N-05").expect("N-05 resolves");
    assert_eq!(resolution.status, "named_variant");
    assert_ne!(resolution.status, "canon");
    // And the minted record says the same: there is no path on
    // which N-05 lands in a record as a full canon entry.
    let rig = rig("adv-ncode-n05", 2);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    let result = call(
        &rig.neemu,
        "neemu_extract_faces",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "page_start": 0,
            "page_end": 0,
            "observations": [obs_with_code(0, "N-05")],
        }),
    );
    let record = rig
        .neemu
        .load_record(
            "records",
            &work_id,
            result["record_ids"][0].as_str().unwrap(),
        )
        .unwrap();
    assert_eq!(
        record["code_resolution"]["registry_status"],
        json!("named_variant")
    );
    assert_ne!(record["code_resolution"]["registry_status"], json!("canon"));
}

#[test]
fn ratified_n_codes_cannot_reenter_as_candidates() {
    let rig = rig("adv-ncode-collision", 2);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    for code in ["N-03", "N-05"] {
        let mut observation = face_observation(0);
        observation["f_code"] = Value::Null;
        observation["new_candidate"] = json!({
            "channels": observation["channels"].clone(),
            "beats": ["induction"],
            "proposed_code": code,
        });
        let message = faces_args_err(&rig, &work_id, &preflight_id, observation);
        assert!(
            message.contains("collides with an existing face code"),
            "proposing ratified {code} as a candidate must refuse: {message}"
        );
    }
}

#[test]
fn unratified_code_enters_only_as_candidate_never_as_canon() {
    // The candidate space stays open (a genuinely new state is
    // first-class output), but it mints as a candidate record —
    // no code_resolution, no canon status — never as canon.
    let rig = rig("adv-ncode-candidate", 2);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    let mut observation = face_observation(0);
    observation["f_code"] = Value::Null;
    observation["new_candidate"] = json!({
        "channels": observation["channels"].clone(),
        "beats": ["induction"],
        "proposed_code": "N-07",
    });
    let result = call(
        &rig.neemu,
        "neemu_extract_faces",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "page_start": 0,
            "page_end": 0,
            "observations": [observation],
        }),
    );
    let record = rig
        .neemu
        .load_record(
            "records",
            &work_id,
            result["record_ids"][0].as_str().unwrap(),
        )
        .unwrap();
    assert_eq!(record["f_code"], Value::Null);
    assert_eq!(record["new_candidate"]["proposed_code"], json!("N-07"));
    assert!(
        record.get("code_resolution").is_none(),
        "a candidate must not carry a canon resolution: {record}"
    );
}
