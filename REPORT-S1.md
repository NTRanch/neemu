# Neemu S1 — build report (2026-10-27)

Crate `neemu` 0.1.0 at `~/workspace/conte-mcp-20261027/` (program
root; SPEC.md / REVIEW-20261027.md / research/ untouched, the
pre-review backup untouched). Hand-rolled newline-delimited
JSON-RPC over stdio, following the estate sibling pattern
(torogao-session-host): serde + serde_json + sha2 only. Registry =
directory tree of JSON record files (`works/`, `preflights/`,
`screens/`, `records/`, `exports/`), root via `--home` /
`$NEEMU_HOME` / `$XDG_DATA_HOME/neemu`.

## Tools built (8 — the S1 slice of the §4 surface; S2+ absent)

| Tool | Annotations (RO / idempotent) | Gate behaviour |
|---|---|---|
| `neemu_register_work` | write / yes | Tier T without rights evidence → `tier_refusal`; unreadable/empty media → `media_unreadable`; >512 files / >64 MiB file / >512 MiB total → `bound_exceeded`; idempotent by content hash |
| `neemu_validate_ingest` | write / yes | Unregistered → `screen_refusal`; mints PreflightRecord (hash/format/access/bounds checks + blocklist floor + delta-14 integrity states); failed checks are a record with `passed:false`, not an error |
| `neemu_screen_work` | write / yes | Fail-closed determination: blocklist hit, any risk flag (school_coded / age_stated_under_18 / age_readable_under_18 / loli_shota_coded), or empty adult_basis → `excluded`, reasons recorded; else `cleared` |
| `neemu_segment_scene` | read / yes | Full gate chain; deterministic scenes (dimension-change or max-pages boundary), content-derived scene ids, file-metric metadata record, paginated |
| `neemu_extract_faces` | write / yes | Full gate chain; 8-channel + F-code/new-candidate validation; delta-5 lineage required for model-estimate; delta-6 tainted lineage on a Tier T work → `tier_refusal`; oversize range → declared partial |
| `neemu_extract_dialogue` | write / yes | Full gate chain; delta-8 word nesting derived from text; `known_limitations` echo; face_ref must name a record of the work |
| `neemu_list_records` | read / yes | Unregistered → `screen_refusal`; extraction kinds on a non-cleared work → `screen_refusal`; gate kinds stay listable (refusal trail); server counts, pagination |
| `neemu_export` | read / yes | Non-cleared → `screen_refusal`; training bundle of a Tier A work → `tier_refusal` with `blocking_works`; A-capped record in a training bundle → `tier_refusal` |

Gate chain order for extraction tools: registered → determination
`cleared` (else `screen_refusal`) → preflight present, known,
work-matching, passed (else `refusal_not_screened`). Canonical
screen: `BLOCKLIST_TERMS` + `check_blocklist` semantics ported
verbatim from `legal_scraper.py` (12 terms, same order, lowercased
title + space-joined tags, substring, first term reported) in
`src/blocklist.rs`.

## Gates run (sandbox, cargo 1.98.1, offline registry)

- `cargo build` (dev + release): PASS
- `cargo test`: **57 passed, 0 failed** — 8 unit (blocklist/media)
  + 20 validation + 29 adversarial integration
- `cargo clippy --all-targets -- -D warnings`: PASS (clean)
- `cargo fmt --check`: PASS (clean)
- Served `tools/list`: 8 tools, every `inputSchema` has explicit
  `properties`, every tool carries all four behaviour annotations;
  payload 9,908 bytes ≈ 2,477 tokens by the bytes/4 estimate —
  inside the Rule 1 ≤2,500 budget by that (crude) measure; a real
  tokenizer measurement belongs to the S4 EFLO record.

## §7 S1 gate checklist

- Contract gate (Inspector + rose + phlow, `local-mcp-contract-testing`):
  **NOT RUN in the sandbox** — deferred with the primo mirror step,
  like the Diver gate (headless nvim on primo; not run, not claimed).
  Sandbox-side substitutes that did run: tools/list surface contract
  test + a stdio handshake/call smoke test against the real binary.
- Extraction on unscreened work MUST refuse: PASS (`screen_refusal`,
  adversarial test).
- Extraction on excluded work MUST refuse: PASS (`screen_refusal`).
- Extraction without a passing pre-flight MUST refuse: PASS
  (`refusal_not_screened`: missing / unknown / cross-work / failed
  pre-flight all covered).
- School-coded fixture MUST screen out with a clean title: PASS
  (`school_coded` evidence flag test; blocklist-title test beside it).

## Interpretation calls (flagged, not hidden)

1. **Segment annotation tension.** §4 annotates
   `neemu_segment_scene` read-only while its description emits
   records. Resolved: segmentation is a pure deterministic function
   of the registered manifest; scene ids are content-derived and
   stable, the metadata record is returned in the result, and
   nothing is persisted — so the read-only annotation is honest.
2. **Extraction measurement input.** S1's face/dialogue tools mint
   records from caller-supplied measurements (grid-read per
   `local-face-state-measurement`; transcription per §3.2) and do
   the validation/record work server-side. The server has no eyes
   of its own in S1; automated perceptual extractors are model
   stages (the §10/S7 lane), not silently faked here. Tool
   descriptions state this.
3. **Delta 14 in S1.** C2PA-manifest presence is checked from media
   bytes; watermark detection is recorded as `unavailable_in_s1`
   and `integrity_clash` is only ever asserted when both checks
   ran — in S1 it is therefore always false, with the check states
   on the record.
4. **Malformed arguments** (missing/invalid fields, unknown F-code,
   oversized text) return JSON-RPC `-32602` invalid params; the
   §5 typed kinds are reserved for gate refusals as execution
   results, per Rule 6's split.
5. **Record ids** hash the canonical record body excluding
   `record_id` and `created_at`, keeping §7's byte-determinism for
   deterministic stages while still stamping creation time.

Nothing in the spec was found unbuildable as written for S1.
No commits, no pushes.
