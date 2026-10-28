//! Tool surface for neemu S1+S2+S3: schemas, annotations, dispatch.
//!
//! Fifteen tools — the S1 slice of the SPEC §4 surface (intake:
//! register / validate / screen; extraction: segment / faces /
//! dialogue; registry reads + tier-checked export), the S2 slice
//! (enrich, acts, understand), and the S3 voice slice (analyze /
//! assess / derive / emit_torori_cue). Every tool ships a full
//! object `inputSchema` with an explicit `properties` key, an
//! `outputSchema`, and behaviour annotations from birth (Tiger MCP
//! Rules 3 and 5; SPEC §4 delta 15). Tools from S4+ are absent.

use crate::*;
use serde_json::{json, Map, Value};

fn annotations(read_only: bool) -> Value {
    json!({
        "readOnlyHint": read_only,
        "destructiveHint": false,
        "idempotentHint": true,
        "openWorldHint": false,
    })
}

fn string_field(description: &str) -> Value {
    json!({ "type": "string", "description": description })
}

fn work_id_field() -> Value {
    string_field("Work id from neemu_register_work.")
}

fn preflight_field() -> Value {
    string_field("PreflightRecord id (neemu_validate_ingest); extraction requires a passing one.")
}

/// The served tool definitions (`tools/list` payload).
pub fn tool_definitions() -> Value {
    json!([
        {
            "name": "neemu_register_work",
            "description": "Register a page-sequence work. Hashes all media; idempotent by content hash. Tier T requires rights evidence in the call.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": string_field("Work title (blocklist-screened)."),
                    "tags": { "type": "array", "items": { "type": "string" }, "description": "Work tags (screened with title)." },
                    "series": string_field("Series name."),
                    "source": {
                        "type": "object",
                        "properties": {
                            "kind": string_field("Provenance kind."),
                            "ref": string_field("Provenance ref.")
                        },
                        "required": ["kind", "ref"]
                    },
                    "media_dir": string_field("Directory of page images (png/jpg/jpeg/webp/gif)."),
                    "tier_claim": { "type": "string", "enum": ["A", "T"], "description": "A = analysis; T = training-cleared (evidence required)." },
                    "rights_evidence": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "kind": string_field("Evidence kind."),
                                "ref": string_field("Evidence ref.")
                            },
                            "required": ["kind", "ref"]
                        },
                        "description": "Required for a tier T claim."
                    }
                },
                "required": ["title", "source", "media_dir", "tier_claim"],
                "additionalProperties": false
            },
            "outputSchema": {
                "type": "object",
                "properties": {
                    "work_id": { "type": "string" },
                    "tier": { "type": "string" },
                    "file_count": { "type": "integer" },
                    "content_hash": { "type": "string" }
                }
            },
            "annotations": annotations(false)
        },
        {
            "name": "neemu_validate_ingest",
            "description": "Validate-first pre-flight (SPEC 3.16): access/hash/format/bounds re-check + blocklist floor; mints the PreflightRecord extraction requires.",
            "inputSchema": {
                "type": "object",
                "properties": { "work_id": work_id_field() },
                "required": ["work_id"],
                "additionalProperties": false
            },
            "outputSchema": {
                "type": "object",
                "properties": {
                    "record_id": { "type": "string" },
                    "passed": { "type": "boolean" },
                    "failures": { "type": "array" }
                }
            },
            "annotations": annotations(false)
        },
        {
            "name": "neemu_screen_work",
            "description": "Record the screen determination (SPEC 2.1). Fail-closed: blocklist hit, any risk flag, or no positive adult evidence excludes, reason recorded.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "work_id": work_id_field(),
                    "evidence": {
                        "type": "object",
                        "properties": {
                            "adult_basis": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "kind": string_field("Evidence kind."),
                                        "detail": string_field("What it shows.")
                                    },
                                    "required": ["kind", "detail"]
                                },
                                "description": "Positive adult-coding evidence; one entry required to clear."
                            },
                            "school_coded": { "type": "boolean" },
                            "age_stated_under_18": { "type": "boolean" },
                            "age_readable_under_18": { "type": "boolean" },
                            "loli_shota_coded": { "type": "boolean" }
                        },
                        "additionalProperties": false
                    }
                },
                "required": ["work_id", "evidence"],
                "additionalProperties": false
            },
            "outputSchema": {
                "type": "object",
                "properties": {
                    "record_id": { "type": "string" },
                    "determination": { "type": "string", "enum": ["cleared", "excluded"] },
                    "reasons": { "type": "array" }
                }
            },
            "annotations": annotations(false)
        },
        {
            "name": "neemu_enrich",
            "description": "System-0 enrichment (SPEC 3.15): record caller-supplied candidate source data as an advisory EnrichmentBundle. Never a determination; never clears or overrides a screen. Batch form: items[] with per-item errors + resume cursor.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "work_id": work_id_field(),
                    "sources": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "source": string_field("Source name (SauceNAO-class similarity results are leads, never citations)."),
                                "match_key": string_field("Identity key this source asserts (title / circle / external id)."),
                                "match_evidence": string_field("What the match rests on."),
                                "retrieved_at": string_field("Retrieval timestamp (caller-stamped; neemu never fetches)."),
                                "basis": { "type": "string", "enum": ["resolved_record", "lead"], "description": "resolved_record = citable; lead = pointer only." },
                                "similarity": { "type": "number", "description": "Similarity score when the source reports one (<0.90 forces lead)." },
                                "tags": { "type": "array", "items": { "type": "string" }, "description": "Candidate tags; enter the bundle only from resolved_record sources." },
                                "escalation_flags": { "type": "array", "items": { "type": "string" }, "description": "Caller-declared escalation signals (single-source, never cancelled)." },
                                "translated": { "type": "boolean", "description": "Tags are translated-tag data (CC BY-NC-SA): stamped non-commercial." },
                                "payload": { "description": "Source payload, stored verbatim + flagged untrusted." }
                            },
                            "required": ["source", "match_key"],
                            "additionalProperties": false
                        }
                    },
                    "grouping": {
                        "type": "object",
                        "properties": {
                            "basis": { "type": "string", "enum": ["official_chain", "heuristic_match", "standalone"] },
                            "confidence": { "type": "number", "minimum": 0, "maximum": 1 },
                            "group_ref": string_field("Series/group id asserted.")
                        },
                        "required": ["basis"],
                        "additionalProperties": false
                    },
                    "versions": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "version": string_field("Version label."),
                                "tags": { "type": "array", "items": { "type": "string" } }
                            },
                            "required": ["version"],
                            "additionalProperties": false
                        },
                        "description": "Version chain; a tag change across versions is an escalation signal."
                    },
                    "items": { "type": "array", "description": "Batch form: array of per-work enrich objects (same fields); order-preserving, per-item errors, resume cursor." },
                    "resume_cursor": {
                        "type": "object",
                        "properties": { "next_index": { "type": "integer", "minimum": 0 } },
                        "additionalProperties": false
                    }
                },
                "additionalProperties": false
            },
            "outputSchema": {
                "type": "object",
                "properties": {
                    "record_id": { "type": "string" },
                    "results": { "type": "array" },
                    "partial": { "type": "boolean" },
                    "resume_cursor": {}
                }
            },
            "annotations": annotations(false)
        },
        {
            "name": "neemu_segment_scene",
            "description": "Deterministic page segmentation into scenes (boundary: dimension change or max-pages) + file-metric metadata. Read-only; stable scene ids. Paginated.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "work_id": work_id_field(),
                    "preflight_id": preflight_field(),
                    "max_pages_per_scene": { "type": "integer", "minimum": 1, "maximum": 64, "description": "Segmentation parameter (default 24), recorded." },
                    "offset": { "type": "integer", "minimum": 0, "description": "First scene (default 0)." },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 64, "description": "Max scenes (default 32)." }
                },
                "required": ["work_id", "preflight_id"],
                "additionalProperties": false
            },
            "outputSchema": {
                "type": "object",
                "properties": {
                    "metadata": { "type": "object" },
                    "total_scenes": { "type": "integer" },
                    "scenes": { "type": "array" },
                    "partial": { "type": "boolean" }
                }
            },
            "annotations": annotations(true)
        },
        {
            "name": "neemu_extract_faces",
            "description": "Mint FaceObservation records for a page range from caller grid-read / model-estimate measurements. Validates 8 channels, F-code or new-candidate, basis, lineage caps. Oversize ranges: declared partial.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "work_id": work_id_field(),
                    "preflight_id": preflight_field(),
                    "page_start": { "type": "integer", "minimum": 0 },
                    "page_end": { "type": "integer", "minimum": 0 },
                    "observations": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "page": { "type": "integer", "minimum": 0 },
                                "character_slot": string_field("Character slot/name."),
                                "channels": {
                                    "type": "object",
                                    "properties": {
                                        "gaze": { "type": "string" },
                                        "pupils": { "type": "string" },
                                        "crossing": { "type": "string" },
                                        "lids": { "type": "string" },
                                        "brows": { "type": "string" },
                                        "mouth": { "type": "string" },
                                        "flush": { "type": "string" },
                                        "secondary": { "type": "string" }
                                    },
                                    "required": ["gaze", "pupils", "crossing", "lids", "brows", "mouth", "flush", "secondary"]
                                },
                                "f_code": { "type": ["string", "null"], "description": "F-01..F-12, or null + new_candidate." },
                                "new_candidate": {
                                    "type": "object",
                                    "properties": {
                                        "channels": { "type": "object", "description": "Full 8-channel description." },
                                        "beats": { "type": "array", "items": { "type": "string" }, "description": "Beats the state carries." },
                                        "proposed_code": string_field("Proposed code (no F-01..F-12 collision).")
                                    },
                                    "required": ["channels", "beats", "proposed_code"]
                                },
                                "basis": { "type": "string", "enum": ["grid-read", "model-estimate"] },
                                "model_lineage": {
                                    "type": "object",
                                    "properties": {
                                        "code_license": string_field("Model code license (delta 5)."),
                                        "weights_license": string_field("Model weights license (delta 5)."),
                                        "training_data_licenses": { "type": "array", "items": { "type": "string" } },
                                        "trained_on": { "type": "array", "items": { "type": "string" }, "description": "Training lineages (delta-6 cap)." }
                                    },
                                    "required": ["code_license", "weights_license", "training_data_licenses"]
                                }
                            },
                            "required": ["page", "channels", "basis"],
                            "additionalProperties": false
                        }
                    }
                },
                "required": ["work_id", "preflight_id", "page_start", "page_end"],
                "additionalProperties": false
            },
            "outputSchema": {
                "type": "object",
                "properties": {
                    "record_ids": { "type": "array" },
                    "count": { "type": "integer" },
                    "partial": { "type": "boolean" }
                }
            },
            "annotations": annotations(false)
        },
        {
            "name": "neemu_extract_dialogue",
            "description": "Mint DialogueLine records for a page range from caller-transcribed lines ([unclear] = illegible, never guessed). Word-level nesting (delta 8) + known_limitations.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "work_id": work_id_field(),
                    "preflight_id": preflight_field(),
                    "page_start": { "type": "integer", "minimum": 0 },
                    "page_end": { "type": "integer", "minimum": 0 },
                    "lines": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "page": { "type": "integer", "minimum": 0 },
                                "speaker": string_field("Character slot/name."),
                                "text": string_field("Exact transcription; [unclear] for illegible."),
                                "beat": string_field("Scene context, one line."),
                                "face_ref": string_field("FaceObservation id this line lands in.")
                            },
                            "required": ["page", "speaker", "text"],
                            "additionalProperties": false
                        }
                    }
                },
                "required": ["work_id", "preflight_id", "page_start", "page_end"],
                "additionalProperties": false
            },
            "outputSchema": {
                "type": "object",
                "properties": {
                    "record_ids": { "type": "array" },
                    "count": { "type": "integer" },
                    "partial": { "type": "boolean" }
                }
            },
            "annotations": annotations(false)
        },
        {
            "name": "neemu_extract_acts",
            "description": "Mint ActRecord(s) for one deterministic scene (SPEC 3.3): ACT code resolved against the registry seed (unknown codes enter as candidates, unmapped codes labelled, none dropped), contact pairs in the D-076 collider/compliance vocabulary.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "work_id": work_id_field(),
                    "preflight_id": preflight_field(),
                    "scene_id": string_field("Deterministic scene id from neemu_segment_scene."),
                    "max_pages_per_scene": { "type": "integer", "minimum": 1, "maximum": 64, "description": "Must match the segmentation the scene id came from." },
                    "acts": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "act_code": string_field("ACT code (B-canon / KISS / M-01 / SOLO; unknown codes become candidates)."),
                                "participants": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "slot": string_field("Participant slot."),
                                            "role": string_field("Role in this act.")
                                        },
                                        "required": ["slot", "role"],
                                        "additionalProperties": false
                                    }
                                },
                                "contact_pairs": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "a": {
                                                "type": "object",
                                                "properties": {
                                                    "slot": { "type": "string" },
                                                    "part": { "type": "string" },
                                                    "collider": { "type": "string", "enum": ["sphere", "capsule", "surface"] }
                                                },
                                                "required": ["slot", "part", "collider"],
                                                "additionalProperties": false
                                            },
                                            "b": {
                                                "type": "object",
                                                "properties": {
                                                    "slot": { "type": "string" },
                                                    "part": { "type": "string" },
                                                    "collider": { "type": "string", "enum": ["sphere", "capsule", "surface"] }
                                                },
                                                "required": ["slot", "part", "collider"],
                                                "additionalProperties": false
                                            },
                                            "compliance_class": { "type": "string", "enum": ["hard_non_penetration", "neighbour", "attachment", "falloff_weighted"] }
                                        },
                                        "required": ["a", "b", "compliance_class"],
                                        "additionalProperties": false
                                    }
                                },
                                "pose_track": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "label": { "type": "string" },
                                            "joint_configuration": { "type": "string" },
                                            "support": { "type": "string" },
                                            "orientation": { "type": "string" }
                                        },
                                        "required": ["label", "joint_configuration", "support"],
                                        "additionalProperties": false
                                    }
                                },
                                "motion": {
                                    "type": "object",
                                    "properties": {
                                        "beats_per_cycle": { "type": "number" },
                                        "tempo_curve": { "type": "string" },
                                        "intensity_curve": { "type": "array", "items": { "type": "number" } },
                                        "entry": { "type": "string" },
                                        "exit": { "type": "string" }
                                    },
                                    "additionalProperties": false
                                },
                                "face_timeline": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "beat": { "type": "string" },
                                            "ref": string_field("F-code or FaceObservation record id.")
                                        },
                                        "required": ["beat", "ref"],
                                        "additionalProperties": false
                                    }
                                },
                                "sfx_hooks": { "type": "array", "items": { "type": "string" }, "description": "Torori cue refs (e.g. pair.torori.*)." },
                                "props": { "type": "array", "items": { "type": "string" } }
                            },
                            "required": ["act_code"],
                            "additionalProperties": false
                        }
                    }
                },
                "required": ["work_id", "preflight_id", "scene_id", "acts"],
                "additionalProperties": false
            },
            "outputSchema": {
                "type": "object",
                "properties": {
                    "record_ids": { "type": "array" },
                    "count": { "type": "integer" },
                    "unmapped_act_codes": { "type": "array" },
                    "candidate_act_codes": { "type": "array" }
                }
            },
            "annotations": annotations(false)
        },
        {
            "name": "neemu_analyze_voice",
            "description": "Analyse voice segments of a cleared work's audio/video (SPEC 3.4): server-measured WAV envelope features (duration, pauses, breath candidates, clipping) + caller transcript/features range-checked against torori's frozen conditioning ranges; mints VoicePerformanceRecords. No playback path exists.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "work_id": work_id_field(),
                    "preflight_id": preflight_field(),
                    "segments": {
                        "type": "array",
                        "description": "Segments over the work's AV media (max 512).",
                        "items": {
                            "type": "object",
                            "properties": {
                                "media_name": string_field("Manifest file name (audio/video)."),
                                "start_s": { "type": "number", "minimum": 0 },
                                "end_s": { "type": "number", "exclusiveMinimum": 0 },
                                "speaker": string_field("Speaker label."),
                                "segment_kind": { "type": "string", "enum": ["speech", "non_speech"] },
                                "transcript": string_field("Speech only: caller-side ASR transcript (pinned pipeline)."),
                                "non_speech_class": string_field("Non-speech only: class label, recorded as [non-speech: <class>]."),
                                "scene_function": string_field("What the segment does in the scene."),
                                "is_fragment": { "type": "boolean", "description": "Breath-bracket source: a fragment implies Required." },
                                "features": {
                                    "type": "object",
                                    "description": "Caller-measured features (f0_median_hz, f0_range_semitones, f0_contour_class, tempo_syllables_per_sec, pause_structure, breathiness, roughness, grade, snr_db); range-checked, never re-derived.",
                                    "additionalProperties": true
                                },
                                "accent_heard": {
                                    "type": "object",
                                    "description": "Delta-9 shape: distribution over registry profiles + top1 + margin + intensity; top1 must be the distribution argmax.",
                                    "additionalProperties": true
                                }
                            },
                            "required": ["media_name", "start_s", "end_s", "speaker", "segment_kind", "scene_function"],
                            "additionalProperties": false
                        }
                    }
                },
                "required": ["work_id", "preflight_id", "segments"],
                "additionalProperties": false
            },
            "outputSchema": {
                "type": "object",
                "properties": {
                    "record_ids": { "type": "array" },
                    "segments": { "type": "integer" },
                    "source_kinds": { "type": "array" }
                }
            },
            "annotations": annotations(false)
        },
        {
            "name": "neemu_assess_voice_clonability",
            "description": "Two-axis clonability verdict over one speaker's VoicePerformanceRecords (SPEC 3.10). Rights axis: a complete ConsentArtifact is a hard gate - absent or incomplete is a typed rights_artifact_missing refusal, never a pass; unknown rights basis fails the axis. Technical axis measured against constants recorded on the record. Verdict CLONABLE / NOT-CLONABLE; assessments are immutable and supersede by reference.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "work_id": work_id_field(),
                    "preflight_id": preflight_field(),
                    "voice_record_ids": { "type": "array", "items": { "type": "string" }, "description": "VoicePerformanceRecord ids, one speaker (max 64)." },
                    "consent_artifact": {
                        "type": "object",
                        "description": "ConsentArtifact: written_permission_ref, disclosure_version_given, consent_recording_hash (sha256 hex), consent_vs_training_speaker_match (verified|failed|not_run), approved_use_scope, retained_until. All six required.",
                        "additionalProperties": true
                    },
                    "rights_basis": {
                        "type": "object",
                        "description": "The right relied on: {basis, evidence_ref}. Absent = unknown basis = rights axis FAIL.",
                        "additionalProperties": true
                    },
                    "presence_verified": { "type": "boolean", "description": "A separate liveness/presence fact; with a complete artifact it raises the rights-evidence class to 'strong'. Never substitutes for the artifact." },
                    "supersedes": string_field("Prior assessment record id this one supersedes.")
                },
                "required": ["work_id", "preflight_id", "voice_record_ids"],
                "additionalProperties": false
            },
            "outputSchema": {
                "type": "object",
                "properties": {
                    "record_id": { "type": "string" },
                    "verdict": { "type": "string" },
                    "speaker": { "type": "string" }
                }
            },
            "annotations": annotations(false)
        },
        {
            "name": "neemu_derive_voice",
            "description": "Derive a NEW estate VoiceIdentity from a CLONABLE assessment (SPEC 3.11): adaptation plan mandatory (an identity plan refuses), derivation class checked against the assessment's rights-evidence class (delta 3, refusal_gate_tier), engine license evaluated at derivation time and fails closed (delta 4). Output carries an embedded watermark whose payload is the derivation record id (delta 14).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "work_id": work_id_field(),
                    "preflight_id": preflight_field(),
                    "assessment_id": string_field("voice_clonability_assessment record id (verdict must be CLONABLE)."),
                    "derivation_class": { "type": "string", "enum": ["characterisation-only", "adapted-derivation", "high-fidelity"], "description": "Default adapted-derivation. high-fidelity requires 'strong' rights evidence." },
                    "adaptation": {
                        "type": "object",
                        "properties": {
                            "pitch_semitones": { "type": "number", "minimum": -12, "maximum": 12 },
                            "formant_ratio": { "type": "number", "minimum": 0.5, "maximum": 2.0 },
                            "tempo_character": string_field("Tempo character (\"unchanged\" counts toward an identity plan)."),
                            "accent_target": string_field("Accent registry profile id, or null.")
                        },
                        "required": ["pitch_semitones", "formant_ratio", "tempo_character"],
                        "additionalProperties": false
                    },
                    "engine": {
                        "type": "object",
                        "properties": {
                            "engine": string_field("Engine name."),
                            "license": string_field("License name."),
                            "license_alive": { "type": "boolean" },
                            "commercial_ok": { "type": "boolean" }
                        },
                        "required": ["engine", "license", "license_alive", "commercial_ok"],
                        "additionalProperties": false
                    },
                    "target_identity": string_field("New estate identity ref (never the source speaker)."),
                    "design_targets": {
                        "type": "object",
                        "properties": {
                            "warmth": { "type": "number", "minimum": 0, "maximum": 1 },
                            "breathiness_balance": { "type": "number", "minimum": 0, "maximum": 1 },
                            "pitch_placement": { "type": "number", "minimum": 0, "maximum": 1 },
                            "articulation_clarity": { "type": "number", "minimum": 0, "maximum": 1 }
                        },
                        "required": ["warmth", "breathiness_balance", "pitch_placement", "articulation_clarity"],
                        "additionalProperties": false
                    }
                },
                "required": ["work_id", "preflight_id", "assessment_id", "adaptation", "engine", "target_identity", "design_targets"],
                "additionalProperties": false
            },
            "outputSchema": {
                "type": "object",
                "properties": {
                    "derivation_id": { "type": "string" },
                    "identity_id": { "type": "string" },
                    "identity_ref": { "type": "string" },
                    "watermark": { "type": "string" }
                }
            },
            "annotations": annotations(false)
        },
        {
            "name": "neemu_emit_torori_cue",
            "description": "Emit a torori cue bundle (bundle.ron, frozen format) from dialogue lines (SPEC 3.5): delivery normalised to torori's frozen conditioning schema, ranges refused never clamped, target_voice REQUIRED and must resolve to an estate VoiceIdentity - never a source performer. File bundles only; neemu never calls torori.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "work_id": work_id_field(),
                    "preflight_id": preflight_field(),
                    "target_voice": string_field("Estate VoiceIdentity identity_ref."),
                    "cues": {
                        "type": "array",
                        "description": "Cues (max 64, torori's bundle cap).",
                        "items": {
                            "type": "object",
                            "properties": {
                                "dialogue_id": string_field("dialogue_line record id; its text becomes the cue text."),
                                "voice_id": string_field("Optional voice_performance record id (tier + breath defaults)."),
                                "delivery": {
                                    "type": "object",
                                    "description": "Delivery overrides merged over the line's delivery map; keys restricted to the frozen conditioning DELIVERY set.",
                                    "additionalProperties": true
                                },
                                "sfx_pairs": { "type": "array", "items": { "type": "string" }, "description": "Paired SFX names for torori's pairing engine." }
                            },
                            "required": ["dialogue_id"],
                            "additionalProperties": false
                        }
                    }
                },
                "required": ["work_id", "preflight_id", "target_voice", "cues"],
                "additionalProperties": false
            },
            "outputSchema": {
                "type": "object",
                "properties": {
                    "record_id": { "type": "string" },
                    "bundle_id": { "type": "string" },
                    "content_hash": { "type": "string" },
                    "bundle_dir": { "type": "string" }
                }
            },
            "annotations": annotations(false)
        },
        {
            "name": "neemu_understand_scene",
            "description": "Mint a SceneUnderstanding over one deterministic scene (SPEC 3.6, delta 12): beats + machinery + dynamics, every claim carrying evidence refs that must resolve to this work's records - an unlinked claim is an evidence_refusal. Attaches only; never creates/merges/splits scenes.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "work_id": work_id_field(),
                    "preflight_id": preflight_field(),
                    "scene_id": string_field("Deterministic scene id from neemu_segment_scene."),
                    "max_pages_per_scene": { "type": "integer", "minimum": 1, "maximum": 64 },
                    "beats": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "type": { "type": "string", "enum": ["setup", "induction", "deepening", "trigger_installation", "trigger_use", "testing", "act", "escalation", "aftermath", "transition"] },
                                "page_start": { "type": "integer", "minimum": 0 },
                                "page_end": { "type": "integer", "minimum": 0 },
                                "summary": string_field("One-line beat summary."),
                                "evidence_refs": { "type": "array", "items": { "type": "string" }, "description": "Record ids evidencing this beat (required, non-empty)." }
                            },
                            "required": ["type", "page_start", "page_end", "evidence_refs"],
                            "additionalProperties": false
                        }
                    },
                    "machinery": {
                        "type": "object",
                        "properties": {
                            "mechanism": { "type": "string", "enum": ["device", "app", "eye_contact", "drug", "magic", "voice", "other"] },
                            "stage_model": string_field("How control deepens (the caller's read)."),
                            "reset_wakener": { "type": "string", "enum": ["present", "absent"], "description": "Present-or-absent is itself a recorded finding." },
                            "reset_detail": string_field("Reset/wakener detail when present."),
                            "evidence_refs": { "type": "array", "items": { "type": "string" } }
                        },
                        "required": ["mechanism", "stage_model", "reset_wakener", "evidence_refs"],
                        "additionalProperties": false
                    },
                    "dynamics": {
                        "type": "object",
                        "properties": {
                            "roles": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "slot": { "type": "string" },
                                        "role": { "type": "string" }
                                    },
                                    "required": ["slot", "role"],
                                    "additionalProperties": false
                                }
                            },
                            "who_directs": string_field("Directing slot."),
                            "who_responds": string_field("Responding slot."),
                            "note": string_field("Dynamics note."),
                            "evidence_refs": { "type": "array", "items": { "type": "string" } }
                        },
                        "required": ["who_directs", "who_responds", "evidence_refs"],
                        "additionalProperties": false
                    }
                },
                "required": ["work_id", "preflight_id", "scene_id", "beats", "machinery", "dynamics"],
                "additionalProperties": false
            },
            "outputSchema": {
                "type": "object",
                "properties": {
                    "record_id": { "type": "string" },
                    "scene_id": { "type": "string" },
                    "beat_count": { "type": "integer" },
                    "link_count": { "type": "integer" }
                }
            },
            "annotations": annotations(false)
        },
        {
            "name": "neemu_list_records",
            "description": "Bounded paginated record listing by kind, server-computed counts. Gate records stay listable on excluded works (refusal trail); extraction kinds on a non-cleared work refuse.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "work_id": work_id_field(),
                    "kind": { "type": "string", "enum": ["preflight", "screen", "enrichment_bundle", "face_observation", "dialogue_line", "act_record", "scene_understanding", "voice_performance", "voice_clonability_assessment", "voice_derivation", "voice_identity", "torori_cue", "torori_cue_bundle", "export_bundle"], "description": "Kind filter (default: all)." },
                    "offset": { "type": "integer", "minimum": 0 },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 100 }
                },
                "required": ["work_id"],
                "additionalProperties": false
            },
            "outputSchema": {
                "type": "object",
                "properties": {
                    "total": { "type": "integer" },
                    "records": { "type": "array" },
                    "partial": { "type": "boolean" }
                }
            },
            "annotations": annotations(true)
        },
        {
            "name": "neemu_export",
            "description": "Tier-checked bundle export of a cleared work's records (file emitted; no registry change). Training bundles refuse Tier A material (tier_refusal names the blocker); analysis accepts either tier.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "work_id": work_id_field(),
                    "bundle_kind": { "type": "string", "enum": ["analysis", "training"] },
                    "record_ids": { "type": "array", "items": { "type": "string" }, "description": "Records to include (default: all extraction records)." }
                },
                "required": ["work_id", "bundle_kind"],
                "additionalProperties": false
            },
            "outputSchema": {
                "type": "object",
                "properties": {
                    "bundle_id": { "type": "string" },
                    "path": { "type": "string" },
                    "record_count": { "type": "integer" },
                    "tier": { "type": "string" }
                }
            },
            // WS2-W5 continuation (D-147): neemu_export WRITES its
            // bundle into the registry store (store_export), so the
            // S1/S2 `annotations(true)` was a false read-only claim.
            // Served honestly as a write from 0.2.0; behaviour is
            // unchanged — annotation delta only.
            "annotations": annotations(false)
        }
    ])
}

