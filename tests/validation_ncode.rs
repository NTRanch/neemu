//! N-code validation suite (D-326 follow-up): the six ratified
//! Marsha N-codes resolve server-side through the same face-code
//! lookup as the F-family — in `neemu_extract_faces`, in act face
//! timelines, and identically across work tiers (the canon entry's
//! lane labels are provenance, never relabelled by the caller).

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

/// Mint one face observation with `code` on a cleared rig and
/// return the stored record.
fn mint_face(rig: &Rig, work_id: &str, preflight_id: &str, code: &str) -> Value {
    let result = call(
        &rig.neemu,
        "neemu_extract_faces",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "page_start": 0,
            "page_end": 0,
            "observations": [obs_with_code(0, code)],
        }),
    );
    assert_eq!(result["count"], json!(1), "minting {code}: {result}");
    let record_id = result["record_ids"][0].as_str().unwrap();
    rig.neemu
        .load_record("records", work_id, record_id)
        .expect("face record stored")
}

// ---- The resolver itself ----

#[test]
fn resolver_resolves_all_six_n_codes_with_ratified_status() {
    let expected = [
        ("N-01", "canon", "Defiant snarl (resistance)"),
        ("N-02", "canon", "Scheming smirk (predatory confidence)"),
        ("N-03", "canon", "Compelled-compliance shock"),
        ("N-04", "canon", "Gritted defiance under stimulation"),
        (
            "N-05",
            "named_variant",
            "White-roll peak (pupil dissolution)",
        ),
        ("N-06", "canon", "Dominant predator grin (dark-form)"),
    ];
    for (code, status, name) in expected {
        let resolution = resolve_face_code(code).expect("N-code resolves");
        assert_eq!(resolution.family, "N", "{code}");
        assert_eq!(resolution.status, status, "{code}");
        assert_eq!(resolution.name, Some(name), "{code}");
    }
    // The F-family resolves through the same lookup.
    let f05 = resolve_face_code("F-05").expect("F-code resolves");
    assert_eq!(f05.family, "F");
    assert_eq!(f05.status, "canon");
}

// ---- extract_faces ----

#[test]
fn n_code_observations_mint_with_resolution_snapshot() {
    let rig = rig("ncode-mint", 2);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    for code in ["N-01", "N-02", "N-03", "N-04", "N-06"] {
        let record = mint_face(&rig, &work_id, &preflight_id, code);
        assert_eq!(record["f_code"], json!(code));
        let snapshot = &record["code_resolution"];
        assert_eq!(snapshot["code"], json!(code), "{code}");
        assert_eq!(snapshot["family"], json!("N"), "{code}");
        assert_eq!(snapshot["registry_status"], json!("canon"), "{code}");
        assert_eq!(snapshot["origin_lane"], json!("ntranch"), "{code}");
        assert_eq!(snapshot["content_label"], json!("nsfw"), "{code}");
        assert!(
            snapshot["source"].as_str().unwrap().contains("D-326"),
            "{code} snapshot names its ratification: {snapshot}"
        );
    }
}

#[test]
fn n05_observation_mints_as_named_variant() {
    let rig = rig("ncode-n05", 2);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    let record = mint_face(&rig, &work_id, &preflight_id, "N-05");
    assert_eq!(record["f_code"], json!("N-05"));
    assert_eq!(
        record["code_resolution"]["registry_status"],
        json!("named_variant")
    );
    assert_eq!(
        record["code_resolution"]["name"],
        json!("White-roll peak (pupil dissolution)")
    );
}

#[test]
fn f_code_observations_keep_pre_n_record_shape() {
    // Parity guard: F-code records gain no code_resolution key, so
    // historical F observations re-mint identical record bodies.
    let rig = rig("ncode-f-parity", 2);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    let record = mint_face(&rig, &work_id, &preflight_id, "F-03");
    assert_eq!(record["f_code"], json!("F-03"));
    assert!(
        record.get("code_resolution").is_none(),
        "F record must not carry a resolution block: {record}"
    );
}

#[test]
fn n_code_resolution_is_identical_across_work_tiers() {
    // Cross-lane parity with the F path: code validity never
    // depended on the work's tier, and the canon entry's lane
    // labels ride the record unchanged in both tiers — a Tier T
    // caller neither relabels nor is refused the N-canon ref.
    let rig_a = rig("ncode-tier-a", 2);
    let (work_a, preflight_a) = cleared_work(&rig_a, "Moonlit Wife", "A");
    let record_a = mint_face(&rig_a, &work_a, &preflight_a, "N-03");

    let rig_t = rig("ncode-tier-t", 2);
    let (work_t, preflight_t) = cleared_work(&rig_t, "Moonlit Wife", "T");
    let record_t = mint_face(&rig_t, &work_t, &preflight_t, "N-03");

    assert_eq!(
        record_a["code_resolution"], record_t["code_resolution"],
        "resolution snapshot must not vary with the caller's tier"
    );
    assert_eq!(record_t["code_resolution"]["origin_lane"], json!("ntranch"));
    assert_eq!(record_t["code_resolution"]["content_label"], json!("nsfw"));
}

// ---- Act face timelines ----

/// A minimal valid act (mirrors the S2 suite's shape).
fn simple_act(code: &str) -> Value {
    json!({
        "act_code": code,
        "participants": [
            { "slot": "lead", "role": "receiver" },
            { "slot": "partner", "role": "giver" }
        ],
        "contact_pairs": [
            {
                "a": { "slot": "partner", "part": "hand", "collider": "sphere" },
                "b": { "slot": "lead", "part": "cheek", "collider": "surface" },
                "compliance_class": "attachment"
            }
        ]
    })
}

#[test]
fn act_face_timeline_resolves_n_codes() {
    let rig = rig("ncode-timeline", 4);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    let scenes = call(
        &rig.neemu,
        "neemu_segment_scene",
        json!({ "work_id": work_id, "preflight_id": preflight_id }),
    );
    let scene_id = scenes["scenes"][0]["scene_id"].as_str().unwrap();
    let mut act = simple_act("B-16");
    act["face_timeline"] = json!([
        { "beat": "setup", "ref": "N-03" },
        { "beat": "peak", "ref": "N-05" },
        { "beat": "release", "ref": "F-05" }
    ]);
    let result = call(
        &rig.neemu,
        "neemu_extract_acts",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "scene_id": scene_id,
            "acts": [act],
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
    let timeline = &record["face_timeline"];
    assert_eq!(timeline[0]["f_code"], json!("N-03"));
    assert_eq!(
        timeline[0]["code_resolution"]["registry_status"],
        json!("canon")
    );
    assert_eq!(timeline[1]["f_code"], json!("N-05"));
    assert_eq!(
        timeline[1]["code_resolution"]["registry_status"],
        json!("named_variant")
    );
    // F entries keep their pre-N shape.
    assert_eq!(timeline[2]["f_code"], json!("F-05"));
    assert!(timeline[2].get("code_resolution").is_none());
}
