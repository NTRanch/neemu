// The tools/list schema literal outgrew the default macro recursion
// limit at S2 (eleven tools); the limit is raised for that literal.
#![recursion_limit = "512"]

//! Neemu MCP — S1 core: registry store, the screen/tier gates, and the
//! image-lane tool implementations.
//!
//! S1 scope (SPEC §7): server skeleton under Tiger MCP rules; register /
//! screen with the canonical blocklist imported from the legal scraper
//! (see [`blocklist`]); validate-first ingest (§3.16, delta 16);
//! segment / faces / dialogue for page-sequence works. Tools from later
//! stages are absent, not stubbed.
//!
//! Gate law implemented here, in code (SPEC §2, §5):
//! - Every extraction tool requires a registered work whose recorded
//!   determination is `cleared` (else `screen_refusal`) AND a passing
//!   PreflightRecord belonging to that work (missing / unknown /
//!   mismatched / failed pre-flight → `refusal_not_screened`).
//! - Tier T is set only with rights-ladder evidence recorded at
//!   registration (else `tier_refusal`); Tier A material never enters a
//!   training-shaped export (else `tier_refusal`).
//! - Records are JSON, immutable, content-addressed by the SHA-256 of
//!   their canonical form (record id and `created_at` excluded from the
//!   hashed form, so identical inputs re-mint identical ids).

