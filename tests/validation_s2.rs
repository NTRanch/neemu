//! S2 validation suite: happy-path behaviour of the act /
//! understanding / enrichment slice, the registry seed's provenance,
//! and the human-read completeness gate (SPEC §7 S2).

mod common;

use common::*;
use neemu::acts::{act_entry, mapping_status, ACT_CANDIDATES, ACT_REGISTRY};
use neemu::tools::tool_definitions;
use serde_json::{json, Value};

/// A face observation with a chosen slot and F-code.
fn obs(page: usize, slot: &str, f_code: &str) -> Value {
    let mut observation = face_observation(page);
    observation["character_slot"] = json!(slot);
    observation["f_code"] = json!(f_code);
    observation
}

/// Segment a cleared work and return its scenes.
fn scenes(rig: &Rig, work_id: &str, preflight_id: &str) -> Vec<Value> {
    let result = call(
        &rig.neemu,
        "neemu_segment_scene",
        json!({ "work_id": work_id, "preflight_id": preflight_id }),
    );
    result["scenes"].as_array().expect("scenes").clone()
}

/// A minimal valid act for `scene_id`.
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

fn extract_simple_act(
    rig: &Rig,
    work_id: &str,
    preflight_id: &str,
    scene_id: &str,
    code: &str,
) -> Value {
    call(
        &rig.neemu,
        "neemu_extract_acts",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "scene_id": scene_id,
            "acts": [simple_act(code)],
        }),
    )
}

/// A fully gated one-scene work plus one face record to cite.
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