/// Names of the served tools, in `tools/list` order (§4 order:
/// enrich sits after screen; acts after dialogue; the S3 voice
/// tools after acts; understand after the S3 slot).
pub const TOOL_NAMES: [&str; 15] = [
    "neemu_register_work",
    "neemu_validate_ingest",
    "neemu_screen_work",
    "neemu_enrich",
    "neemu_segment_scene",
    "neemu_extract_faces",
    "neemu_extract_dialogue",
    "neemu_extract_acts",
    "neemu_analyze_voice",
    "neemu_assess_voice_clonability",
    "neemu_derive_voice",
    "neemu_emit_torori_cue",
    "neemu_understand_scene",
    "neemu_list_records",
    "neemu_export",
];

/// Dispatch one `tools/call` by name.
pub fn call_tool(neemu: &Neemu, name: &str, args: &Value) -> CallResult {
    match name {
        "neemu_register_work" => neemu.register_work(args),
        "neemu_validate_ingest" => neemu.validate_ingest(args),
        "neemu_screen_work" => neemu.screen_work(args),
        "neemu_enrich" => crate::s2::enrich(neemu, args),
        "neemu_segment_scene" => segment_scene(neemu, args),
        "neemu_extract_faces" => extract_faces(neemu, args),
        "neemu_extract_dialogue" => extract_dialogue(neemu, args),
        "neemu_extract_acts" => crate::s2::extract_acts(neemu, args),
        "neemu_analyze_voice" => crate::s3::analyze_voice(neemu, args),
        "neemu_assess_voice_clonability" => crate::s3::assess_voice_clonability(neemu, args),
        "neemu_derive_voice" => crate::s3::derive_voice(neemu, args),
        "neemu_emit_torori_cue" => crate::s3::emit_torori_cue(neemu, args),
        "neemu_understand_scene" => crate::s2::understand_scene(neemu, args),
        "neemu_list_records" => list_records(neemu, args),
        "neemu_export" => export(neemu, args),
        other => Err(CallError::Args(format!("unknown tool: {other}"))),
    }
}

