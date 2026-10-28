//! S3 adversarial battery (SPEC §7 S3 gates): every refusal path
//! the SPEC names — screening gates, the consent-artifact gate,
//! derivation refusals (unassessed / NOT-CLONABLE / identity plan /
//! over-class / dead license / target-is-source), emit refusals
//! (missing target voice, out-of-range conditioning), and the
//! silence rule (no audio-output dependency exists to call).

mod common;

use common::*;
use serde_json::{json, Value};
use std::fs;

fn derive_args(work_id: &str, preflight_id: &str, assessment_id: &str) -> Value {
    json!({
        "work_id": work_id,
        "preflight_id": preflight_id,
        "assessment_id": assessment_id,
        "adaptation": {
            "pitch_semitones": 1.0,
            "formant_ratio": 1.05,
            "tempo_character": "brisker",
            "accent_target": null
        },
        "engine": {
            "engine": "estate-synth",
            "license": "estate-internal",
            "license_alive": true,
            "commercial_ok": true
        },
        "target_identity": "voice.derived.other",
        "design_targets": {
            "warmth": 0.5,
            "breathiness_balance": 0.5,
            "pitch_placement": 0.5,
            "articulation_clarity": 0.5
        }
    })
}

#[test]
fn analyze_refuses_unscreened_and_bad_preflight() {
    let rig = av_rig("s3-adv-gates");
    let (work_id, preflight_id) = register_and_validate(&rig, "Voice Study", "A");
    // Unscreened work: extraction gate refuses before any analysis.
    let kind = call_kind(
        &rig.neemu,
        "neemu_analyze_voice",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "segments": [speech_segment()],
        }),
    );
    assert_eq!(kind, "screen_refusal");
    screen_cleared(&rig.neemu, &work_id);
    // A preflight id that does not exist refuses as not-screened.
    let kind = call_kind(
        &rig.neemu,
        "neemu_analyze_voice",
        json!({
            "work_id": work_id,
            "preflight_id": "preflight_nope",
            "segments": [speech_segment()],
        }),
    );
    assert_eq!(kind, "refusal_not_screened");
}

#[test]
fn analyze_rejects_bad_segments() {
    let rig = av_rig("s3-adv-segments");
    let (work_id, preflight_id) = cleared_work(&rig, "Voice Study", "A");
    let base = |segment: Value| json!({ "work_id": work_id, "preflight_id": preflight_id, "segments": [segment] });
    // End beyond the measured duration.
    let mut seg = speech_segment();
    seg["end_s"] = json!(31.0);
    call_args_err(&rig.neemu, "neemu_analyze_voice", base(seg));
    // Speech without a transcript.
    let mut seg = speech_segment();
    seg.as_object_mut().unwrap().remove("transcript");
    let message = call_args_err(&rig.neemu, "neemu_analyze_voice", base(seg));
    assert!(message.contains("transcript"), "{message}");
    // Non-speech without a class.
    let seg = json!({
        "media_name": "clip.wav", "start_s": 26.8, "end_s": 27.5,
        "speaker": "performer.one", "segment_kind": "non_speech",
        "scene_function": "pause"
    });
    call_args_err(&rig.neemu, "neemu_analyze_voice", base(seg));
    // An image file is not analysable by the voice lane.
    let mut seg = speech_segment();
    seg["media_name"] = json!("a-page.png");
    call_args_err(&rig.neemu, "neemu_analyze_voice", base(seg));
    // Unknown media name.
    let mut seg = speech_segment();
    seg["media_name"] = json!("nope.wav");
    call_args_err(&rig.neemu, "neemu_analyze_voice", base(seg));
    // Caller f0 outside the frozen range.
    let mut seg = speech_segment();
    seg["features"]["f0_median_hz"] = json!(700.0);
    call_args_err(&rig.neemu, "neemu_analyze_voice", base(seg));
    // Accent distribution whose top1 is not its argmax.
    let mut seg = speech_segment();
    seg["accent_heard"]["top1"] = json!("en-canadian");
    seg["accent_heard"]["profile_ref"] = json!("en-canadian");
    call_args_err(&rig.neemu, "neemu_analyze_voice", base(seg));
    // Unknown accent profile.
    let mut seg = speech_segment();
    seg["accent_heard"] = json!({
        "profile_ref": "en-xx-nope",
        "distribution": { "en-xx-nope": 1.0 },
        "top1": "en-xx-nope",
        "margin": 1.0,
        "intensity": 0.5
    });
    call_args_err(&rig.neemu, "neemu_analyze_voice", base(seg));
}

