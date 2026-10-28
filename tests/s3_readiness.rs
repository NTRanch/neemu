//! S3 readiness harness — post-build form (2026-10-28).
//!
//! The W5 provisional harness validated a brief-§8 draft fixture
//! (four JSON files) against shapes S3 *would* emit. Torori T3 has
//! since frozen the real artefacts, and S3 is built; this harness
//! now validates, in torori's ingest order:
//!
//! 1. neemu's conditioning key set + required set against torori's
//!    frozen schema descriptor (properties / required), when the
//!    torori tree is locatable beside this crate;
//! 2. the recorded descriptor hashes (the parity claim that stands
//!    even when the torori tree is absent);
//! 3. the emitted-format fixture `tests/fixtures/s3/bundle-v1/` —
//!    a `bundle.ron` in the frozen directory format whose final
//!    provenance link must equal the content hash re-derived from
//!    the fixture's recorded cue values via torori's canonical
//!    recipe (sha256 over serde_json of {cues, pairs,
//!    asset_sha256s}).

use neemu::s3::{
    CONDITIONING_KEYS, CONDITIONING_REQUIRED, CONDITIONING_SCHEMA_SHA256, CUE_BUNDLE_SCHEMA_SHA256,
    DELIVERY_KEYS,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("s3")
        .join("bundle-v1")
}

fn torori_schema(name: &str) -> Option<(Vec<u8>, Value)> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../torori-build-20261027/data/schemas")
        .join(name);
    let bytes = std::fs::read(path).ok()?;
    let value: Value = serde_json::from_slice(&bytes).ok()?;
    Some((bytes, value))
}

/// The frozen required set, corrected by the W5 §5 diff: the
/// provisional harness required `grade` (optional in the frozen
/// schema) and omitted `determinism_class` (required).
const FROZEN_REQUIRED: [&str; 4] = [
    "breath_bracket",
    "determinism_class",
    "identity_ref",
    "text",
];

#[test]
fn conditioning_keys_match_frozen_schema() {
    let mut mine: Vec<&str> = CONDITIONING_KEYS.to_vec();
    mine.sort();
    assert_eq!(mine.len(), 14);
    // Delivery keys = the property set minus `text` (the line itself).
    let mut delivery: Vec<&str> = DELIVERY_KEYS.to_vec();
    delivery.sort();
    let expect_delivery: Vec<&str> = mine.iter().filter(|k| **k != "text").copied().collect();
    assert_eq!(delivery, expect_delivery);
    let mut required: Vec<&str> = CONDITIONING_REQUIRED.to_vec();
    required.sort();
    let mut frozen: Vec<&str> = FROZEN_REQUIRED.to_vec();
    frozen.sort();
    assert_eq!(required, frozen, "required-set drift (the W5 §5 fix)");

    let Some((bytes, descriptor)) = torori_schema("conditioning.schema.json") else {
        eprintln!("torori tree not locatable; descriptor parity rests on the recorded hash");
        return;
    };
    assert_eq!(sha256_hex(&bytes), CONDITIONING_SCHEMA_SHA256);
    let mut properties: Vec<String> = descriptor["properties"]
        .as_object()
        .expect("descriptor properties")
        .keys()
        .cloned()
        .collect();
    properties.sort();
    assert_eq!(
        properties,
        mine.iter().map(|k| k.to_string()).collect::<Vec<_>>()
    );
    let mut descriptor_required: Vec<String> = descriptor["required"]
        .as_array()
        .expect("descriptor required")
        .iter()
        .map(|r| r.as_str().unwrap().to_string())
        .collect();
    descriptor_required.sort();
    assert_eq!(
        descriptor_required,
        frozen.iter().map(|k| k.to_string()).collect::<Vec<_>>()
    );
}

#[test]
fn bundle_descriptor_hash_recorded() {
    let Some((bytes, _)) = torori_schema("cue-bundle.schema.json") else {
        eprintln!("torori tree not locatable; bundle parity rests on the recorded hash");
        return;
    };
    assert_eq!(sha256_hex(&bytes), CUE_BUNDLE_SCHEMA_SHA256);
}

#[test]
fn fixture_bundle_is_frozen_format() {
    let dir = fixture_dir();
    let ron = std::fs::read_to_string(dir.join("bundle.ron"))
        .expect("fixture bundle.ron (frozen directory format)");
    let fixture: Value =
        serde_json::from_slice(&std::fs::read(dir.join("fixture.json")).expect("fixture.json"))
            .expect("fixture values");
    // Schema version first (torori's ingest order).
    assert!(ron.contains("(schema_version: 1"), "{ron}");
    assert!(ron.contains("producer: \"neemu\""), "{ron}");
    // Tier law: the fixture cue is Measured and carries an f0.
    assert!(ron.contains("tier: Measured"), "{ron}");
    assert!(ron.contains("f0_median_hz: Some(196.0)"), "{ron}");
    // Content hash: re-derive from the recorded cue values with
    // torori's canonical recipe; the RON's FINAL provenance link
    // must carry exactly it.
    let canonical = json!({
        "cues": [fixture["cue"]],
        "pairs": [],
        "asset_sha256s": [],
    });
    let derived = sha256_hex(serde_json::to_string(&canonical).unwrap().as_bytes());
    assert_eq!(derived, fixture["content_hash"].as_str().unwrap());
    let final_link = format!("artifact_sha256: \"{derived}\"");
    let position = ron.rfind(&final_link).expect("final provenance link");
    assert!(
        position > ron.find("provenance: [").unwrap(),
        "the content-hash link is the final provenance link"
    );
    assert!(ron.contains(&format!(
        "bundle_id: \"{}\"",
        fixture["bundle_id"].as_str().unwrap()
    )));
}

#[test]
fn fixture_readme_names_the_frozen_artefacts() {
    let readme = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("s3")
            .join("README.md"),
    )
    .expect("fixtures README");
    assert!(readme.contains("VALIDATED"), "README must be flipped");
    assert!(readme.contains(CONDITIONING_SCHEMA_SHA256));
    assert!(readme.contains(CUE_BUNDLE_SCHEMA_SHA256));
}
