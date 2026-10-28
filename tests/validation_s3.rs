//! S3 validation battery (SPEC §7 S3): AV registration + preflight,
//! voice analysis with server-measured features, the clonability
//! happy path, derivation with its watermark, cue emission in
//! torori's frozen bundle format, the accent seed, and parity
//! against torori's frozen schema descriptors on disk.

mod common;

use common::*;
use neemu::s3::{
    conditioning_keys_sorted, CONDITIONING_REQUIRED, CONDITIONING_SCHEMA_SHA256,
    CUE_BUNDLE_SCHEMA_SHA256,
};
use serde_json::{json, Value};
use std::fs;

fn sha256_hex(text: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(text);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn av_registration_and_preflight() {
    let rig = av_rig("s3-av-register");
    let (work_id, _preflight_id) = register_and_validate(&rig, "Voice Study", "A");
    let work = rig.neemu.load_work(&work_id).expect("work");
    let media = work["media"].as_array().expect("media");
    assert_eq!(media.len(), 2);
    let image = &media[0];
    assert_eq!(image["format"], json!("png"));
    // The image entry shape is the S1 shape — no AV keys (parity).
    assert!(image.get("media_kind").is_none());
    assert!(image.get("duration_seconds").is_none());
    let wav = &media[1];
    assert_eq!(wav["name"], json!("clip.wav"));
    assert_eq!(wav["format"], json!("wav"));
    assert_eq!(wav["media_kind"], json!("audio"));
    assert_eq!(wav["sample_rate"], json!(8000));
    assert_eq!(wav["channels"], json!(1));
    let duration = wav["duration_seconds"].as_f64().expect("duration");
    assert!((duration - 30.0).abs() < 0.01, "duration {duration}");
}

#[test]
fn analyze_voice_measures_the_file() {
    let rig = av_rig("s3-analyze");
    let (work_id, preflight_id) = cleared_work(&rig, "Voice Study", "A");
    let result = call(
        &rig.neemu,
        "neemu_analyze_voice",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "segments": [
                speech_segment(),
                {
                    "media_name": "clip.wav",
                    "start_s": 26.8,
                    "end_s": 27.5,
                    "speaker": "performer.one",
                    "segment_kind": "non_speech",
                    "non_speech_class": "breath",
                    "scene_function": "held breath before the drop"
                }
            ],
        }),
    );
    assert_eq!(result["segments"], json!(2));
    assert_eq!(result["source_kinds"], json!(["image+audio"]));
    let speech_id = result["record_ids"][0].as_str().unwrap().to_string();
    let record = rig
        .neemu
        .load_record("records", &work_id, &speech_id)
        .expect("voice record");
    assert_eq!(record["record_kind"], json!("voice_performance"));
    let measured = &record["measured_features"];
    assert_eq!(measured["sample_rate"], json!(8000));
    assert_eq!(measured["clipped_fraction"], json!(0.0));
    assert_eq!(measured["pause_structure"]["pause_count"], json!(3));
    let speech_seconds = measured["pause_structure"]["speech_seconds_estimate"]
        .as_f64()
        .unwrap();
    assert!(
        (speech_seconds - 24.3).abs() < 0.2,
        "speech estimate {speech_seconds}"
    );
    assert_eq!(measured["breath_events_estimate"], json!(1));
    assert_eq!(record["accent_heard"]["top1"], json!("en-us-general"));
    assert!(
        record["known_limitations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l.as_str().unwrap().contains("delta 8")),
        "known_limitations must echo delta 8: {record}"
    );
    let breath_id = result["record_ids"][1].as_str().unwrap().to_string();
    let breath = rig
        .neemu
        .load_record("records", &work_id, &breath_id)
        .expect("breath record");
    assert_eq!(breath["transcript"], Value::Null);
    assert_eq!(breath["non_speech_class"], json!("breath"));

    // Idempotent replay: the same call mints the same record ids.
    let replay = call(
        &rig.neemu,
        "neemu_analyze_voice",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "segments": [speech_segment()],
        }),
    );
    assert_eq!(replay["record_ids"][0], json!(speech_id));
}

