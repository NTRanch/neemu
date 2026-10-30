# neemu

Neemu MCP — media extraction for animation and voice. Neemu turns
screened source works — doujin page sequences, video, and images
with sound — into structured production records that the estate's
builders consume: face observations, transcribed dialogue, act
records, scene understanding, and voice performances packaged as
conditioning cues for torori, which does the actual synthetic
generation.

Neemu extracts, characterizes, understands, and translates. It
does NOT animate, model, or synthesize, and it never fetches from
the web: it receives local media plus a registry entry, nothing
else.

- **Protocol:** MCP, modern era only (2026-07-28), served on the
  official Rust MCP SDK (`rmcp =3.8.0`). Sessions open with
  `server/discover`; there is no `initialize` handshake. Legacy
  contact is refused at the stdio front door with the typed
  `-32022` unsupported-version error.
- **Transport:** newline-delimited JSON-RPC over stdio.
- **Surface:** 15 tools (see the tool reference below).
- **Audio silence by construction:** no audio-output crate is in
  the dependency set and no player/device path exists in the
  sources; voice work is measured from file bytes only.

The normative specification is `SPEC.md` in this repository.

## Non-negotiable gates (enforced in server code)

1. **Screen gate (decide()-class).** Every work exists in the
   registry with a determination before any extraction tool will
   touch it. Ingest runs as a separate validate-first step
   (`neemu_validate_ingest`) minting a PreflightRecord, and every
   extraction tool takes that record's ID as a required input — a
   missing or failed pre-flight returns the typed refusal
   `refusal_not_screened`. The determination itself is per-work,
   evidence-based, fail-closed: school-coded, age-stated or
   age-readable as under 18, loli/shota-coded, or ambiguous on any
   of these → EXCLUDED, reason recorded, nothing extracted.
   Enrichment never clears a work and never overrides an EXCLUDE.
2. **Tier gate.** Every record carries its work's provenance tier
   (Tier A analysis / Tier T training-cleared). The export tool
   refuses to emit Tier A material into a training-shaped bundle
   (`tier_refusal`).
3. **Voice likeness rule — determination first, derivation only.**
   Clonability is determined per voice on two axes: **rights**
   (a structured ConsentArtifact; a boolean attestation is not an
   artifact — absent or incomplete is a typed
   `rights_artifact_missing` refusal) and **technical
   sufficiency**. Either axis failing records a NOT-CLONABLE
   verdict. Derivation is itself gated: an identity adaptation
   plan, a target identity equal to the source speaker, an
   over-class request (`refusal_gate_tier`), or a dead /
   non-commercial engine license all refuse.

## Install

From crates.io (NTRanch account):

```sh
cargo install neemu --version 1.0.0
```

From a checkout of this repository:

```sh
cargo build --release
# binary: target/release/neemu
```

## Usage

Run the server over stdio:

```sh
neemu [--home <dir>]
```

Registry root resolution, in order: `--home <dir>`, else
`$NEEMU_HOME`, else `$XDG_DATA_HOME/neemu`, else
`~/.local/share/neemu`. The server opens (creating if needed)
the registry at that root before reading a single frame and
fails fast if it cannot.

A typical session flows through the gates in order:

1. `neemu_register_work` — register a work (media hashed,
   idempotent by content hash; a Tier T claim requires rights
   evidence in the call).
2. `neemu_validate_ingest` — validate-first pre-flight; mints
   the PreflightRecord every extraction tool requires.
3. `neemu_screen_work` — record the screen determination.
4. Extraction and analysis: `neemu_segment_scene`,
   `neemu_extract_faces`, `neemu_extract_dialogue`,
   `neemu_extract_acts`, `neemu_analyze_voice`,
   `neemu_understand_scene`; advisory context via
   `neemu_enrich`.
5. Voice lane: `neemu_assess_voice_clonability`,
   `neemu_derive_voice`, `neemu_emit_torori_cue`.
6. Reads and export: `neemu_list_records`, `neemu_export`.

## Verification