fn gated_work(neemu: &Neemu, obj: &Map<String, Value>) -> Result<(String, Value), CallError> {
    let work_id = req_str(obj, "work_id")?.to_string();
    let preflight_id = opt_str(obj, "preflight_id")?;
    let work = neemu.gate_extraction(&work_id, preflight_id)?;
    Ok((work_id, work))
}

fn page_range(obj: &Map<String, Value>, work: &Value) -> Result<(usize, usize, bool), CallError> {
    let page_start = opt_usize(obj, "page_start", usize::MAX)?;
    let page_end = opt_usize(obj, "page_end", usize::MAX)?;
    if page_start == usize::MAX || page_end == usize::MAX {
        return Err(CallError::Args(
            "fields 'page_start' and 'page_end' are required".to_string(),
        ));
    }
    let page_count = work["media"].as_array().map(|m| m.len()).unwrap_or(0);
    Neemu::bounded_range(page_start, page_end, page_count)
}

fn segment_scene(neemu: &Neemu, args: &Value) -> CallResult {
    let obj = arg_object(args)?;
    let (work_id, work) = gated_work(neemu, obj)?;
    let max_pages = opt_usize(obj, "max_pages_per_scene", DEFAULT_MAX_PAGES_PER_SCENE)?;
    if max_pages == 0 || max_pages > MAX_PAGES_PER_SCENE_LIMIT {
        return Err(CallError::Args(format!(
            "max_pages_per_scene must be within 1..={MAX_PAGES_PER_SCENE_LIMIT}"
        )));
    }
    let offset = opt_usize(obj, "offset", 0)?;
    let limit = opt_usize(obj, "limit", 32)?;
    if limit == 0 || limit > MAX_SCENES_PER_CALL {
        return Err(CallError::Args(format!(
            "limit must be within 1..={MAX_SCENES_PER_CALL}"
        )));
    }
    let (metadata, scenes) = Neemu::segment_pages(&work, max_pages);
    let total = scenes.len();
    let page: Vec<Value> = scenes.into_iter().skip(offset).take(limit).collect();
    let returned = page.len();
    let partial = offset + returned < total;
    Ok(json!({
        "work_id": work_id,
        "metadata": metadata,
        "segmentation_parameters": {
            "max_pages_per_scene": max_pages,
            "boundary_rule": "dimension_change_or_max_pages",
        },
        "total_scenes": total,
        "scenes": page,
        "partial": partial,
        "next_offset": if partial { json!(offset + returned) } else { Value::Null },
    }))
}