pub mod accents;
pub mod acts;
pub mod blocklist;
pub mod guard;
pub mod media;
pub mod s2;
pub mod s3;
pub mod server;
pub mod tools;

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Extractor stamp placed on every record (SPEC §3: extractor name +
/// version + model pins; S1 has no model pins — its stages are
/// deterministic or caller-measured, and say so per record `basis`).
pub const EXTRACTOR_ID: &str = "neemu/0.1.0";

/// The eight channels of the estate face language (SPEC §3.1).
pub const FACE_CHANNELS: [&str; 8] = [
    "gaze",
    "pupils",
    "crossing",
    "lids",
    "brows",
    "mouth",
    "flush",
    "secondary",
];

/// Canonical F-codes (face-states reference, F-01..F-12).
pub const F_CODES: [&str; 12] = [
    "F-01", "F-02", "F-03", "F-04", "F-05", "F-06", "F-07", "F-08", "F-09", "F-10", "F-11", "F-12",
];

/// Training lineages that hard-cap a model-derived record at Tier A
/// (delta 6), normalised (lowercase, alphanumerics only) for comparison.
pub const TAINTED_LINEAGES: [&str; 3] = ["affectnet", "rafdb", "fer2013"];

// ---- Per-call bounds (SPEC §5: declared, enforced, partials declared) ----

/// Maximum media files (pages) in one registered work.
pub const MAX_MEDIA_FILES: usize = 512;
/// Maximum bytes in one media file.
pub const MAX_MEDIA_FILE_BYTES: u64 = 64 * 1024 * 1024;
/// Maximum total media bytes in one work.
pub const MAX_MEDIA_TOTAL_BYTES: u64 = 512 * 1024 * 1024;
/// Maximum pages processed by one extraction call; larger ranges return
/// a declared partial with a continuation range.
pub const MAX_PAGES_PER_CALL: usize = 32;
/// Default segmentation parameter: maximum pages in one scene.
pub const DEFAULT_MAX_PAGES_PER_SCENE: usize = 24;
/// Maximum caller-settable pages per scene.
pub const MAX_PAGES_PER_SCENE_LIMIT: usize = 64;
/// Maximum face observations accepted in one extraction call.
pub const MAX_OBSERVATIONS_PER_CALL: usize = 128;
/// Maximum dialogue lines accepted in one extraction call.
pub const MAX_LINES_PER_CALL: usize = 256;
/// Maximum scenes returned by one segmentation call.
pub const MAX_SCENES_PER_CALL: usize = 64;
/// Maximum records returned by one list call.
pub const MAX_LIST_LIMIT: usize = 100;
/// Maximum characters in one free-text field (dialogue text, notes).
pub const MAX_TEXT_CHARS: usize = 4096;
/// Maximum characters in a short field (title, speaker, slot, code).
pub const MAX_SHORT_CHARS: usize = 512;
/// Maximum tags on one work.
pub const MAX_TAGS: usize = 64;

// ---- S2 per-call bounds (SPEC §3.3 / §3.6 / §3.15) ----

/// Maximum acts accepted in one `neemu_extract_acts` call.
pub const MAX_ACTS_PER_CALL: usize = 16;
/// Maximum participants in one act.
pub const MAX_PARTICIPANTS_PER_ACT: usize = 8;
/// Maximum contact pairs in one act.
pub const MAX_CONTACT_PAIRS_PER_ACT: usize = 32;
/// Maximum pose-track entries in one act.
pub const MAX_POSE_KEYS_PER_ACT: usize = 32;
/// Maximum entries in one short string list (sfx hooks, props).
pub const MAX_STRING_LIST: usize = 32;
/// Maximum beats in one `neemu_understand_scene` call.
pub const MAX_BEATS_PER_SCENE: usize = 32;
/// Maximum evidence refs on one claim (beat, machinery, dynamics).
pub const MAX_EVIDENCE_REFS: usize = 32;
/// Maximum candidate sources in one enrichment item.
pub const MAX_ENRICH_SOURCES_PER_CALL: usize = 16;
/// Maximum items processed in one enrichment batch call; the rest
/// are reported with a resume cursor (delta 23).
pub const MAX_ENRICH_ITEMS_PER_CALL: usize = 4;
/// Maximum serialized bytes of one enrichment source payload.
pub const MAX_ENRICH_PAYLOAD_BYTES: usize = 65_536;
/// Maximum versions in one enrichment version chain (delta 22).
pub const MAX_VERSIONS_PER_CALL: usize = 16;
/// Grouping confidence floor (delta 21): a heuristic match below it
/// stays separate. Builder-chosen constant, recorded on the bundle.
pub const GROUPING_CONFIDENCE_FLOOR: f64 = 0.5;

// ---- S3 per-call bounds (SPEC §3.4 / §3.5 / §3.10 / §3.11) ----

/// Maximum segments in one `neemu_analyze_voice` call.
pub const MAX_VOICE_SEGMENTS: usize = 512;
/// Maximum voice performance records in one clonability assessment.
pub const MAX_ASSESS_VOICE_RECORDS: usize = 64;
/// Maximum cues in one `neemu_emit_torori_cue` call. Torori's own
/// bundle cap is 64; neemu refuses above it so a bundle it emits
/// always validates at torori's GAP5 gate.
pub const MAX_EMIT_CUES: usize = 64;
/// Maximum SFX pair names attached to one emitted cue.
pub const MAX_EMIT_PAIRS: usize = 32;
/// Transcript / cue-line text length cap, bytes.
pub const MAX_VOICE_TEXT_BYTES: usize = 8 * 1024;

// ---- Typed gate errors (SPEC §5: snake_case `error.kind`) ----

pub const KIND_SCREEN_REFUSAL: &str = "screen_refusal";
pub const KIND_TIER_REFUSAL: &str = "tier_refusal";
pub const KIND_MEDIA_UNREADABLE: &str = "media_unreadable";
pub const KIND_BOUND_EXCEEDED: &str = "bound_exceeded";
pub const KIND_REFUSAL_NOT_SCREENED: &str = "refusal_not_screened";
/// S2: an understanding claim whose evidence refs are missing or do
/// not resolve to records of the same work (SPEC §7 S2 gate: an
/// unlinked beat is a typed refusal/failure).
pub const KIND_EVIDENCE_REFUSAL: &str = "evidence_refusal";
/// S3: a derivation refused on the rights/clonability gate (SPEC
/// §3.11: unassessed or NOT-CLONABLE voice, identity adaptation
/// plan, over-class request (delta 3), dead engine license
/// (delta 4), or a target identity that is the source speaker).
pub const KIND_CLONABILITY_REFUSAL: &str = "clonability_refusal";
/// S3: a conditioning value outside torori's frozen schema range at
/// cue-emit time — refused, never clamped (schema law).
pub const KIND_CONDITIONING_REFUSAL: &str = "conditioning_refusal";

/// A gate refusal: an execution result with `isError` and a typed
/// `error.kind`, never a protocol error (SPEC §5).
#[derive(Debug, Clone)]
pub struct ToolError {
    pub kind: &'static str,
    pub message: String,
    pub details: Value,
}

impl ToolError {
    pub fn new(kind: &'static str, message: impl Into<String>) -> Self {
        ToolError {
            kind,
            message: message.into(),
            details: json!({}),
        }
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = details;
        self
    }

    /// The JSON body placed in the tool result's text content.
    pub fn to_json(&self) -> Value {
        let mut error = Map::new();
        error.insert("kind".to_string(), json!(self.kind));
        error.insert("message".to_string(), json!(self.message));
        if let Value::Object(details) = &self.details {
            for (key, value) in details {
                error.insert(key.clone(), value.clone());
            }
        }
        json!({ "error": Value::Object(error) })
    }
}

/// Ways a tool call can fail: gate refusals (execution results) vs
/// malformed arguments (JSON-RPC invalid params, -32602).
#[derive(Debug)]
pub enum CallError {
    Gate(ToolError),
    Args(String),
}

impl From<ToolError> for CallError {
    fn from(error: ToolError) -> Self {
        CallError::Gate(error)
    }
}

pub type CallResult = Result<Value, CallError>;

fn args_err(message: impl Into<String>) -> CallError {
    CallError::Args(message.into())
}

// ---- Canonical JSON + content addressing ----

/// Canonical JSON: object keys sorted recursively, compact separators.
/// The hashed form of every record is this string.
pub fn canonical_json(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut out = String::from("{");
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(key).unwrap_or_default());
                out.push(':');
                out.push_str(&canonical_json(&map[*key]));
            }
            out.push('}');
            out
        }
        Value::Array(items) => {
            let mut out = String::from("[");
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&canonical_json(item));
            }
            out.push(']');
            out
        }
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

