//! S2 tool implementations (SPEC §7 S2): act extraction against the
//! ACT registry seed, the SceneUnderstanding layer over the
//! deterministic scenes (delta 12), and System-0 enrichment
//! (delta 17-23).
//!
//! Gate posture, in code:
//! - Act extraction and scene understanding run the full S1 gate
//!   chain (`gate_extraction`: cleared determination + passing
//!   pre-flight) and attach only to deterministic scene ids.
//! - Every understanding claim carries evidence refs that must
//!   resolve to records of the same work; a missing or dangling ref
//!   is a typed `evidence_refusal`, never a silent drop.
//! - Enrichment is advisory only (delta 17): it never reads or
//!   writes a determination. It requires a *registered* work only —
//!   it sits between census and screening, so an unscreened or even
//!   excluded work may carry a bundle; the bundle changes nothing.

use crate::acts::*;
use crate::blocklist::blocklist_hit;
use crate::*;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

/// The delta 18 corroboration rule, carried verbatim as gate text on
/// every EnrichmentBundle.
pub const CORROBORATION_RULE: &str = "Two independent sources pin identity; ONE source (or official metadata) suffices to escalate; silence never cancels an escalation; no enrichment result ever clears a work or overrides an EXCLUDE - clearance is the work-open screen's alone.";

/// Delta 19: similarity below this is a lead, never an identification.
pub const TRACE_SIMILARITY_THRESHOLD: f64 = 0.90;

// ---- shared S2 helpers ----

/// The S1 gate chain + deterministic scene resolution. An unknown
/// scene id is an arguments error: the id is a caller claim about a
/// pure function of the work, and delta 12 forbids minting scenes.
fn gated_scene(
    neemu: &Neemu,
    obj: &Map<String, Value>,
) -> Result<(String, Value, Value), CallError> {
    let work_id = req_str(obj, "work_id")?.to_string();
    let preflight_id = opt_str(obj, "preflight_id")?;
    let work = neemu.gate_extraction(&work_id, preflight_id)?;
    let max_pages = opt_usize(obj, "max_pages_per_scene", DEFAULT_MAX_PAGES_PER_SCENE)?;
    if max_pages == 0 || max_pages > MAX_PAGES_PER_SCENE_LIMIT {
        return Err(CallError::Args(format!(
            "max_pages_per_scene must be within 1..={MAX_PAGES_PER_SCENE_LIMIT}"
        )));
    }
    let scene_id = req_str(obj, "scene_id")?;
    let scene = Neemu::find_scene(&work, scene_id, max_pages).ok_or_else(|| {
        CallError::Args(format!(
            "scene_id {scene_id} is not a deterministic scene of this work under max_pages_per_scene={max_pages} (delta 12: acts and understanding attach to deterministic scenes only)"
        ))
    })?;
    Ok((work_id, work, scene))
}

fn req_page(obj: &Map<String, Value>, key: &str) -> Result<usize, CallError> {
    match obj.get(key) {
        Some(Value::Number(number)) => number.as_u64().map(|n| n as usize).ok_or_else(|| {
            CallError::Args(format!("field '{key}' must be a non-negative integer"))
        }),
        Some(_) => Err(CallError::Args(format!("field '{key}' must be an integer"))),
        None => Err(CallError::Args(format!("missing required field '{key}'"))),
    }
}

/// Evidence-link discipline (S2 gate): a claim's refs must be a
/// non-empty list, and every ref must resolve to a record of the
/// same work. Anything else is a typed `evidence_refusal`.
fn check_evidence_refs(
    neemu: &Neemu,
    work_id: &str,
    refs: &[String],
    context: &str,
) -> Result<(), CallError> {
    if refs.is_empty() {
        return Err(ToolError::new(
            KIND_EVIDENCE_REFUSAL,
            format!("{context}: no evidence refs - an unlinked claim is refused, not recorded"),
        )
        .with_details(json!({ "work_id": work_id, "context": context }))
        .into());
    }
    for reference in refs {
        if !neemu.record_exists(work_id, reference) {
            return Err(ToolError::new(
                KIND_EVIDENCE_REFUSAL,
                format!(
                    "{context}: evidence ref {reference} does not resolve to a record of this work"
                ),
            )
            .with_details(json!({ "work_id": work_id, "context": context, "ref": reference }))
            .into());
        }
    }
    Ok(())
}

