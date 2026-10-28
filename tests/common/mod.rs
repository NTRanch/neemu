#![allow(dead_code)]
//! Shared fixtures for the neemu S1 test suites: temp registries,
//! synthetic page images (header-valid PNGs — the S1 lane sniffs
//! headers and hashes bytes, it does not decode pixels), and call
//! helpers over the library's tool dispatch.

use neemu::tools::call_tool;
use neemu::{CallError, Neemu};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A unique temp directory that the caller owns (removed by the OS
/// eventually; each test's registry is isolated).
pub fn temp_dir(tag: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("neemu-test-{}-{tag}-{n}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

/// Minimal PNG-shaped bytes: magic + IHDR with the given dimensions,
/// plus `seed` filler so distinct pages hash distinctly.
pub fn png_bytes(width: u32, height: u32, seed: u8) -> Vec<u8> {
    let mut bytes = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    bytes.extend_from_slice(&13u32.to_be_bytes());
    bytes.extend_from_slice(b"IHDR");
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.extend_from_slice(&[8, 6, 0, 0, 0]);
    bytes.extend_from_slice(&[seed; 16]);
    bytes
}

/// Write `count` pages named page-001.png.. into `dir`, all with the
/// same dimensions, and return the directory path as a string.
pub fn write_pages(dir: &PathBuf, count: usize, width: u32, height: u32) -> String {
    fs::create_dir_all(dir).expect("create media dir");
    for i in 0..count {
        let name = format!("page-{:03}.png", i + 1);
        fs::write(dir.join(name), png_bytes(width, height, i as u8)).expect("write page");
    }
    dir.to_string_lossy().to_string()
}

/// Write pages with per-page dimensions (S2 fixtures: dimension
/// changes are what make deterministic scene boundaries).
pub fn write_pages_dims(dir: &PathBuf, dims: &[(u32, u32)]) -> String {
    fs::create_dir_all(dir).expect("create media dir");
    for (i, (width, height)) in dims.iter().enumerate() {
        let name = format!("page-{:03}.png", i + 1);
        fs::write(dir.join(name), png_bytes(*width, *height, i as u8)).expect("write page");
    }
    dir.to_string_lossy().to_string()
}

/// A rig whose pages carry the given per-page dimensions.
pub fn rig_dims(tag: &str, dims: &[(u32, u32)]) -> Rig {
    let root = temp_dir(tag);
    let media = root.join("media");
    let media_dir = write_pages_dims(&media, dims);
    let neemu = Neemu::open(root.join("registry")).expect("open registry");
    Rig {
        neemu,
        root,
        media_dir,
    }
}

/// A ready registry + media fixture.
pub struct Rig {
    pub neemu: Neemu,
    pub root: PathBuf,
    pub media_dir: String,
}

pub fn rig(tag: &str, pages: usize) -> Rig {
    let root = temp_dir(tag);
    let media = root.join("media");
    let media_dir = write_pages(&media, pages, 800, 1200);
    let neemu = Neemu::open(root.join("registry")).expect("open registry");
    Rig {
        neemu,
        root,
        media_dir,
    }
}

/// Call a tool and expect success; returns the result value.
pub fn call(neemu: &Neemu, name: &str, args: Value) -> Value {
    match call_tool(neemu, name, &args) {
        Ok(result) => result,
        Err(error) => panic!("tool {name} unexpectedly failed: {error:?}"),
    }
}

/// Call a tool and expect a gate refusal; returns its typed kind.
pub fn call_kind(neemu: &Neemu, name: &str, args: Value) -> String {
    match call_tool(neemu, name, &args) {
        Ok(result) => panic!("tool {name} unexpectedly succeeded: {result}"),
        Err(CallError::Gate(error)) => error.kind.to_string(),
        Err(CallError::Args(message)) => panic!("tool {name} failed as args, not gate: {message}"),
    }
}

/// Call a tool and expect an invalid-arguments failure.
pub fn call_args_err(neemu: &Neemu, name: &str, args: Value) -> String {
    match call_tool(neemu, name, &args) {
        Ok(result) => panic!("tool {name} unexpectedly succeeded: {result}"),
        Err(CallError::Args(message)) => message,
        Err(CallError::Gate(error)) => {
            panic!("tool {name} failed as gate {}, not args", error.kind)
        }
    }
}

/// Register a work on the rig's media dir.
pub fn register(rig: &Rig, title: &str, tier: &str) -> Value {
    let mut args = json!({
        "title": title,
        "tags": ["hypnosis", "adult"],
        "source": { "kind": "owner_supplied", "ref": "test fixture" },
        "media_dir": rig.media_dir,
        "tier_claim": tier,
    });
    if tier == "T" {
        args["rights_evidence"] = json!([{ "kind": "license", "ref": "fixture-license-001" }]);
    }
    call(&rig.neemu, "neemu_register_work", args)
}

/// Register + validate; returns (work_id, preflight_id).
pub fn register_and_validate(rig: &Rig, title: &str, tier: &str) -> (String, String) {
    let work = register(rig, title, tier);
    let work_id = work["work_id"].as_str().expect("work id").to_string();
    let preflight = call(
        &rig.neemu,
        "neemu_validate_ingest",
        json!({ "work_id": work_id }),
    );
    assert_eq!(
        preflight["passed"],
        json!(true),
        "preflight should pass: {preflight}"
    );
    let preflight_id = preflight["record_id"]
        .as_str()
        .expect("preflight id")
        .to_string();
    (work_id, preflight_id)
}

/// Screen a work with positive adult evidence and no risk flags;
/// returns the screen record.
pub fn screen_cleared(neemu: &Neemu, work_id: &str) -> Value {
    let record = call(
        neemu,
        "neemu_screen_work",
        json!({
            "work_id": work_id,
            "evidence": {
                "adult_basis": [
                    { "kind": "age_statement", "detail": "work states all characters are adults" }
                ]
            }
        }),
    );
    assert_eq!(
        record["determination"],
        json!("cleared"),
        "screen should clear: {record}"
    );
    record
}

/// A fully gated work: registered, validated, screened cleared.
/// Returns (work_id, preflight_id).
pub fn cleared_work(rig: &Rig, title: &str, tier: &str) -> (String, String) {
    let (work_id, preflight_id) = register_and_validate(rig, title, tier);
    screen_cleared(&rig.neemu, &work_id);
    (work_id, preflight_id)
}

/// A synthetic 16-bit PCM mono WAV at 8 kHz with a fixed envelope:
/// loud (0.5 FS) [0,10) + [12,26), silence [10,12) + [26,27) +
/// [27.3,30), and a quiet (0.1 FS) burst [27,27.3) — 30 s total,
/// 3 pauses ≥ 150 ms, 1 breath candidate, 24.3 s of envelope speech.
pub fn wav_bytes() -> Vec<u8> {
    let rate: u32 = 8000;
    let seconds = 30usize;
    let mut samples: Vec<i16> = vec![0; rate as usize * seconds];
    let loud: i16 = 16384;
    let quiet: i16 = 3280;
    for (i, sample) in samples.iter_mut().enumerate() {
        let t = i as f64 / rate as f64;
        *sample = if t < 10.0 || (12.0..26.0).contains(&t) {
            loud
        } else if (27.0..27.3).contains(&t) {
            quiet
        } else {
            0
        };
    }
    let data_len = (samples.len() * 2) as u32;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&1u16.to_le_bytes()); // mono
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * 2).to_le_bytes()); // byte rate
    bytes.extend_from_slice(&2u16.to_le_bytes()); // block align
    bytes.extend_from_slice(&16u16.to_le_bytes()); // bits
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for sample in &samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}

