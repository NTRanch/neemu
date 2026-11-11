//! NE-1 / D-1265 (Pax ruling, 2026-11-10): a voice cue is an input
//! that conditions synthesis, not a measurement. These tests pin
//! the two refusals and the positive path:
//!
//! - a cue without a voice lock, without lane labels, or without
//!   evidence does not construct (`voice_cue_refusal`, and nothing
//!   is minted);
//! - a cross-project or cross-lane export of a cue is refused, as
//!   is an export with no declared target and any export of an
//!   unlabelled legacy cue;
//! - a fully-labelled, voice-locked cue constructs, carries the
//!   stamp on its records from birth, and round-trips through the
//!   registry store and a same-project / same-lane export.
//!
//! An unclassified cue is stamped `nsfw` (Bloch fail-closed
//! default) and the record says the label was defaulted.

mod common;

use common::*;
use serde_json::{json, Value};
use std::fs;

struct CueRig {
    rig: Rig,
    work_id: String,
    preflight_id: String,
    voice_id: String,
    dialogue_id: String,
    identity_id: String,
    derivation_id: String,
}

/// The S3 happy path up to a derived, locked estate identity plus
/// one dialogue line ready to cue.
fn cue_rig(tag: &str) -> CueRig {
    let rig = av_rig(tag);
    let (work_id, preflight_id, voice_id, assessment_id) = clonable_assessment(&rig, true);
    let derived = call(
        &rig.neemu,
        "neemu_derive_voice",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "assessment_id": assessment_id,
            "adaptation": {
                "pitch_semitones": -1.5,
                "formant_ratio": 0.95,
                "tempo_character": "brisker",
                "accent_target": null
            },
            "engine": {
                "engine": "estate-synth",
                "license": "estate-internal",
                "license_alive": true,
                "commercial_ok": true
            },
            "target_identity": "voice.derived.locked",
            "design_targets": {
                "warmth": 0.5,
                "breathiness_balance": 0.5,
                "pitch_placement": 0.5,
                "articulation_clarity": 0.5
            }
        }),
    );
    let dialogue = call(
        &rig.neemu,
        "neemu_extract_dialogue",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "page_start": 0,
            "page_end": 0,
            "lines": [
                { "page": 0, "speaker": "lead", "text": "Look into my eyes now", "beat": "induction" }
            ],
        }),
    );
    CueRig {
        dialogue_id: dialogue["record_ids"][0].as_str().unwrap().to_string(),
        identity_id: derived["identity_id"].as_str().unwrap().to_string(),
        derivation_id: derived["derivation_id"].as_str().unwrap().to_string(),
        rig,
        work_id,
        preflight_id,
        voice_id,
    }
}

/// One fully-labelled, voice-locked emit call.
fn labelled_emit(cue_rig: &CueRig) -> Value {
    json!({
        "work_id": cue_rig.work_id,
        "preflight_id": cue_rig.preflight_id,
        "target_voice": "voice.derived.locked",
        "voice_lock_ref": cue_rig.identity_id,
        "project_id": "strobe",
        "origin_lane": "ntranch",
        "content_label": "nsfw",
        "evidence": [{ "kind": "derivation", "ref": cue_rig.derivation_id }],
        "cues": [
            {
                "dialogue_id": cue_rig.dialogue_id,
                "voice_id": cue_rig.voice_id,
                "delivery": { "seed": 42 },
                "sfx_pairs": ["pair.breath.hold"]
            }
        ],
    })
}

fn cue_records(cue_rig: &CueRig) -> Vec<Value> {
    cue_rig
        .rig
        .neemu
        .all_records(&cue_rig.work_id)
        .into_iter()
        .filter(|r| {
            matches!(
                r["record_kind"].as_str(),
                Some("torori_cue") | Some("torori_cue_bundle")
            )
        })
        .collect()
}

#[test]
fn cue_without_lock_or_labels_or_evidence_does_not_construct() {
    let cue_rig = cue_rig("d1265-refuse-construct");
    for missing in ["voice_lock_ref", "project_id", "origin_lane", "evidence"] {
        let mut args = labelled_emit(&cue_rig);
        args.as_object_mut().unwrap().remove(missing);
        let kind = call_kind(&cue_rig.rig.neemu, "neemu_emit_torori_cue", args);
        assert_eq!(kind, "voice_cue_refusal", "missing {missing}");
    }
    // A lock reference that is not the resolved identity's record
    // is a detached lock: refused the same way.
    let mut args = labelled_emit(&cue_rig);
    args["voice_lock_ref"] = json!("ident_00000000000000000000000000000000");
    let kind = call_kind(&cue_rig.rig.neemu, "neemu_emit_torori_cue", args);
    assert_eq!(kind, "voice_cue_refusal");
    // A content label outside the Bloch set is refused, never
    // silently re-labelled.
    let mut args = labelled_emit(&cue_rig);
    args["content_label"] = json!("public");
    let kind = call_kind(&cue_rig.rig.neemu, "neemu_emit_torori_cue", args);
    assert_eq!(kind, "voice_cue_refusal");
    // Refused at construction means: nothing was minted, no cue or
    // bundle record exists for the work.
    assert!(
        cue_records(&cue_rig).is_empty(),
        "refused emits must mint nothing: {:?}",
        cue_records(&cue_rig)
    );
}

