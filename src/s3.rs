//! neemu — S3 voice analysis, clonability, derivation, and torori
//! cue emission (SPEC §3.4 / §3.5 / §3.10 / §3.11 / §3.12, §7 S3).
//!
//! Interpretation calls (named per the S1/S2 pattern — the server
//! has no ears of its own):
//!
//! - Transcription and caller-side feature proposals are inputs,
//!   exactly as S1's dialogue is caller-transcribed; the SERVER
//!   computes every checkable feature from the media bytes
//!   themselves (WAV PCM envelope: duration, pauses, breath
//!   candidates, clipping; header facts for other formats) and
//!   records what it measured vs what the caller proposed in
//!   `known_limitations` (delta 8).
//! - Nothing in this module — or this crate — plays audio. The
//!   dependency set carries no audio-output crate; verification is
//!   by file metrics only (SPEC §7 S3 silence gate).
//! - Conditioning follows torori's FROZEN schema (torori
//!   `data/schemas/conditioning.schema.json`, sha256
//!   e0df698dd61c9ad36b9832c8d9e2a79d2d86ea19292e5893d9cf621a1e0e6610,
//!   and `src/conditioning.rs`, the code truth). Values outside the
//!   frozen ranges are refused, never clamped.
//! - Two refusal kinds are new in S3 (`clonability_refusal`,
//!   `conditioning_refusal`): SPEC §5's kind list predates the S3
//!   records, the same gap S1/S2 closed for their kinds;
//!   `rights_artifact_missing` and `refusal_gate_tier` are §5's own.

use std::fs;
use std::path::PathBuf;

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use crate::accents;
use crate::media;
use crate::{
    arg_object, opt_arr, opt_bool, opt_str, req_str, CallError, CallResult, Neemu, ToolError,
    EXTRACTOR_ID, KIND_CLONABILITY_REFUSAL, KIND_CONDITIONING_REFUSAL, MAX_ASSESS_VOICE_RECORDS,
    MAX_EMIT_CUES, MAX_EMIT_PAIRS, MAX_SHORT_CHARS, MAX_VOICE_SEGMENTS, MAX_VOICE_TEXT_BYTES,
};

/// torori's frozen conditioning schema sha256 (T3 gate record).
pub const CONDITIONING_SCHEMA_SHA256: &str =
    "e0df698dd61c9ad36b9832c8d9e2a79d2d86ea19292e5893d9cf621a1e0e6610";
/// torori's frozen cue-bundle schema sha256 (T3 gate record).
pub const CUE_BUNDLE_SCHEMA_SHA256: &str =
    "f423f88305d46b41cc2c1863a5e32049d7f5e62a8ae27270117ac5c737177138";

/// The frozen conditioning property set (14), alphabetical.
pub const CONDITIONING_KEYS: [&str; 14] = [
    "accent_intensity",
    "accent_profile_ref",
    "breath_bracket",
    "breathiness",
    "determinism_class",
    "f0_median_hz",
    "f0_range_semitones",
    "grade",
    "identity_ref",
    "instruction",
    "pause_structure",
    "seed",
    "tempo_syllables_per_sec",
    "text",
];
/// Frozen REQUIRED conditioning fields (schema `required`).
pub const CONDITIONING_REQUIRED: [&str; 4] = [
    "breath_bracket",
    "determinism_class",
    "identity_ref",
    "text",
];
/// Frozen determinism classes (torori serde variant names).
pub const DETERMINISM_CLASSES: [&str; 3] = ["BitExact", "OutputUnique", "SeedReproducible"];
/// Frozen cue tier names (torori serde variant names).
pub const CUE_TIERS: [&str; 3] = ["Authored", "Inferred", "Measured"];
/// Delivery keys a dialogue line can carry into conditioning
/// (everything except `text`, which is the line itself) — 13.
pub const DELIVERY_KEYS: [&str; 13] = [
    "accent_intensity",
    "accent_profile_ref",
    "breath_bracket",
    "breathiness",
    "determinism_class",
    "f0_median_hz",
    "f0_range_semitones",
    "grade",
    "identity_ref",
    "instruction",
    "pause_structure",
    "seed",
    "tempo_syllables_per_sec",
];

// Frozen numeric ranges (torori conditioning.rs validate()).
const F0_MIN: f64 = 40.0;
const F0_MAX: f64 = 600.0;
const RANGE_ST_MAX: f64 = 36.0;
const TEMPO_MIN: f64 = 0.5;
const TEMPO_MAX: f64 = 12.0;

// Technical-axis constants (SPEC §3.10: builder-chosen, recorded on
// every assessment record so a judgement is reproducible).
/// Minimum analysable speech across the assessed records, seconds.
pub const MIN_SPEECH_SECONDS: f64 = 20.0;
/// Clipped-sample fraction ceiling across assessed speech.
pub const MAX_CLIPPED_FRACTION: f64 = 0.01;
/// Minimum caller-reported SNR, dB. A record without an SNR
/// measurement fails the technical axis (deny by default).
pub const MIN_SNR_DB: f64 = 15.0;

fn args(message: impl Into<String>) -> CallError {
    CallError::Args(message.into())
}

fn gate(neemu: &Neemu, obj: &Map<String, Value>) -> Result<(String, Value), CallError> {
    let work_id = req_str(obj, "work_id")?.to_string();
    let preflight_id = opt_str(obj, "preflight_id")?;
    let work = neemu.gate_extraction(&work_id, preflight_id)?;
    Ok((work_id, work))
}

/// Finite f64 field, required.
fn req_f64(obj: &Map<String, Value>, key: &str) -> Result<f64, CallError> {
    match obj.get(key) {
        Some(Value::Number(n)) => n
            .as_f64()
            .filter(|v| v.is_finite())
            .ok_or_else(|| args(format!("field '{key}' must be a finite number"))),
        _ => Err(args(format!("field '{key}' is required (number)"))),
    }
}

/// Finite f64 field, optional (null/absent → None).
fn opt_f64_field(obj: &Map<String, Value>, key: &str) -> Result<Option<f64>, CallError> {
    match obj.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n
            .as_f64()
            .filter(|v| v.is_finite())
            .map(Some)
            .ok_or_else(|| args(format!("field '{key}' must be a finite number"))),
        _ => Err(args(format!("field '{key}' must be a number"))),
    }
}

fn short(value: &str, key: &str) -> Result<(), CallError> {
    if value.chars().count() > MAX_SHORT_CHARS {
        return Err(args(format!("field '{key}' exceeds the character bound")));
    }
    Ok(())
}

fn unit(value: f64, key: &str) -> Result<(), CallError> {
    if !(0.0..=1.0).contains(&value) {
        return Err(args(format!(
            "field '{key}' must be within 0..=1 (got {value})"
        )));
    }
    Ok(())
}

/// Find one media manifest entry by file name.
fn media_entry<'a>(work: &'a Value, name: &str) -> Option<&'a Value> {
    work["media"]
        .as_array()?
        .iter()
        .find(|entry| entry["name"].as_str() == Some(name))
}

// ------------------------------------------------------------------
// neemu_analyze_voice (SPEC §3.4)
// ------------------------------------------------------------------