#[test]
fn assess_artifact_gate_is_a_typed_refusal() {
    let rig = av_rig("s3-adv-artifact");
    let (work_id, preflight_id) = cleared_work(&rig, "Voice Study", "A");
    let analyzed = call(
        &rig.neemu,
        "neemu_analyze_voice",
        json!({ "work_id": work_id, "preflight_id": preflight_id, "segments": [speech_segment()] }),
    );
    let voice_id = analyzed["record_ids"][0].clone();
    let base = |extra: Value| {
        let mut args = json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "voice_record_ids": [voice_id],
            "rights_basis": { "basis": "estate-owned performance", "evidence_ref": "estate://rights/x" },
        });
        args.as_object_mut()
            .unwrap()
            .extend(extra.as_object().cloned().unwrap());
        args
    };
    // Absent artifact: typed refusal, not an assessment.
    let kind = call_kind(
        &rig.neemu,
        "neemu_assess_voice_clonability",
        base(json!({})),
    );
    assert_eq!(kind, "rights_artifact_missing");
    // A boolean attestation is not an artifact.
    let kind = call_kind(
        &rig.neemu,
        "neemu_assess_voice_clonability",
        base(json!({ "consent_artifact": true })),
    );
    assert_eq!(kind, "rights_artifact_missing");
    // One field missing: still a refusal (incomplete artifact).
    let mut artifact = consent_artifact();
    artifact.as_object_mut().unwrap().remove("retained_until");
    let kind = call_kind(
        &rig.neemu,
        "neemu_assess_voice_clonability",
        base(json!({ "consent_artifact": artifact })),
    );
    assert_eq!(kind, "rights_artifact_missing");
    // A malformed consent recording hash: refusal.
    let mut artifact = consent_artifact();
    artifact["consent_recording_hash"] = json!("not-a-hash");
    let kind = call_kind(
        &rig.neemu,
        "neemu_assess_voice_clonability",
        base(json!({ "consent_artifact": artifact })),
    );
    assert_eq!(kind, "rights_artifact_missing");
}

#[test]
fn assess_failed_axes_record_not_clonable() {
    let rig = av_rig("s3-adv-axes");
    let (work_id, preflight_id) = cleared_work(&rig, "Voice Study", "A");
    let analyzed = call(
        &rig.neemu,
        "neemu_analyze_voice",
        json!({ "work_id": work_id, "preflight_id": preflight_id, "segments": [speech_segment()] }),
    );
    let voice_id = analyzed["record_ids"][0].clone();
    let assess = |artifact: Value, basis: Value| {
        call(
            &rig.neemu,
            "neemu_assess_voice_clonability",
            json!({
                "work_id": work_id,
                "preflight_id": preflight_id,
                "voice_record_ids": [voice_id],
                "consent_artifact": artifact,
                "rights_basis": basis,
            }),
        )
    };
    // Speaker match failed → NOT-CLONABLE record (not a refusal).
    let mut artifact = consent_artifact();
    artifact["consent_vs_training_speaker_match"] = json!("failed");
    let result = assess(
        artifact,
        json!({ "basis": "estate-owned performance", "evidence_ref": "estate://rights/x" }),
    );
    assert_eq!(result["verdict"], json!("NOT-CLONABLE"));
    // Unknown rights basis → NOT-CLONABLE.
    let result = assess(consent_artifact(), Value::Null);
    assert_eq!(result["verdict"], json!("NOT-CLONABLE"));
    let record = rig
        .neemu
        .load_record("records", &work_id, result["record_id"].as_str().unwrap())
        .unwrap();
    assert_eq!(record["rights_axis"]["pass"], json!(false));
}

