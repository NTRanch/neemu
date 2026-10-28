//! S2 adversarial suite: the gate cases — unmapped acts kept and
//! labelled, unlinked understanding claims refused with the typed
//! kind, and enrichment that can never move a determination
//! (SPEC §7 S2 gate, adversarial half).

mod common;

use common::*;
use serde_json::{json, Value};

fn scenes(rig: &Rig, work_id: &str, preflight_id: &str) -> Vec<Value> {
    let result = call(
        &rig.neemu,
        "neemu_segment_scene",
        json!({ "work_id": work_id, "preflight_id": preflight_id }),
    );
    result["scenes"].as_array().expect("scenes").clone()
}

fn cleared_with_face(tag: &str) -> (Rig, String, String, String, String) {
    let rig = rig(tag, 4);
    let (work_id, preflight_id) = cleared_work(&rig, "Moonlit Wife", "A");
    let scene_id = scenes(&rig, &work_id, &preflight_id)[0]["scene_id"]
        .as_str()
        .unwrap()
        .to_string();
    let faces = call(
        &rig.neemu,
        "neemu_extract_faces",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "page_start": 0,
            "page_end": 0,
            "observations": [face_observation(0)],
        }),
    );
    let face_id = faces["record_ids"][0].as_str().unwrap().to_string();
    (rig, work_id, preflight_id, scene_id, face_id)
}

fn simple_act(code: &str) -> Value {
    json!({
        "act_code": code,
        "participants": [{ "slot": "lead", "role": "receiver" }],
        "contact_pairs": []
    })
}

fn acts_args(work_id: &str, preflight_id: &str, scene_id: &str, acts: Value) -> Value {
    json!({
        "work_id": work_id,
        "preflight_id": preflight_id,
        "scene_id": scene_id,
        "acts": acts,
    })
}

fn understanding_with_beats(
    work_id: &str,
    preflight_id: &str,
    scene_id: &str,
    beats: Value,
    face_id: &str,
) -> Value {
    json!({
        "work_id": work_id,
        "preflight_id": preflight_id,
        "scene_id": scene_id,
        "beats": beats,
        "machinery": {
            "mechanism": "voice",
            "stage_model": "spoken induction",
            "reset_wakener": "present",
            "reset_detail": "a spoken name",
            "evidence_refs": [face_id]
        },
        "dynamics": {
            "roles": [{ "slot": "lead", "role": "subject" }],
            "who_directs": "partner",
            "who_responds": "lead",
            "evidence_refs": [face_id]
        }
    })
}

// ---- Act gate chain ----

#[test]
fn acts_on_excluded_work_refused() {
    let rig = rig("s2-adv-acts-excluded", 2);
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
        "neemu_extract_acts",
        acts_args(
            &work_id,
            &preflight_id,
            "scene_whatever",
            json!([simple_act("B-16")]),
        ),
    );
    assert_eq!(kind, "screen_refusal");
}

#[test]
fn acts_without_preflight_refused() {
    let rig = rig("s2-adv-acts-nopf", 2);
    let (work_id, _preflight_id) = register_and_validate(&rig, "Moonlit Wife", "A");
    screen_cleared(&rig.neemu, &work_id);
    let kind = call_kind(
        &rig.neemu,
        "neemu_extract_acts",
        json!({
            "work_id": work_id,
            "scene_id": "scene_whatever",
            "acts": [simple_act("B-16")],
        }),
    );
    assert_eq!(kind, "refusal_not_screened");
}

#[test]
fn acts_unknown_scene_is_args_error() {
    let (rig, work_id, preflight_id, _, _) = cleared_with_face("s2-adv-acts-scene");
    let message = call_args_err(
        &rig.neemu,
        "neemu_extract_acts",
        acts_args(
            &work_id,
            &preflight_id,
            "scene_forged",
            json!([simple_act("B-16")]),
        ),
    );
    assert!(message.contains("not a deterministic scene"), "{message}");
    // Nothing was minted against the forged scene.
    let list = call(
        &rig.neemu,
        "neemu_list_records",
        json!({ "work_id": work_id, "kind": "act_record" }),
    );
    assert_eq!(list["total"], json!(0));
}

// ---- Unmapped / candidate discipline (S2 gate) ----