fn understanding_args(work_id: &str, preflight_id: &str, scene_id: &str, face_id: &str) -> Value {
    json!({
        "work_id": work_id,
        "preflight_id": preflight_id,
        "scene_id": scene_id,
        "beats": [
            { "type": "setup", "page_start": 0, "page_end": 0, "evidence_refs": [face_id] }
        ],
        "machinery": {
            "mechanism": "eye_contact",
            "stage_model": "held gaze deepens compliance",
            "reset_wakener": "absent",
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

// ---- Registry seed ----

#[test]
fn registry_seed_counts_and_provenance() {
    // W6-SPEC §1 arithmetic: 80 B-codes + KISS / M-01 / SOLO.
    assert_eq!(ACT_REGISTRY.len(), 83);
    let families = ACT_REGISTRY
        .iter()
        .filter(|e| e.disposition == "family")
        .count();
    assert_eq!(families, 37);
    assert_eq!(ACT_CANDIDATES.len(), 7);
    for entry in ACT_REGISTRY {
        assert!(!entry.source.is_empty(), "{} lacks provenance", entry.code);
    }
    // The only source-stated stratum links (ATLAS §4).
    let b48 = act_entry("B-48").unwrap();
    assert_eq!(b48.strata, ["S-18"]);
    assert_eq!(mapping_status(b48), "mapped");
    let m01 = act_entry("M-01").unwrap();
    assert_eq!(m01.strata, ["S-17", "S-18"]);
    let b60 = act_entry("B-60").unwrap();
    assert_eq!(b60.strata, ["S-12"]);
    assert_eq!(b60.disposition, "section");
    // A family is mapped via its W6 group even with no stratum stated.
    let b16 = act_entry("B-16").unwrap();
    assert_eq!(b16.group, Some("PEN"));
    assert_eq!(b16.physics_class, Some("P-A"));
    // A non-family code with no stated stratum is honestly unmapped.
    let b13 = act_entry("B-13").unwrap();
    assert_eq!(b13.disposition, "modifier");
    assert_eq!(mapping_status(b13), "unmapped");
    assert!(act_entry("B-99").is_none());
}

// ---- Acts ----

#[test]
fn extract_acts_mapped_family_record() {
    let (rig, work_id, preflight_id, scene_id, _) = cleared_with_face("s2-act-family");
    let result = extract_simple_act(&rig, &work_id, &preflight_id, &scene_id, "B-16");
    assert_eq!(result["count"], json!(1));
    assert_eq!(result["unmapped_act_codes"], json!([]));
    assert_eq!(result["candidate_act_codes"], json!([]));
    let record_id = result["record_ids"][0].as_str().unwrap();
    let record = rig
        .neemu
        .load_record("records", &work_id, record_id)
        .expect("act record stored");
    assert_eq!(record["record_kind"], json!("act_record"));
    assert_eq!(record["registry"]["w6_group"], json!("PEN"));
    assert_eq!(record["registry"]["physics_class"], json!("P-A"));
    assert_eq!(record["registry"]["mapping"], json!("mapped"));
    assert_eq!(record["registry"]["registry_status"], json!("canon"));
    assert_eq!(
        record["contact_pairs"][0]["compliance_class"],
        json!("attachment")
    );
    assert_eq!(record["contact_pairs"][0]["a"]["collider"], json!("sphere"));
    assert_eq!(record["locus"]["scene_id"], json!(scene_id));
}

#[test]
fn extract_acts_face_timeline_mixed_refs() {
    let (rig, work_id, preflight_id, scene_id, face_id) = cleared_with_face("s2-act-timeline");
    let mut act = simple_act("B-16");
    act["face_timeline"] = json!([
        { "beat": "setup", "ref": face_id },
        { "beat": "escalation", "ref": "F-05" }
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
    assert_eq!(record["face_timeline"][0]["face_ref"], json!(face_id));
    assert_eq!(record["face_timeline"][1]["f_code"], json!("F-05"));
}

#[test]
fn extract_acts_idempotent_replay() {
    let (rig, work_id, preflight_id, scene_id, _) = cleared_with_face("s2-act-idem");
    let first = extract_simple_act(&rig, &work_id, &preflight_id, &scene_id, "B-16");
    let second = extract_simple_act(&rig, &work_id, &preflight_id, &scene_id, "B-16");
    assert_eq!(first["record_ids"], second["record_ids"]);
    let list = call(
        &rig.neemu,
        "neemu_list_records",
        json!({ "work_id": work_id, "kind": "act_record" }),
    );
    assert_eq!(list["total"], json!(1));
}

// ---- Human-read completeness (the S2 known-scene gate) ----

#[test]
fn human_read_scene_completeness() {
    // The read is builder-authored and says so; the test ties its
    // checklist tokens to the document so the two cannot drift apart.
    let read = include_str!("fixtures/human_read_scene.md");
    for token in [
        "BUILDER-AUTHORED",
        "B-16",
        "F-02",
        "F-05",
        "F-08",
        "hard_non_penetration",
        "eye_contact",
        "absent",
    ] {
        assert!(read.contains(token), "human read lost token {token}");
    }

    let dims = [
        (800, 1200),
        (800, 1200),
        (1024, 768),
        (1024, 768),
        (1024, 768),
        (800, 1200),
    ];
    let rig = rig_dims("s2-human-read", &dims);
    let (work_id, preflight_id) = cleared_work(&rig, "Harbour Wife", "A");
    let all_scenes = scenes(&rig, &work_id, &preflight_id);
    assert_eq!(all_scenes.len(), 3);
    let scene = &all_scenes[1];
    assert_eq!(scene["page_start"], json!(2));
    assert_eq!(scene["page_end"], json!(4));
    let scene_id = scene["scene_id"].as_str().unwrap().to_string();

    // Faces per the read: (page, slot, F-code) x4.
    let faces = call(
        &rig.neemu,
        "neemu_extract_faces",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "page_start": 2,
            "page_end": 4,
            "observations": [
                obs(2, "lead", "F-02"),
                obs(3, "lead", "F-03"),
                obs(3, "partner", "F-08"),
                obs(4, "lead", "F-05"),
            ],
        }),
    );
    let face_ids: Vec<String> = faces["record_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_str().unwrap().to_string())
        .collect();
    assert_eq!(face_ids.len(), 4);
    // The faces timeline is complete and exact: each record matches
    // one read row, and there are no extras anywhere in the work.
    let expected = [
        (2u64, "lead", "F-02"),
        (3, "lead", "F-03"),
        (3, "partner", "F-08"),
        (4, "lead", "F-05"),
    ];
    for (id, (page, slot, f)) in face_ids.iter().zip(expected.iter()) {
        let record = rig.neemu.load_record("records", &work_id, id).unwrap();
        assert_eq!(record["locus"]["page"], json!(page));
        assert_eq!(record["character_slot"], json!(slot));
        assert_eq!(record["f_code"], json!(f));
    }
    let face_list = call(
        &rig.neemu,
        "neemu_list_records",
        json!({ "work_id": work_id, "kind": "face_observation" }),
    );
    assert_eq!(face_list["total"], json!(4));

    // Dialogue per the read, landing in the partner's page-3 face.
    let dialogue = call(
        &rig.neemu,
        "neemu_extract_dialogue",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "page_start": 2,
            "page_end": 4,
            "lines": [
                { "page": 3, "speaker": "partner", "text": "Look at me.", "face_ref": face_ids[2] }
            ],
        }),
    );
    let dialogue_id = dialogue["record_ids"][0].as_str().unwrap().to_string();
    let line = rig
        .neemu
        .load_record("records", &work_id, &dialogue_id)
        .unwrap();
    assert_eq!(line["text"], json!("Look at me."));
    assert_eq!(line["face_ref"], json!(face_ids[2]));

    // The act per the read.
    let act_result = call(
        &rig.neemu,
        "neemu_extract_acts",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "scene_id": scene_id,
            "acts": [{
                "act_code": "B-16",
                "participants": [
                    { "slot": "lead", "role": "receiver" },
                    { "slot": "partner", "role": "giver" }
                ],
                "contact_pairs": [
                    {
                        "a": { "slot": "partner", "part": "hand", "collider": "sphere" },
                        "b": { "slot": "lead", "part": "cheek", "collider": "surface" },
                        "compliance_class": "attachment"
                    },
                    {
                        "a": { "slot": "partner", "part": "pelvis", "collider": "surface" },
                        "b": { "slot": "lead", "part": "pelvis", "collider": "surface" },
                        "compliance_class": "hard_non_penetration"
                    }
                ],
                "pose_track": [
                    { "label": "entry", "joint_configuration": "supine, knees bent", "support": "bed surface", "orientation": "face-to-face" },
                    { "label": "cycle", "joint_configuration": "hips raised, legs wrapped", "support": "bed surface", "orientation": "face-to-face" }
                ],
                "motion": {
                    "beats_per_cycle": 2,
                    "tempo_curve": "steady then rising",
                    "intensity_curve": [0.3, 0.6, 0.9],
                    "entry": "from setup hold",
                    "exit": "hold"
                },
                "face_timeline": [
                    { "beat": "setup", "ref": face_ids[0] },
                    { "beat": "act", "ref": face_ids[1] },
                    { "beat": "escalation", "ref": "F-05" }
                ],
                "sfx_hooks": ["pair.torori.conditioning-kit"],
                "props": ["bed"]
            }],
        }),
    );
    let act_id = act_result["record_ids"][0].as_str().unwrap().to_string();
    let act = rig.neemu.load_record("records", &work_id, &act_id).unwrap();
    // Contact pairs present and matching, endpoint for endpoint.
    let pairs = act["contact_pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0]["a"]["part"], json!("hand"));
    assert_eq!(pairs[0]["a"]["collider"], json!("sphere"));
    assert_eq!(pairs[0]["b"]["part"], json!("cheek"));
    assert_eq!(pairs[0]["compliance_class"], json!("attachment"));
    assert_eq!(pairs[1]["a"]["collider"], json!("surface"));
    assert_eq!(pairs[1]["compliance_class"], json!("hard_non_penetration"));
    assert_eq!(act["registry"]["mapping"], json!("mapped"));
    assert_eq!(act["face_timeline"][0]["face_ref"], json!(face_ids[0]));
    assert_eq!(act["face_timeline"][2]["f_code"], json!("F-05"));
    assert_eq!(act["motion"]["intensity_curve"], json!([0.3, 0.6, 0.9]));

    // Understanding per the read.
    let understanding = call(
        &rig.neemu,
        "neemu_understand_scene",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "scene_id": scene_id,
            "beats": [
                { "type": "setup", "page_start": 2, "page_end": 2, "evidence_refs": [face_ids[0]] },
                { "type": "act", "page_start": 3, "page_end": 3, "evidence_refs": [act_id, dialogue_id] },
                { "type": "escalation", "page_start": 4, "page_end": 4, "evidence_refs": [face_ids[3]] }
            ],
            "machinery": {
                "mechanism": "eye_contact",
                "stage_model": "held gaze deepens compliance across the scene",
                "reset_wakener": "absent",
                "evidence_refs": [dialogue_id]
            },
            "dynamics": {
                "roles": [
                    { "slot": "lead", "role": "subject" },
                    { "slot": "partner", "role": "director" }
                ],
                "who_directs": "partner",
                "who_responds": "lead",
                "evidence_refs": [act_id]
            }
        }),
    );
    let record = rig
        .neemu
        .load_record(
            "records",
            &work_id,
            understanding["record_id"].as_str().unwrap(),
        )
        .unwrap();
    let beats = record["beats"].as_array().unwrap();
    assert_eq!(beats.len(), 3);
    assert_eq!(beats[0]["type"], json!("setup"));
    assert_eq!(beats[1]["type"], json!("act"));
    assert_eq!(beats[2]["type"], json!("escalation"));
    assert_eq!(record["machinery"]["reset_wakener"], json!("absent"));
    assert_eq!(record["dynamics"]["who_directs"], json!("partner"));
    // Every claim traces: the link union covers all five evidence
    // records the read names.
    let links = record["links"].as_array().unwrap();
    assert_eq!(links.len(), 4);
    for id in [&face_ids[0], &face_ids[3], &act_id, &dialogue_id] {
        assert!(
            links.iter().any(|l| l.as_str() == Some(id.as_str())),
            "link {id} missing"
        );
    }
}