#[test]
fn assess_technical_axis_fails_unmeasured() {
    let rig = av_rig("s3-adv-technical");
    let (work_id, preflight_id) = cleared_work(&rig, "Voice Study", "A");
    // A short, unmeasured segment: too little speech, no SNR.
    let analyzed = call(
        &rig.neemu,
        "neemu_analyze_voice",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "segments": [{
                "media_name": "clip.wav", "start_s": 12.0, "end_s": 17.0,
                "speaker": "performer.one", "segment_kind": "speech",
                "transcript": "A short line.", "scene_function": "aside"
            }],
        }),
    );
    let voice_id = analyzed["record_ids"][0].clone();
    let result = call(
        &rig.neemu,
        "neemu_assess_voice_clonability",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "voice_record_ids": [voice_id],
            "consent_artifact": consent_artifact(),
            "rights_basis": { "basis": "estate-owned performance", "evidence_ref": "estate://rights/x" },
        }),
    );
    assert_eq!(result["verdict"], json!("NOT-CLONABLE"));
    let record = rig
        .neemu
        .load_record("records", &work_id, result["record_id"].as_str().unwrap())
        .unwrap();
    assert_eq!(record["technical_axis"]["pass"], json!(false));
    assert!(
        !record["technical_axis"]["reasons"]
            .as_array()
            .unwrap()
            .is_empty(),
        "reasons recorded: {record}"
    );
}

#[test]
fn derive_refusal_battery() {
    let rig = av_rig("s3-adv-derive");
    let (work_id, preflight_id, _voice_id, assessment_id) = clonable_assessment(&rig, false);
    // Identity adaptation plan (neutral everything, no accent move).
    let mut args = derive_args(&work_id, &preflight_id, &assessment_id);
    args["adaptation"] = json!({
        "pitch_semitones": 0.0, "formant_ratio": 1.0,
        "tempo_character": "unchanged", "accent_target": null
    });
    let kind = call_kind(&rig.neemu, "neemu_derive_voice", args);
    assert_eq!(kind, "clonability_refusal");
    // Over-class request: high-fidelity on standard evidence (delta 3).
    let mut args = derive_args(&work_id, &preflight_id, &assessment_id);
    args["derivation_class"] = json!("high-fidelity");
    let kind = call_kind(&rig.neemu, "neemu_derive_voice", args);
    assert_eq!(kind, "refusal_gate_tier");
    // Dead engine license fails closed (delta 4).
    let mut args = derive_args(&work_id, &preflight_id, &assessment_id);
    args["engine"]["license_alive"] = json!(false);
    let kind = call_kind(&rig.neemu, "neemu_derive_voice", args);
    assert_eq!(kind, "clonability_refusal");
    // Target identity IS the source speaker.
    let mut args = derive_args(&work_id, &preflight_id, &assessment_id);
    args["target_identity"] = json!("performer.one");
    let kind = call_kind(&rig.neemu, "neemu_derive_voice", args);
    assert_eq!(kind, "clonability_refusal");
    // Unknown assessment id is an argument error.
    let args = derive_args(&work_id, &preflight_id, "assess_nope");
    call_args_err(&rig.neemu, "neemu_derive_voice", args);
}

#[test]
fn derive_from_not_clonable_refuses() {
    let rig = av_rig("s3-adv-notclonable");
    let (work_id, preflight_id) = cleared_work(&rig, "Voice Study", "A");
    let analyzed = call(
        &rig.neemu,
        "neemu_analyze_voice",
        json!({ "work_id": work_id, "preflight_id": preflight_id, "segments": [speech_segment()] }),
    );
    let voice_id = analyzed["record_ids"][0].clone();
    let mut artifact = consent_artifact();
    artifact["consent_vs_training_speaker_match"] = json!("not_run");
    let assessed = call(
        &rig.neemu,
        "neemu_assess_voice_clonability",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "voice_record_ids": [voice_id],
            "consent_artifact": artifact,
            "rights_basis": { "basis": "estate-owned performance", "evidence_ref": "estate://rights/x" },
        }),
    );
    assert_eq!(assessed["verdict"], json!("NOT-CLONABLE"));
    let kind = call_kind(
        &rig.neemu,
        "neemu_derive_voice",
        derive_args(
            &work_id,
            &preflight_id,
            assessed["record_id"].as_str().unwrap(),
        ),
    );
    assert_eq!(kind, "clonability_refusal");
}