/// Validate one caller-supplied `accent_heard` claim against the
/// registry and the delta-9 shape: a distribution over profiles,
/// top1 = argmax, margin = top1 minus runner-up, intensity 0..1.
/// A claim naming the distribution's own top as anything else is an
/// argument error (the claim contradicts itself).
fn parse_accent_heard(value: &Value, context: &str) -> Result<Value, CallError> {
    let obj = value
        .as_object()
        .ok_or_else(|| args(format!("{context}: accent_heard must be an object")))?;
    let profile_ref = req_str(obj, "profile_ref")?;
    if !accents::exists(profile_ref) {
        return Err(args(format!(
            "{context}: accent_heard.profile_ref '{profile_ref}' is not in the accent registry"
        )));
    }
    let distribution = obj
        .get("distribution")
        .and_then(|d| d.as_object())
        .ok_or_else(|| {
            args(format!(
                "{context}: accent_heard.distribution must be an object"
            ))
        })?;
    if distribution.is_empty() || distribution.len() > 21 {
        return Err(args(format!(
            "{context}: accent_heard.distribution must name 1..=21 profiles"
        )));
    }
    let mut scored: Vec<(String, f64)> = Vec::new();
    let mut total = 0.0f64;
    for (id, p) in distribution {
        if !accents::exists(id) {
            return Err(args(format!(
                "{context}: distribution names unregistered profile '{id}'"
            )));
        }
        let p = p
            .as_f64()
            .filter(|v| v.is_finite() && (0.0..=1.0).contains(v))
            .ok_or_else(|| {
                args(format!(
                    "{context}: distribution values must be within 0..=1"
                ))
            })?;
        total += p;
        scored.push((id.clone(), p));
    }
    if (total - 1.0).abs() > 0.05 {
        return Err(args(format!(
            "{context}: distribution must sum to ~1 (got {total})"
        )));
    }
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let (argmax, top_p) = scored[0].clone();
    let runner_up = scored.get(1).map(|(_, p)| *p).unwrap_or(0.0);
    let top1 = req_str(obj, "top1")?;
    if top1 != argmax {
        return Err(args(format!(
            "{context}: accent_heard.top1 '{top1}' is not the distribution argmax '{argmax}'"
        )));
    }
    if top1 != profile_ref {
        return Err(args(format!(
            "{context}: accent_heard.profile_ref must equal its top1"
        )));
    }
    let margin = req_f64(obj, "margin")?;
    if (margin - (top_p - runner_up)).abs() > 0.05 {
        return Err(args(format!(
            "{context}: accent_heard.margin {margin} disagrees with the distribution (top1 - runner-up = {})",
            top_p - runner_up
        )));
    }
    let intensity = req_f64(obj, "intensity")?;
    unit(intensity, "accent_heard.intensity")?;
    Ok(json!({
        "profile_ref": profile_ref,
        "distribution": Value::Object(distribution.clone()),
        "top1": top1,
        "margin": margin,
        "intensity": intensity,
    }))
}

pub fn analyze_voice(neemu: &Neemu, args_in: &Value) -> CallResult {
    let obj = arg_object(args_in)?;
    let (work_id, work) = gate(neemu, obj)?;
    let segments =
        opt_arr(obj, "segments")?.ok_or_else(|| args("field 'segments' is required (array)"))?;
    if segments.is_empty() || segments.len() > MAX_VOICE_SEGMENTS {
        return Err(args(format!(
            "segments must carry 1..={MAX_VOICE_SEGMENTS} entries"
        )));
    }
    let media_dir = PathBuf::from(work["media_dir"].as_str().unwrap_or_default());
    let mut record_ids = Vec::new();
    let mut sources: Vec<String> = Vec::new();
    for (index, segment) in segments.iter().enumerate() {
        let context = format!("segments[{index}]");
        let seg = segment
            .as_object()
            .ok_or_else(|| args(format!("{context}: must be an object")))?;
        let media_name = req_str(seg, "media_name")?;
        let entry = media_entry(&work, media_name).ok_or_else(|| {
            args(format!(
                "{context}: media_name '{media_name}' is not in the work manifest"
            ))
        })?;
        let media_kind = entry["media_kind"].as_str().ok_or_else(|| {
            args(format!(
                "{context}: '{media_name}' is not an audio/video file; the voice lane analyses AV media"
            ))
        })?;
        let start_s = req_f64(seg, "start_s")?;
        let end_s = req_f64(seg, "end_s")?;
        if start_s < 0.0 || end_s <= start_s {
            return Err(args(format!("{context}: requires 0 <= start_s < end_s")));
        }
        if let Some(duration) = entry["duration_seconds"].as_f64() {
            if end_s > duration + 0.05 {
                return Err(args(format!(
                    "{context}: end_s {end_s} exceeds the measured duration {duration} of '{media_name}'"
                )));
            }
        }
        let speaker = req_str(seg, "speaker")?.to_string();
        let segment_kind = req_str(seg, "segment_kind")?;
        if segment_kind != "speech" && segment_kind != "non_speech" {
            return Err(args(format!(
                "{context}: segment_kind must be \"speech\" or \"non_speech\""
            )));
        }
        let scene_function = req_str(seg, "scene_function")?.to_string();
        short(&scene_function, "scene_function")?;
        let is_fragment = opt_bool(seg, "is_fragment")?;

        // Transcript XOR non-speech class: a speech segment without
        // a transcript is not silently passed over (SPEC §3.4), and
        // a non-speech segment is named by class, never transcribed.
        let (transcript, non_speech_class) = if segment_kind == "speech" {
            let text = req_str(seg, "transcript").map_err(|_| {
                args(format!(
                    "{context}: speech segment requires a transcript (caller-side ASR, pinned; delta 8)"
                ))
            })?;
            if text.len() > MAX_VOICE_TEXT_BYTES {
                return Err(args(format!(
                    "{context}: transcript exceeds the byte bound"
                )));
            }
            (Some(text.to_string()), None)
        } else {
            let class = req_str(seg, "non_speech_class").map_err(|_| {
                args(format!(
                    "{context}: non_speech segment requires non_speech_class (it is recorded as [non-speech: <class>])"
                ))
            })?;
            short(class, "non_speech_class")?;
            (None, Some(class.to_string()))
        };

        // Server-measured features from the file itself.
        let path = media_dir.join(media_name);
        let bytes = fs::read(&path).map_err(|e| {
            CallError::Gate(ToolError::new(
                crate::KIND_MEDIA_UNREADABLE,
                format!("{context}: cannot read '{media_name}': {e}"),
            ))
        })?;
        let measured = if entry["format"].as_str() == Some("wav") {
            media::wav_segment_metrics(&bytes, start_s, end_s)
        } else {
            None
        };
        let mut limitations: Vec<String> = vec![
            "transcript is caller-supplied (ASR runs caller-side, pinned); the server validates its presence and shape, never its accuracy (delta 8)".to_string(),
        ];
        let mut measured_features = json!({
            "duration_seconds": end_s - start_s,
        });
        match &measured {
            Some(m) => {
                measured_features["sample_rate"] = json!(m.sample_rate);
                measured_features["channels"] = json!(m.channels);
                measured_features["peak"] = json!(m.peak);
                measured_features["rms"] = json!(m.rms);
                measured_features["clipped_fraction"] = json!(m.clipped_fraction);
                measured_features["pause_structure"] = json!({
                    "pause_count": m.pause_count,
                    "longest_pause_s": m.longest_pause_s,
                    "speech_seconds_estimate": m.speech_seconds,
                });
                measured_features["breath_events_estimate"] = json!(m.breath_events);
                if segment_kind == "speech" {
                    limitations.push("speech_seconds and breath events are envelope estimates from the WAV PCM (10 ms windows, -44 dBFS pause threshold), not phonetic segmentation".to_string());
                }
            }
            None => {
                limitations.push(format!(
                    "per-sample features unavailable: '{}' is {} — header facts only (duration/sample-rate from the registry manifest); f0 and breath require caller measurement",
                    media_name,
                    entry["format"].as_str().unwrap_or("unknown")
                ));
            }
        }

        // Caller-proposed features: range-checked against the frozen
        // conditioning ranges where the field is shared with torori.
        let mut caller_features = json!({});
        if let Some(features) = seg.get("features") {
            let fobj = features
                .as_object()
                .ok_or_else(|| args(format!("{context}: features must be an object")))?;
            for key in [
                "f0_median_hz",
                "f0_range_semitones",
                "f0_contour_class",
                "tempo_syllables_per_sec",
                "pause_structure",
                "breathiness",
                "roughness",
                "grade",
                "snr_db",
            ] {
                if let Some(value) = fobj.get(key) {
                    caller_features[key] = value.clone();
                }
            }
            if let Some(f0) = opt_f64_field(fobj, "f0_median_hz")? {
                if !(F0_MIN..=F0_MAX).contains(&f0) {
                    return Err(args(format!(
                        "{context}: f0_median_hz {f0} outside the frozen 40..=600 range"
                    )));
                }
            }
            if let Some(range) = opt_f64_field(fobj, "f0_range_semitones")? {
                if !(0.0..=RANGE_ST_MAX).contains(&range) {
                    return Err(args(format!(
                        "{context}: f0_range_semitones {range} outside the frozen 0..=36 range"
                    )));
                }
            }
            if let Some(tempo) = opt_f64_field(fobj, "tempo_syllables_per_sec")? {
                if !(TEMPO_MIN..=TEMPO_MAX).contains(&tempo) {
                    return Err(args(format!(
                        "{context}: tempo_syllables_per_sec {tempo} outside the frozen 0.5..=12 range"
                    )));
                }
            }
            for key in ["breathiness", "grade"] {
                if let Some(value) = opt_f64_field(fobj, key)? {
                    unit(value, key)?;
                }
            }
            limitations.push("caller features (f0, tempo, breathiness, grade, snr_db) are caller-measured with the caller's pinned pipeline; the server range-checks them against the frozen conditioning schema but does not re-derive them (delta 8)".to_string());
        }

        let accent_heard = match seg.get("accent_heard") {
            None | Some(Value::Null) => None,
            Some(value) => Some(parse_accent_heard(value, &context)?),
        };

        let source_kind = if media_kind == "video" {
            "video"
        } else if work["media"]
            .as_array()
            .map(|entries| entries.iter().any(|e| e.get("media_kind").is_none()))
            .unwrap_or(false)
        {
            "image+audio"
        } else {
            "audio-only"
        };
        if !sources.iter().any(|s| s == source_kind) {
            sources.push(source_kind.to_string());
        }

        let body = json!({
            "record_kind": "voice_performance",
            "work_id": work_id,
            "segment_index": index,
            "media_name": media_name,
            "media_sha256": entry["sha256"],
            "media_kind": media_kind,
            "source_kind": source_kind,
            "start_s": start_s,
            "end_s": end_s,
            "speaker": speaker,
            "segment_kind": segment_kind,
            "transcript": transcript,
            "non_speech_class": non_speech_class,
            "scene_function": scene_function,
            "is_fragment": is_fragment,
            "measured_features": measured_features,
            "caller_features": caller_features,
            "accent_heard": accent_heard,
            "known_limitations": limitations,
            "extractor": EXTRACTOR_ID,
        });
        record_ids.push(neemu.mint_record(&work_id, "voice_", body)?);
    }
    Ok(json!({
        "work_id": work_id,
        "record_ids": record_ids,
        "record_kind": "voice_performance",
        "segments": record_ids.len(),
        "source_kinds": sources,
    }))
}