#[test]
fn fully_labelled_locked_cue_constructs_and_round_trips() {
    let cue_rig = cue_rig("d1265-positive");
    let emitted = call(
        &cue_rig.rig.neemu,
        "neemu_emit_torori_cue",
        labelled_emit(&cue_rig),
    );
    assert_eq!(emitted["voice_lock_ref"], json!(cue_rig.identity_id));
    assert_eq!(emitted["project_id"], json!("strobe"));
    assert_eq!(emitted["origin_lane"], json!("ntranch"));
    assert_eq!(emitted["content_label"], json!("nsfw"));

    // Round-trip through the registry store: the cue record and the
    // bundle record both carry the full stamp from birth.
    let cue_id = emitted["cue_record_ids"][0].as_str().unwrap().to_string();
    for record_id in [cue_id, emitted["record_id"].as_str().unwrap().to_string()] {
        let record = cue_rig
            .rig
            .neemu
            .load_record("records", &cue_rig.work_id, &record_id)
            .expect("record round-trips from the store");
        assert_eq!(record["voice_lock_ref"], json!(cue_rig.identity_id));
        assert_eq!(record["voice_lock_state"], json!("designed"));
        assert_eq!(record["project_id"], json!("strobe"));
        assert_eq!(record["origin_lane"], json!("ntranch"));
        assert_eq!(record["content_label"], json!("nsfw"));
        assert_eq!(record["content_label_defaulted"], json!(false));
        assert_eq!(
            record["evidence"],
            json!([{ "kind": "derivation", "ref": cue_rig.derivation_id }])
        );
    }
}

#[test]
fn unclassified_cue_is_stamped_nsfw_fail_closed() {
    let cue_rig = cue_rig("d1265-default-label");
    let mut args = labelled_emit(&cue_rig);
    args.as_object_mut().unwrap().remove("content_label");
    let emitted = call(&cue_rig.rig.neemu, "neemu_emit_torori_cue", args);
    assert_eq!(emitted["content_label"], json!("nsfw"));
    let cue_id = emitted["cue_record_ids"][0].as_str().unwrap().to_string();
    let record = cue_rig
        .rig
        .neemu
        .load_record("records", &cue_rig.work_id, &cue_id)
        .expect("cue record");
    assert_eq!(record["content_label"], json!("nsfw"));
    assert_eq!(record["content_label_defaulted"], json!(true));
}

#[test]
fn cross_project_and_cross_lane_cue_export_is_refused() {
    let cue_rig = cue_rig("d1265-export");
    let emitted = call(
        &cue_rig.rig.neemu,
        "neemu_emit_torori_cue",
        labelled_emit(&cue_rig),
    );
    let cue_id = emitted["cue_record_ids"][0].as_str().unwrap().to_string();
    let export = |extra: Value| {
        let mut args = json!({
            "work_id": cue_rig.work_id,
            "bundle_kind": "analysis",
            "record_ids": [cue_id],
        });
        args.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        args
    };
    // Cross-project.
    let kind = call_kind(
        &cue_rig.rig.neemu,
        "neemu_export",
        export(json!({ "target_project_id": "other-project", "target_lane": "ntranch" })),
    );
    assert_eq!(kind, "voice_cue_refusal");
    // Cross-lane.
    let kind = call_kind(
        &cue_rig.rig.neemu,
        "neemu_export",
        export(json!({ "target_project_id": "strobe", "target_lane": "qompassai" })),
    );
    assert_eq!(kind, "voice_cue_refusal");
    // No declared target: a cue never travels detached.
    let kind = call_kind(&cue_rig.rig.neemu, "neemu_export", export(json!({})));
    assert_eq!(kind, "voice_cue_refusal");

    // Same project + same lane: the export succeeds and the cue
    // inside the bundle still carries its stamp (round-trip).
    let exported = call(
        &cue_rig.rig.neemu,
        "neemu_export",
        export(json!({ "target_project_id": "strobe", "target_lane": "ntranch" })),
    );
    let bundle_text = fs::read_to_string(exported["path"].as_str().unwrap()).expect("bundle");
    let bundle: Value = serde_json::from_str(&bundle_text).expect("bundle json");
    let travelled = &bundle["records"][0];
    assert_eq!(travelled["record_kind"], json!("torori_cue"));
    assert_eq!(travelled["voice_lock_ref"], json!(cue_rig.identity_id));
    assert_eq!(travelled["project_id"], json!("strobe"));
    assert_eq!(travelled["origin_lane"], json!("ntranch"));
    assert_eq!(travelled["content_label"], json!("nsfw"));
}

#[test]
fn unlabelled_legacy_cue_is_not_exportable() {
    let cue_rig = cue_rig("d1265-legacy");
    // A pre-D-1265 cue record: minted directly, carrying no lock
    // and no labels (the shape every legacy record on disk has).
    let legacy_id = cue_rig
        .rig
        .neemu
        .mint_record(
            &cue_rig.work_id,
            "cue_",
            json!({
                "record_kind": "torori_cue",
                "work_id": cue_rig.work_id,
                "cue_id": "cue_001",
                "target_voice": "voice.derived.locked",
            }),
        )
        .expect("mint legacy cue");
    let kind = call_kind(
        &cue_rig.rig.neemu,
        "neemu_export",
        json!({
            "work_id": cue_rig.work_id,
            "bundle_kind": "analysis",
            "record_ids": [legacy_id],
            "target_project_id": "strobe",
            "target_lane": "ntranch",
        }),
    );
    assert_eq!(kind, "voice_cue_refusal");
}