fn evidence_refusal(message: String, work_id: &str, reference: &str) -> CallError {
    ToolError::new(KIND_EVIDENCE_REFUSAL, message)
        .with_details(json!({ "work_id": work_id, "ref": reference }))
        .into()
}

// ---- neemu_extract_acts (SPEC §3.3, §6) ----

fn parse_endpoint(value: &Value, context: &str) -> Result<Value, CallError> {
    let endpoint = value
        .as_object()
        .ok_or_else(|| CallError::Args(format!("{context}: contact endpoint must be an object")))?;
    let slot = req_str(endpoint, "slot")?;
    let part = req_str(endpoint, "part")?;
    let collider = req_str(endpoint, "collider")?;
    if !COLLIDER_KINDS.contains(&collider) {
        return Err(CallError::Args(format!(
            "{context}: collider '{collider}' is not in the D-076 vocabulary ({})",
            COLLIDER_KINDS.join(", ")
        )));
    }
    Ok(json!({ "slot": slot, "part": part, "collider": collider }))
}

pub fn extract_acts(neemu: &Neemu, args: &Value) -> CallResult {
    let obj = arg_object(args)?;
    let (work_id, work, scene) = gated_scene(neemu, obj)?;
    let acts = opt_arr(obj, "acts")?.cloned().unwrap_or_default();
    if acts.len() > MAX_ACTS_PER_CALL {
        return Err(ToolError::new(
            KIND_BOUND_EXCEEDED,
            format!(
                "{} acts in one call; the bound is {MAX_ACTS_PER_CALL}",
                acts.len()
            ),
        )
        .with_details(json!({ "work_id": work_id }))
        .into());
    }
    let page_start = scene["page_start"].as_u64().unwrap_or(0) as usize;
    let page_end = scene["page_end"].as_u64().unwrap_or(0) as usize;
    let scene_id = scene["scene_id"].as_str().unwrap_or_default().to_string();
    let tier = work["tier"].as_str().unwrap_or("A").to_string();
    let mut record_ids = Vec::new();
    let mut unmapped: Vec<String> = Vec::new();
    let mut candidates: Vec<String> = Vec::new();
    for (index, act) in acts.iter().enumerate() {
        let context = format!("acts[{index}]");
        let act_obj = act
            .as_object()
            .ok_or_else(|| CallError::Args(format!("{context}: act must be an object")))?;
        let act_code = req_str(act_obj, "act_code")?.to_string();

        // Participants: slots + roles (SPEC §3.3).
        let participants_raw = opt_arr(act_obj, "participants")?
            .cloned()
            .unwrap_or_default();
        if participants_raw.len() > MAX_PARTICIPANTS_PER_ACT {
            return Err(CallError::Args(format!(
                "{context}: participants exceed {MAX_PARTICIPANTS_PER_ACT}"
            )));
        }
        let mut participants = Vec::new();
        for participant in &participants_raw {
            let participant = participant.as_object().ok_or_else(|| {
                CallError::Args(format!("{context}: participant must be an object"))
            })?;
            participants.push(json!({
                "slot": req_str(participant, "slot")?,
                "role": req_str(participant, "role")?,
            }));
        }

        // Contact pairs in the D-076 vocabulary.
        let pairs_raw = opt_arr(act_obj, "contact_pairs")?
            .cloned()
            .unwrap_or_default();
        if pairs_raw.len() > MAX_CONTACT_PAIRS_PER_ACT {
            return Err(CallError::Args(format!(
                "{context}: contact_pairs exceed {MAX_CONTACT_PAIRS_PER_ACT}"
            )));
        }
        let mut contact_pairs = Vec::new();
        for pair in &pairs_raw {
            let pair = pair.as_object().ok_or_else(|| {
                CallError::Args(format!("{context}: contact pair must be an object"))
            })?;
            let compliance = req_str(pair, "compliance_class")?;
            if !COMPLIANCE_CLASSES.contains(&compliance) {
                return Err(CallError::Args(format!(
                    "{context}: compliance_class '{compliance}' is not in the D-076 vocabulary ({})",
                    COMPLIANCE_CLASSES.join(", ")
                )));
            }
            let a = pair
                .get("a")
                .ok_or_else(|| CallError::Args(format!("{context}: contact pair needs 'a'")))?;
            let b = pair
                .get("b")
                .ok_or_else(|| CallError::Args(format!("{context}: contact pair needs 'b'")))?;
            contact_pairs.push(json!({
                "a": parse_endpoint(a, &context)?,
                "b": parse_endpoint(b, &context)?,
                "compliance_class": compliance,
            }));
        }

        // Pose track: joint configuration classes, support, orientation.
        let pose_raw = opt_arr(act_obj, "pose_track")?.cloned().unwrap_or_default();
        if pose_raw.len() > MAX_POSE_KEYS_PER_ACT {
            return Err(CallError::Args(format!(
                "{context}: pose_track exceeds {MAX_POSE_KEYS_PER_ACT}"
            )));
        }
        let mut pose_track = Vec::new();
        for pose in &pose_raw {
            let pose = pose.as_object().ok_or_else(|| {
                CallError::Args(format!("{context}: pose entry must be an object"))
            })?;
            pose_track.push(json!({
                "label": req_str(pose, "label")?,
                "joint_configuration": req_str(pose, "joint_configuration")?,
                "support": req_str(pose, "support")?,
                "orientation": opt_str(pose, "orientation")?,
            }));
        }

        // Motion: beats per cycle, tempo/intensity curves, entry/exit.
        let motion = match act_obj.get("motion") {
            None | Some(Value::Null) => Value::Null,
            Some(value) => {
                let motion = value.as_object().ok_or_else(|| {
                    CallError::Args(format!("{context}: motion must be an object"))
                })?;
                let mut intensity = Vec::new();
                if let Some(curve) = opt_arr(motion, "intensity_curve")? {
                    if curve.len() > MAX_STRING_LIST {
                        return Err(CallError::Args(format!(
                            "{context}: intensity_curve exceeds {MAX_STRING_LIST} points"
                        )));
                    }
                    for point in curve {
                        intensity.push(point.as_f64().filter(|n| n.is_finite()).ok_or_else(
                            || {
                                CallError::Args(format!(
                                    "{context}: intensity_curve points must be finite numbers"
                                ))
                            },
                        )?);
                    }
                }
                json!({
                    "beats_per_cycle": opt_f64(motion, "beats_per_cycle")?,
                    "tempo_curve": opt_str(motion, "tempo_curve")?,
                    "intensity_curve": intensity,
                    "entry": opt_str(motion, "entry")?,
                    "exit": opt_str(motion, "exit")?,
                })
            }
        };

        // Face timeline: each entry is a canonical face code
        // (F-code or ratified N-code, resolved through the one
        // lookup) or a FaceObservation record id of this work; a
        // dangling ref is an evidence refusal, never a silent drop.
        let timeline_raw = opt_arr(act_obj, "face_timeline")?
            .cloned()
            .unwrap_or_default();
        if timeline_raw.len() > MAX_EVIDENCE_REFS {
            return Err(CallError::Args(format!(
                "{context}: face_timeline exceeds {MAX_EVIDENCE_REFS} entries"
            )));
        }
        let mut face_timeline = Vec::new();
        for entry in &timeline_raw {
            let entry = entry.as_object().ok_or_else(|| {
                CallError::Args(format!("{context}: face_timeline entry must be an object"))
            })?;
            let beat = req_str(entry, "beat")?;
            let reference = req_str(entry, "ref")?;
            if let Some(resolution) = resolve_face_code(reference) {
                let mut entry = json!({ "beat": beat, "f_code": reference });
                // N-code refs carry their canon resolution; F
                // entries keep their pre-N shape byte-for-byte.
                if resolution.family == "N" {
                    entry["code_resolution"] = resolution.snapshot();
                }
                face_timeline.push(entry);
            } else if neemu.record_exists(&work_id, reference) {
                if neemu.record_kind(&work_id, reference).as_deref() != Some("face_observation") {
                    return Err(CallError::Args(format!(
                        "{context}: face_timeline ref {reference} is not a face_observation record"
                    )));
                }
                face_timeline.push(json!({ "beat": beat, "face_ref": reference }));
            } else {
                return Err(evidence_refusal(
                    format!("{context}: face_timeline ref {reference} does not resolve"),
                    &work_id,
                    reference,
                ));
            }
        }

        let registry = match act_entry(&act_code) {
            Some(entry) => {
                if mapping_status(entry) == "unmapped" && !unmapped.contains(&act_code) {
                    unmapped.push(act_code.clone());
                }
                entry_json(entry)
            }
            None => {
                if !candidates.contains(&act_code) {
                    candidates.push(act_code.clone());
                }
                if !unmapped.contains(&act_code) {
                    unmapped.push(act_code.clone());
                }
                candidate_json(&act_code)
            }
        };

        let body = json!({
            "record_kind": "act_record",
            "work_id": work_id,
            "tier": tier,
            "basis": "grid-read",
            "locus": {
                "scene_id": scene_id,
                "page_start": page_start,
                "page_end": page_end,
            },
            "act_code": act_code,
            "registry": registry,
            "participants": participants,
            "contact_pairs": contact_pairs,
            "pose_track": pose_track,
            "motion": motion,
            "face_timeline": face_timeline,
            "sfx_hooks": parse_string_list(act_obj, "sfx_hooks", MAX_STRING_LIST)?,
            "props": parse_string_list(act_obj, "props", MAX_STRING_LIST)?,
            "input_hashes": Neemu::page_hashes(&work, page_start, page_end),
            "extractor": EXTRACTOR_ID,
        });
        let record_id = neemu.mint_record(&work_id, "act_", body)?;
        record_ids.push(record_id);
    }
    Ok(json!({
        "record_ids": record_ids,
        "count": record_ids.len(),
        "scene_id": scene_id,
        "unmapped_act_codes": unmapped,
        "candidate_act_codes": candidates,
        "partial": false,
    }))
}

