# Neemu — Gate Report (G-N1: post-waiver gate + S3 gate, on file)

**Tree:** this repository, crate `neemu`. The 0.2.0 surface was
gated in two stages before landing (commit `22ffe7d`): the
post-waiver gate on the rmcp-swapped tree before any S3 edits,
and the S3 final gate on the completed 15-tool tree. This file
adapts both records into the repo verbatim in substance; the
1.0.0 re-gate on the bumped tree follows in §3.

## 1. Post-waiver gate (11-tool surface) — GATE PASS, all legs green

| Leg | Result |
|---|---|
| Sandbox `cargo test` | **128/128 PASS** (22 lib unit + 29 adversarial + 23 adversarial_s2 + 7 protocol + 11 s3_readiness + 20 validation + 16 validation_s2) |
| Primo `cargo test --release` | **128/128 PASS** (9 suites ok) |
| Clippy `--all-targets -- -D warnings` | **Clean** — resolved clippy **0.1.100 (f7575a9da8)**, toolchain pin nightly-2026-09-25 (cargo 1.100.0-nightly 98a09e7e7), `RUSTC_WRAPPER=`, `CARGO_BUILD_JOBS=2` |
| `cargo fmt --check` | **Clean** |
| EFLO (live tools/list) | **4,201 cl100k_base ≤ 4,600 — PASS under the D-147 waiver** (o200k_base 4,440, informational); 11 tools, 18,906 bytes |
| Surface parity vs pre-swap baseline | Exactly **one** differing field in the whole surface: `neemu_export.annotations.readOnlyHint` true→false — the D-147-commissioned honesty correction, verified live |
| Mirror integrity | Sandbox ≡ primo aggregate md5 **3e4786b7d49b7cb62390c49cddc0002a** (primo carries 3 extra primo-side helper files excluded from the hash: `d116_session.py`, `run-gate-modern.sh`, `run-diver-gate-modern.sh`) |
| Diver gate (primo mirror, staged config) | **11/11 files PASS, total_diagnostics=0** |

Waiver terms applied exactly as written (D-147, Owner
2026-10-27 16:34): NEEMU ONLY tools/list cap 4,600 tokens
(cl100k_base), standing payload 4,201; the fleet ≤2,500 budget
unchanged for every other server; revisit triggers — a fourth
heavy nested tool, an always-on consumer, or payload growth
past the cap — return the question to the Owner. The EFLO
record is closed in-tree in `EFLO.md` (G-N4).

## 2. S3 final gate (15-tool surface) — S3 BUILT AND GATED

Four new tools, all write/idempotent, annotations honest from
birth: `neemu_analyze_voice`, `neemu_assess_voice_clonability`,
`neemu_derive_voice`, `neemu_emit_torori_cue` (15 tools total),
consuming torori's frozen conditioning schema v1 (conditioning
sha256 `e0df698d…e6610`, cue-bundle sha256 `f423f883…7138`).

| Leg | Result |
|---|---|
| Sandbox `cargo test` (debug + release) | **138/138 PASS** (was 128; +21 S3-era tests, −11 replaced readiness tests) |
| Primo `cargo test --release` | **138/138 PASS** |
| Clippy `--all-targets -- -D warnings` | **Clean** — clippy 0.1.100 (f7575a9da8), pin nightly-2026-09-25 |
| `cargo fmt --check` | **Clean** |
| Diver gate (primo mirror) | **15/15 files PASS, total_diagnostics=0** |
| Mirror integrity | Sandbox ≡ primo; per-file md5 list aggregate **8cbad787779cb2a7f767fc211def85b7** (same 3 primo-side helpers excluded) |
| Torori round-trip (G-N2) | **ALL PASS** — see §2.1 |
| Silence rule | **Verified by construction + test** — no audio-output crate in the dependency set, no player/device path in sources (`silence_rule_no_audio_output_path`); all verification by file metrics |

### 2.1 Torori round-trip (SPEC §7 S3 gate / G-N2) — ALL PASS

A real neemu session (register → preflight → screen → analyze →
assess CLONABLE → derive → dialogue → emit) produced a bundle;
the real torori binary then, with **no hand-editing**:

- `torori_bundle_validate` → `{"valid": true, "cues": 1, "pairs": 0, "assets": 0}`
- `torori_bundle_ingest` → promoted into torori's bundle store
- ingest replay → `idempotent: true`
- **torori's recomputed content_hash == neemu's emitted hash**
  (`624d959354c8cdbc5d10a2967a7afa06fe86ba906f1064d026b7141a4b70e218`) —
  torori re-derives the hash from the parsed RON, so equality
  proves the replication and the final-provenance-link rule
  simultaneously.

### 2.2 Utilization evidence (D-116)

The torori round-trip in §2.1 is a real consumer session on
record: neemu's emitted bundle was validated, ingested, and
replay-checked by the production torori binary — a downstream
estate consumer exercising neemu's S3 delivery surface
end-to-end, not a synthetic probe. This roadmap carries no
separate utilization item; the evidence is filed here per the
fleet D-116 mandate.

### 2.3 EFLO at S3

Live tools/list on the 15-tool surface: **6,139 cl100k_base**
(o200k 6,458; 27,769 bytes) — 1,539 over the D-147 waived cap
of 4,600, reported as-is at S3 gate time (revisit trigger #3).
Owner ruling D-215 (2026-10-28) accepts 6,139 as the standing
1.0 record; see `EFLO.md`.

## 3. 1.0.0 re-gate

Re-gate run 2026-10-28 on the version-bumped (1.0.0) tree —
docs + version only, no code change from the gated 0.2.0
surface. Toolchain pin nightly-2026-09-25 on both machines,
`RUSTC_WRAPPER=`, `CARGO_BUILD_JOBS=2`.

| Leg | Result |
|---|---|
| Primo `cargo test` (in-tree) | **138/138 PASS** (11 suites: lib 22, bin 0, adversarial 29, adversarial_s2 23, adversarial_s3 10, protocol 7, s3_readiness 4, validation 20, validation_s2 16, validation_s3 7, doc-tests 0) |
| Sandbox `cargo test` (staging copy) | **138/138 PASS** (same suite split) |
| Clippy `--all-targets -- -D warnings` | **Clean both machines** — resolved clippy **0.1.100 (f7575a9da8 2026-09-24)** |
| `cargo fmt --check` | **Clean both machines** |