/// A rig whose media dir holds one PNG page (index 0) and the
/// synthetic WAV (index 1) — an image+audio work for the S3 lane.
pub fn av_rig(tag: &str) -> Rig {
    let root = temp_dir(tag);
    let media = root.join("media");
    fs::create_dir_all(&media).expect("create media dir");
    fs::write(media.join("a-page.png"), png_bytes(800, 1200, 7)).expect("write page");
    fs::write(media.join("clip.wav"), wav_bytes()).expect("write wav");
    let media_dir = media.to_string_lossy().to_string();
    let neemu = Neemu::open(root.join("registry")).expect("open registry");
    Rig {
        neemu,
        root,
        media_dir,
    }
}

/// One speech segment over the whole synthetic WAV, with caller
/// features that pass the technical axis (f0 + SNR).
pub fn speech_segment() -> Value {
    json!({
        "media_name": "clip.wav",
        "start_s": 0.0,
        "end_s": 30.0,
        "speaker": "performer.one",
        "segment_kind": "speech",
        "transcript": "Look into my eyes and listen to my voice.",
        "scene_function": "induction patter",
        "features": { "f0_median_hz": 196.0, "f0_range_semitones": 6.0, "snr_db": 30.0 },
        "accent_heard": {
            "profile_ref": "en-us-general",
            "distribution": { "en-us-general": 0.8, "en-canadian": 0.2 },
            "top1": "en-us-general",
            "margin": 0.6,
            "intensity": 0.5
        }
    })
}

/// A complete ConsentArtifact (SPEC §3.10 shape).
pub fn consent_artifact() -> Value {
    json!({
        "written_permission_ref": "estate://consent/performer-one-2026",
        "disclosure_version_given": "cloning-disclosure-v3",
        "consent_recording_hash": "a".repeat(64),
        "consent_vs_training_speaker_match": "verified",
        "approved_use_scope": "estate voice derivation, commercial",
        "retained_until": "2036-10-28"
    })
}

/// The full S3 happy path up to a CLONABLE assessment; returns
/// (work_id, preflight_id, voice_record_id, assessment_id).
pub fn clonable_assessment(rig: &Rig, presence_verified: bool) -> (String, String, String, String) {
    let (work_id, preflight_id) = cleared_work(rig, "Voice Study", "A");
    let analyzed = call(
        &rig.neemu,
        "neemu_analyze_voice",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "segments": [speech_segment()],
        }),
    );
    let voice_id = analyzed["record_ids"][0]
        .as_str()
        .expect("voice id")
        .to_string();
    let assessed = call(
        &rig.neemu,
        "neemu_assess_voice_clonability",
        json!({
            "work_id": work_id,
            "preflight_id": preflight_id,
            "voice_record_ids": [voice_id],
            "consent_artifact": consent_artifact(),
            "rights_basis": { "basis": "estate-owned performance", "evidence_ref": "estate://rights/performer-one" },
            "presence_verified": presence_verified,
        }),
    );
    assert_eq!(
        assessed["verdict"],
        json!("CLONABLE"),
        "assessment: {assessed}"
    );
    let assessment_id = assessed["record_id"]
        .as_str()
        .expect("assessment id")
        .to_string();
    (work_id, preflight_id, voice_id, assessment_id)
}

/// One valid grid-read face observation for `page`.
pub fn face_observation(page: usize) -> Value {
    json!({
        "page": page,
        "character_slot": "lead",
        "channels": {
            "gaze": "focused, straining",
            "pupils": "hearts, solid",
            "crossing": "slight convergence",
            "lids": "0.42 aperture",
            "brows": "lowered, knit",
            "mouth": "parted gasp, small oval",
            "flush": "mid blush",
            "secondary": "sweat present, tears absent"
        },
        "f_code": "F-03",
        "basis": "grid-read"
    })
}