#[test]
fn emit_refusal_battery() {
    let rig = av_rig("s3-adv-emit");
    let (work_id, preflight_id, voice_id, assessment_id) = clonable_assessment(&rig, true);
    call(
        &rig.neemu,
        "neemu_derive_voice",
        derive_args(&work_id, &preflight_id, &assessment_id),
    );
    let dialogue = call(
        &rig.neemu,
        "neemu_extract_dialogue",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "page_start": 0,
            "page_end": 0,
            "lines": [{ "page": 0, "speaker": "lead", "text": "Look into my eyes now" }],
        }),
    );
    let dialogue_id = dialogue["record_ids"][0].as_str().unwrap().to_string();
    let emit = |cues: Value, target: Value| {
        let mut args = json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "cues": cues,
        });
        if !target.is_null() {
            args["target_voice"] = target;
        }
        args
    };
    let one_cue = |delivery: Value| json!([{ "dialogue_id": dialogue_id, "voice_id": voice_id, "delivery": delivery }]);
    // Missing target_voice entirely.
    let kind = call_kind(
        &rig.neemu,
        "neemu_emit_torori_cue",
        emit(one_cue(json!({})), Value::Null),
    );
    assert_eq!(kind, "missing_target_voice");
    // A target that is a source performer, not an estate identity.
    let kind = call_kind(
        &rig.neemu,
        "neemu_emit_torori_cue",
        emit(one_cue(json!({})), json!("performer.one")),
    );
    assert_eq!(kind, "missing_target_voice");
    // Out-of-range f0: refused, never clamped.
    let kind = call_kind(
        &rig.neemu,
        "neemu_emit_torori_cue",
        emit(
            one_cue(json!({ "f0_median_hz": 700.0 })),
            json!("voice.derived.other"),
        ),
    );
    assert_eq!(kind, "conditioning_refusal");
    // Accent intensity without a profile: frozen pair law.
    let kind = call_kind(
        &rig.neemu,
        "neemu_emit_torori_cue",
        emit(
            one_cue(json!({ "accent_intensity": 0.5 })),
            json!("voice.derived.other"),
        ),
    );
    assert_eq!(kind, "conditioning_refusal");
    // Unknown delivery key: argument error, never silently dropped.
    call_args_err(
        &rig.neemu,
        "neemu_emit_torori_cue",
        emit(
            one_cue(json!({ "vibrato": 0.3 })),
            json!("voice.derived.other"),
        ),
    );
}

#[test]
fn preflight_failed_av_work_refuses_analysis() {
    // A .wav whose bytes are not WAV: registration hashes it, the
    // preflight fails on format, and analysis refuses on that
    // preflight (no silent pass).
    let root = temp_dir("s3-adv-badwav");
    let media = root.join("media");
    fs::create_dir_all(&media).unwrap();
    fs::write(media.join("a-page.png"), png_bytes(800, 1200, 3)).unwrap();
    fs::write(media.join("clip.wav"), b"this is not a wav file at all").unwrap();
    let rig = Rig {
        neemu: neemu::Neemu::open(root.join("registry")).unwrap(),
        root,
        media_dir: media.to_string_lossy().to_string(),
    };
    let work = register(&rig, "Voice Study", "A");
    let work_id = work["work_id"].as_str().unwrap().to_string();
    let preflight = call(
        &rig.neemu,
        "neemu_validate_ingest",
        json!({ "work_id": work_id }),
    );
    assert_eq!(preflight["passed"], json!(false));
    screen_cleared(&rig.neemu, &work_id);
    let preflight_id = preflight["record_id"].as_str().unwrap().to_string();
    let kind = call_kind(
        &rig.neemu,
        "neemu_analyze_voice",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "segments": [speech_segment()],
        }),
    );
    assert_eq!(kind, "refusal_not_screened");
}

#[test]
fn silence_rule_no_audio_output_path() {
    // SPEC §7 S3 silence gate: no playback path in the codebase,
    // checked not assumed. (1) The dependency set carries no
    // audio-output crate. (2) No source file shells out to a player
    // or opens an audio device API.
    let manifest =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).expect("Cargo.toml");
    for banned in [
        "rodio",
        "cpal",
        "alsa-sys",
        "libpulse",
        "jack",
        "ao-sys",
        "portaudio",
    ] {
        assert!(
            !manifest.contains(banned),
            "audio-output dependency present: {banned}"
        );
    }
    let src_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    for entry in fs::read_dir(&src_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "rs") {
            let text = fs::read_to_string(&path).unwrap();
            for banned in [
                "aplay", "paplay", "pw-play", "ffplay", "mpv --", "afplay", "/dev/snd",
            ] {
                assert!(
                    !text.contains(banned),
                    "playback path {banned} in {}",
                    path.display()
                );
            }
        }
    }
}