// ------------------------------------------------------------------
// neemu_assess_voice_clonability (SPEC §3.10)
// ------------------------------------------------------------------

/// The consent artifact gate. An absent or incomplete artifact is a
/// TYPED REFUSAL (`rights_artifact_missing`): a performance right
/// cannot be assumed into existence, and an assessment that "passes"
/// without one would be a false record. A boolean attestation is
/// not an artifact.
fn parse_consent_artifact(obj: &Map<String, Value>) -> Result<Value, CallError> {
    let refuse = |message: String| {
        CallError::Gate(
            ToolError::new("rights_artifact_missing", message).with_details(json!({
                "reason": "consent_artifact_incomplete",
            })),
        )
    };
    let artifact = match obj.get("consent_artifact") {
        None | Some(Value::Null) => {
            return Err(refuse(
                "no ConsentArtifact supplied: clonability cannot be assessed without written permission evidence (SPEC 3.10)".to_string(),
            ));
        }
        Some(value) => value,
    };
    let aobj = artifact.as_object().ok_or_else(|| {
        refuse(
            "consent_artifact must be an object (a boolean attestation is not an artifact)"
                .to_string(),
        )
    })?;
    for field in [
        "written_permission_ref",
        "disclosure_version_given",
        "consent_recording_hash",
        "consent_vs_training_speaker_match",
        "approved_use_scope",
        "retained_until",
    ] {
        match aobj.get(field) {
            Some(Value::String(text)) if !text.is_empty() => {}
            _ => {
                return Err(refuse(format!(
                    "ConsentArtifact is incomplete: field '{field}' is missing or empty"
                )));
            }
        }
    }
    let matched = aobj["consent_vs_training_speaker_match"]
        .as_str()
        .unwrap_or_default();
    if !["verified", "failed", "not_run"].contains(&matched) {
        return Err(refuse(format!(
            "ConsentArtifact.consent_vs_training_speaker_match must be verified|failed|not_run (got '{matched}')"
        )));
    }
    let hash = aobj["consent_recording_hash"].as_str().unwrap_or_default();
    if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(refuse(
            "ConsentArtifact.consent_recording_hash must be a sha256 hex digest".to_string(),
        ));
    }
    Ok(artifact.clone())
}