// ---- Understanding ----

#[test]
fn understand_scene_records_model_estimate() {
    let (rig, work_id, preflight_id, scene_id, face_id) = cleared_with_face("s2-und-basic");
    let result = call(
        &rig.neemu,
        "neemu_understand_scene",
        understanding_args(&work_id, &preflight_id, &scene_id, &face_id),
    );
    assert_eq!(result["basis"], json!("model-estimate"));
    assert_eq!(result["beat_count"], json!(1));
    assert_eq!(result["link_count"], json!(1));
    let record = rig
        .neemu
        .load_record("records", &work_id, result["record_id"].as_str().unwrap())
        .unwrap();
    assert_eq!(record["record_kind"], json!("scene_understanding"));
    assert_eq!(record["advisory"], json!(true));
    assert_eq!(record["machinery"]["reset_wakener"], json!("absent"));
}

#[test]
fn understand_scene_never_changes_segmentation() {
    let (rig, work_id, preflight_id, scene_id, face_id) = cleared_with_face("s2-und-delta12");
    let before: Vec<Value> = scenes(&rig, &work_id, &preflight_id)
        .iter()
        .map(|s| s["scene_id"].clone())
        .collect();
    call(
        &rig.neemu,
        "neemu_understand_scene",
        understanding_args(&work_id, &preflight_id, &scene_id, &face_id),
    );
    let after: Vec<Value> = scenes(&rig, &work_id, &preflight_id)
        .iter()
        .map(|s| s["scene_id"].clone())
        .collect();
    assert_eq!(before, after);
}

