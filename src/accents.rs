//! neemu — accent profile registry (SPEC §3.12).
//!
//! Seed list of 21 profiles, each carrying phonetic features, a
//! prosody profile, a reaction set, expected confusions, and
//! conditioning defaults. Profiles are *for performance* (delta 9):
//! the registry also defines how a voice sounds when it laughs,
//! gasps, and cries, so every profile's reaction set is non-empty —
//! none are speech-only (SPEC §7 S3 gate). The seeded set is static
//! in S3: callers may reference profiles but not edit them, so the
//! registry is identical in every neemu process (determinism).

use serde_json::{json, Value};

/// The §3.12 seed, embedded at build time.
const SEED_JSON: &str = include_str!("accents_seed.json");

/// All seed profiles, in seed order.
pub fn profiles() -> Vec<Value> {
    serde_json::from_str(SEED_JSON).expect("accent seed JSON is valid (build-time constant)")
}

/// One profile by id, when seeded.
pub fn profile(profile_id: &str) -> Option<Value> {
    profiles()
        .into_iter()
        .find(|p| p["profile_id"].as_str() == Some(profile_id))
}

/// Whether a profile id is seeded.
pub fn exists(profile_id: &str) -> bool {
    profile(profile_id).is_some()
}

/// The complete registry as a value (record embedding + tests).
pub fn registry() -> Value {
    json!({
        "registry": "accent-profiles",
        "version": 1,
        "profile_count": 21,
        "profiles": profiles(),
    })
}