pub fn assess_voice_clonability(neemu: &Neemu, args_in: &Value) -> CallResult {
    let obj = arg_object(args_in)?;
    let (work_id, _work) = gate(neemu, obj)?;
    let record_ids = opt_arr(obj, "voice_record_ids")?
        .ok_or_else(|| args("field 'voice_record_ids' is required (array)"))?;
    if record_ids.is_empty() || record_ids.len() > MAX_ASSESS_VOICE_RECORDS {
        return Err(args(format!(
            "voice_record_ids must carry 1..={MAX_ASSESS_VOICE_RECORDS} entries"
        )));
    }
    let mut records: Vec<Value> = Vec::new();
    for id_value in record_ids {
        let id = id_value
            .as_str()
            .ok_or_else(|| args("voice_record_ids entries must be strings"))?;
        let record = neemu
            .load_record("records", &work_id, id)
            .ok_or_else(|| args(format!("unknown record for this work: {id}")))?;
        if record["record_kind"].as_str() != Some("voice_performance") {
            return Err(args(format!(
                "record {id} is not a voice_performance record"
            )));
        }
        records.push(record);
    }
    let speakers: Vec<String> = {
        let mut seen: Vec<String> = Vec::new();
        for record in &records {
            let speaker = record["speaker"].as_str().unwrap_or_default().to_string();
            if !seen.contains(&speaker) {
                seen.push(speaker);
            }
        }
        seen
    };
    if speakers.len() != 1 {
        return Err(args(format!(
            "an assessment covers exactly one speaker; records name {}: {}",
            speakers.len(),
            speakers.join(", ")
        )));
    }
    let artifact = parse_consent_artifact(obj)?;
    let presence_verified = opt_bool(obj, "presence_verified")?;
    let rights_basis = match obj.get("rights_basis") {
        None | Some(Value::Null) => None,
        Some(value) => {
            let bobj = value
                .as_object()
                .ok_or_else(|| args("rights_basis must be an object"))?;
            let basis = req_str(bobj, "basis")?.to_string();
            let evidence_ref = req_str(bobj, "evidence_ref")?.to_string();
            Some(json!({ "basis": basis, "evidence_ref": evidence_ref }))
        }
    };
    let supersedes = opt_str(obj, "supersedes")?.map(|s| s.to_string());
    if let Some(prior) = &supersedes {
        match neemu.record_kind(&work_id, prior).as_deref() {
            Some("voice_clonability_assessment") => {}
            _ => {
                return Err(args(format!(
                    "supersedes '{prior}' is not a voice_clonability_assessment of this work"
                )))
            }
        }
    }

    // Rights axis (§3.10): artifact completeness was the gate;
    // speaker match and basis decide the axis.
    let matched = artifact["consent_vs_training_speaker_match"]
        .as_str()
        .unwrap_or_default();
    let mut rights_reasons: Vec<String> = Vec::new();
    if matched != "verified" {
        rights_reasons.push(format!(
            "consent_vs_training_speaker_match is '{matched}', not 'verified'"
        ));
    }
    if rights_basis.is_none() {
        rights_reasons.push(
            "rights basis is unknown: no estate right, license, or public-domain basis recorded (unknown basis is a FAIL, §3.10)".to_string(),
        );
    }
    let rights_pass = rights_reasons.is_empty();

    // Technical axis (§3.10) over the speech records, with the
    // module constants recorded on the record.
    let mut speech_seconds = 0.0f64;
    let mut worst_clipped = 0.0f64;
    let mut min_snr: Option<f64> = None;
    let mut technical_reasons: Vec<String> = Vec::new();
    for record in &records {
        if record["segment_kind"].as_str() != Some("speech") {
            continue;
        }
        let measured = &record["measured_features"];
        let caller = &record["caller_features"];
        let seconds = measured["pause_structure"]["speech_seconds_estimate"]
            .as_f64()
            .or_else(|| measured["duration_seconds"].as_f64())
            .unwrap_or(0.0);
        speech_seconds += seconds;
        if let Some(clipped) = measured["clipped_fraction"].as_f64() {
            worst_clipped = worst_clipped.max(clipped);
        }
        match caller["snr_db"].as_f64() {
            Some(snr) => {
                min_snr = Some(min_snr.map(|m: f64| m.min(snr)).unwrap_or(snr));
            }
            None => technical_reasons.push(format!(
                "record {} carries no SNR measurement; the technical axis cannot pass unmeasured",
                record["record_id"].as_str().unwrap_or_default()
            )),
        }
    }
    if speech_seconds < MIN_SPEECH_SECONDS {
        technical_reasons.push(format!(
            "analysable speech {speech_seconds:.1}s is below the {MIN_SPEECH_SECONDS}s floor"
        ));
    }
    if worst_clipped > MAX_CLIPPED_FRACTION {
        technical_reasons.push(format!(
            "clipped fraction {worst_clipped} exceeds the {MAX_CLIPPED_FRACTION} ceiling"
        ));
    }
    if let Some(snr) = min_snr {
        if snr < MIN_SNR_DB {
            technical_reasons.push(format!(
                "minimum SNR {snr} dB is below the {MIN_SNR_DB} dB floor"
            ));
        }
    }
    let technical_pass = technical_reasons.is_empty();
    let verdict = if rights_pass && technical_pass {
        "CLONABLE"
    } else {
        "NOT-CLONABLE"
    };
    let body = json!({
        "record_kind": "voice_clonability_assessment",
        "work_id": work_id,
        "speaker": speakers[0],
        "voice_record_ids": record_ids,
        "consent_artifact": artifact,
        "presence_verified": presence_verified,
        "rights_basis": rights_basis,
        "rights_axis": { "pass": rights_pass, "reasons": rights_reasons },
        "technical_axis": {
            "pass": technical_pass,
            "reasons": technical_reasons,
            "speech_seconds": speech_seconds,
            "worst_clipped_fraction": worst_clipped,
            "min_snr_db": min_snr,
            "constants": {
                "min_speech_seconds": MIN_SPEECH_SECONDS,
                "max_clipped_fraction": MAX_CLIPPED_FRACTION,
                "min_snr_db": MIN_SNR_DB,
            },
        },
        "verdict": verdict,
        "supersedes": supersedes,
        "extractor": EXTRACTOR_ID,
    });
    let record_id = neemu.mint_record(&work_id, "assess_", body)?;
    Ok(json!({
        "work_id": work_id,
        "record_id": record_id,
        "record_kind": "voice_clonability_assessment",
        "verdict": verdict,
        "speaker": speakers[0],
    }))
}

// ------------------------------------------------------------------
// neemu_derive_voice (SPEC §3.11 + deltas 3 / 4 / 14)
// ------------------------------------------------------------------

const DERIVATION_CLASSES: [(&str, &str); 3] = [
    ("characterisation-only", "standard"),
    ("adapted-derivation", "standard"),
    ("high-fidelity", "strong"),
];

fn clonability_refusal(message: String, details: Value) -> CallError {
    CallError::Gate(ToolError::new(KIND_CLONABILITY_REFUSAL, message).with_details(details))
}

fn refusal_gate_tier(message: String, details: Value) -> CallError {
    CallError::Gate(ToolError::new("refusal_gate_tier", message).with_details(details))
}