// ---- neemu_understand_scene (SPEC §3.6, delta 12) ----

pub fn understand_scene(neemu: &Neemu, args: &Value) -> CallResult {
    let obj = arg_object(args)?;
    let (work_id, work, scene) = gated_scene(neemu, obj)?;
    let page_start = scene["page_start"].as_u64().unwrap_or(0) as usize;
    let page_end = scene["page_end"].as_u64().unwrap_or(0) as usize;
    let scene_id = scene["scene_id"].as_str().unwrap_or_default().to_string();

    let beats_raw = opt_arr(obj, "beats")?.cloned().unwrap_or_default();
    if beats_raw.is_empty() {
        return Err(CallError::Args(
            "field 'beats' must name at least one beat".to_string(),
        ));
    }
    if beats_raw.len() > MAX_BEATS_PER_SCENE {
        return Err(ToolError::new(
            KIND_BOUND_EXCEEDED,
            format!(
                "{} beats in one call; the bound is {MAX_BEATS_PER_SCENE}",
                beats_raw.len()
            ),
        )
        .with_details(json!({ "work_id": work_id }))
        .into());
    }
    let mut links: Vec<String> = Vec::new();
    let mut beats = Vec::new();
    for (index, beat) in beats_raw.iter().enumerate() {
        let context = format!("beats[{index}]");
        let beat = beat
            .as_object()
            .ok_or_else(|| CallError::Args(format!("{context}: beat must be an object")))?;
        let beat_type = req_str(beat, "type")?;
        if !BEAT_TYPES.contains(&beat_type) {
            return Err(CallError::Args(format!(
                "{context}: beat type '{beat_type}' is not in the §3.6 vocabulary ({})",
                BEAT_TYPES.join(", ")
            )));
        }
        let beat_start = req_page(beat, "page_start")?;
        let beat_end = req_page(beat, "page_end")?;
        if beat_start > beat_end || beat_start < page_start || beat_end > page_end {
            return Err(CallError::Args(format!(
                "{context}: locus {beat_start}..={beat_end} lies outside the scene's pages {page_start}..={page_end} (delta 12: understanding attaches inside its scene only)"
            )));
        }
        let refs = parse_string_list(beat, "evidence_refs", MAX_EVIDENCE_REFS)?;
        check_evidence_refs(neemu, &work_id, &refs, &context)?;
        for reference in &refs {
            if !links.contains(reference) {
                links.push(reference.clone());
            }
        }
        beats.push(json!({
            "type": beat_type,
            "page_start": beat_start,
            "page_end": beat_end,
            "summary": opt_str(beat, "summary")?,
            "evidence_refs": refs,
        }));
    }

    // Machinery: mechanism as evidenced + the reset/wakener finding.
    let machinery_obj = obj
        .get("machinery")
        .and_then(|m| m.as_object())
        .ok_or_else(|| CallError::Args("field 'machinery' is required".to_string()))?;
    let mechanism = req_str(machinery_obj, "mechanism")?;
    if !MECHANISMS.contains(&mechanism) {
        return Err(CallError::Args(format!(
            "machinery.mechanism '{mechanism}' is not in the §3.6 vocabulary ({})",
            MECHANISMS.join(", ")
        )));
    }
    let reset = req_str(machinery_obj, "reset_wakener")?;
    if reset != "present" && reset != "absent" {
        return Err(CallError::Args(
            "machinery.reset_wakener must be \"present\" or \"absent\"".to_string(),
        ));
    }
    let machinery_refs = parse_string_list(machinery_obj, "evidence_refs", MAX_EVIDENCE_REFS)?;
    check_evidence_refs(neemu, &work_id, &machinery_refs, "machinery")?;
    for reference in &machinery_refs {
        if !links.contains(reference) {
            links.push(reference.clone());
        }
    }
    let machinery = json!({
        "mechanism": mechanism,
        "stage_model": req_text(machinery_obj, "stage_model")?,
        "reset_wakener": reset,
        "reset_detail": opt_str(machinery_obj, "reset_detail")?,
        "evidence_refs": machinery_refs,
    });

    // Dynamics: who directs, who responds - as evidenced.
    let dynamics_obj = obj
        .get("dynamics")
        .and_then(|d| d.as_object())
        .ok_or_else(|| CallError::Args("field 'dynamics' is required".to_string()))?;
    let roles_raw = opt_arr(dynamics_obj, "roles")?.cloned().unwrap_or_default();
    if roles_raw.len() > MAX_PARTICIPANTS_PER_ACT {
        return Err(CallError::Args(format!(
            "dynamics.roles exceeds {MAX_PARTICIPANTS_PER_ACT}"
        )));
    }
    let mut roles = Vec::new();
    for role in &roles_raw {
        let role = role
            .as_object()
            .ok_or_else(|| CallError::Args("dynamics.roles entries must be objects".to_string()))?;
        roles.push(json!({
            "slot": req_str(role, "slot")?,
            "role": req_str(role, "role")?,
        }));
    }
    let dynamics_refs = parse_string_list(dynamics_obj, "evidence_refs", MAX_EVIDENCE_REFS)?;
    check_evidence_refs(neemu, &work_id, &dynamics_refs, "dynamics")?;
    for reference in &dynamics_refs {
        if !links.contains(reference) {
            links.push(reference.clone());
        }
    }
    let dynamics = json!({
        "roles": roles,
        "who_directs": req_str(dynamics_obj, "who_directs")?,
        "who_responds": req_str(dynamics_obj, "who_responds")?,
        "note": opt_str(dynamics_obj, "note")?,
        "evidence_refs": dynamics_refs,
    });

    links.sort();
    let beat_count = beats.len();
    let link_count = links.len();
    let body = json!({
        "record_kind": "scene_understanding",
        "work_id": work_id,
        "tier": work["tier"],
        "basis": "model-estimate",
        "advisory": true,
        "locus": {
            "scene_id": scene_id,
            "page_start": page_start,
            "page_end": page_end,
        },
        "beats": beats,
        "machinery": machinery,
        "dynamics": dynamics,
        "links": links,
        "input_hashes": Neemu::page_hashes(&work, page_start, page_end),
        "extractor": EXTRACTOR_ID,
    });
    let record_id = neemu.mint_record(&work_id, "und_", body)?;
    Ok(json!({
        "record_id": record_id,
        "scene_id": scene_id,
        "basis": "model-estimate",
        "beat_count": beat_count,
        "link_count": link_count,
        "partial": false,
    }))
}