/// Validate a channels object: exactly the eight channels, each a
/// non-empty short string.
fn parse_channels(value: &Value, context: &str) -> Result<Value, CallError> {
    let channels = value
        .as_object()
        .ok_or_else(|| CallError::Args(format!("{context}: channels must be an object")))?;
    for key in channels.keys() {
        if !FACE_CHANNELS.contains(&key.as_str()) {
            return Err(CallError::Args(format!(
                "{context}: unknown face channel '{key}'"
            )));
        }
    }
    let mut parsed = Map::new();
    for channel in FACE_CHANNELS {
        let text = channels
            .get(channel)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                CallError::Args(format!(
                    "{context}: channel '{channel}' is missing or empty"
                ))
            })?;
        if text.chars().count() > MAX_SHORT_CHARS {
            return Err(CallError::Args(format!(
                "{context}: channel '{channel}' exceeds the character bound"
            )));
        }
        parsed.insert(channel.to_string(), json!(text));
    }
    Ok(Value::Object(parsed))
}

fn extract_faces(neemu: &Neemu, args: &Value) -> CallResult {
    let obj = arg_object(args)?;
    let (work_id, work) = gated_work(neemu, obj)?;
    let (start, end, partial) = page_range(obj, &work)?;
    let observations = opt_arr(obj, "observations")?.cloned().unwrap_or_default();
    if observations.len() > MAX_OBSERVATIONS_PER_CALL {
        return Err(ToolError::new(
            KIND_BOUND_EXCEEDED,
            format!(
                "{} observations in one call; the bound is {MAX_OBSERVATIONS_PER_CALL}",
                observations.len()
            ),
        )
        .into());
    }
    let media = work["media"].as_array().cloned().unwrap_or_default();
    let work_tier = work["tier"].as_str().unwrap_or("A").to_string();
    let mut record_ids = Vec::new();
    for (index, observation) in observations.iter().enumerate() {
        let context = format!("observations[{index}]");
        let obs = observation
            .as_object()
            .ok_or_else(|| CallError::Args(format!("{context}: must be an object")))?;
        let page = opt_usize(obs, "page", usize::MAX)?;
        if page == usize::MAX {
            return Err(CallError::Args(format!(
                "{context}: field 'page' is required"
            )));
        }
        if page < start || page > end {
            return Err(CallError::Args(format!(
                "{context}: page {page} is outside the processed range {start}..={end}"
            )));
        }
        let channels = parse_channels(obs.get("channels").unwrap_or(&Value::Null), &context)?;
        let basis = req_str(obs, "basis")?;
        if basis != "grid-read" && basis != "model-estimate" {
            return Err(CallError::Args(format!(
                "{context}: basis must be \"grid-read\" or \"model-estimate\""
            )));
        }
        // F-code: an existing code, or null + a complete new-candidate.
        let (f_code, new_candidate) = match obs.get("f_code") {
            Some(Value::Null) | None => {
                let candidate = obs
                    .get("new_candidate")
                    .and_then(|c| c.as_object())
                    .ok_or_else(|| {
                        CallError::Args(format!(
                            "{context}: f_code is null but no new_candidate block was supplied (candidates are never dropped)"
                        ))
                    })?;
                let candidate_channels = parse_channels(
                    candidate.get("channels").unwrap_or(&Value::Null),
                    &format!("{context}.new_candidate"),
                )?;
                let beats = candidate
                    .get("beats")
                    .and_then(|b| b.as_array())
                    .filter(|b| !b.is_empty())
                    .ok_or_else(|| {
                        CallError::Args(format!(
                            "{context}.new_candidate: beats must be a non-empty array"
                        ))
                    })?;
                for beat in beats {
                    if beat.as_str().filter(|s| !s.is_empty()).is_none() {
                        return Err(CallError::Args(format!(
                            "{context}.new_candidate: beats entries must be non-empty strings"
                        )));
                    }
                }
                let proposed = req_str(candidate, "proposed_code")?;
                if F_CODES.contains(&proposed) {
                    return Err(CallError::Args(format!(
                        "{context}.new_candidate: proposed_code {proposed} collides with an existing F-code"
                    )));
                }
                (
                    Value::Null,
                    json!({
                        "channels": candidate_channels,
                        "beats": beats,
                        "proposed_code": proposed,
                    }),
                )
            }
            Some(Value::String(code)) => {
                if !F_CODES.contains(&code.as_str()) {
                    return Err(CallError::Args(format!(
                        "{context}: unknown f_code {code} (canonical codes are F-01..F-12; use null + new_candidate for a new state)"
                    )));
                }
                (json!(code), Value::Null)
            }
            Some(_) => {
                return Err(CallError::Args(format!(
                    "{context}: f_code must be a string or null"
                )))
            }
        };
        // Lineage (deltas 5 + 6): required for model-estimate reads; a
        // tainted lineage caps the record at Tier A and can never mint
        // into a Tier T work.
        let mut lineage_out = Value::Null;
        let mut tier_cap = work_tier.clone();
        let mut derived_restricted = false;
        if basis == "model-estimate" {
            let lineage = obs
                .get("model_lineage")
                .and_then(|l| l.as_object())
                .ok_or_else(|| {
                    CallError::Args(format!(
                        "{context}: basis model-estimate requires model_lineage (delta 5)"
                    ))
                })?;
            let code_license = req_str(lineage, "code_license")?.to_string();
            let weights_license = req_str(lineage, "weights_license")?.to_string();
            let training_data_licenses = match lineage.get("training_data_licenses") {
                Some(Value::Array(items)) => {
                    let mut parsed = Vec::new();
                    for item in items {
                        parsed.push(
                            item.as_str()
                                .ok_or_else(|| {
                                    CallError::Args(format!(
                                        "{context}: training_data_licenses entries must be strings"
                                    ))
                                })?
                                .to_string(),
                        );
                    }
                    parsed
                }
                _ => {
                    return Err(CallError::Args(format!(
                        "{context}: model_lineage.training_data_licenses must be an array"
                    )))
                }
            };
            let trained_on = match lineage.get("trained_on") {
                None | Some(Value::Null) => Vec::new(),
                Some(Value::Array(items)) => items.clone(),
                Some(_) => {
                    return Err(CallError::Args(format!(
                        "{context}: model_lineage.trained_on must be an array"
                    )))
                }
            };
            if let Some(tainted) = tainted_lineage(&trained_on) {
                if work_tier == "T" {
                    return Err(ToolError::new(
                        KIND_TIER_REFUSAL,
                        format!(
                            "face model lineage includes {tainted}: its outputs are hard-capped at Tier A and can never contribute to a Tier T record (delta 6)"
                        ),
                    )
                    .with_details(json!({ "work_id": work_id, "lineage": tainted }))
                    .into());
                }
                tier_cap = "A".to_string();
                derived_restricted = tainted == "rafdb";
            }
            lineage_out = json!({
                "code_license": code_license,
                "weights_license": weights_license,
                "training_data_licenses": training_data_licenses,
                "trained_on": trained_on,
            });
        }
        let body = json!({
            "record_kind": "face_observation",
            "work_id": work_id,
            "tier": tier_cap,
            "locus": { "page": page },
            "character_slot": opt_str(obs, "character_slot")?,
            "channels": channels,
            "f_code": f_code,
            "new_candidate": new_candidate,
            "basis": basis,
            "model_lineage": lineage_out,
            "derived_data_restricted": derived_restricted,
            "input_hashes": [media[page]["sha256"]],
            "extractor": EXTRACTOR_ID,
        });
        let record_id = neemu.mint_record(&work_id, "face_", body)?;
        record_ids.push(record_id);
    }
    Ok(json!({
        "record_ids": record_ids,
        "count": record_ids.len(),
        "partial": partial,
        "pages_processed": { "page_start": start, "page_end": end },
        "continuation": if partial {
            json!({ "page_start": end + 1, "note": "range exceeded the per-call page bound; re-call with the continuation range" })
        } else {
            Value::Null
        },
    }))
}