// ---- Enrichment ----

fn registered_only(tag: &str, title: &str) -> (Rig, String) {
    let rig = rig(tag, 2);
    let work = register(&rig, title, "A");
    (rig, work["work_id"].as_str().unwrap().to_string())
}

fn resolved_source(name: &str, key: &str, tags: Vec<&str>) -> Value {
    json!({
        "source": name,
        "match_key": key,
        "match_evidence": "title + circle match",
        "retrieved_at": "2026-10-27T00:00:00Z",
        "basis": "resolved_record",
        "tags": tags,
        "payload": { "id": 42 }
    })
}

#[test]
fn enrich_lead_source_yields_no_citable_tags() {
    let (rig, work_id) = registered_only("s2-enr-lead", "Harbour Wife");
    let bundle = call(
        &rig.neemu,
        "neemu_enrich",
        json!({
            "work_id": work_id,
            "sources": [{
                "source": "some-board",
                "match_key": "Harbour Wife",
                "basis": "lead",
                "tags": ["hypnosis"],
                "payload": { "thread": 7 }
            }],
        }),
    );
    assert_eq!(bundle["record_kind"], json!("enrichment_bundle"));
    assert_eq!(bundle["advisory_only"], json!(true));
    assert_eq!(bundle["determination_effect"], json!("none"));
    assert_eq!(bundle["tags"], json!([]));
    assert_eq!(bundle["identity"]["pinned"], json!(false));
    assert_eq!(bundle["sub_records"][0]["citable"], json!(false));
    assert_eq!(bundle["sub_records"][0]["untrusted_text"], json!(true));
}

