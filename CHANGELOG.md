# Changelog

All notable changes to neemu. Versions follow the estate MCP
1.0 program: the 0.2.0 tree was gate-passed and landed first;
1.0.0 closes the in-tree records (G-N1 gate report, G-N4 EFLO
record) and publishes the gated surface.

## [1.0.0] — 2026-10-28

### Added

- `README.md` — install, usage, verification, and the full
  15-tool reference (schemas, annotations, error kinds).
- `GATE-REPORT.md` — the post-waiver + S3 gate record (G-N1) on
  file in-tree, including the D-116 utilization evidence (the
  torori round-trip consumer session).
- `EFLO.md` — the EFLO record closure (G-N4): the 6,139
  cl100k_base real-tokenizer measure for the 15-tool surface
  and the D-147 waiver terms, with the Owner's D-215 acceptance
  of 6,139 as the standing 1.0 record.
- `server.json` — MCP Registry metadata for
  `io.github.ntranch/neemu`.

### Changed

- Version bumped 0.2.0 → 1.0.0 (`Cargo.toml` + `Cargo.lock`).
  No tool-surface, schema, or behaviour change from 0.2.0: the
  1.0.0 release is the gated 0.2.0 surface with its records
  closed in-tree.

## [0.2.0] — 2026-10-28 (landed, commit `22ffe7d`)

The first landed release: the post-waiver gate-passed tree plus
the SPEC S3 delivery/conditioning stage.

### Added — S3 voice / delivery stage

- Four new tools (15 total), all write/idempotent with honest
  annotations from birth: `neemu_analyze_voice`,
  `neemu_assess_voice_clonability`, `neemu_derive_voice`,
  `neemu_emit_torori_cue` (`src/s3.rs`, `src/accents.rs` +
  `src/accents_seed.json`; the AV lane in `src/media.rs`).
- Media lane: registration accepts audio (wav/mp3/flac/ogg) and
  video (mp4/webm/mkv) alongside images; AV manifest entries add
  `media_kind`, `duration_seconds`, `sample_rate`, `channels`
  from real header facts.
- `neemu_analyze_voice`: server-measured WAV PCM envelope
  (peak, RMS, clipped fraction, pause structure, breath-event
  candidates, speech-seconds estimate); caller features
  range-checked against the frozen ranges; limits echoed in
  `known_limitations`.
- `neemu_assess_voice_clonability`: two-axis verdict (SPEC
  §3.10). A missing or incomplete ConsentArtifact is a typed
  `rights_artifact_missing` refusal — never a pass.
  Assessments are immutable and supersede by reference.
- `neemu_derive_voice`: refuses on an unassessed/NOT-CLONABLE
  voice, an identity adaptation plan, a target identity equal
  to the source speaker, and a dead or non-commercial engine
  license — all `clonability_refusal`; an over-class request
  refuses as `refusal_gate_tier`. Outputs carry the derivation
  watermark.
- `neemu_emit_torori_cue`: writes a real torori bundle
  directory (`bundle.ron`) under the registry exports against
  torori's frozen conditioning schema v1 (conditioning sha256
  `e0df698d…e6610`, cue-bundle sha256 `f423f883…7138`);
  out-of-range values refuse as `conditioning_refusal` —
  refused, never clamped. Proven by the torori round-trip:
  torori's recomputed content hash equals neemu's emitted hash
  (`624d959354c8cdbc5d10a2967a7afa06fe86ba906f1064d026b7141a4b70e218`).
- Accent registry (SPEC §3.12): all 21 seeded profiles, each
  with phonetic features, prosody, expected confusions, and a
  non-empty reaction set — none speech-only.
- Two new refusal kinds: `clonability_refusal`,
  `conditioning_refusal`.

### Changed — WS2 rmcp swap (fleet standardization, D-113)

- Framing moved to the official Rust MCP SDK, `rmcp =3.8.0`,
  modern protocol era only (2026-07-28): sessions open with
  `server/discover`, every request carries its protocol version
  in `_meta`, and legacy `initialize` contact is refused with
  the typed `-32022` unsupported-version error.
- Exactly one differing field in the whole tools/list surface
  versus the pre-swap baseline: `neemu_export`'s
  `annotations.readOnlyHint` true→false — the D-147-commissioned
  honesty correction (export writes a bundle file), verified
  live.

### The 0.2.0 baseline (S1 + S2, landed in the same tree)

- S1: registry + screen/tier gates + image lane —
  `neemu_register_work`, `neemu_validate_ingest`,
  `neemu_screen_work`, `neemu_segment_scene`,
  `neemu_extract_faces`, `neemu_extract_dialogue`,
  `neemu_list_records`, `neemu_export`.
- S2: `neemu_enrich` (System-0 advisory enrichment, batch form
  with per-item errors + resume cursor), `neemu_extract_acts`,
  `neemu_understand_scene` (evidence-linked scene
  understanding; an unlinked claim is a typed
  `evidence_refusal`).
- Typed gate errors throughout (SPEC §5): `screen_refusal`,
  `tier_refusal`, `media_unreadable`, `bound_exceeded`,
  `refusal_not_screened`, `evidence_refusal`.
- Audio silence by construction: no audio-output crate in the
  dependency set, no player/device path in the sources —
  verified by test.