fn extract_dialogue(neemu: &Neemu, args: &Value) -> CallResult {
    let obj = arg_object(args)?;
    let (work_id, work) = gated_work(neemu, obj)?;
    let (start, end, partial) = page_range(obj, &work)?;
    let lines = opt_arr(obj, "lines")?.cloned().unwrap_or_default();
    if lines.len() > MAX_LINES_PER_CALL {
        return Err(ToolError::new(
            KIND_BOUND_EXCEEDED,
            format!(
                "{} lines in one call; the bound is {MAX_LINES_PER_CALL}",
                lines.len()
            ),
        )
        .into());
    }
    let media = work["media"].as_array().cloned().unwrap_or_default();
    let work_tier = work["tier"].as_str().unwrap_or("A").to_string();
    let mut record_ids = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let context = format!("lines[{index}]");
        let line_obj = line
            .as_object()
            .ok_or_else(|| CallError::Args(format!("{context}: must be an object")))?;
        let page = opt_usize(line_obj, "page", usize::MAX)?;
        if page == usize::MAX {
            return Err(CallError::Args(format!(
                "{context}: field 'page' is required"
            )));
        }
        if page < start || page > end {
            return Err(CallError::Args(format!(
                "{context}: page {page} is outside the processed range {start}..={end}"
            )));
        }
        let speaker = req_str(line_obj, "speaker")?.to_string();
        let text = req_text(line_obj, "text")?.to_string();
        let beat = opt_str(line_obj, "beat")?.map(|s| s.to_string());
        let face_ref = opt_str(line_obj, "face_ref")?.map(|s| s.to_string());
        if let Some(face_ref) = &face_ref {
            if !neemu.record_exists(&work_id, face_ref) {
                return Err(CallError::Args(format!(
                    "{context}: face_ref {face_ref} is not a record of this work"
                )));
            }
        }
        // Delta 8 word-level spine, derived deterministically from the
        // line text; the page lane carries no timing (nulls, declared
        // in known_limitations).
        let words: Vec<Value> = text
            .split_whitespace()
            .map(|word| {
                json!({
                    "word": word,
                    "start": null,
                    "end": null,
                    "score": null,
                    "speaker": speaker,
                })
            })
            .collect();
        let body = json!({
            "record_kind": "dialogue_line",
            "work_id": work_id,
            "tier": work_tier,
            "locus": { "page": page },
            "speaker": speaker,
            "text": text,
            "beat": beat,
            "face_ref": face_ref,
            "segment": { "page": page, "speaker": speaker },
            "words": words,
            "known_limitations": DIALOGUE_PAGE_LIMITATIONS,
            "input_hashes": [media[page]["sha256"]],
            "extractor": EXTRACTOR_ID,
        });
        let record_id = neemu.mint_record(&work_id, "dlg_", body)?;
        record_ids.push(record_id);
    }
    Ok(json!({
        "record_ids": record_ids,
        "count": record_ids.len(),
        "partial": partial,
        "pages_processed": { "page_start": start, "page_end": end },
        "continuation": if partial {
            json!({ "page_start": end + 1, "note": "range exceeded the per-call page bound; re-call with the continuation range" })
        } else {
            Value::Null
        },
    }))
}