#[test]
fn enrich_two_resolved_sources_pin_identity() {
    let (rig, work_id) = registered_only("s2-enr-pin", "Harbour Wife");
    let bundle = call(
        &rig.neemu,
        "neemu_enrich",
        json!({
            "work_id": work_id,
            "sources": [
                resolved_source("vndb", "Harbour Wife", vec!["hypnosis", "adult"]),
                resolved_source("official-store", "Harbour Wife", vec!["mind control"]),
            ],
        }),
    );
    assert_eq!(bundle["identity"]["pinned"], json!(true));
    assert_eq!(bundle["identity"]["match_key"], json!("Harbour Wife"));
    assert_eq!(bundle["queue_hint"], json!("candidate"));
    let tags = bundle["tags"].as_array().unwrap();
    assert_eq!(tags.len(), 3);
    for tag in tags {
        assert_eq!(tag["untrusted_text"], json!(true));
        assert!(tag["source"].as_str().is_some());
    }
    // The corroboration gate text rides on the record (delta 18).
    assert!(bundle["corroboration_rule"]
        .as_str()
        .unwrap()
        .contains("clears a work or overrides an EXCLUDE"));
}

#[test]
fn enrich_version_chain_diff_flagged() {
    let (rig, work_id) = registered_only("s2-enr-versions", "Harbour Wife");
    let bundle = call(
        &rig.neemu,
        "neemu_enrich",
        json!({
            "work_id": work_id,
            "sources": [
                resolved_source("vndb", "Harbour Wife", vec!["hypnosis"]),
                resolved_source("official-store", "Harbour Wife", vec!["hypnosis"]),
            ],
            "versions": [
                { "version": "1.0", "tags": ["hypnosis", "drama"] },
                { "version": "1.1", "tags": ["hypnosis", "ntr"] }
            ],
        }),
    );
    let diffs = bundle["version_chain"]["diffs"].as_array().unwrap();
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0]["added"], json!(["ntr"]));
    assert_eq!(diffs[0]["removed"], json!(["drama"]));
    // A tag change is itself an escalation signal (delta 22), so the
    // queue hint stays off even with identity pinned.
    assert!(bundle["escalation_flags"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f.as_str().unwrap().starts_with("version_tag_change")));
    assert_eq!(bundle["queue_hint"], Value::Null);
}

#[test]
fn enrich_grouping_confidence_floor() {
    let (rig, work_id) = registered_only("s2-enr-group", "Harbour Wife");
    let low = call(
        &rig.neemu,
        "neemu_enrich",
        json!({
            "work_id": work_id,
            "sources": [resolved_source("vndb", "Harbour Wife", vec![])],
            "grouping": { "basis": "heuristic_match", "confidence": 0.3, "group_ref": "series-x" },
        }),
    );
    assert_eq!(low["grouping"]["applied"], json!(false));
    let high = call(
        &rig.neemu,
        "neemu_enrich",
        json!({
            "work_id": work_id,
            "sources": [resolved_source("vndb", "Harbour Wife", vec!["revised"])],
            "grouping": { "basis": "official_chain", "confidence": 0.9, "group_ref": "series-x" },
        }),
    );
    assert_eq!(high["grouping"]["applied"], json!(true));
}

