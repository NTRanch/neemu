//! neemu — D-1265 voice-cue enforcement (Pax ruling, 2026-11-10).
//!
//! A voice cue is an **input, not a measurement**: it conditions
//! synthesis. It therefore inherits the source performance's
//! per-character voice lock (Aria discipline: a locked reference per
//! character, versioned, swap = re-approval) and carries
//! `origin_lane` + `content_label` + `evidence` from birth (Bloch
//! law). Two refusals are enforced here, at construction and at
//! export — never by filtering after the fact:
//!
//! - Construction (`build_stamp`, called by `emit_torori_cue`
//!   before any cue is built or anything touches disk): a cue
//!   without a voice-lock reference, or without lane labels, or
//!   without evidence does not construct. The lock reference must
//!   name the resolved VoiceIdentity record itself, and that record
//!   must carry a lock state. An unclassified cue is stamped
//!   `nsfw` — the Bloch fail-closed default — and the record says
//!   the label was defaulted; it never travels unlabelled.
//! - Export (`check_cue_export`, called by `neemu_export`): a cue
//!   record may leave only for a declared target project and lane
//!   identical to the ones it was born with. A cross-project or
//!   cross-lane export is refused, and a legacy cue record carrying
//!   no labels at all is refused outright (fail-closed).
//!
//! Every refusal is the typed gate kind `voice_cue_refusal`
//! (an execution result with `isError`, never a protocol error).
//! This module performs no I/O and no network access, matching
//! neemu's own law: it never fetches from the web.

use serde_json::{json, Map, Value};

use crate::{
    opt_str, parse_evidence, CallError, ToolError, KIND_VOICE_CUE_REFUSAL, MAX_SHORT_CHARS,
};

/// Content labels a voice cue may carry from birth (Bloch law).
pub const CONTENT_LABELS: [&str; 2] = ["nsfw", "sfw"];
/// The fail-closed label stamped when a cue is unclassified.
pub const DEFAULT_CONTENT_LABEL: &str = "nsfw";
/// Record kinds that are voice cues for the D-1265 export gate.
pub const CUE_RECORD_KINDS: [&str; 2] = ["torori_cue", "torori_cue_bundle"];

/// A D-1265 construction refusal (typed gate error).
pub fn voice_cue_refusal(message: String, details: Value) -> CallError {
    CallError::Gate(ToolError::new(KIND_VOICE_CUE_REFUSAL, message).with_details(details))
}

/// The D-1265 stamp every voice-cue record carries from birth: the
/// voice lock it inherits, the project and lane it belongs to, its
/// content label, and the evidence the label rests on.
#[derive(Debug, Clone, PartialEq)]
pub struct VoiceCueStamp {
    /// Record id of the VoiceIdentity record that is the lock.
    pub voice_lock_ref: String,
    /// Lock state carried by that identity record (Aria discipline).
    pub voice_lock_state: String,
    /// Project the cue belongs to; it may not travel detached.
    pub project_id: String,
    /// Origin lane the cue was born in (Bloch law).
    pub origin_lane: String,
    /// Content label from birth; `nsfw` when defaulted.
    pub content_label: String,
    /// True when `content_label` is the fail-closed default.
    pub content_label_defaulted: bool,
    /// Evidence entries (`{kind, ref}`) recorded at construction.
    pub evidence: Vec<Value>,
}

impl VoiceCueStamp {
    /// The stamp as record fields, merged into cue and bundle
    /// record bodies at mint time.
    pub fn fields(&self) -> Map<String, Value> {
        let mut fields = Map::new();
        fields.insert("voice_lock_ref".to_string(), json!(self.voice_lock_ref));
        fields.insert("voice_lock_state".to_string(), json!(self.voice_lock_state));
        fields.insert("project_id".to_string(), json!(self.project_id));
        fields.insert("origin_lane".to_string(), json!(self.origin_lane));
        fields.insert("content_label".to_string(), json!(self.content_label));
        fields.insert(
            "content_label_defaulted".to_string(),
            json!(self.content_label_defaulted),
        );
        fields.insert("evidence".to_string(), Value::Array(self.evidence.clone()));
        fields
    }
}