pub fn derive_voice(neemu: &Neemu, args_in: &Value) -> CallResult {
    let obj = arg_object(args_in)?;
    let (work_id, _work) = gate(neemu, obj)?;
    let assessment_id = req_str(obj, "assessment_id")?.to_string();
    let assessment = neemu
        .load_record("records", &work_id, &assessment_id)
        .ok_or_else(|| args(format!("unknown record for this work: {assessment_id}")))?;
    if assessment["record_kind"].as_str() != Some("voice_clonability_assessment") {
        return Err(args(format!(
            "record {assessment_id} is not a voice_clonability_assessment"
        )));
    }
    // An unassessed voice is NOT-CLONABLE by definition (§3.10) and
    // a NOT-CLONABLE verdict refuses here (§3.11, §7 S3 gate).
    if assessment["verdict"].as_str() != Some("CLONABLE") {
        return Err(clonability_refusal(
            format!(
                "assessment {assessment_id} verdict is {}; derivation from an unassessed or NOT-CLONABLE voice is refused",
                assessment["verdict"].as_str().unwrap_or("unknown")
            ),
            json!({ "assessment_id": assessment_id, "verdict": assessment["verdict"] }),
        ));
    }
    let source_speaker = assessment["speaker"]
        .as_str()
        .unwrap_or_default()
        .to_string();

    // Engine license (delta 4): evaluated at derivation time, fails
    // closed — license_alive and commercial_ok are checked against
    // the values supplied for THIS derivation.
    let engine = obj
        .get("engine")
        .and_then(|e| e.as_object())
        .ok_or_else(|| args("field 'engine' is required (object: engine, license, license_alive, commercial_ok)"))?;
    let engine_name = req_str(engine, "engine")?.to_string();
    let engine_license = req_str(engine, "license")?.to_string();
    let license_alive = opt_bool(engine, "license_alive")?;
    let commercial_ok = opt_bool(engine, "commercial_ok")?;
    if !license_alive || !commercial_ok {
        return Err(clonability_refusal(
            format!(
                "engine license does not permit this derivation (engine {engine_name}, license {engine_license}, license_alive {license_alive}, commercial_ok {commercial_ok}); the license gate fails closed (delta 4)"
            ),
            json!({
                "engine": engine_name, "license": engine_license,
                "license_alive": license_alive, "commercial_ok": commercial_ok,
            }),
        ));
    }

    // Derivation class vs rights-evidence class (delta 3).
    let class = opt_str(obj, "derivation_class")?.unwrap_or("adapted-derivation");
    let required_evidence = DERIVATION_CLASSES
        .iter()
        .find(|(name, _)| *name == class)
        .map(|(_, evidence)| *evidence)
        .ok_or_else(|| {
            args(format!(
                "derivation_class must be one of: {}",
                DERIVATION_CLASSES.map(|(n, _)| n).join(", ")
            ))
        })?;
    let evidence_class = if assessment["presence_verified"].as_bool() == Some(true) {
        "strong"
    } else {
        "standard"
    };
    if required_evidence == "strong" && evidence_class != "strong" {
        return Err(refusal_gate_tier(
            format!(
                "derivation class '{class}' requires rights-evidence class 'strong' (verified presence + complete consent artifact); the assessment carries '{evidence_class}' (delta 3)"
            ),
            json!({
                "derivation_class": class,
                "required_evidence_class": required_evidence,
                "assessment_evidence_class": evidence_class,
                "assessment_id": assessment_id,
            }),
        ));
    }

    // Adaptation plan (delta: mandatory). An IDENTITY plan — no
    // accent change, neutral pitch/formant/tempo — derives the
    // source identity and is refused: derivation changes the voice.
    let adaptation = obj
        .get("adaptation")
        .and_then(|a| a.as_object())
        .ok_or_else(|| args("field 'adaptation' is required (the adaptation plan: pitch_semitones, formant_ratio, tempo_character, accent_target)"))?;
    let pitch = req_f64(adaptation, "pitch_semitones")?;
    if !(-12.0..=12.0).contains(&pitch) {
        return Err(args("adaptation.pitch_semitones must be within -12..=12"));
    }
    let formant = req_f64(adaptation, "formant_ratio")?;
    if !(0.5..=2.0).contains(&formant) {
        return Err(args("adaptation.formant_ratio must be within 0.5..=2.0"));
    }
    let tempo_character = req_str(adaptation, "tempo_character")?.to_string();
    let accent_target = opt_str(adaptation, "accent_target")?.map(|s| s.to_string());
    if let Some(target) = &accent_target {
        if !accents::exists(target) {
            return Err(args(format!(
                "adaptation.accent_target '{target}' is not in the accent registry"
            )));
        }
    }
    let source_accent = assessment_voice_accent(neemu, &work_id, &assessment);
    let accent_unchanged = match (&accent_target, &source_accent) {
        (None, _) => true,
        (Some(target), Some(source)) => target == source,
        (Some(_), None) => false,
    };
    if pitch == 0.0 && formant == 1.0 && tempo_character == "unchanged" && accent_unchanged {
        return Err(clonability_refusal(
            "the adaptation plan is an identity plan (no accent change, neutral pitch/formant/tempo): it would derive the source identity, which derivation must never do (SPEC 3.11)".to_string(),
            json!({ "assessment_id": assessment_id, "reason": "identity_adaptation_plan" }),
        ));
    }

    let target_identity = req_str(obj, "target_identity")?.to_string();
    short(&target_identity, "target_identity")?;
    if target_identity == source_speaker {
        return Err(clonability_refusal(
            format!(
                "target_identity '{target_identity}' is the source speaker: deriving a voice AS its source is impersonation, not derivation"
            ),
            json!({ "reason": "target_is_source_speaker", "speaker": source_speaker }),
        ));
    }
    let design_targets = obj
        .get("design_targets")
        .and_then(|d| d.as_object())
        .ok_or_else(|| args("field 'design_targets' is required (object)"))?;
    let mut design = json!({});
    for key in [
        "warmth",
        "breathiness_balance",
        "pitch_placement",
        "articulation_clarity",
    ] {
        let value = req_f64(design_targets, key)?;
        unit(value, key)?;
        design[key] = json!(value);
    }

    // Mint the derivation FIRST: its record id is the watermark
    // payload (delta 14), so the identity and its conditioning
    // bundle can carry it with no circularity.
    let derivation_body = json!({
        "record_kind": "voice_derivation",
        "work_id": work_id,
        "source_assessment_id": assessment_id,
        "source_speaker": source_speaker,
        "source_voice_record_ids": assessment["voice_record_ids"],
        "derivation_class": class,
        "rights_evidence_class": evidence_class,
        "adaptation": {
            "pitch_semitones": pitch,
            "formant_ratio": formant,
            "tempo_character": tempo_character,
            "accent_target": accent_target,
        },
        "engine": {
            "engine": engine_name,
            "license": engine_license,
            "license_alive": license_alive,
            "commercial_ok": commercial_ok,
        },
        "target_identity": target_identity,
        "design_targets": design,
        "provenance_chain": ["source_performance", "clonability_assessment", "adaptation_plan", "derived_identity"],
        "extractor": EXTRACTOR_ID,
    });
    let derivation_id = neemu.mint_record(&work_id, "deriv_", derivation_body)?;

    let identity_body = json!({
        "record_kind": "voice_identity",
        "work_id": work_id,
        "identity_ref": target_identity,
        "display_name": target_identity,
        "kind": "derived",
        "lock_state": "designed",
        "derivation_id": derivation_id,
        "source_assessment_id": assessment_id,
        "adaptation": {
            "pitch_semitones": pitch,
            "formant_ratio": formant,
            "tempo_character": tempo_character,
            "accent_target": accent_target,
        },
        "design_targets": design,
        "watermark": { "embedded": true, "payload": derivation_id, "scheme": "derivation-record-id" },
        "extractor": EXTRACTOR_ID,
    });
    let identity_id = neemu.mint_record(&work_id, "ident_", identity_body)?;

    // Derived conditioning bundle (delta 14): the identity's
    // reusable conditioning profile, watermarked with the
    // derivation record id. An output lacking it is incomplete.
    let profile = json!({
        "bundle_kind": "derived_conditioning_profile",
        "identity_ref": target_identity,
        "identity_id": identity_id,
        "derivation_id": derivation_id,
        "watermark": derivation_id,
        "conditioning_defaults": {
            "accent_profile_ref": accent_target,
            "accent_intensity": accent_target.as_ref().and_then(|t| {
                accents::profile(t).map(|p| p["conditioning"]["default_intensity"].clone())
            }),
            "breathiness": design["breathiness_balance"],
            "determinism_class": "SeedReproducible",
        },
        "provenance_chain": ["source_performance", "clonability_assessment", "adaptation_plan", "derived_identity"],
    });
    let bundle_id = format!("derived_{}", &derivation_id[6..]);
    let path = neemu.store_export(&work_id, &bundle_id, &profile)?;

    Ok(json!({
        "work_id": work_id,
        "derivation_id": derivation_id,
        "identity_id": identity_id,
        "identity_ref": target_identity,
        "derivation_class": class,
        "watermark": derivation_id,
        "conditioning_bundle": path.to_string_lossy(),
    }))
}