// ---- neemu_enrich (SPEC §3.15, delta 17-23) ----

fn is_similarity_source(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("sauce") || lower.contains("trace.moe") || lower.contains("tracemoe")
}

fn is_ehentai_lane(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("e-hentai") || lower.contains("ehentai") || lower.contains("exhentai")
}

/// Enrich one work from caller-supplied candidate source data.
/// Neemu never fetches: it validates, applies the corroboration
/// discipline, and records. The work must be registered; no screen
/// state is required and none is changed (delta 17).
fn enrich_one(neemu: &Neemu, item: &Value) -> Result<Value, CallError> {
    let obj = arg_object(item)?;
    let work_id = req_str(obj, "work_id")?.to_string();
    let work = neemu.load_work(&work_id)?;
    let sources_raw = opt_arr(obj, "sources")?.cloned().unwrap_or_default();
    if sources_raw.is_empty() {
        return Err(CallError::Args(
            "field 'sources' must name at least one candidate source".to_string(),
        ));
    }
    if sources_raw.len() > MAX_ENRICH_SOURCES_PER_CALL {
        return Err(ToolError::new(
            KIND_BOUND_EXCEEDED,
            format!(
                "{} sources in one item; the bound is {MAX_ENRICH_SOURCES_PER_CALL}",
                sources_raw.len()
            ),
        )
        .with_details(json!({ "work_id": work_id }))
        .into());
    }

    let mut sub_records = Vec::new();
    let mut bundle_tags: Vec<Value> = Vec::new();
    let mut escalation_flags: Vec<String> = Vec::new();
    let mut pin_sets: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut any_citable_tags = false;
    let mut rate_limit_config = Value::Null;
    let mut non_commercial_tags = false;

    for (index, source) in sources_raw.iter().enumerate() {
        let context = format!("sources[{index}]");
        let source = source
            .as_object()
            .ok_or_else(|| CallError::Args(format!("{context}: source must be an object")))?;
        let name = req_str(source, "source")?.to_string();
        let match_key = req_str(source, "match_key")?.to_string();
        let payload = source.get("payload").cloned().unwrap_or(Value::Null);
        if canonical_json(&payload).len() > MAX_ENRICH_PAYLOAD_BYTES {
            return Err(ToolError::new(
                KIND_BOUND_EXCEEDED,
                format!("{context}: payload exceeds {MAX_ENRICH_PAYLOAD_BYTES} serialized bytes"),
            )
            .with_details(json!({ "work_id": work_id }))
            .into());
        }
        let declared_basis = opt_str(source, "basis")?.unwrap_or("lead");
        if declared_basis != "resolved_record" && declared_basis != "lead" {
            return Err(CallError::Args(format!(
                "{context}: basis must be \"resolved_record\" or \"lead\""
            )));
        }
        // Delta 19: similarity services are leads, not citations; a
        // trace.moe-class claim below 0.90 is a lead by construction.
        let similarity = opt_f64(source, "similarity")?;
        let mut citable = declared_basis == "resolved_record";
        let mut forced_lead_reason: Option<String> = None;
        if citable && is_similarity_source(&name) {
            citable = false;
            forced_lead_reason =
                Some("similarity-service results are leads, not citations (delta 19)".to_string());
        }
        if let Some(score) = similarity {
            if score < TRACE_SIMILARITY_THRESHOLD && declared_basis == "resolved_record" {
                citable = false;
                forced_lead_reason = Some(format!(
                    "similarity {score} is below the {TRACE_SIMILARITY_THRESHOLD} identification threshold (delta 19)"
                ));
            }
        }
        let tags = parse_string_list(source, "tags", MAX_TAGS)?;
        let translated = opt_bool(source, "translated")?;
        // Escalation is single-source and never cancelled (delta 18):
        // caller flags + the canonical blocklist over candidate tags.
        for flag in parse_string_list(source, "escalation_flags", MAX_STRING_LIST)? {
            if !escalation_flags.contains(&flag) {
                escalation_flags.push(flag);
            }
        }
        if let Some(term) = blocklist_hit("", &tags) {
            let flag = format!("canonical_blocklist_hit:{term}:{name}");
            if !escalation_flags.contains(&flag) {
                escalation_flags.push(flag);
            }
        }
        if citable {
            for tag in &tags {
                bundle_tags.push(json!({
                    "tag": tag,
                    "source": name,
                    "untrusted_text": true,
                    "non_commercial": translated,
                }));
            }
            if !tags.is_empty() {
                any_citable_tags = true;
            }
            if translated {
                non_commercial_tags = true;
            }
            let names = pin_sets.entry(match_key.clone()).or_default();
            if !names.contains(&name) {
                names.push(name.clone());
            }
        }
        if rate_limit_config.is_null() && is_ehentai_lane(&name) {
            // Delta 23, recorded for the caller's resolver: neemu
            // itself never fetches.
            rate_limit_config = json!({
                "lane": "e-hentai",
                "search_interval_ms": 3000,
                "api_interval_ms": 1250,
                "page_interval_ms": 1000,
                "max_retries": 2,
                "retry_after_priority": true,
            });
        }
        sub_records.push(json!({
            "source": name,
            "match_key": match_key,
            "match_evidence": opt_str(source, "match_evidence")?,
            "retrieved_at": opt_str(source, "retrieved_at")?,
            "declared_basis": declared_basis,
            "citable": citable,
            "forced_lead_reason": forced_lead_reason,
            "similarity": similarity,
            "translated": translated,
            "untrusted_text": true,
            "payload": payload,
        }));
    }

    // Identity pinning: two independent resolved sources on one key.
    let mut pinned_key: Option<String> = None;
    for (key, names) in &pin_sets {
        if names.len() >= 2 {
            pinned_key = Some(key.clone());
            break;
        }
    }
    let identity_pinned = pinned_key.is_some();

    // Grouping (delta 21): the judgement is recorded; a grouping
    // below the confidence floor is NOT applied.
    let grouping = match obj.get("grouping") {
        None | Some(Value::Null) => Value::Null,
        Some(value) => {
            let grouping = value
                .as_object()
                .ok_or_else(|| CallError::Args("field 'grouping' must be an object".to_string()))?;
            let basis = req_str(grouping, "basis")?;
            if !GROUPING_BASES.contains(&basis) {
                return Err(CallError::Args(format!(
                    "grouping.basis '{basis}' must be one of {}",
                    GROUPING_BASES.join(", ")
                )));
            }
            let confidence = opt_f64(grouping, "confidence")?.unwrap_or(0.0);
            if !(0.0..=1.0).contains(&confidence) {
                return Err(CallError::Args(
                    "grouping.confidence must be within 0..=1".to_string(),
                ));
            }
            json!({
                "basis": basis,
                "confidence": confidence,
                "group_ref": opt_str(grouping, "group_ref")?,
                "applied": confidence >= GROUPING_CONFIDENCE_FLOOR,
                "confidence_floor": GROUPING_CONFIDENCE_FLOOR,
            })
        }
    };

    // Version chains (delta 22): a tag change across versions is
    // itself an escalation signal.
    let versions_raw = opt_arr(obj, "versions")?.cloned().unwrap_or_default();
    if versions_raw.len() > MAX_VERSIONS_PER_CALL {
        return Err(CallError::Args(format!(
            "versions exceed {MAX_VERSIONS_PER_CALL}"
        )));
    }
    let mut versions = Vec::new();
    let mut diffs = Vec::new();
    let mut previous: Option<(String, Vec<String>)> = None;
    for version in &versions_raw {
        let version = version
            .as_object()
            .ok_or_else(|| CallError::Args("versions entries must be objects".to_string()))?;
        let label = req_str(version, "version")?.to_string();
        let tags = parse_string_list(version, "tags", MAX_TAGS)?;
        if let Some((prev_label, prev_tags)) = &previous {
            let added: Vec<&String> = tags.iter().filter(|t| !prev_tags.contains(t)).collect();
            let removed: Vec<&String> = prev_tags.iter().filter(|t| !tags.contains(t)).collect();
            if !added.is_empty() || !removed.is_empty() {
                diffs.push(json!({
                    "from": prev_label,
                    "to": label,
                    "added": added,
                    "removed": removed,
                }));
                let flag = format!("version_tag_change:{prev_label}->{label}");
                if !escalation_flags.contains(&flag) {
                    escalation_flags.push(flag);
                }
            }
        }
        previous = Some((label.clone(), tags.clone()));
        versions.push(json!({ "version": label, "tags": tags }));
    }

    let queue_hint = if identity_pinned && any_citable_tags && escalation_flags.is_empty() {
        json!("candidate")
    } else {
        Value::Null
    };
    let body = json!({
        "record_kind": "enrichment_bundle",
        "work_id": work_id,
        "tier": work["tier"],
        "basis": "enrichment",
        "advisory_only": true,
        "determination_effect": "none",
        "untrusted_text": true,
        "corroboration_rule": CORROBORATION_RULE,
        "sub_records": sub_records,
        "tags": bundle_tags,
        "identity": {
            "pinned": identity_pinned,
            "match_key": pinned_key,
            "rule": "two independent resolved sources pin identity (delta 18)",
        },
        "escalation_flags": escalation_flags,
        "grouping": grouping,
        "version_chain": { "versions": versions, "diffs": diffs },
        "queue_hint": queue_hint,
        "rate_limit_config": rate_limit_config,
        "non_commercial_tags": non_commercial_tags,
        "extractor": EXTRACTOR_ID,
    });
    let bundle_id = neemu.mint_record(&work_id, "enrich_", body)?;
    neemu.set_enrichment_pointer(&work_id, &bundle_id)?;
    Ok(neemu
        .load_record("records", &work_id, &bundle_id)
        .unwrap_or_else(|| json!({ "record_id": bundle_id })))
}