#[test]
fn acts_unmapped_code_kept_and_labelled() {
    let (rig, work_id, preflight_id, scene_id, _) = cleared_with_face("s2-adv-unmapped");
    // B-13 (blindfold) is a real registry code with no W6 family and
    // no stated stratum: it must be recorded, labelled unmapped,
    // never dropped.
    let result = call(
        &rig.neemu,
        "neemu_extract_acts",
        acts_args(
            &work_id,
            &preflight_id,
            &scene_id,
            json!([simple_act("B-13")]),
        ),
    );
    assert_eq!(result["count"], json!(1));
    assert_eq!(result["unmapped_act_codes"], json!(["B-13"]));
    assert_eq!(result["candidate_act_codes"], json!([]));
    let record = rig
        .neemu
        .load_record(
            "records",
            &work_id,
            result["record_ids"][0].as_str().unwrap(),
        )
        .unwrap();
    assert_eq!(record["registry"]["mapping"], json!("unmapped"));
    assert_eq!(record["registry"]["disposition"], json!("modifier"));
    assert_eq!(record["registry"]["registry_status"], json!("canon"));
    let list = call(
        &rig.neemu,
        "neemu_list_records",
        json!({ "work_id": work_id, "kind": "act_record" }),
    );
    assert_eq!(list["total"], json!(1));
}

#[test]
fn acts_unknown_code_becomes_candidate_not_canon() {
    let (rig, work_id, preflight_id, scene_id, _) = cleared_with_face("s2-adv-candidate");
    let result = call(
        &rig.neemu,
        "neemu_extract_acts",
        acts_args(
            &work_id,
            &preflight_id,
            &scene_id,
            json!([simple_act("B-99")]),
        ),
    );
    assert_eq!(result["count"], json!(1));
    assert_eq!(result["candidate_act_codes"], json!(["B-99"]));
    assert_eq!(result["unmapped_act_codes"], json!(["B-99"]));
    let record = rig
        .neemu
        .load_record(
            "records",
            &work_id,
            result["record_ids"][0].as_str().unwrap(),
        )
        .unwrap();
    assert_eq!(record["registry"]["registry_status"], json!("candidate"));
    assert_eq!(record["registry"]["mapping"], json!("unmapped"));
    assert_eq!(record["registry"]["name"], Value::Null);
}

// ---- Contact vocabulary discipline ----

#[test]
fn acts_bad_collider_vocabulary_refused() {
    let (rig, work_id, preflight_id, scene_id, _) = cleared_with_face("s2-adv-collider");
    let mut act = simple_act("B-16");
    act["contact_pairs"] = json!([{
        "a": { "slot": "partner", "part": "hand", "collider": "laser" },
        "b": { "slot": "lead", "part": "cheek", "collider": "surface" },
        "compliance_class": "attachment"
    }]);
    let message = call_args_err(
        &rig.neemu,
        "neemu_extract_acts",
        acts_args(&work_id, &preflight_id, &scene_id, json!([act])),
    );
    assert!(message.contains("D-076 vocabulary"), "{message}");
}

#[test]
fn acts_bad_compliance_class_refused() {
    let (rig, work_id, preflight_id, scene_id, _) = cleared_with_face("s2-adv-compliance");
    let mut act = simple_act("B-16");
    act["contact_pairs"] = json!([{
        "a": { "slot": "partner", "part": "hand", "collider": "sphere" },
        "b": { "slot": "lead", "part": "cheek", "collider": "surface" },
        "compliance_class": "velcro"
    }]);
    let message = call_args_err(
        &rig.neemu,
        "neemu_extract_acts",
        acts_args(&work_id, &preflight_id, &scene_id, json!([act])),
    );
    assert!(message.contains("D-076 vocabulary"), "{message}");
}

#[test]
fn acts_dangling_face_timeline_ref_refused() {
    let (rig, work_id, preflight_id, scene_id, _) = cleared_with_face("s2-adv-timeline");
    let mut act = simple_act("B-16");
    act["face_timeline"] = json!([{ "beat": "act", "ref": "face_doesnotexist" }]);
    let kind = call_kind(
        &rig.neemu,
        "neemu_extract_acts",
        acts_args(&work_id, &preflight_id, &scene_id, json!([act])),
    );
    assert_eq!(kind, "evidence_refusal");
}