fn list_records(neemu: &Neemu, args: &Value) -> CallResult {
    let obj = arg_object(args)?;
    let work_id = req_str(obj, "work_id")?.to_string();
    let work = neemu.load_work(&work_id)?;
    let kind = opt_str(obj, "kind")?;
    let offset = opt_usize(obj, "offset", 0)?;
    let limit = opt_usize(obj, "limit", 50)?;
    if limit == 0 || limit > MAX_LIST_LIMIT {
        return Err(CallError::Args(format!(
            "limit must be within 1..={MAX_LIST_LIMIT}"
        )));
    }
    let determination = work
        .get("determination")
        .and_then(|d| d.get("determination"))
        .and_then(|d| d.as_str())
        .unwrap_or("unscreened");
    let mut entries: Vec<Value> = Vec::new();
    let kinds: [(&str, &str); 14] = [
        ("preflight", "preflights"),
        ("screen", "screens"),
        ("enrichment_bundle", "records"),
        ("face_observation", "records"),
        ("dialogue_line", "records"),
        ("act_record", "records"),
        ("scene_understanding", "records"),
        ("voice_performance", "records"),
        ("voice_clonability_assessment", "records"),
        ("voice_derivation", "records"),
        ("voice_identity", "records"),
        ("torori_cue", "records"),
        ("torori_cue_bundle", "records"),
        ("export_bundle", "exports"),
    ];
    for (kind_name, dir) in kinds {
        if let Some(filter) = kind {
            if filter != kind_name {
                continue;
            }
        }
        // Extraction-shaped kinds on a non-cleared work refuse, so the
        // gate is visible in the read surface too; gate records and
        // advisory enrichment bundles stay listable (the refusal
        // trail is inspectable; a bundle is not a determination).
        let extraction_kind = matches!(
            kind_name,
            "face_observation"
                | "dialogue_line"
                | "act_record"
                | "scene_understanding"
                | "voice_performance"
                | "voice_clonability_assessment"
                | "voice_derivation"
                | "voice_identity"
                | "torori_cue"
                | "torori_cue_bundle"
        );
        if extraction_kind && determination != "cleared" {
            return Err(ToolError::new(
                KIND_SCREEN_REFUSAL,
                format!("work {work_id} is not cleared (determination: {determination}); extraction records are not listable"),
            )
            .with_details(json!({ "work_id": work_id, "determination": determination }))
            .into());
        }
        for id in neemu.list_kind_ids(dir, &work_id) {
            if dir == "records" {
                // Both extraction kinds share the records dir; filter by
                // the record's own kind.
                if let Some(record) = neemu.load_record("records", &work_id, &id) {
                    if record["record_kind"].as_str() != Some(kind_name) {
                        continue;
                    }
                } else {
                    continue;
                }
            }
            entries.push(json!({ "record_id": id, "kind": kind_name }));
        }
    }
    entries.sort_by(|a, b| {
        a["record_id"]
            .as_str()
            .unwrap_or_default()
            .cmp(b["record_id"].as_str().unwrap_or_default())
    });
    let total = entries.len();
    let page: Vec<Value> = entries.into_iter().skip(offset).take(limit).collect();
    let returned = page.len();
    let partial = offset + returned < total;
    Ok(json!({
        "work_id": work_id,
        "total": total,
        "records": page,
        "partial": partial,
        "next_offset": if partial { json!(offset + returned) } else { Value::Null },
    }))
}