/// One required bounded short field of the emit call. Absent, empty,
/// or null is a construction refusal (the cue does not construct);
/// a non-string is an argument error.
fn required_stamp_field<'a>(obj: &'a Map<String, Value>, key: &str) -> Result<&'a str, CallError> {
    match opt_str(obj, key)? {
        Some(text) if !text.is_empty() => {
            if text.chars().count() > MAX_SHORT_CHARS {
                return Err(CallError::Args(format!(
                    "field '{key}' exceeds {MAX_SHORT_CHARS} characters"
                )));
            }
            Ok(text)
        }
        _ => Err(voice_cue_refusal(
            format!(
                "voice cue does not construct without '{key}' (D-1265): \
                 a cue carries its voice lock, project, and lane labels from birth"
            ),
            json!({ "missing_field": key }),
        )),
    }
}

/// Build the D-1265 stamp for one emit call, against the resolved
/// VoiceIdentity record (`identity`). Refuses — constructing
/// nothing — when the voice lock or the lane labels are absent,
/// when the named lock is not the resolved identity, when the
/// identity carries no lock state, when the content label is not a
/// Bloch label, or when no evidence is recorded.
pub fn build_stamp(obj: &Map<String, Value>, identity: &Value) -> Result<VoiceCueStamp, CallError> {
    let voice_lock_ref = required_stamp_field(obj, "voice_lock_ref")?.to_string();
    let identity_id = identity["record_id"].as_str().unwrap_or_default();
    if voice_lock_ref != identity_id {
        return Err(voice_cue_refusal(
            format!(
                "voice_lock_ref '{voice_lock_ref}' is not the resolved voice \
                 identity record '{identity_id}' (D-1265): a cue inherits the \
                 lock of the identity it is cued for, never a detached lock"
            ),
            json!({ "voice_lock_ref": voice_lock_ref, "identity_record_id": identity_id }),
        ));
    }
    let voice_lock_state = match identity["lock_state"].as_str() {
        Some(state) if !state.is_empty() => state.to_string(),
        _ => {
            return Err(voice_cue_refusal(
                format!(
                    "voice identity '{identity_id}' carries no lock state \
                     (D-1265): an unlocked identity cannot be cued"
                ),
                json!({ "identity_record_id": identity_id }),
            ))
        }
    };
    let project_id = required_stamp_field(obj, "project_id")?.to_string();
    let origin_lane = required_stamp_field(obj, "origin_lane")?.to_string();
    let (content_label, content_label_defaulted) = match opt_str(obj, "content_label")? {
        None => (DEFAULT_CONTENT_LABEL.to_string(), true),
        Some("") => (DEFAULT_CONTENT_LABEL.to_string(), true),
        Some(label) if CONTENT_LABELS.contains(&label) => (label.to_string(), false),
        Some(label) => {
            return Err(voice_cue_refusal(
                format!(
                    "content_label '{label}' is not a Bloch label (one of: {})",
                    CONTENT_LABELS.join(", ")
                ),
                json!({ "content_label": label }),
            ))
        }
    };
    let evidence = parse_evidence(obj, "evidence")?;
    if evidence.is_empty() {
        return Err(voice_cue_refusal(
            "voice cue does not construct without evidence (D-1265 / Bloch \
             law): records carry origin_lane + content_label + evidence \
             from birth"
                .to_string(),
            json!({ "missing_field": "evidence" }),
        ));
    }
    Ok(VoiceCueStamp {
        voice_lock_ref,
        voice_lock_state,
        project_id,
        origin_lane,
        content_label,
        content_label_defaulted,
        evidence,
    })
}