#[test]
fn acts_over_bound_refused() {
    let (rig, work_id, preflight_id, scene_id, _) = cleared_with_face("s2-adv-actbound");
    let many: Vec<Value> = (0..17).map(|_| simple_act("B-16")).collect();
    let kind = call_kind(
        &rig.neemu,
        "neemu_extract_acts",
        acts_args(&work_id, &preflight_id, &scene_id, json!(many)),
    );
    assert_eq!(kind, "bound_exceeded");
}

// ---- Understanding evidence discipline (S2 gate) ----

#[test]
fn understand_beat_without_evidence_refs_refused() {
    let (rig, work_id, preflight_id, scene_id, face_id) = cleared_with_face("s2-adv-und-noref");
    let beats = json!([{ "type": "act", "page_start": 0, "page_end": 1, "evidence_refs": [] }]);
    let kind = call_kind(
        &rig.neemu,
        "neemu_understand_scene",
        understanding_with_beats(&work_id, &preflight_id, &scene_id, beats, &face_id),
    );
    assert_eq!(kind, "evidence_refusal");
    let list = call(
        &rig.neemu,
        "neemu_list_records",
        json!({ "work_id": work_id, "kind": "scene_understanding" }),
    );
    assert_eq!(list["total"], json!(0));
}

#[test]
fn understand_beat_dangling_evidence_ref_refused() {
    let (rig, work_id, preflight_id, scene_id, face_id) = cleared_with_face("s2-adv-und-dangle");
    let beats = json!([{
        "type": "act",
        "page_start": 0,
        "page_end": 1,
        "evidence_refs": ["face_deadbeefdeadbeefdeadbeefdeadbeef"]
    }]);
    let kind = call_kind(
        &rig.neemu,
        "neemu_understand_scene",
        understanding_with_beats(&work_id, &preflight_id, &scene_id, beats, &face_id),
    );
    assert_eq!(kind, "evidence_refusal");
}

#[test]
fn understand_ref_from_another_work_refused() {
    // Two works in ONE registry need distinct media: work ids are
    // content-derived from the media manifest, so the same pages
    // would be the same work.
    let root = temp_dir("s2-adv-und-crosswork");
    let media_a = root.join("media-a");
    let media_b = root.join("media-b");
    let dir_a = write_pages(&media_a, 4, 800, 1200);
    let dir_b = write_pages(&media_b, 3, 1024, 768);
    let neemu = neemu::Neemu::open(root.join("registry")).expect("open registry");
    let rig = Rig {
        neemu,
        root: root.clone(),
        media_dir: dir_a,
    };
    let (work_a, pf_a) = cleared_work(&rig, "Moonlit Wife", "A");
    let rig_b = Rig {
        neemu: neemu::Neemu::open(root.join("registry")).expect("reopen registry"),
        root: root.clone(),
        media_dir: dir_b,
    };
    let (work_b, pf_b) = cleared_work(&rig_b, "Harbour Wife", "A");
    assert_ne!(work_a, work_b);
    let rig = rig_b;
    let faces = call(
        &rig.neemu,
        "neemu_extract_faces",
        json!({
            "work_id": work_a,
            "preflight_id": pf_a,
            "page_start": 0,
            "page_end": 0,
            "observations": [face_observation(0)],
        }),
    );
    let foreign_face = faces["record_ids"][0].as_str().unwrap().to_string();
    let scene_b = scenes(&rig, &work_b, &pf_b)[0]["scene_id"]
        .as_str()
        .unwrap()
        .to_string();
    // The ref exists - under work A. Cited for work B it dangles.
    let beats = json!([{
        "type": "act",
        "page_start": 0,
        "page_end": 1,
        "evidence_refs": [foreign_face]
    }]);
    let kind = call_kind(
        &rig.neemu,
        "neemu_understand_scene",
        understanding_with_beats(&work_b, &pf_b, &scene_b, beats, &foreign_face),
    );
    assert_eq!(kind, "evidence_refusal");
}

#[test]
fn understand_locus_outside_scene_refused() {
    let (rig, work_id, preflight_id, scene_id, face_id) = cleared_with_face("s2-adv-und-locus");
    // The scene covers pages 0..=3; a beat reaching page 9 escapes it.
    let beats = json!([{
        "type": "act",
        "page_start": 0,
        "page_end": 9,
        "evidence_refs": [face_id]
    }]);
    let message = call_args_err(
        &rig.neemu,
        "neemu_understand_scene",
        understanding_with_beats(&work_id, &preflight_id, &scene_id, beats, &face_id),
    );
    assert!(message.contains("outside the scene"), "{message}");
}