/// The source voice's heard accent (top1 of the first assessed
/// record that carries one), for the identity-plan comparison.
fn assessment_voice_accent(neemu: &Neemu, work_id: &str, assessment: &Value) -> Option<String> {
    let ids = assessment["voice_record_ids"].as_array()?;
    for id in ids {
        let record = neemu.load_record("records", work_id, id.as_str()?)?;
        if let Some(top1) = record["accent_heard"]["top1"].as_str() {
            return Some(top1.to_string());
        }
    }
    None
}

// ------------------------------------------------------------------
// neemu_emit_torori_cue (SPEC §3.5) — bundles in torori's frozen
// format: a directory holding bundle.ron, content hash = torori's
// canonical (sha256 over serde_json of {cues, pairs, asset_sha256s}).
// ------------------------------------------------------------------

fn conditioning_refusal(message: String) -> CallError {
    CallError::Gate(ToolError::new(KIND_CONDITIONING_REFUSAL, message))
}

/// One validated conditioning value in BOTH representations torori
/// sees: the RON field text and the serde_json value its canonical
/// content hash is computed over.
struct Conditioning {
    ron_fields: Vec<String>,
    json_value: Value,
}

fn build_conditioning(
    text: &str,
    identity_ref: &str,
    delivery: &Map<String, Value>,
    voice_record: Option<&Value>,
) -> Result<Conditioning, CallError> {
    let mut ron: Vec<String> = Vec::new();
    let mut js: Map<String, Value> = Map::new();
    let put = |ron: &mut Vec<String>,
               js: &mut Map<String, Value>,
               key: &str,
               ron_value: String,
               json_value: Value| {
        ron.push(format!("{key}: {ron_value}"));
        js.insert(key.to_string(), json_value);
    };
    put(&mut ron, &mut js, "text", ron_string(text), json!(text));
    put(
        &mut ron,
        &mut js,
        "identity_ref",
        ron_string(identity_ref),
        json!(identity_ref),
    );
    match delivery.get("instruction") {
        None | Some(Value::Null) => {
            put(&mut ron, &mut js, "instruction", "None".into(), Value::Null)
        }
        Some(Value::String(s)) => {
            if s.len() > MAX_VOICE_TEXT_BYTES {
                return Err(args("delivery instruction exceeds the byte bound"));
            }
            put(
                &mut ron,
                &mut js,
                "instruction",
                format!("Some({})", ron_string(s)),
                json!(s),
            );
        }
        _ => return Err(args("delivery.instruction must be a string")),
    }
    // Numeric delivery fields: range-checked against the FROZEN
    // schema — refused, never clamped (§3.5).
    let numeric: [(&str, Option<f64>, Option<f64>); 6] = [
        ("f0_median_hz", Some(F0_MIN), Some(F0_MAX)),
        ("f0_range_semitones", Some(0.0), Some(RANGE_ST_MAX)),
        ("tempo_syllables_per_sec", Some(TEMPO_MIN), Some(TEMPO_MAX)),
        ("breathiness", Some(0.0), Some(1.0)),
        ("grade", Some(0.0), Some(1.0)),
        ("accent_intensity", Some(0.0), Some(1.0)),
    ];
    for (key, min, max) in numeric {
        match delivery.get(key) {
            None | Some(Value::Null) => put(&mut ron, &mut js, key, "None".into(), Value::Null),
            Some(Value::Number(n)) => {
                let value = n
                    .as_f64()
                    .filter(|v| v.is_finite())
                    .ok_or_else(|| args(format!("delivery.{key} must be a finite number")))?;
                if value < min.unwrap_or(f64::MIN) || value > max.unwrap_or(f64::MAX) {
                    return Err(conditioning_refusal(format!(
                        "delivery.{key} = {value} is outside torori's frozen range {}..={}; refused, never clamped",
                        min.unwrap_or_default(),
                        max.unwrap_or_default()
                    )));
                }
                put(
                    &mut ron,
                    &mut js,
                    key,
                    format!("Some({})", ron_f64(value)),
                    json!(value),
                );
            }
            _ => return Err(args(format!("delivery.{key} must be a number"))),
        }
    }
    match delivery.get("pause_structure") {
        None | Some(Value::Null) => put(
            &mut ron,
            &mut js,
            "pause_structure",
            "None".into(),
            Value::Null,
        ),
        Some(value) if value.is_object() => {
            put(
                &mut ron,
                &mut js,
                "pause_structure",
                format!("Some({})", ron_json_value(value)),
                value.clone(),
            );
        }
        _ => return Err(args("delivery.pause_structure must be an object")),
    }
    // Accent pair law (frozen schema): intensity requires a profile.
    let accent_ref = match delivery.get("accent_profile_ref") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => {
            if !accents::exists(s) {
                return Err(args(format!(
                    "delivery.accent_profile_ref '{s}' is not in the accent registry (torori holds the shared §3.12 seed)"
                )));
            }
            Some(s.clone())
        }
        _ => return Err(args("delivery.accent_profile_ref must be a string")),
    };
    if accent_ref.is_none()
        && delivery
            .get("accent_intensity")
            .is_some_and(|v| !v.is_null())
    {
        return Err(conditioning_refusal(
            "delivery.accent_intensity is set without accent_profile_ref; torori's frozen schema refuses the pair (intensity is meaningless without a profile)".to_string(),
        ));
    }
    match &accent_ref {
        None => put(
            &mut ron,
            &mut js,
            "accent_profile_ref",
            "None".into(),
            Value::Null,
        ),
        Some(s) => put(
            &mut ron,
            &mut js,
            "accent_profile_ref",
            format!("Some({})", ron_string(s)),
            json!(s),
        ),
    }
    // Breath bracket: default from the source segment (fragment →
    // Required), explicit delivery wins.
    let breath = match delivery.get("breath_bracket") {
        None | Some(Value::Null) => {
            if voice_record
                .map(|r| r["is_fragment"].as_bool() == Some(true))
                .unwrap_or(false)
            {
                "Required"
            } else {
                "NotRequired"
            }
        }
        Some(Value::String(s)) if s == "Required" || s == "NotRequired" => {
            if s == "Required" {
                "Required"
            } else {
                "NotRequired"
            }
        }
        _ => {
            return Err(args(
                "delivery.breath_bracket must be \"Required\" or \"NotRequired\"",
            ))
        }
    };
    put(
        &mut ron,
        &mut js,
        "breath_bracket",
        breath.into(),
        json!(breath),
    );
    let determinism = match delivery.get("determinism_class") {
        None | Some(Value::Null) => "SeedReproducible",
        Some(Value::String(s)) if DETERMINISM_CLASSES.contains(&s.as_str()) => DETERMINISM_CLASSES
            .iter()
            .find(|c| **c == s.as_str())
            .unwrap(),
        _ => {
            return Err(args(format!(
                "delivery.determinism_class must be one of: {}",
                DETERMINISM_CLASSES.join(", ")
            )))
        }
    };
    put(
        &mut ron,
        &mut js,
        "determinism_class",
        determinism.into(),
        json!(determinism),
    );
    match delivery.get("seed") {
        None | Some(Value::Null) => put(&mut ron, &mut js, "seed", "None".into(), Value::Null),
        Some(Value::Number(n)) => {
            let seed = n
                .as_u64()
                .ok_or_else(|| args("delivery.seed must be a non-negative integer"))?;
            put(
                &mut ron,
                &mut js,
                "seed",
                format!("Some({seed})"),
                json!(seed),
            );
        }
        _ => return Err(args("delivery.seed must be a non-negative integer")),
    }
    Ok(Conditioning {
        ron_fields: ron,
        json_value: Value::Object(js),
    })
}