#[test]
fn enrich_batch_order_and_resume_cursor() {
    let rig = rig("s2-enr-batch", 1);
    let mut items = Vec::new();
    for i in 0..6 {
        let work = register(&rig, &format!("Batch Work {}", i + 1), "A");
        items.push(json!({
            "work_id": work["work_id"],
            "sources": [resolved_source("vndb", "Batch", vec!["hypnosis"])],
        }));
    }
    let first = call(
        &rig.neemu,
        "neemu_enrich",
        json!({ "items": items.clone() }),
    );
    assert_eq!(first["processed"], json!(4));
    assert_eq!(first["total_items"], json!(6));
    assert_eq!(first["partial"], json!(true));
    assert_eq!(first["resume_cursor"]["next_index"], json!(4));
    let results = first["results"].as_array().unwrap();
    assert_eq!(results.len(), 4);
    for (index, result) in results.iter().enumerate() {
        assert_eq!(result["ok"], json!(true));
        assert_eq!(result["work_id"], items[index]["work_id"]);
    }
    let second = call(
        &rig.neemu,
        "neemu_enrich",
        json!({ "items": items, "resume_cursor": first["resume_cursor"] }),
    );
    assert_eq!(second["processed"], json!(2));
    assert_eq!(second["partial"], json!(false));
    assert_eq!(second["resume_cursor"], Value::Null);
    // One content-derived batch id across the resume.
    assert_eq!(first["batch_id"], second["batch_id"]);
}

#[test]
fn enrich_ehentai_rate_limits_recorded() {
    let (rig, work_id) = registered_only("s2-enr-eh", "Harbour Wife");
    let bundle = call(
        &rig.neemu,
        "neemu_enrich",
        json!({
            "work_id": work_id,
            "sources": [{
                "source": "e-hentai gdata",
                "match_key": "Harbour Wife",
                "basis": "resolved_record",
                "tags": ["hypnosis"],
                "payload": { "gid": 1 }
            }],
        }),
    );
    let config = &bundle["rate_limit_config"];
    assert_eq!(config["search_interval_ms"], json!(3000));
    assert_eq!(config["api_interval_ms"], json!(1250));
    assert_eq!(config["page_interval_ms"], json!(1000));
    assert_eq!(config["max_retries"], json!(2));
    assert_eq!(config["retry_after_priority"], json!(true));
}

#[test]
fn enrich_translated_tags_flagged_non_commercial() {
    let (rig, work_id) = registered_only("s2-enr-i18n", "Harbour Wife");
    let mut source = resolved_source("eh-tag-db", "Harbour Wife", vec!["hypnosis"]);
    source["translated"] = json!(true);
    let bundle = call(
        &rig.neemu,
        "neemu_enrich",
        json!({ "work_id": work_id, "sources": [source] }),
    );
    assert_eq!(bundle["tags"][0]["non_commercial"], json!(true));
    assert_eq!(bundle["non_commercial_tags"], json!(true));
}

// ---- Registry surface ----

#[test]
fn list_records_includes_s2_kinds() {
    let (rig, work_id, preflight_id, scene_id, face_id) = cleared_with_face("s2-list-kinds");
    extract_simple_act(&rig, &work_id, &preflight_id, &scene_id, "B-16");
    call(
        &rig.neemu,
        "neemu_understand_scene",
        understanding_args(&work_id, &preflight_id, &scene_id, &face_id),
    );
    call(
        &rig.neemu,
        "neemu_enrich",
        json!({
            "work_id": work_id,
            "sources": [resolved_source("vndb", "Moonlit Wife", vec!["hypnosis"])],
        }),
    );
    for (kind, total) in [
        ("act_record", 1),
        ("scene_understanding", 1),
        ("enrichment_bundle", 1),
    ] {
        let list = call(
            &rig.neemu,
            "neemu_list_records",
            json!({ "work_id": work_id, "kind": kind }),
        );
        assert_eq!(list["total"], json!(total), "kind {kind}");
    }
}

#[test]
fn tools_surface_s2() {
    let definitions = tool_definitions();
    let names: Vec<&str> = definitions
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    // S3 landed the four voice tools (SPEC §4 names; the S3 name
    // is neemu_assess_voice_clonability, not the S2-era guess
    // neemu_assess_clonability): 11 → 15.
    assert_eq!(names.len(), 15);
    for expected in [
        "neemu_enrich",
        "neemu_extract_acts",
        "neemu_understand_scene",
        "neemu_analyze_voice",
        "neemu_assess_voice_clonability",
        "neemu_derive_voice",
        "neemu_emit_torori_cue",
    ] {
        assert!(names.contains(&expected), "missing {expected}");
    }
    // S4+ tools stay absent, not stubbed.
    for absent in ["neemu_assess_clonability", "neemu_emit_gap_finding"] {
        assert!(!names.contains(&absent), "S4+ tool leaked: {absent}");
    }
}