#[test]
fn understand_forged_scene_refused() {
    let (rig, work_id, preflight_id, _, face_id) = cleared_with_face("s2-adv-und-forge");
    let beats = json!([{
        "type": "act",
        "page_start": 0,
        "page_end": 1,
        "evidence_refs": [face_id]
    }]);
    // Delta 12: understanding may not create a scene by naming one.
    let message = call_args_err(
        &rig.neemu,
        "neemu_understand_scene",
        understanding_with_beats(&work_id, &preflight_id, "scene_invented", beats, &face_id),
    );
    assert!(message.contains("not a deterministic scene"), "{message}");
}

#[test]
fn understand_bad_beat_type_refused() {
    let (rig, work_id, preflight_id, scene_id, face_id) = cleared_with_face("s2-adv-und-beat");
    let beats = json!([{
        "type": "climax",
        "page_start": 0,
        "page_end": 1,
        "evidence_refs": [face_id]
    }]);
    let message = call_args_err(
        &rig.neemu,
        "neemu_understand_scene",
        understanding_with_beats(&work_id, &preflight_id, &scene_id, beats, &face_id),
    );
    assert!(message.contains("§3.6 vocabulary"), "{message}");
}

// ---- Enrichment can never move a determination (S2 gate) ----

fn benign_bundle() -> Value {
    json!({
        "sources": [
            {
                "source": "vndb",
                "match_key": "Wholesome Adult Romance",
                "basis": "resolved_record",
                "tags": ["adult", "romance"],
                "payload": { "id": 1 }
            },
            {
                "source": "official-store",
                "match_key": "Wholesome Adult Romance",
                "basis": "resolved_record",
                "tags": ["adult"],
                "payload": { "id": 2 }
            }
        ]
    })
}

#[test]
fn enrich_cannot_clear_exclude() {
    let rig = rig("s2-adv-enr-exclude", 2);
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
    // A fully corroborated, benign identity bundle lands - as an
    // advisory record only.
    let mut args = benign_bundle();
    args["work_id"] = json!(work_id);
    let bundle = call(&rig.neemu, "neemu_enrich", args);
    assert_eq!(bundle["identity"]["pinned"], json!(true));
    assert_eq!(bundle["determination_effect"], json!("none"));
    // The screen stands, untouched.
    let work = rig.neemu.load_work(&work_id).unwrap();
    assert_eq!(work["determination"]["determination"], json!("excluded"));
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
    // And the bundle itself stays listable on the excluded work
    // (advisory records are part of the inspectable trail).
    let list = call(
        &rig.neemu,
        "neemu_list_records",
        json!({ "work_id": work_id, "kind": "enrichment_bundle" }),
    );
    assert_eq!(list["total"], json!(1));
}

#[test]
fn enrich_cannot_override_cleared_screen_either() {
    let (rig, work_id, preflight_id, _, _) = cleared_with_face("s2-adv-enr-override");
    // An escalating bundle (canonical blocklist term in candidate
    // tags) must not flip a cleared work either: enrichment never
    // overrides the screen in any direction.
    let bundle = call(
        &rig.neemu,
        "neemu_enrich",
        json!({
            "work_id": work_id,
            "sources": [{
                "source": "some-board",
                "match_key": "Moonlit Wife",
                "basis": "resolved_record",
                "tags": ["schoolgirl"],
                "payload": {}
            }],
        }),
    );
    assert!(bundle["escalation_flags"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f
            .as_str()
            .unwrap()
            .contains("canonical_blocklist_hit:schoolgirl")));
    let work = rig.neemu.load_work(&work_id).unwrap();
    assert_eq!(work["determination"]["determination"], json!("cleared"));
    // Extraction still works: the determination is the screen's.
    let segmented = call(
        &rig.neemu,
        "neemu_segment_scene",
        json!({ "work_id": work_id, "preflight_id": preflight_id }),
    );
    assert_eq!(segmented["total_scenes"], json!(1));
}

#[test]
fn enrich_unregistered_work_refused() {
    let rig = rig("s2-adv-enr-unreg", 1);
    let kind = call_kind(
        &rig.neemu,
        "neemu_enrich",
        json!({
            "work_id": "work_nope",
            "sources": [{ "source": "vndb", "match_key": "X", "payload": {} }],
        }),
    );
    assert_eq!(kind, "screen_refusal");
}