```sh
cargo test                              # full suite (138 tests at 1.0.0)
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

The gate record for this release is `GATE-REPORT.md`; the
tools/list payload size record (EFLO) is `EFLO.md`.

## Tool reference

All 15 tools ship a full object `inputSchema` with an explicit
`properties` key, an `outputSchema`, and behaviour annotations
from birth. Unless noted, annotations are `readOnlyHint: false`,
`destructiveHint: false`, `idempotentHint: true`,
`openWorldHint: false`. The two read tools
(`neemu_segment_scene`, `neemu_list_records`) carry
`readOnlyHint: true`.

| Tool | Required arguments | What it does |
|---|---|---|
| `neemu_register_work` | `title`, `source`, `media_dir`, `tier_claim` | Register a page-sequence work. Hashes all media; idempotent by content hash. Tier T requires rights evidence in the call. Media may be images (png/jpg/jpeg/webp/gif) and, since S3, audio (wav/mp3/flac/ogg) and video (mp4/webm/mkv). |
| `neemu_validate_ingest` | `work_id` | Validate-first pre-flight (SPEC §3.16): access/hash/format/bounds re-check + blocklist floor; mints the PreflightRecord extraction requires. |
| `neemu_screen_work` | `work_id`, `evidence` | Record the screen determination (SPEC §2.1). Fail-closed: blocklist hit, any risk flag, or no positive adult evidence excludes, reason recorded. |
| `neemu_enrich` | — (single form takes per-work fields; batch form takes `items[]`) | System-0 enrichment (SPEC §3.15): record caller-supplied candidate source data as an advisory EnrichmentBundle. Never a determination; never clears or overrides a screen. Batch form: `items[]` with per-item errors + resume cursor. |
| `neemu_segment_scene` | `work_id`, `preflight_id` | Deterministic page segmentation into scenes (boundary: dimension change or max-pages) + file-metric metadata. Read-only; stable scene ids. Paginated. |
| `neemu_extract_faces` | `work_id`, `preflight_id`, `page_start`, `page_end` | Mint FaceObservation records for a page range from caller grid-read / model-estimate measurements. Validates 8 channels, face code (F-01..F-12 or ratified N-01..N-06, D-326; N-05 resolves as a named variant) or new-candidate, basis, lineage caps. Page bound: `MAX_PAGES_PER_CALL` = 32 pages per call; oversize ranges return a declared partial with a continuation range. |
| `neemu_extract_dialogue` | `work_id`, `preflight_id`, `page_start`, `page_end` | Mint DialogueLine records for a page range from caller-transcribed lines (`[unclear]` = illegible, never guessed). Word-level nesting + `known_limitations`. Page bound: `MAX_PAGES_PER_CALL` = 32 pages per call; oversize ranges return a declared partial with a continuation range. |
| `neemu_extract_acts` | `work_id`, `preflight_id`, `scene_id`, `acts` | Mint ActRecord(s) for one deterministic scene (SPEC §3.3): ACT code resolved against the registry seed (unknown codes enter as candidates, unmapped codes labelled, none dropped). |
| `neemu_analyze_voice` | `work_id`, `preflight_id`, `segments` | Analyse voice segments of a cleared work's audio/video (SPEC §3.4): server-measured WAV envelope features (duration, pauses, breath candidates, clipping) + caller transcript/features range-checked against the frozen ranges. Transcripts are caller-side (pinned ASR). |
| `neemu_assess_voice_clonability` | `work_id`, `preflight_id`, `voice_record_ids` | Two-axis clonability verdict over one speaker's VoicePerformanceRecords (SPEC §3.10). Rights axis: a complete ConsentArtifact is a hard gate — absent or incomplete is a typed `rights_artifact_missing` refusal. Technical axis uses spec constants recorded on every record (min speech 20.0 s, max clipped fraction 0.01, min SNR 15.0 dB; an unmeasured record fails the axis). Assessments are immutable, supersede by reference. |
| `neemu_derive_voice` | `work_id`, `preflight_id`, `assessment_id`, `adaptation`, `engine`, `target_identity`, `design_targets` | Derive a NEW estate VoiceIdentity from a CLONABLE assessment (SPEC §3.11): adaptation plan mandatory (an identity plan refuses), derivation class checked against the assessment's rights-evidence class, engine license fails closed. Output carries the derivation watermark. |
| `neemu_emit_torori_cue` | `work_id`, `preflight_id`, `target_voice`, `cues` | Emit a torori cue bundle (`bundle.ron`, frozen format) from dialogue lines (SPEC §3.5): delivery normalised to torori's frozen conditioning schema, ranges refused never clamped, `target_voice` REQUIRED and must resolve to an estate VoiceIdentity. |
| `neemu_understand_scene` | `work_id`, `preflight_id`, `scene_id`, `beats`, `machinery`, `dynamics` | Mint a SceneUnderstanding over one deterministic scene (SPEC §3.6): beats + machinery + dynamics, every claim carrying evidence refs that must resolve to this work's records — an unlinked claim is a typed refusal. Records are model analysis, labelled advisory. |
| `neemu_list_records` | `work_id` | Bounded paginated record listing by kind, server-computed counts. Gate records stay listable on excluded works (refusal trail); extraction kinds on a non-cleared work refuse. |
| `neemu_export` | `work_id`, `bundle_kind` | Tier-checked bundle export of a cleared work's records (file emitted; no registry change). Training bundles refuse Tier A material (`tier_refusal` names the blocker); analysis accepts either tier. |

## Error kinds

Gate refusals from a valid tool call are tool results with
`isError` and a typed `error.kind` (SPEC §5) — never protocol
errors. Malformed arguments are JSON-RPC invalid params
(`-32602`); legacy-era contact is refused with `-32022`.

| Kind | Meaning |
|---|---|
| `screen_refusal` | Extraction attempted on a work whose screen determination is not `cleared`. |
| `refusal_not_screened` | Missing, mismatched, or failed pre-flight record for an extraction call. |
| `tier_refusal` | Tier gate: Tier T claimed without registration evidence, or Tier A material requested in a training-shaped export. |
| `media_unreadable` | A media file cannot be read or its header facts cannot be established. |
| `bound_exceeded` | A declared bound (pages, segments, records, cues, text bytes) was exceeded. |
| `evidence_refusal` | An understanding claim whose evidence refs are missing or do not resolve to records of the same work. |
| `clonability_refusal` | A derivation refused on the rights/clonability gate: unassessed or NOT-CLONABLE voice, identity adaptation plan, target identity equal to the source speaker, or dead/non-commercial engine license. Detail kinds include `rights_artifact_missing` (no complete ConsentArtifact) and `refusal_gate_tier` (requested class above the evidence class). |
| `conditioning_refusal` | A conditioning value outside torori's frozen schema range at cue-emit time — refused, never clamped. The emit-specific detail kind `missing_target_voice` names an unresolvable required `target_voice`. |

## License

Apache-2.0.