/// SHA-256 of a string's UTF-8 bytes, lowercase hex.
pub fn sha256_str(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Content-addressed id: `<prefix>` + first 32 hex chars of the SHA-256
/// of the value's canonical form.
pub fn content_id(prefix: &str, value: &Value) -> String {
    let hash = sha256_str(&canonical_json(value));
    format!("{prefix}{}", &hash[..32])
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ---- Argument parsing helpers (failures are invalid-params, -32602) ----

pub fn arg_object(args: &Value) -> Result<&Map<String, Value>, CallError> {
    args.as_object()
        .ok_or_else(|| args_err("arguments must be a JSON object"))
}

pub fn req_str<'a>(obj: &'a Map<String, Value>, key: &str) -> Result<&'a str, CallError> {
    match obj.get(key) {
        Some(Value::String(text)) if !text.is_empty() => {
            if text.chars().count() > MAX_SHORT_CHARS && key != "media_dir" {
                return Err(args_err(format!(
                    "field '{key}' exceeds {MAX_SHORT_CHARS} characters"
                )));
            }
            Ok(text.as_str())
        }
        Some(Value::String(_)) => Err(args_err(format!("field '{key}' must not be empty"))),
        Some(_) => Err(args_err(format!("field '{key}' must be a string"))),
        None => Err(args_err(format!("missing required field '{key}'"))),
    }
}

pub fn opt_str<'a>(obj: &'a Map<String, Value>, key: &str) -> Result<Option<&'a str>, CallError> {
    match obj.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.as_str())),
        Some(_) => Err(args_err(format!("field '{key}' must be a string"))),
    }
}

pub fn req_text<'a>(obj: &'a Map<String, Value>, key: &str) -> Result<&'a str, CallError> {
    match obj.get(key) {
        Some(Value::String(text)) if !text.is_empty() => {
            if text.chars().count() > MAX_TEXT_CHARS {
                return Err(args_err(format!(
                    "field '{key}' exceeds {MAX_TEXT_CHARS} characters"
                )));
            }
            Ok(text.as_str())
        }
        Some(Value::String(_)) => Err(args_err(format!("field '{key}' must not be empty"))),
        Some(_) => Err(args_err(format!("field '{key}' must be a string"))),
        None => Err(args_err(format!("missing required field '{key}'"))),
    }
}

pub fn opt_arr<'a>(
    obj: &'a Map<String, Value>,
    key: &str,
) -> Result<Option<&'a Vec<Value>>, CallError> {
    match obj.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(items)) => Ok(Some(items)),
        Some(_) => Err(args_err(format!("field '{key}' must be an array"))),
    }
}

pub fn opt_usize(obj: &Map<String, Value>, key: &str, default: usize) -> Result<usize, CallError> {
    match obj.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Number(number)) => number
            .as_u64()
            .map(|n| n as usize)
            .ok_or_else(|| args_err(format!("field '{key}' must be a non-negative integer"))),
        Some(_) => Err(args_err(format!("field '{key}' must be an integer"))),
    }
}

pub fn opt_bool(obj: &Map<String, Value>, key: &str) -> Result<bool, CallError> {
    match obj.get(key) {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(flag)) => Ok(*flag),
        Some(_) => Err(args_err(format!("field '{key}' must be a boolean"))),
    }
}

/// An optional finite float field (tempo curves, confidences).
pub fn opt_f64(obj: &Map<String, Value>, key: &str) -> Result<Option<f64>, CallError> {
    match obj.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(number)) => number
            .as_f64()
            .filter(|n| n.is_finite())
            .map(Some)
            .ok_or_else(|| args_err(format!("field '{key}' must be a finite number"))),
        Some(_) => Err(args_err(format!("field '{key}' must be a number"))),
    }
}

/// Parse a bounded short-string list argument (hooks, props, flags).
pub fn parse_string_list(
    obj: &Map<String, Value>,
    key: &str,
    max_items: usize,
) -> Result<Vec<String>, CallError> {
    match opt_arr(obj, key)? {
        None => Ok(Vec::new()),
        Some(items) => {
            if items.len() > max_items {
                return Err(args_err(format!(
                    "field '{key}' exceeds {max_items} entries"
                )));
            }
            let mut out = Vec::new();
            for item in items {
                let text = item
                    .as_str()
                    .ok_or_else(|| args_err(format!("'{key}' entries must be strings")))?;
                if text.chars().count() > MAX_SHORT_CHARS {
                    return Err(args_err(format!(
                        "'{key}' entry exceeds the character bound"
                    )));
                }
                out.push(text.to_string());
            }
            Ok(out)
        }
    }
}

// ---- The registry store ----

/// The neemu registry: a directory tree of JSON record files.
///
/// Layout under the root:
/// `works/<work_id>.json`, `preflights/<work_id>/<id>.json`,
/// `screens/<work_id>/<id>.json`, `records/<work_id>/<id>.json`,
/// `exports/<work_id>/<id>.json`.
pub struct Neemu {
    root: PathBuf,
}

impl Neemu {
    /// Open (creating if needed) the registry at `root`.
    pub fn open(root: PathBuf) -> std::io::Result<Neemu> {
        for sub in ["works", "preflights", "screens", "records", "exports"] {
            fs::create_dir_all(root.join(sub))?;
        }
        Ok(Neemu { root })
    }

    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    fn read_json(&self, path: &PathBuf) -> Option<Value> {
        let text = fs::read_to_string(path).ok()?;
        serde_json::from_str(&text).ok()
    }