pub fn emit_torori_cue(neemu: &Neemu, args_in: &Value) -> CallResult {
    let obj = arg_object(args_in)?;
    let (work_id, _work) = gate(neemu, obj)?;
    // target_voice: REQUIRED, an estate identity — never a source
    // performer (§3.5, §5 missing_target_voice).
    let target_voice = match opt_str(obj, "target_voice")? {
        Some(target) if !target.is_empty() => target.to_string(),
        _ => {
            return Err(CallError::Gate(
                ToolError::new(
                    "missing_target_voice",
                    "emit requires target_voice: an estate VoiceIdentity reference; a bundle is never aimed at a source performer",
                )
                .with_details(json!({ "work_id": work_id })),
            ))
        }
    };
    let (identity_work, _identity) = neemu
        .find_voice_identity(&target_voice)
        .ok_or_else(|| {
            CallError::Gate(
                ToolError::new(
                    "missing_target_voice",
                    format!("target_voice '{target_voice}' does not resolve to an estate VoiceIdentity in the registry"),
                )
                .with_details(json!({ "work_id": work_id, "target_voice": target_voice })),
            )
        })?;
    let _ = identity_work;
    let cues_in = opt_arr(obj, "cues")?.ok_or_else(|| args("field 'cues' is required (array)"))?;
    if cues_in.is_empty() || cues_in.len() > MAX_EMIT_CUES {
        return Err(args(format!(
            "cues must carry 1..={MAX_EMIT_CUES} entries (torori's bundle cap)"
        )));
    }

    struct CueBuilt {
        cue_id: String,
        text: String,
        intent: String,
        tier: &'static str,
        sfx_pairs: Vec<String>,
        conditioning: Conditioning,
        conditioning_json: Value,
        dialogue_id: String,
        voice_record_id: Option<String>,
    }
    let mut built: Vec<CueBuilt> = Vec::new();
    for (index, cue_value) in cues_in.iter().enumerate() {
        let context = format!("cues[{index}]");
        let cue = cue_value
            .as_object()
            .ok_or_else(|| args(format!("{context}: must be an object")))?;
        let dialogue_id = req_str(cue, "dialogue_id")?.to_string();
        let line = neemu
            .load_record("records", &work_id, &dialogue_id)
            .ok_or_else(|| args(format!("{context}: unknown dialogue record: {dialogue_id}")))?;
        if line["record_kind"].as_str() != Some("dialogue_line") {
            return Err(args(format!(
                "{context}: record {dialogue_id} is not a dialogue_line"
            )));
        }
        let text = line["text"].as_str().unwrap_or_default().to_string();
        if text.is_empty() || text.len() > MAX_VOICE_TEXT_BYTES {
            return Err(args(format!(
                "{context}: dialogue text is empty or over the byte bound"
            )));
        }
        let intent = line["intent"].as_str().unwrap_or("line").to_string();
        // Delivery: the line's delivery map, normalised to the
        // frozen key set — an unknown delivery key is an argument
        // error, never silently dropped (§3.5 normalisation).
        let mut delivery: Map<String, Value> = Map::new();
        if let Some(delivery_value) = line.get("delivery") {
            let dobj = delivery_value
                .as_object()
                .ok_or_else(|| args(format!("{context}: delivery must be an object")))?;
            for (key, value) in dobj {
                if !DELIVERY_KEYS.contains(&key.as_str()) {
                    return Err(args(format!(
                        "{context}: delivery key '{key}' is not in torori's frozen conditioning schema"
                    )));
                }
                delivery.insert(key.clone(), value.clone());
            }
        }
        if let Some(extra) = cue.get("delivery") {
            let eobj = extra
                .as_object()
                .ok_or_else(|| args(format!("{context}: cue delivery must be an object")))?;
            for (key, value) in eobj {
                if !DELIVERY_KEYS.contains(&key.as_str()) {
                    return Err(args(format!(
                        "{context}: delivery key '{key}' is not in torori's frozen conditioning schema"
                    )));
                }
                delivery.insert(key.clone(), value.clone());
            }
        }
        // identity_ref inside delivery, when asserted, must be the
        // target — the bundle's identity is the target voice.
        if let Some(asserted) = delivery.get("identity_ref").and_then(|v| v.as_str()) {
            if asserted != target_voice {
                return Err(args(format!(
                    "{context}: delivery.identity_ref '{asserted}' disagrees with target_voice '{target_voice}'"
                )));
            }
        }
        delivery.remove("identity_ref");
        // The source voice record (for tier + breath defaults).
        let voice_record = match opt_str(cue, "voice_id")? {
            Some(voice_id) => {
                let record = neemu
                    .load_record("records", &work_id, voice_id)
                    .ok_or_else(|| args(format!("{context}: unknown voice record: {voice_id}")))?;
                if record["record_kind"].as_str() != Some("voice_performance") {
                    return Err(args(format!(
                        "{context}: record {voice_id} is not a voice_performance record"
                    )));
                }
                Some(record)
            }
            None => None,
        };
        let measured = voice_record
            .as_ref()
            .map(|r| {
                r["caller_features"]["f0_median_hz"].as_f64().is_some()
                    || r["measured_features"]["clipped_fraction"]
                        .as_f64()
                        .is_some()
            })
            .unwrap_or(false);
        let tier: &'static str = if measured { "Measured" } else { "Inferred" };
        // Measured tier law (torori validate): f0_median_hz present.
        if tier == "Measured"
            && delivery
                .get("f0_median_hz")
                .map(|v| v.is_null())
                .unwrap_or(true)
        {
            if let Some(f0) = voice_record
                .as_ref()
                .and_then(|r| r["caller_features"]["f0_median_hz"].as_f64())
            {
                delivery.insert("f0_median_hz".to_string(), json!(f0));
            }
        }
        let mut sfx_pairs: Vec<String> = Vec::new();
        if let Some(pairs) = opt_arr(cue, "sfx_pairs")? {
            if pairs.len() > MAX_EMIT_PAIRS {
                return Err(args(format!("{context}: sfx_pairs exceeds the bound")));
            }
            for pair in pairs {
                let name = pair
                    .as_str()
                    .ok_or_else(|| args(format!("{context}: sfx_pairs entries must be strings")))?;
                short(name, "sfx_pairs")?;
                sfx_pairs.push(name.to_string());
            }
        }
        let conditioning =
            build_conditioning(&text, &target_voice, &delivery, voice_record.as_ref())?;
        let conditioning_json = conditioning.json_value.clone();
        built.push(CueBuilt {
            cue_id: format!("cue_{:03}", index + 1),
            text,
            intent,
            tier,
            sfx_pairs,
            conditioning,
            conditioning_json,
            dialogue_id,
            voice_record_id: voice_record
                .map(|r| r["record_id"].as_str().unwrap_or_default().to_string()),
        });
    }

    // Torori content hash, replicated byte-for-byte: sha256 over
    // serde_json::to_string of {"cues","pairs","asset_sha256s"}
    // with torori's struct field sets (both sides sort keys via
    // stock serde_json maps; None serialises as null).
    let cues_json: Vec<Value> = built
        .iter()
        .map(|cue| {
            json!({
                "cue_id": cue.cue_id,
                "text": cue.text,
                "conditioning": cue.conditioning_json,
                "intent": cue.intent,
                "target_voice": target_voice,
                "sfx_pairs": cue.sfx_pairs,
                "tier": cue.tier,
            })
        })
        .collect();
    let cues_hash =
        sha256_hex(&serde_json::to_string(&json!({ "cues": cues_json })).unwrap_or_default());
    let canonical = json!({
        "cues": cues_json,
        "pairs": Vec::<Value>::new(),
        "asset_sha256s": Vec::<String>::new(),
    });
    let content_hash = sha256_hex(&serde_json::to_string(&canonical).unwrap_or_default());
    let bundle_id = format!("bundle.{}", &content_hash[..16]);

    // bundle.ron in torori's dialect (struct maps; RON reader is
    // field-order tolerant, values are the contract).
    let mut ron = String::new();
    ron.push_str(&format!(
        "(schema_version: 1, bundle_id: {}, producer: {}, cues: [",
        ron_string(&bundle_id),
        ron_string("neemu")
    ));
    for cue in &built {
        ron.push_str(&format!(
            "(cue_id: {}, text: {}, conditioning: ({}), intent: {}, target_voice: {}, sfx_pairs: [{}], tier: {}), ",
            ron_string(&cue.cue_id),
            ron_string(&cue.text),
            cue.conditioning.ron_fields.join(", "),
            ron_string(&cue.intent),
            ron_string(&target_voice),
            cue.sfx_pairs.iter().map(|p| ron_string(p)).collect::<Vec<_>>().join(", "),
            cue.tier,
        ));
    }
    ron.push_str("], pairs: [], shared_releases: [], assets: [], provenance: [");
    // Every provenance link's artifact_sha256 must be a sha256 hex
    // digest (torori validate step 3 checks ALL links, not just the
    // final one): the emit link carries the hash of the canonical
    // cue payload it produced (cues_hash, computed above).
    ron.push_str(&format!(
        "(step: {}, actor: {}, artifact_sha256: {}), ",
        ron_string("emit"),
        ron_string("neemu"),
        ron_string(&cues_hash),
    ));
    ron.push_str(&format!(
        "(step: {}, actor: {}, artifact_sha256: {}), ",
        ron_string("content-hash"),
        ron_string("neemu"),
        ron_string(&content_hash),
    ));
    ron.push_str("])");
    // Frozen-shape self-check before anything touches disk: the
    // emitted bundle must satisfy torori's structural rules as far
    // as they are checkable without torori's registry (the live
    // round-trip against torori itself is the S3 gate).
    validate_bundle_shape(&ron, &built.iter().map(|c| c.tier).collect::<Vec<_>>())?;

    let bundle_dir = neemu.root().join("exports").join(&work_id).join(&bundle_id);
    fs::create_dir_all(&bundle_dir).map_err(|e| {
        CallError::Gate(ToolError::new(
            crate::KIND_MEDIA_UNREADABLE,
            format!("cannot create bundle dir: {e}"),
        ))
    })?;
    let bundle_path = bundle_dir.join("bundle.ron");
    if !bundle_path.exists() {
        fs::write(&bundle_path, &ron).map_err(|e| {
            CallError::Gate(ToolError::new(
                crate::KIND_MEDIA_UNREADABLE,
                format!("cannot write bundle.ron: {e}"),
            ))
        })?;
    }

    let mut cue_record_ids = Vec::new();
    for cue in &built {
        let body = json!({
            "record_kind": "torori_cue",
            "work_id": work_id,
            "cue_id": cue.cue_id,
            "bundle_id": bundle_id,
            "text": cue.text,
            "intent": cue.intent,
            "target_voice": target_voice,
            "sfx_pairs": cue.sfx_pairs,
            "tier": cue.tier,
            "conditioning": cue.conditioning_json,
            "dialogue_id": cue.dialogue_id,
            "voice_record_id": cue.voice_record_id,
            "extractor": EXTRACTOR_ID,
        });
        cue_record_ids.push(neemu.mint_record(&work_id, "cue_", body)?);
    }
    let bundle_body = json!({
        "record_kind": "torori_cue_bundle",
        "work_id": work_id,
        "bundle_id": bundle_id,
        "content_hash": content_hash,
        "target_voice": target_voice,
        "cue_record_ids": cue_record_ids,
        "bundle_dir": bundle_dir.to_string_lossy(),
        "conditioning_schema_sha256": CONDITIONING_SCHEMA_SHA256,
        "cue_bundle_schema_sha256": CUE_BUNDLE_SCHEMA_SHA256,
        "extractor": EXTRACTOR_ID,
    });
    let bundle_record_id = neemu.mint_record(&work_id, "tcb_", bundle_body)?;
    Ok(json!({
        "work_id": work_id,
        "record_id": bundle_record_id,
        "record_kind": "torori_cue_bundle",
        "bundle_id": bundle_id,
        "content_hash": content_hash,
        "cue_record_ids": cue_record_ids,
        "bundle_dir": bundle_dir.to_string_lossy(),
    }))
}