fn export(neemu: &Neemu, args: &Value) -> CallResult {
    let obj = arg_object(args)?;
    let work_id = req_str(obj, "work_id")?.to_string();
    let work = neemu.load_work(&work_id)?;
    let bundle_kind = req_str(obj, "bundle_kind")?;
    if bundle_kind != "analysis" && bundle_kind != "training" {
        return Err(CallError::Args(
            "field 'bundle_kind' must be \"analysis\" or \"training\"".to_string(),
        ));
    }
    let determination = work
        .get("determination")
        .and_then(|d| d.get("determination"))
        .and_then(|d| d.as_str())
        .unwrap_or("unscreened");
    if determination != "cleared" {
        return Err(ToolError::new(
            KIND_SCREEN_REFUSAL,
            format!("work {work_id} is not cleared for export (determination: {determination})"),
        )
        .with_details(json!({ "work_id": work_id, "determination": determination }))
        .into());
    }
    let work_tier = work["tier"].as_str().unwrap_or("A");
    if bundle_kind == "training" && work_tier == "A" {
        return Err(ToolError::new(
            KIND_TIER_REFUSAL,
            format!("tier A work {work_id} cannot enter a training-shaped bundle"),
        )
        .with_details(json!({ "blocking_works": [work_id], "work_tier": "A" }))
        .into());
    }
    let all = neemu.all_records(&work_id);
    let selected: Vec<Value> = match opt_arr(obj, "record_ids")? {
        None => all,
        Some(ids) => {
            let mut picked = Vec::new();
            for id_value in ids {
                let id = id_value.as_str().ok_or_else(|| {
                    CallError::Args("record_ids entries must be strings".to_string())
                })?;
                let record = all
                    .iter()
                    .find(|r| r["record_id"].as_str() == Some(id))
                    .cloned()
                    .ok_or_else(|| {
                        CallError::Args(format!("record {id} is not a record of this work"))
                    })?;
                picked.push(record);
            }
            picked
        }
    };
    if bundle_kind == "training" {
        for record in &selected {
            if record["tier"].as_str() == Some("A") {
                return Err(ToolError::new(
                    KIND_TIER_REFUSAL,
                    "a selected record is tier-capped at A (delta 6 lineage) and cannot enter a training-shaped bundle",
                )
                .with_details(json!({ "blocking_records": [record["record_id"]] }))
                .into());
            }
        }
    }
    let selected_ids: Vec<Value> = selected.iter().map(|r| r["record_id"].clone()).collect();
    let bundle_body = json!({
        "work_id": work_id,
        "bundle_kind": bundle_kind,
        "tier": work_tier,
        "record_ids": selected_ids,
    });
    let bundle_id = content_id("bundle_", &bundle_body);
    let bundle = json!({
        "record_kind": "export_bundle",
        "bundle_id": bundle_id,
        "work_id": work_id,
        "bundle_kind": bundle_kind,
        "tier": work_tier,
        "records": selected,
        "extractor": EXTRACTOR_ID,
    });
    let path = neemu.store_export(&work_id, &bundle_id, &bundle)?;
    Ok(json!({
        "bundle_id": bundle_id,
        "path": path.to_string_lossy(),
        "record_count": bundle["records"].as_array().map(|r| r.len()).unwrap_or(0),
        "tier": work_tier,
        "bundle_kind": bundle_kind,
    }))
}