    fn write_json(&self, path: &PathBuf, value: &Value) -> Result<(), ToolError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                ToolError::new(KIND_MEDIA_UNREADABLE, format!("registry write failed: {e}"))
            })?;
        }
        let text = serde_json::to_string_pretty(value).unwrap_or_default();
        // Write-then-rename keeps a record file never half-visible.
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, text).map_err(|e| {
            ToolError::new(KIND_MEDIA_UNREADABLE, format!("registry write failed: {e}"))
        })?;
        fs::rename(&tmp, path).map_err(|e| {
            ToolError::new(KIND_MEDIA_UNREADABLE, format!("registry write failed: {e}"))
        })?;
        Ok(())
    }

    fn work_path(&self, work_id: &str) -> PathBuf {
        self.root.join("works").join(format!("{work_id}.json"))
    }

    /// Load a work record, or the typed refusal for an unregistered work.
    pub fn load_work(&self, work_id: &str) -> Result<Value, ToolError> {
        self.read_json(&self.work_path(work_id)).ok_or_else(|| {
            ToolError::new(KIND_SCREEN_REFUSAL, format!("unregistered work: {work_id}"))
                .with_details(json!({ "work_id": work_id }))
        })
    }

    fn save_work(&self, work: &Value) -> Result<(), ToolError> {
        let work_id = work["work_id"].as_str().unwrap_or_default().to_string();
        self.write_json(&self.work_path(&work_id), work)
    }

    fn record_dir(&self, kind_dir: &str, work_id: &str) -> PathBuf {
        self.root.join(kind_dir).join(work_id)
    }

    fn store_record(
        &self,
        kind_dir: &str,
        work_id: &str,
        record_id: &str,
        record: &Value,
    ) -> Result<(), ToolError> {
        let path = self
            .record_dir(kind_dir, work_id)
            .join(format!("{record_id}.json"));
        if path.exists() {
            return Ok(()); // content-addressed: an identical record already rests
        }
        self.write_json(&path, record)
    }

    pub fn load_record(&self, kind_dir: &str, work_id: &str, record_id: &str) -> Option<Value> {
        self.read_json(
            &self
                .record_dir(kind_dir, work_id)
                .join(format!("{record_id}.json")),
        )
    }

    fn list_record_ids(&self, kind_dir: &str, work_id: &str) -> Vec<String> {
        let dir = self.record_dir(kind_dir, work_id);
        let mut ids = Vec::new();
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if let Some(id) = name.strip_suffix(".json") {
                    ids.push(id.to_string());
                }
            }
        }
        ids.sort();
        ids
    }

    // ---- Gate chain shared by every extraction tool ----

    /// The §2.1 + §3.16 gate: registered work, determination `cleared`,
    /// and a passing PreflightRecord belonging to the work. Returns the
    /// loaded work record on success.
    pub fn gate_extraction(
        &self,
        work_id: &str,
        preflight_id: Option<&str>,
    ) -> Result<Value, ToolError> {
        let work = self.load_work(work_id)?;
        let determination = work
            .get("determination")
            .and_then(|d| d.get("determination"))
            .and_then(|d| d.as_str())
            .unwrap_or("unscreened");
        if determination != "cleared" {
            return Err(ToolError::new(
                KIND_SCREEN_REFUSAL,
                format!(
                    "work {work_id} is not cleared for extraction (determination: {determination})"
                ),
            )
            .with_details(json!({ "work_id": work_id, "determination": determination })));
        }
        let preflight_id = preflight_id.ok_or_else(|| {
            ToolError::new(
                KIND_REFUSAL_NOT_SCREENED,
                "missing preflight_id: extraction requires a passing PreflightRecord (neemu_validate_ingest)",
            )
            .with_details(json!({ "work_id": work_id }))
        })?;
        let preflight = self
            .load_record("preflights", work_id, preflight_id)
            .ok_or_else(|| {
                ToolError::new(
                    KIND_REFUSAL_NOT_SCREENED,
                    format!("unknown preflight record for this work: {preflight_id}"),
                )
                .with_details(json!({ "work_id": work_id, "preflight_id": preflight_id }))
            })?;
        if preflight.get("passed").and_then(|p| p.as_bool()) != Some(true) {
            let failures = preflight
                .get("failures")
                .cloned()
                .unwrap_or_else(|| json!([]));
            return Err(ToolError::new(
                KIND_REFUSAL_NOT_SCREENED,
                format!("preflight {preflight_id} did not pass; extraction refused"),
            )
            .with_details(
                json!({ "work_id": work_id, "preflight_id": preflight_id, "failures": failures }),
            ));
        }
        Ok(work)
    }

    // ---- neemu_register_work ----

    pub fn register_work(&self, args: &Value) -> CallResult {
        let obj = arg_object(args)?;
        let title = req_str(obj, "title")?.to_string();
        let tags = parse_tags(obj)?;
        let series = opt_str(obj, "series")?.map(|s| s.to_string());
        let source = obj
            .get("source")
            .and_then(|s| s.as_object())
            .ok_or_else(|| args_err("field 'source' must be an object {kind, ref}"))?;
        let source_kind = req_str(source, "kind")?.to_string();
        let source_ref = req_str(source, "ref")?.to_string();
        let media_dir = req_str(obj, "media_dir")?.to_string();
        let tier = req_str(obj, "tier_claim")?;
        if tier != "A" && tier != "T" {
            return Err(args_err("field 'tier_claim' must be \"A\" or \"T\""));
        }
        let rights_evidence = parse_evidence(obj, "rights_evidence")?;
        if tier == "T" && rights_evidence.is_empty() {
            return Err(ToolError::new(
                KIND_TIER_REFUSAL,
                "tier T claim without rights-ladder evidence: tiers are set only with evidence recorded in the registry",
            )
            .with_details(json!({ "tier_claim": "T" }))
            .into());
        }

        let dir = PathBuf::from(&media_dir);
        if !dir.is_dir() {
            return Err(ToolError::new(
                KIND_MEDIA_UNREADABLE,
                format!("media_dir is not a readable directory: {media_dir}"),
            )
            .into());
        }
        let paths = media::list_all_media_files(&dir).map_err(|e| {
            ToolError::new(KIND_MEDIA_UNREADABLE, format!("cannot list media_dir: {e}"))
        })?;
        if paths.is_empty() {
            return Err(ToolError::new(
                KIND_MEDIA_UNREADABLE,
                "media_dir contains no media files (png/jpg/jpeg/webp/gif images; wav/mp3/flac/ogg audio; mp4/webm/mkv video)",
            )
            .into());
        }
        if paths.len() > MAX_MEDIA_FILES {
            return Err(ToolError::new(
                KIND_BOUND_EXCEEDED,
                format!(
                    "work has {} media files; the per-work bound is {MAX_MEDIA_FILES}",
                    paths.len()
                ),
            )
            .with_details(json!({ "file_count": paths.len(), "bound": MAX_MEDIA_FILES }))
            .into());
        }
        let mut media_entries = Vec::new();
        let mut total_bytes: u64 = 0;
        for path in &paths {
            let (size, sha256) = media::hash_file(path).map_err(|e| {
                ToolError::new(
                    KIND_MEDIA_UNREADABLE,
                    format!("cannot read media file {path:?}: {e}"),
                )
            })?;
            if size > MAX_MEDIA_FILE_BYTES {
                return Err(ToolError::new(
                    KIND_BOUND_EXCEEDED,
                    format!("media file {path:?} is {size} bytes; the per-file bound is {MAX_MEDIA_FILE_BYTES}"),
                )
                .into());
            }
            total_bytes += size;
            if total_bytes > MAX_MEDIA_TOTAL_BYTES {
                return Err(ToolError::new(
                    KIND_BOUND_EXCEEDED,
                    format!("work media exceeds the total bound of {MAX_MEDIA_TOTAL_BYTES} bytes"),
                )
                .into());
            }
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .unwrap_or_default();
            if media::AUDIO_EXTENSIONS.contains(&ext.as_str())
                || media::VIDEO_EXTENSIONS.contains(&ext.as_str())
            {
                // S3 voice lane: header facts only. The image entry
                // shape is untouched so image-only works keep their
                // S1 work ids (parity).
                let info = media::sniff_av(path).map_err(|e| {
                    ToolError::new(
                        KIND_MEDIA_UNREADABLE,
                        format!("cannot read media file {path:?}: {e}"),
                    )
                })?;
                media_entries.push(json!({
                    "name": name,
                    "sha256": sha256,
                    "size": size,
                    "format": info.format,
                    "media_kind": info.media_kind,
                    "duration_seconds": info.duration_seconds,
                    "sample_rate": info.sample_rate,
                    "channels": info.channels,
                }));
            } else {
                let (format, width, height) = media::sniff_image(path).map_err(|e| {
                    ToolError::new(
                        KIND_MEDIA_UNREADABLE,
                        format!("cannot read media file {path:?}: {e}"),
                    )
                })?;
                media_entries.push(json!({
                    "name": name,
                    "sha256": sha256,
                    "size": size,
                    "format": format,
                    "width": width,
                    "height": height,
                }));
            }
        }

        let manifest = json!({ "media": media_entries });
        let work_id = content_id("work_", &manifest);
        let content_hash = sha256_str(&canonical_json(&manifest));

        if let Ok(existing) = self.load_work(&work_id) {
            return Ok(json!({
                "work_id": work_id,
                "tier": existing["tier"],
                "file_count": media_entries.len(),
                "total_bytes": total_bytes,
                "content_hash": content_hash,
                "idempotent_replay": true,
            }));
        }

        let work = json!({
            "record_kind": "work",
            "work_id": work_id,
            "title": title,
            "tags": tags,
            "series": series,
            "source": { "kind": source_kind, "ref": source_ref },
            "media_dir": media_dir,
            "tier": tier,
            "rights_evidence": rights_evidence,
            "media": media_entries,
            "content_hash": content_hash,
            "determination": null,
            "extractor": EXTRACTOR_ID,
            "created_at": now_unix(),
        });
        self.save_work(&work)?;
        Ok(json!({
            "work_id": work_id,
            "tier": tier,
            "file_count": work["media"].as_array().map(|m| m.len()).unwrap_or(0),
            "total_bytes": total_bytes,
            "content_hash": content_hash,
            "idempotent_replay": false,
        }))
    }

    // ---- neemu_validate_ingest ----

    pub fn validate_ingest(&self, args: &Value) -> CallResult {
        let obj = arg_object(args)?;
        let work_id = req_str(obj, "work_id")?.to_string();
        let work = self.load_work(&work_id)?;
        let media_dir = PathBuf::from(work["media_dir"].as_str().unwrap_or_default());
        let mut failures: Vec<Value> = Vec::new();
        let mut formats_recognised = true;
        let mut total_bytes: u64 = 0;
        if let Some(entries) = work["media"].as_array() {
            for entry in entries {
                let name = entry["name"].as_str().unwrap_or_default();
                let path = media_dir.join(name);
                if !path.is_file() {
                    failures.push(json!({ "file": name, "reason": "missing at validate time" }));
                    continue;
                }
                match media::hash_file(&path) {
                    Ok((size, sha256)) => {
                        total_bytes += size;
                        if sha256 != entry["sha256"].as_str().unwrap_or_default() {
                            failures.push(json!({ "file": name, "reason": "content hash does not match the registry manifest" }));
                        }
                    }
                    Err(e) => {
                        failures.push(json!({ "file": name, "reason": format!("unreadable: {e}") }))
                    }
                }
                let sniffed = if entry.get("media_kind").is_some() {
                    media::sniff_av(&path).map(|info| info.format)
                } else {
                    media::sniff_image(&path).map(|(format, _, _)| format)
                };
                match sniffed {
                    Ok(format) => {
                        if format == "unknown" {
                            formats_recognised = false;
                            failures.push(json!({ "file": name, "reason": "format not recognised from content" }));
                        } else if format != entry["format"].as_str().unwrap_or_default() {
                            formats_recognised = false;
                            failures.push(json!({ "file": name, "reason": format!("format changed since registration (was {}, now {format})", entry["format"]) }));
                        }
                    }
                    Err(e) => {
                        failures.push(json!({ "file": name, "reason": format!("unreadable: {e}") }))
                    }
                }
            }
        }

        let title = work["title"].as_str().unwrap_or_default();
        let tags: Vec<String> = work["tags"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|t| t.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let hit = blocklist::blocklist_hit(title, &tags);

        // Integrity (delta 14): C2PA manifest presence is checked from the
        // media bytes; watermark detection is not available in S1 and is
        // recorded as such — a clash is asserted only when both checks
        // actually ran and disagreed.
        let mut c2pa_present = false;
        if let Some(entries) = work["media"].as_array() {
            for entry in entries {
                let path = media_dir.join(entry["name"].as_str().unwrap_or_default());
                if let Ok(bytes) = fs::read(&path) {
                    let probe = &bytes[..bytes.len().min(256 * 1024)];
                    if probe.windows(4).any(|w| w == b"c2pa") {
                        c2pa_present = true;
                    }
                }
            }
        }

        let file_count = work["media"].as_array().map(|m| m.len()).unwrap_or(0);
        let checks = json!({
            "media_accessible": failures.is_empty(),
            "hashes_match": !failures.iter().any(|f| f["reason"].as_str().unwrap_or_default().contains("hash")),
            "formats_recognised": formats_recognised,
            "sizes_within_bounds": file_count <= MAX_MEDIA_FILES && total_bytes <= MAX_MEDIA_TOTAL_BYTES,
        });
        let passed = failures.is_empty();
        let body = json!({
            "record_kind": "preflight",
            "work_id": work_id,
            "passed": passed,
            "checks": checks,
            "failures": failures,
            "screen_result": {
                "blocklist_hit": hit,
                "passed": hit.is_none(),
            },
            "integrity": {
                "c2pa_manifest": if c2pa_present { "present" } else { "absent" },
                "watermark_detection": "unavailable_in_s1",
                "integrity_clash": false,
            },
            "media_summary": { "file_count": file_count, "total_bytes": total_bytes },
            "extractor": EXTRACTOR_ID,
        });
        let record_id = content_id("preflight_", &body);
        let mut record = body.clone();
        record["record_id"] = json!(record_id);
        record["created_at"] = json!(now_unix());
        self.store_record("preflights", &work_id, &record_id, &record)?;
        Ok(record)
    }

    // ---- neemu_screen_work ----

    pub fn screen_work(&self, args: &Value) -> CallResult {
        let obj = arg_object(args)?;
        let work_id = req_str(obj, "work_id")?.to_string();
        let mut work = self.load_work(&work_id)?;
        let evidence = obj
            .get("evidence")
            .and_then(|e| e.as_object())
            .ok_or_else(|| args_err("field 'evidence' must be an object"))?;
        let adult_basis = match evidence.get("adult_basis") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(items)) => {
                let mut parsed = Vec::new();
                for item in items {
                    let item_obj = item.as_object().ok_or_else(|| {
                        args_err("adult_basis entries must be objects {kind, detail}")
                    })?;
                    parsed.push(json!({
                        "kind": req_str(item_obj, "kind")?,
                        "detail": req_str(item_obj, "detail")?,
                    }));
                }
                parsed
            }
            Some(_) => return Err(args_err("field 'evidence.adult_basis' must be an array")),
        };
        let flags = [
            ("school_coded", opt_bool(evidence, "school_coded")?),
            (
                "age_stated_under_18",
                opt_bool(evidence, "age_stated_under_18")?,
            ),
            (
                "age_readable_under_18",
                opt_bool(evidence, "age_readable_under_18")?,
            ),
            ("loli_shota_coded", opt_bool(evidence, "loli_shota_coded")?),
        ];

        let title = work["title"].as_str().unwrap_or_default().to_string();
        let tags: Vec<String> = work["tags"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|t| t.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let hit = blocklist::blocklist_hit(&title, &tags);

        // Fail-closed determination (SPEC §2.1): any blocklist hit, any
        // risk flag, or the absence of positive adult evidence excludes.
        let mut reasons: Vec<String> = Vec::new();
        if let Some(term) = hit {
            reasons.push(format!("blocklist term: {term}"));
        }
        for (name, set) in flags {
            if set {
                reasons.push(format!("evidence flag: {name}"));
            }
        }
        if adult_basis.is_empty() {
            reasons.push(
                "insufficient_evidence: no positive adult-coding evidence recorded (ambiguous resolves against extraction)"
                    .to_string(),
            );
        }
        let determination = if reasons.is_empty() {
            "cleared"
        } else {
            "excluded"
        };

        let evidence_record = json!({
            "adult_basis": adult_basis,
            "school_coded": flags[0].1,
            "age_stated_under_18": flags[1].1,
            "age_readable_under_18": flags[2].1,
            "loli_shota_coded": flags[3].1,
        });
        let body = json!({
            "record_kind": "screen",
            "work_id": work_id,
            "determination": determination,
            "reasons": reasons,
            "blocklist_hit": hit,
            "evidence": evidence_record,
            "extractor": EXTRACTOR_ID,
        });
        let record_id = content_id("screen_", &body);
        let mut record = body.clone();
        record["record_id"] = json!(record_id);
        record["created_at"] = json!(now_unix());
        self.store_record("screens", &work_id, &record_id, &record)?;
        work["determination"] = json!({
            "screen_record_id": record_id,
            "determination": determination,
        });
        self.save_work(&work)?;
        Ok(record)
    }

    // ---- Segmentation (deterministic spine, delta 12) ----

    /// Compute the segmentation of a work's pages. Pure function of the
    /// registered manifest + parameters; scene ids are content-derived,
    /// so the read-only tool's output is stable across calls.
    pub fn segment_pages(work: &Value, max_pages_per_scene: usize) -> (Value, Vec<Value>) {
        let media = work["media"].as_array().cloned().unwrap_or_default();
        let work_id = work["work_id"].as_str().unwrap_or_default();
        let content_hash = work["content_hash"].as_str().unwrap_or_default();
        let mut scenes: Vec<Value> = Vec::new();
        let mut start = 0usize;
        while start < media.len() {
            let dims = (
                media[start]["width"].as_u64().unwrap_or(0),
                media[start]["height"].as_u64().unwrap_or(0),
            );
            let mut end = start;
            while end + 1 < media.len() && end + 1 - start < max_pages_per_scene {
                let next = &media[end + 1];
                let next_dims = (
                    next["width"].as_u64().unwrap_or(0),
                    next["height"].as_u64().unwrap_or(0),
                );
                if next_dims != dims {
                    break;
                }
                end += 1;
            }
            let scene_body = json!({
                "work_id": work_id,
                "content_hash": content_hash,
                "page_start": start,
                "page_end": end,
                "max_pages_per_scene": max_pages_per_scene,
            });
            let scene_id = content_id("scene_", &scene_body);
            scenes.push(json!({
                "scene_id": scene_id,
                "index": scenes.len(),
                "page_start": start,
                "page_end": end,
                "page_count": end - start + 1,
                "width": media[start]["width"],
                "height": media[start]["height"],
            }));
            start = end + 1;
        }
        let mut formats = Map::new();
        let mut total_bytes: u64 = 0;
        for entry in &media {
            let format = entry["format"].as_str().unwrap_or("unknown").to_string();
            let count = formats.get(&format).and_then(|c| c.as_u64()).unwrap_or(0);
            formats.insert(format, json!(count + 1));
            total_bytes += entry["size"].as_u64().unwrap_or(0);
        }
        let metadata = json!({
            "record_kind": "media_metadata",
            "basis": "file-metric",
            "media_kind": "page-sequence",
            "page_count": media.len(),
            "total_bytes": total_bytes,
            "formats": Value::Object(formats),
            "content_hash": content_hash,
            "extractor": EXTRACTOR_ID,
        });
        (metadata, scenes)
    }

    /// Resolve a bounded page range for an extraction call, declaring
    /// partiality when the range exceeds the per-call bound.
    pub fn bounded_range(
        page_start: usize,
        page_end: usize,
        page_count: usize,
    ) -> Result<(usize, usize, bool), CallError> {
        if page_start > page_end {
            return Err(args_err("page_start must not exceed page_end"));
        }
        if page_end >= page_count {
            return Err(args_err(format!(
                "page_end {page_end} is outside the work's {page_count} pages"
            )));
        }
        let span = page_end - page_start + 1;
        if span > MAX_PAGES_PER_CALL {
            Ok((page_start, page_start + MAX_PAGES_PER_CALL - 1, true))
        } else {
            Ok((page_start, page_end, false))
        }
    }

    // ---- Record minting helpers ----

    /// Stamp, address, and store one extraction record; returns its id.
    pub fn mint_record(
        &self,
        work_id: &str,
        prefix: &str,
        body: Value,
    ) -> Result<String, ToolError> {
        let record_id = content_id(prefix, &body);
        if self.load_record("records", work_id, &record_id).is_none() {
            let mut record = body;
            record["record_id"] = json!(record_id);
            record["created_at"] = json!(now_unix());
            self.store_record("records", work_id, &record_id, &record)?;
        }
        Ok(record_id)
    }

    pub fn record_exists(&self, work_id: &str, record_id: &str) -> bool {
        self.load_record("records", work_id, record_id).is_some()
    }

    /// The `record_kind` of one stored record, when it exists.
    pub fn record_kind(&self, work_id: &str, record_id: &str) -> Option<String> {
        self.load_record("records", work_id, record_id)?
            .get("record_kind")
            .and_then(|k| k.as_str())
            .map(str::to_string)
    }

    /// Resolve a caller-supplied scene id against the deterministic
    /// segmentation of a work (delta 12): understanding and act
    /// extraction attach to these ids and may not mint their own.
    pub fn find_scene(work: &Value, scene_id: &str, max_pages_per_scene: usize) -> Option<Value> {
        let (_, scenes) = Neemu::segment_pages(work, max_pages_per_scene);
        scenes
            .into_iter()
            .find(|scene| scene["scene_id"].as_str() == Some(scene_id))
    }

    /// The media entries (with per-page hashes) covering a page range.
    pub fn page_hashes(work: &Value, page_start: usize, page_end: usize) -> Vec<Value> {
        let media = work["media"].as_array().cloned().unwrap_or_default();
        media
            .iter()
            .enumerate()
            .filter(|(index, _)| *index >= page_start && *index <= page_end)
            .map(|(index, entry)| {
                json!({
                    "page_index": index,
                    "sha256": entry["sha256"],
                })
            })
            .collect()
    }

    /// Point a work record at its latest enrichment bundle. This is
    /// the work record's only S2 mutation; enrichment never touches
    /// the determination (delta 17).
    pub fn set_enrichment_pointer(&self, work_id: &str, bundle_id: &str) -> Result<(), ToolError> {
        let mut work = self.load_work(work_id)?;
        work["enrichment_bundle_id"] = json!(bundle_id);
        self.save_work(&work)
    }

    pub fn all_records(&self, work_id: &str) -> Vec<Value> {
        self.list_record_ids("records", work_id)
            .iter()
            .filter_map(|id| self.load_record("records", work_id, id))
            .collect()
    }

    /// S3: resolve an estate voice identity by reference across the
    /// whole registry (SPEC §3.5: emit's target_voice is an estate
    /// identity, which may live under any work). Returns the owning
    /// work id and the identity record.
    pub fn find_voice_identity(&self, identity_ref: &str) -> Option<(String, Value)> {
        let works_dir = self.root.join("works");
        let mut work_ids: Vec<String> = Vec::new();
        if let Ok(entries) = fs::read_dir(&works_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".json") {
                        work_ids.push(name.trim_end_matches(".json").to_string());
                    }
                }
            }
        }
        work_ids.sort();
        for work_id in work_ids {
            for record in self.all_records(&work_id) {
                if record["record_kind"].as_str() == Some("voice_identity")
                    && record["identity_ref"].as_str() == Some(identity_ref)
                {
                    return Some((work_id, record));
                }
            }
        }
        None
    }

    pub fn store_export(
        &self,
        work_id: &str,
        bundle_id: &str,
        bundle: &Value,
    ) -> Result<PathBuf, ToolError> {
        let path = self
            .record_dir("exports", work_id)
            .join(format!("{bundle_id}.json"));
        if !path.exists() {
            self.write_json(&path, bundle)?;
        }
        Ok(path)
    }

    pub fn list_kind_ids(&self, kind_dir: &str, work_id: &str) -> Vec<String> {
        self.list_record_ids(kind_dir, work_id)
    }
}