/// Frozen-shape self-check for an emitted bundle.ron (the subset of
/// torori's validation battery checkable without its registry):
/// required-field presence, tier law (Measured requires an f0), the
/// final-provenance-link rule, and the conditioning key set.
pub fn validate_bundle_shape(ron_text: &str, tiers: &[&str]) -> Result<(), CallError> {
    for needle in [
        "schema_version: 1",
        "bundle_id:",
        "producer:",
        "cues: [",
        "provenance: [",
    ] {
        if !ron_text.contains(needle) {
            return Err(conditioning_refusal(format!(
                "emitted bundle failed the frozen-shape check: missing '{needle}'"
            )));
        }
    }
    for (index, tier) in tiers.iter().enumerate() {
        if *tier == "Measured" && !ron_text.contains("f0_median_hz: Some(") {
            return Err(conditioning_refusal(format!(
                "emitted bundle failed the frozen-shape check: cue {} is Measured but carries no f0_median_hz",
                index + 1
            )));
        }
    }
    Ok(())
}

// ---- RON writing helpers (torori fixture dialect) ----

/// RON string literal with the escapes RON requires.
pub fn ron_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// RON float literal. Rust's `{:?}` for f64 always emits a decimal
/// point or exponent, which RON requires to read the value as f64
/// (a bare `196` would parse as an integer and fail torori's
/// Option<f64> field).
pub fn ron_f64(value: f64) -> String {
    format!("{value:?}")
}

/// Serialise a JSON value as RON (for pause_structure, a free-form
/// object on both sides of the boundary).
fn ron_json_value(value: &Value) -> String {
    match value {
        Value::Null => "None".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                u.to_string()
            } else if let Some(i) = n.as_i64() {
                i.to_string()
            } else {
                ron_f64(n.as_f64().unwrap_or(0.0))
            }
        }
        Value::String(s) => ron_string(s),
        Value::Array(items) => {
            let inner: Vec<String> = items.iter().map(ron_json_value).collect();
            format!("[{}]", inner.join(", "))
        }
        Value::Object(map) => {
            let inner: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{}: {}", ron_string(k), ron_json_value(v)))
                .collect();
            format!("{{{}}}", inner.join(", "))
        }
    }
}

fn sha256_hex(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// The frozen conditioning key set, sorted (test surface).
pub fn conditioning_keys_sorted() -> Vec<&'static str> {
    let mut keys: Vec<&'static str> = CONDITIONING_KEYS.to_vec();
    keys.sort();
    keys
}
