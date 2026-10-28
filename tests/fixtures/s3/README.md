# S3 fixtures — VALIDATED against torori's frozen artefacts

Status: **VALIDATED** (2026-10-28, neemu S3 build). The provisional
brief-§8 draft shape (four JSON files) was replaced by a real
emitted-format sample after the field-by-field diff against
torori's frozen T3 artefacts (W5 §5 checklist):

- Conditioning schema: `torori-build-20261027/data/schemas/conditioning.schema.json`,
  sha256 `e0df698dd61c9ad36b9832c8d9e2a79d2d86ea19292e5893d9cf621a1e0e6610`.
- Cue-bundle schema: `torori-build-20261027/data/schemas/cue-bundle.schema.json`,
  sha256 `f423f88305d46b41cc2c1863a5e32049d7f5e62a8ae27270117ac5c737177138`.
- Code truth: torori `src/conditioning.rs` (validation ranges,
  required set `[text, identity_ref, breath_bracket,
  determinism_class]` — the provisional fixture's required set
  wrongly carried `grade` and omitted `determinism_class`) and
  `src/bundles.rs` (the `bundle.ron` directory format and the
  canonical content-hash recipe).

`bundle-v1/bundle.ron` is byte-shaped exactly as
`neemu_emit_torori_cue` writes it (same writer discipline, same
content-hash recipe); `bundle-v1/fixture.json` records the cue
values the hash was computed over so `tests/s3_readiness.rs` can
re-derive and compare. The live round-trip — this fixture class
ingested by torori's own `torori_bundle_validate` /
`torori_bundle_ingest` — is evidenced in the S3 report
(`~/workspace/neemu-1.0-20261028/`).
