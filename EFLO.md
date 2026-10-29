# EFLO record — neemu tools/list payload size

EFLO = the measured token size of the server's live `tools/list`
payload under a real tokenizer. This file is the in-tree record
closure for program item G-N4.

## Standing 1.0 measure (15-tool surface)

Measured live on the S3-final tree (the surface released as
1.0.0; the 1.0.0 bump changes no tool definition):

| Measure | Value |
|---|---|
| Tools | 15 |
| Payload | 27,769 bytes |
| **cl100k_base** | **6,139** |
| o200k_base (informational) | 6,458 |

Per-tool cl100k_base:

| Tool | Tokens |
|---|---|
| `neemu_register_work` | 327 |
| `neemu_validate_ingest` | 136 |
| `neemu_screen_work` | 262 |
| `neemu_enrich` | 601 |
| `neemu_segment_scene` | 244 |
| `neemu_extract_faces` | 565 |
| `neemu_extract_dialogue` | 301 |
| `neemu_extract_acts` | 720 |
| `neemu_analyze_voice` | 514 |
| `neemu_assess_voice_clonability` | 413 |
| `neemu_derive_voice` | 597 |
| `neemu_emit_torori_cue` | 386 |
| `neemu_understand_scene` | 648 |
| `neemu_list_records` | 236 |
| `neemu_export` | 187 |
| **Total** | **6,139** |

## History of the measure

- **Pre-S3 (11 tools):** 4,201 cl100k_base (o200k_base 4,440;
  18,906 bytes) — inside the D-147 waived cap, recorded in the
  post-waiver gate report (see `GATE-REPORT.md`).
- **S3 (15 tools):** the four S3 tools add 1,910 (analyze 514,
  assess 413, derive 597, emit 386) and `neemu_list_records`
  grew 208 → 236 with the S3 record kinds, bringing the surface
  to 6,139 — 1,539 over the D-147 waived cap of 4,600. The S3
  surface was not shrunk to fit; the overage was reported as-is
  as D-147 revisit trigger #3, fired by the mandated S3 build
  itself.

## The D-147 waiver terms (Owner, 2026-10-27 16:34)

- NEEMU ONLY: tools/list cap **4,600 tokens (cl100k_base)**;
  the pre-S3 standing payload was 4,201.
- The fleet ≤2,500 budget is unchanged for every other server.
- Revisit triggers: a fourth heavy nested tool, an always-on
  consumer, or payload growth past the cap — any returns the
  question to the Owner.

## Closure (G-N4)

By Owner ruling **D-215 (2026-10-28 06:45)**, the 6,139
cl100k_base measure is **accepted as the standing record for
1.0**: the 15-tool surface ships at 6,139, above the D-147
waived cap, on the Owner's word. The D-147 terms above remain
the record of the waiver under which the 11-tool surface was
gated; the fleet ≤2,500 budget for every other server is
unchanged. Any further payload growth past 6,139 returns the
question to the Owner under the same revisit logic.