#[test]
fn assess_clonable_and_supersede() {
    let rig = av_rig("s3-assess");
    let (work_id, preflight_id, voice_id, assessment_id) = clonable_assessment(&rig, true);
    let record = rig
        .neemu
        .load_record("records", &work_id, &assessment_id)
        .expect("assessment");
    assert_eq!(record["verdict"], json!("CLONABLE"));
    assert_eq!(record["rights_axis"]["pass"], json!(true));
    assert_eq!(record["technical_axis"]["pass"], json!(true));
    assert_eq!(
        record["technical_axis"]["constants"]["min_speech_seconds"],
        json!(20.0)
    );
    // A superseding assessment names its predecessor and mints anew.
    let next = call(
        &rig.neemu,
        "neemu_assess_voice_clonability",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "voice_record_ids": [voice_id],
            "consent_artifact": consent_artifact(),
            "rights_basis": { "basis": "estate-owned performance", "evidence_ref": "estate://rights/performer-one" },
            "presence_verified": true,
            "supersedes": assessment_id,
        }),
    );
    assert_ne!(next["record_id"], json!(assessment_id));
    let stored = rig
        .neemu
        .load_record("records", &work_id, next["record_id"].as_str().unwrap())
        .expect("superseding assessment");
    assert_eq!(stored["supersedes"], json!(assessment_id));
}

#[test]
fn derive_mints_watermarked_identity() {
    let rig = av_rig("s3-derive");
    let (work_id, preflight_id, _voice_id, assessment_id) = clonable_assessment(&rig, true);
    let derived = call(
        &rig.neemu,
        "neemu_derive_voice",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "assessment_id": assessment_id,
            "derivation_class": "high-fidelity",
            "adaptation": {
                "pitch_semitones": 2.0,
                "formant_ratio": 1.1,
                "tempo_character": "slower, deliberate",
                "accent_target": "en-gb-rp"
            },
            "engine": {
                "engine": "estate-synth",
                "license": "estate-internal",
                "license_alive": true,
                "commercial_ok": true
            },
            "target_identity": "voice.derived.motte2",
            "design_targets": {
                "warmth": 0.7,
                "breathiness_balance": 0.4,
                "pitch_placement": 0.6,
                "articulation_clarity": 0.8
            }
        }),
    );
    let derivation_id = derived["derivation_id"].as_str().unwrap().to_string();
    // Delta 14: the watermark payload IS the derivation record id.
    assert_eq!(derived["watermark"], json!(derivation_id));
    let identity = rig
        .neemu
        .load_record(
            "records",
            &work_id,
            derived["identity_id"].as_str().unwrap(),
        )
        .expect("identity record");
    assert_eq!(identity["record_kind"], json!("voice_identity"));
    assert_eq!(identity["kind"], json!("derived"));
    assert_eq!(identity["watermark"]["payload"], json!(derivation_id));
    assert_eq!(identity["watermark"]["embedded"], json!(true));
    // The conditioning bundle on disk carries the same payload.
    let bundle_text = fs::read_to_string(derived["conditioning_bundle"].as_str().unwrap())
        .expect("conditioning bundle file");
    assert!(
        bundle_text.contains(&derivation_id),
        "bundle must carry the watermark payload"
    );
    // Registry-wide resolution finds the identity from any work.
    let (owner, _) = rig
        .neemu
        .find_voice_identity("voice.derived.motte2")
        .expect("identity resolves");
    assert_eq!(owner, work_id);
}

#[test]
fn emit_bundle_matches_torori_frozen_shape() {
    let rig = av_rig("s3-emit");
    let (work_id, preflight_id, voice_id, assessment_id) = clonable_assessment(&rig, true);
    call(
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
            "target_identity": "voice.derived.motte2",
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
    let dialogue_id = dialogue["record_ids"][0].as_str().unwrap().to_string();
    let emitted = call(
        &rig.neemu,
        "neemu_emit_torori_cue",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "target_voice": "voice.derived.motte2",
            "cues": [
                {
                    "dialogue_id": dialogue_id,
                    "voice_id": voice_id,
                    "delivery": { "seed": 42 },
                    "sfx_pairs": ["pair.breath.hold"]
                }
            ],
        }),
    );
    let bundle_dir = emitted["bundle_dir"].as_str().unwrap().to_string();
    let ron = fs::read_to_string(format!("{bundle_dir}/bundle.ron")).expect("bundle.ron");
    for needle in [
        "(schema_version: 1",
        "producer: \"neemu\"",
        "cue_id: \"cue_001\"",
        "text: \"Look into my eyes now\"",
        "identity_ref: \"voice.derived.motte2\"",
        "f0_median_hz: Some(196.0)",
        "breath_bracket: NotRequired",
        "determinism_class: SeedReproducible",
        "seed: Some(42)",
        "tier: Measured",
        "sfx_pairs: [\"pair.breath.hold\"]",
        "target_voice: \"voice.derived.motte2\"",
    ] {
        assert!(ron.contains(needle), "bundle.ron missing {needle}:\n{ron}");
    }
    // The final provenance link carries the content hash (torori's
    // rule), and the hash replicates torori's canonical recipe:
    // sha256 over serde_json of {cues, pairs, asset_sha256s} built
    // from torori's struct field sets.
    let content_hash = emitted["content_hash"].as_str().unwrap();
    let final_link = format!("artifact_sha256: \"{content_hash}\"");
    assert!(ron.contains(&final_link), "final provenance link: {ron}");
    let canonical = json!({
        "cues": [{
            "cue_id": "cue_001",
            "text": "Look into my eyes now",
            "conditioning": {
                "text": "Look into my eyes now",
                "identity_ref": "voice.derived.motte2",
                "instruction": null,
                "f0_median_hz": 196.0,
                "f0_range_semitones": null,
                "tempo_syllables_per_sec": null,
                "pause_structure": null,
                "breathiness": null,
                "grade": null,
                "accent_profile_ref": null,
                "accent_intensity": null,
                "breath_bracket": "NotRequired",
                "determinism_class": "SeedReproducible",
                "seed": 42
            },
            "intent": "line",
            "target_voice": "voice.derived.motte2",
            "sfx_pairs": ["pair.breath.hold"],
            "tier": "Measured"
        }],
        "pairs": [],
        "asset_sha256s": []
    });
    let replicated = sha256_hex(serde_json::to_string(&canonical).unwrap().as_bytes());
    assert_eq!(replicated, content_hash, "torori content-hash replication");
    // Idempotent: the same emit mints the same bundle record.
    let replay = call(
        &rig.neemu,
        "neemu_emit_torori_cue",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "target_voice": "voice.derived.motte2",
            "cues": [
                {
                    "dialogue_id": dialogue_id,
                    "voice_id": voice_id,
                    "delivery": { "seed": 42 },
                    "sfx_pairs": ["pair.breath.hold"]
                }
            ],
        }),
    );
    assert_eq!(replay["record_id"], emitted["record_id"]);
}