/// The D-1265 export gate for one selected record. Non-cue records
/// pass untouched. A cue record passes only when it carries its
/// birth project and lane and the caller declared a target project
/// and lane identical to them; anything else is refused.
pub fn check_cue_export(
    record: &Value,
    target_project_id: Option<&str>,
    target_lane: Option<&str>,
) -> Result<(), CallError> {
    let record_kind = record["record_kind"].as_str().unwrap_or_default();
    if !CUE_RECORD_KINDS.contains(&record_kind) {
        return Ok(());
    }
    let record_id = record["record_id"].as_str().unwrap_or_default();
    let born_project = record["project_id"].as_str().unwrap_or_default();
    let born_lane = record["origin_lane"].as_str().unwrap_or_default();
    if born_project.is_empty() || born_lane.is_empty() {
        return Err(voice_cue_refusal(
            format!(
                "voice cue record {record_id} carries no project / lane \
                 labels (D-1265): an unlabelled cue is not exportable"
            ),
            json!({ "record_id": record_id, "record_kind": record_kind }),
        ));
    }
    let (target_project, target_lane) = match (target_project_id, target_lane) {
        (Some(project), Some(lane)) if !project.is_empty() && !lane.is_empty() => (project, lane),
        _ => {
            return Err(voice_cue_refusal(
                format!(
                    "export of voice cue record {record_id} requires a \
                     declared target_project_id and target_lane (D-1265): \
                     a cue never travels detached from its record"
                ),
                json!({ "record_id": record_id, "record_kind": record_kind }),
            ))
        }
    };
    if target_project != born_project {
        return Err(voice_cue_refusal(
            format!(
                "voice cue record {record_id} belongs to project \
                 '{born_project}'; export to project '{target_project}' is \
                 refused (D-1265 cross-project)"
            ),
            json!({
                "record_id": record_id,
                "project_id": born_project,
                "target_project_id": target_project,
            }),
        ));
    }
    if target_lane != born_lane {
        return Err(voice_cue_refusal(
            format!(
                "voice cue record {record_id} was born in lane \
                 '{born_lane}'; export to lane '{target_lane}' is refused \
                 (D-1265 cross-lane)"
            ),
            json!({
                "record_id": record_id,
                "origin_lane": born_lane,
                "target_lane": target_lane,
            }),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn identity() -> Value {
        json!({
            "record_kind": "voice_identity",
            "record_id": "ident_0123456789abcdef",
            "identity_ref": "voice.derived.test",
            "lock_state": "designed",
        })
    }

    fn labelled_args() -> Map<String, Value> {
        json!({
            "voice_lock_ref": "ident_0123456789abcdef",
            "project_id": "strobe",
            "origin_lane": "ntranch",
            "content_label": "nsfw",
            "evidence": [{ "kind": "derivation", "ref": "deriv_0123" }],
        })
        .as_object()
        .expect("object")
        .clone()
    }

    #[test]
    fn stamp_builds_when_fully_labelled_and_locked() {
        let stamp = build_stamp(&labelled_args(), &identity()).expect("stamp");
        assert_eq!(stamp.content_label, "nsfw");
        assert!(!stamp.content_label_defaulted);
        assert_eq!(stamp.fields()["project_id"], json!("strobe"));
    }

    #[test]
    fn stamp_refuses_without_lock_or_labels_or_evidence() {
        for missing in ["voice_lock_ref", "project_id", "origin_lane", "evidence"] {
            let mut args = labelled_args();
            args.remove(missing);
            let Err(CallError::Gate(error)) = build_stamp(&args, &identity()) else {
                panic!("missing {missing} must refuse at construction");
            };
            assert_eq!(error.kind, KIND_VOICE_CUE_REFUSAL);
        }
    }

    #[test]
    fn stamp_defaults_unclassified_to_nsfw_fail_closed() {
        let mut args = labelled_args();
        args.remove("content_label");
        let stamp = build_stamp(&args, &identity()).expect("stamp");
        assert_eq!(stamp.content_label, "nsfw");
        assert!(stamp.content_label_defaulted);
    }

    #[test]
    fn export_refuses_cross_project_and_cross_lane() {
        let record = json!({
            "record_kind": "torori_cue",
            "record_id": "cue_0123",
            "project_id": "strobe",
            "origin_lane": "ntranch",
        });
        assert!(check_cue_export(&record, Some("strobe"), Some("ntranch")).is_ok());
        assert!(check_cue_export(&record, Some("other"), Some("ntranch")).is_err());
        assert!(check_cue_export(&record, Some("strobe"), Some("qompassai")).is_err());
        assert!(check_cue_export(&record, None, None).is_err());
    }
}
