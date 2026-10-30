# Roadmap — neemu (mcp) — v0.2 → v1.0
<!-- bloch:roadmap schema=1 kind=mcp lane=general baseline=0.2.0 target=1.0.0 -->

How to work this roadmap (any agent): run `bloch task start R-xxx --agent <you>` before working a task, `bloch task done R-xxx` when you believe it is complete — done runs every `verify:` command and flips the task green only if all pass; there is no override. `bloch task fail R-xxx --error "..."` records a failure. Fixes learned the hard way land in `.ai/memory/lessons.md` via the inbox/promote flow and are surfaced in every HANDOFF.

Curated 2026-10-28 (D-313 Phase 0 — NTRanch four → 1.0 program): baseline is the landed 0.2.0 tree (22ffe7d — post-waiver gate + S3 gate-passed). The 1.0 path per the fleet gap list: G-N1 gate report on file, G-N4 EFLO record closure, 1.0.0 bump, publish 1.0.0. Note the landed tree carries NO README/CHANGELOG/EFLO file — Phase 5 is genuinely open work, not a formality.

## Phase 1 — Baseline truth
- [x] R-101 Build is green from a clean checkout
  - verify: cargo build
  - evidence: 1 verify commands, all exit 0 at 2026-10-29T04:50:02Z
- [x] R-102 Existing tests pass at the baseline
  - verify: cargo test
  - evidence: 1 verify commands, all exit 0 at 2026-10-29T04:50:04Z
- [x] R-103 Repo identity, baseline version, and this roadmap are recorded
  - verify: test -f .ai/roadmap.md
  - evidence: 1 verify commands, all exit 0 at 2026-10-29T04:50:04Z

## Phase 2 — Core complete
- [x] R-201 Tool surface is contracted: every tool has a complete schema and annotations from birth
  - verify: cargo test
  - evidence: 1 verify commands, all exit 0 at 2026-10-29T04:50:05Z
- [x] R-202 Tool count stays within the declared budget (<= 25 tools)
  - verify: cargo test
  - evidence: 1 verify commands, all exit 0 at 2026-10-29T04:50:06Z
- [x] R-203 Typed error and refusal behaviour is implemented in server code, never left to client hints
  - verify: cargo test
  - evidence: 1 verify commands, all exit 0 at 2026-10-29T04:50:07Z

## Phase 3 — Tests and gates
- [x] R-301 Validation tests cover the happy path end to end (half the test split)
  - verify: cargo test
  - evidence: 1 verify commands, all exit 0 at 2026-10-29T04:50:07Z
- [x] R-302 Adversarial tests cover malformed, oversized, and hostile inputs (half the test split)
  - verify: cargo test
  - evidence: 1 verify commands, all exit 0 at 2026-10-29T04:50:07Z
- [x] R-303 Refusal and typed-error behaviour is covered by tests, including oversized payloads
  - verify: cargo test
  - evidence: 1 verify commands, all exit 0 at 2026-10-29T04:50:08Z

## Phase 4 — Hardening
- [x] R-401 MCP Inspector conformance pass is recorded as evidence
  - verify: cargo test
  - evidence: 1 verify commands, all exit 0 at 2026-10-29T04:50:11Z
- [x] R-402 Client matrix is verified with at least two real clients
  - verify: cargo test
  - evidence: 1 verify commands, all exit 0 at 2026-10-29T04:50:11Z
- [x] R-403 Declared-vs-detected access (EFLO) check passes: no tool exceeds its annotations
  - verify: cargo clippy -- -D warnings
  - evidence: 1 verify commands, all exit 0 at 2026-10-29T04:50:20Z
- [x] R-404 Registry/release packaging builds from a clean checkout
  - verify: cargo build
  - evidence: 1 verify commands, all exit 0 at 2026-10-29T04:50:20Z

## Phase 5 — Docs
- [x] R-501 README documents install, usage, and verification
  - verify: test -f README.md
  - evidence: 1 verify commands, all exit 0 at 2026-10-29T05:08:27Z
- [x] R-502 CHANGELOG records every user-facing change
  - verify: test -f CHANGELOG.md
  - evidence: 1 verify commands, all exit 0 at 2026-10-29T05:08:27Z
- [x] R-503 Tool reference documents every tool, schema, annotation, and error kind
  - verify: test -f README.md
  - evidence: 1 verify commands, all exit 0 at 2026-10-29T05:08:27Z
- [x] R-504 EFLO record closed in-tree (G-N4): EFLO.md states the 6,139 real-tokenizer measure against the 20-tool surface and the D-147 waiver terms in the file itself
  - verify: test -f EFLO.md
  - evidence: 1 verify commands, all exit 0 at 2026-10-29T05:08:27Z
- [x] R-505 Post-waiver gate report on file (G-N1): full WS2-W5 gate re-run against the in-tree rmcp build — suite both machines, tools/list real-tokenizer measure, Diver gate — written as a GATE PASS report in this repo
  - verify: test -f GATE-REPORT.md
  - evidence: 1 verify commands, all exit 0 at 2026-10-29T05:10:49Z

## Phase 6 — Release v1
- [x] R-601 Version is bumped to 1.0.0 and the changelog records the release
  - verify: grep -q 'version = "1.0.0"' Cargo.toml
  - evidence: 1 verify commands, all exit 0 at 2026-10-29T05:10:49Z
- [x] R-602 Clean-environment install and run is verified
  - verify: cargo build
  - evidence: 1 verify commands, all exit 0 at 2026-10-29T05:10:52Z
- [x] R-603 Release artefacts are built from a clean checkout and checksummed
  - verify: cargo build
  - evidence: 1 verify commands, all exit 0 at 2026-10-29T05:10:53Z
- [x] R-604 SPEC S3 delivery/conditioning stage complete: S3 tools consume torori's frozen conditioning schema v1 (e0df698d…); SPEC §7 S3 battery + torori-shaped bundle round-trip green (landed in 22ffe7d)
  - verify: cargo test
  - evidence: 1 verify commands, all exit 0 at 2026-10-29T04:50:08Z
- [x] R-605 neemu 1.0.0 published to crates.io under the NTRanch account
  - verify: curl -sf -H "User-Agent: bloch-roadmap" https://crates.io/api/v1/crates/neemu/1.0.0 -o /dev/null
  - evidence: 1 verify commands, all exit 0 at 2026-10-29T05:11:24Z

## Phase 7 — Post-1.0 canon follow-ups
- [ ] R-701 N-code face-state resolution (D-326 follow-up): N-01..N-06 resolve server-side through the F-code lookup (extract_faces + act face timelines), N-05 as named variant only, lane labels carried on the resolution snapshot; the 32-page per-call bound (MAX_PAGES_PER_CALL) named in tool descriptions + README
  - verify: cargo test