#[test]
fn accent_seed_is_complete_and_performative() {
    let registry = neemu::accents::registry();
    assert_eq!(registry["profile_count"], json!(21));
    let profiles = registry["profiles"].as_array().unwrap();
    assert_eq!(profiles.len(), 21);
    for profile in profiles {
        let reactions = &profile["reaction_set"];
        for key in ["gasps", "laughter", "cries", "interjections"] {
            let items = reactions[key].as_array().unwrap_or_else(|| {
                panic!("profile {} reaction {key} missing", profile["profile_id"])
            });
            assert!(
                !items.is_empty(),
                "profile {} reaction {key} is empty (speech-only profile)",
                profile["profile_id"]
            );
        }
        assert!(
            profile["phonetic_features"]
                .as_array()
                .is_some_and(|f| !f.is_empty()),
            "profile {} lacks phonetic features",
            profile["profile_id"]
        );
    }
    for expected in [
        "en-us-general",
        "en-gb-rp",
        "en-scottish",
        "en-pt-pt",
        "en-ja-japanese",
    ] {
        assert!(neemu::accents::exists(expected), "seed missing {expected}");
    }
    assert!(!neemu::accents::exists("en-xx-nope"));
}

#[test]
fn frozen_schema_parity_with_torori_tree() {
    // The torori tree sits beside the neemu tree in this workspace;
    // when it is absent (a lone checkout) the parity claim rests on
    // the recorded hashes in the S3 report instead.
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let conditioning_path =
        root.join("../torori-build-20261027/data/schemas/conditioning.schema.json");
    let bundle_path = root.join("../torori-build-20261027/data/schemas/cue-bundle.schema.json");
    if !conditioning_path.exists() || !bundle_path.exists() {
        eprintln!("torori schema descriptors not locatable; parity rests on recorded hashes");
        return;
    }
    let conditioning_bytes = fs::read(&conditioning_path).unwrap();
    assert_eq!(sha256_hex(&conditioning_bytes), CONDITIONING_SCHEMA_SHA256);
    let bundle_bytes = fs::read(&bundle_path).unwrap();
    assert_eq!(sha256_hex(&bundle_bytes), CUE_BUNDLE_SCHEMA_SHA256);
    let descriptor: Value = serde_json::from_slice(&conditioning_bytes).unwrap();
    let mut properties: Vec<String> = descriptor["properties"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    properties.sort();
    let mine: Vec<String> = conditioning_keys_sorted()
        .iter()
        .map(|k| k.to_string())
        .collect();
    assert_eq!(properties, mine, "conditioning key set drifted");
    let mut required: Vec<String> = descriptor["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r.as_str().unwrap().to_string())
        .collect();
    required.sort();
    let mut mine_required: Vec<String> = CONDITIONING_REQUIRED
        .iter()
        .map(|k| k.to_string())
        .collect();
    mine_required.sort();
    assert_eq!(required, mine_required, "conditioning required set drifted");
}