fn item_error(error: &CallError) -> Value {
    match error {
        CallError::Gate(tool) => json!({
            "kind": tool.kind,
            "message": tool.message,
        }),
        CallError::Args(message) => json!({
            "kind": "invalid_arguments",
            "message": message,
        }),
    }
}

pub fn enrich(neemu: &Neemu, args: &Value) -> CallResult {
    let obj = arg_object(args)?;
    if let Some(items) = opt_arr(obj, "items")? {
        // Delta 23: order-preserving batches, per-item errors,
        // resume cursors. The batch id is content-derived from the
        // item list, so a resumed call recognises its own batch.
        let items = items.clone();
        let batch_id = content_id("enrbatch_", &Value::Array(items.clone()));
        let start = match obj.get("resume_cursor") {
            None | Some(Value::Null) => 0,
            Some(cursor) => {
                let cursor = cursor.as_object().ok_or_else(|| {
                    CallError::Args("field 'resume_cursor' must be an object".to_string())
                })?;
                opt_usize(cursor, "next_index", 0)?
            }
        };
        if start > items.len() {
            return Err(CallError::Args(
                "resume_cursor.next_index is past the end of items".to_string(),
            ));
        }
        let end = (start + MAX_ENRICH_ITEMS_PER_CALL).min(items.len());
        let mut results = Vec::new();
        for item in &items[start..end] {
            let work_id = item
                .get("work_id")
                .and_then(|w| w.as_str())
                .map(str::to_string);
            match enrich_one(neemu, item) {
                Ok(bundle) => results.push(json!({
                    "work_id": work_id,
                    "ok": true,
                    "bundle_id": bundle["record_id"],
                })),
                Err(error) => results.push(json!({
                    "work_id": work_id,
                    "ok": false,
                    "error": item_error(&error),
                })),
            }
        }
        let partial = end < items.len();
        return Ok(json!({
            "batch_id": batch_id,
            "results": results,
            "processed": end - start,
            "total_items": items.len(),
            "partial": partial,
            "resume_cursor": if partial { json!({ "next_index": end }) } else { Value::Null },
        }));
    }
    enrich_one(neemu, args)
}