/// Parse a string-array argument with per-item and count bounds.
pub fn parse_tags(obj: &Map<String, Value>) -> Result<Vec<String>, CallError> {
    match opt_arr(obj, "tags")? {
        None => Ok(Vec::new()),
        Some(items) => {
            if items.len() > MAX_TAGS {
                return Err(args_err(format!("field 'tags' exceeds {MAX_TAGS} entries")));
            }
            let mut tags = Vec::new();
            for item in items {
                let tag = item
                    .as_str()
                    .ok_or_else(|| args_err("tags entries must be strings"))?;
                if tag.chars().count() > MAX_SHORT_CHARS {
                    return Err(args_err("tag exceeds the per-field character bound"));
                }
                tags.push(tag.to_string());
            }
            Ok(tags)
        }
    }
}

/// Parse an evidence array of `{kind, ref|detail}` objects.
pub fn parse_evidence(obj: &Map<String, Value>, key: &str) -> Result<Vec<Value>, CallError> {
    match opt_arr(obj, key)? {
        None => Ok(Vec::new()),
        Some(items) => {
            let mut parsed = Vec::new();
            for item in items {
                let item_obj = item
                    .as_object()
                    .ok_or_else(|| args_err(format!("{key} entries must be objects")))?;
                let kind = req_str(item_obj, "kind")?.to_string();
                let reference = match opt_str(item_obj, "ref")? {
                    Some(r) => r.to_string(),
                    None => req_str(item_obj, "detail")?.to_string(),
                };
                parsed.push(json!({ "kind": kind, "ref": reference }));
            }
            Ok(parsed)
        }
    }
}

/// Normalise a lineage name for the delta-6 taint comparison:
/// lowercase, alphanumerics only ("RAF-DB" → "rafdb").
pub fn normalise_lineage(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// True when a `trained_on` list carries a delta-6 tainted lineage;
/// returns the matching canonical name when it does.
pub fn tainted_lineage(trained_on: &[Value]) -> Option<&'static str> {
    for item in trained_on {
        if let Some(name) = item.as_str() {
            let normalised = normalise_lineage(name);
            for tainted in TAINTED_LINEAGES {
                if normalised == tainted {
                    return Some(tainted);
                }
            }
        }
    }
    None
}

/// Limitations echo for the page-image dialogue lane (delta 8: each
/// record carries its `known_limitations`).
pub const DIALOGUE_PAGE_LIMITATIONS: &str =
    "page-image lane: lines are caller-transcribed from page images; word timing is absent (null), speaker is bound per line as supplied, and no audio features exist for this source kind";