#[test]
fn enrich_single_source_cannot_pin_but_can_escalate() {
    let rig = rig("s2-adv-enr-single", 1);
    let work = register(&rig, "Harbour Wife", "A");
    let work_id = work["work_id"].as_str().unwrap();
    let bundle = call(
        &rig.neemu,
        "neemu_enrich",
        json!({
            "work_id": work_id,
            "sources": [{
                "source": "one-board",
                "match_key": "Harbour Wife",
                "basis": "resolved_record",
                "tags": ["hypnosis"],
                "escalation_flags": ["unverified_age_claim"],
                "payload": {}
            }],
        }),
    );
    // One source never pins identity (delta 18)...
    assert_eq!(bundle["identity"]["pinned"], json!(false));
    assert_eq!(bundle["queue_hint"], Value::Null);
    // ...but one source suffices to escalate.
    assert!(bundle["escalation_flags"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f.as_str() == Some("unverified_age_claim")));
}

#[test]
fn enrich_saucenao_claimed_resolved_forced_lead() {
    let rig = rig("s2-adv-enr-sauce", 1);
    let work = register(&rig, "Harbour Wife", "A");
    let work_id = work["work_id"].as_str().unwrap();
    let bundle = call(
        &rig.neemu,
        "neemu_enrich",
        json!({
            "work_id": work_id,
            "sources": [{
                "source": "saucenao",
                "match_key": "Harbour Wife",
                "basis": "resolved_record",
                "similarity": 0.97,
                "tags": ["hypnosis"],
                "payload": {}
            }],
        }),
    );
    // Delta 19: a similarity service is a lead even when the caller
    // claims resolved_record and the score is high.
    assert_eq!(bundle["sub_records"][0]["citable"], json!(false));
    assert_eq!(bundle["tags"], json!([]));
    assert!(bundle["sub_records"][0]["forced_lead_reason"]
        .as_str()
        .unwrap()
        .contains("delta 19"));
}

#[test]
fn enrich_tracemoe_below_threshold_forced_lead() {
    let rig = rig("s2-adv-enr-trace", 1);
    let work = register(&rig, "Harbour Wife", "A");
    let work_id = work["work_id"].as_str().unwrap();
    let bundle = call(
        &rig.neemu,
        "neemu_enrich",
        json!({
            "work_id": work_id,
            "sources": [{
                "source": "trace.moe",
                "match_key": "Some Anime 12:34",
                "basis": "resolved_record",
                "similarity": 0.85,
                "tags": ["hypnosis"],
                "payload": {}
            }],
        }),
    );
    assert_eq!(bundle["sub_records"][0]["citable"], json!(false));
    assert_eq!(bundle["tags"], json!([]));
}

#[test]
fn enrich_oversized_payload_refused() {
    let rig = rig("s2-adv-enr-payload", 1);
    let work = register(&rig, "Harbour Wife", "A");
    let work_id = work["work_id"].as_str().unwrap();
    let big = "x".repeat(70_000);
    let kind = call_kind(
        &rig.neemu,
        "neemu_enrich",
        json!({
            "work_id": work_id,
            "sources": [{
                "source": "vndb",
                "match_key": "Harbour Wife",
                "payload": { "blob": big }
            }],
        }),
    );
    assert_eq!(kind, "bound_exceeded");
}

#[test]
fn enrich_batch_per_item_error_preserved_in_order() {
    let rig = rig("s2-adv-enr-batcherr", 1);
    let good_a = register(&rig, "Batch Good A", "A");
    let good_b = register(&rig, "Batch Good B", "A");
    let source = json!([{ "source": "vndb", "match_key": "X", "payload": {} }]);
    let result = call(
        &rig.neemu,
        "neemu_enrich",
        json!({
            "items": [
                { "work_id": good_a["work_id"], "sources": source },
                { "work_id": "work_missing", "sources": source },
                { "work_id": good_b["work_id"], "sources": source }
            ]
        }),
    );
    let results = result["results"].as_array().unwrap();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0]["ok"], json!(true));
    assert_eq!(results[1]["ok"], json!(false));
    assert_eq!(results[1]["error"]["kind"], json!("screen_refusal"));
    assert_eq!(results[2]["ok"], json!(true));
    // Order is preserved: result i answers item i.
    assert_eq!(results[0]["work_id"], good_a["work_id"]);
    assert_eq!(results[2]["work_id"], good_b["work_id"]);
}
