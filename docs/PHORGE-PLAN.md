# neemu — media extraction for animation and voice — evaluation and modular plan (D-1261)

**Status: DRAFT for the Owner's gate. Nothing here is built, and
nothing in this repository was changed except this file.** Written
2026-11-10. Gate numbers are proposals; a plan never declares a
keeper.

**Evidence basis.** Read at `6b8290a` (2026-11-04; 5 commits,
56 tracked files). **Read, not run**: no build, test or
playthrough was executed for this plan, so every state claim below
is the repository's own statement unless marked *seen* (a file was
opened and the fact is in it).

**The template referred to** is the studio template, `phorge`
repo, `docs/STUDIO-TEMPLATE.md` (stages T0–T9; a ❄ freeze point is
where a proposal becomes hashed data). The quality target is the
estate's bar library — all seven families — measured, when the
instrument exists, by the motion-study instrument (fathom plan
§6). No title is claimed to be at the bar by this document.

## What it is

An MCP server that turns screened local source works into
structured production records — face observations, transcribed
dialogue, act records, scene understanding, voice performances
packaged as conditioning cues for the sound server. Per the README
it does not animate, model or synthesize, and never fetches from
the web.

## Where it stands

| Fact | Basis |
|---|---|
| Rust crate with tests, spec, gate report, two stage reports | seen |
| Roadmap boxes: 23 done / 0 open | seen |
| Live in this session with 2 tools (list records, export) | session |

## How the studio template applies

A **T2 (Study)** seat beside fathom: fathom measures motion and
systems; neemu characterizes performance (faces, dialogue, voice).
The template needs the boundary written down so the two do not
both claim video: motion records come from the fathom instrument;
neemu's records reference a motion record by hash where they
describe the same clip.

**Voice cues — ruled (D-1265).** A voice cue is an **input, not a
measurement**: it conditions synthesis. It therefore inherits the
source performance's per-character voice lock (Aria discipline),
carries `origin_lane` + `content_label` from birth, and never crosses
projects or lanes detached from that record.

## Extraction lessons it consumes

It produces them. It should emit the estate lesson-record shape
(`origin_lane` + `content_label` + `evidence`).

## Modules

- **U1 — Boundary note with fathom** (who measures what; hash links).
- **U2 — Lesson-record output shape.**
- **U3 — Enforce the voice-cue ruling**: a cue record without its
  voice-lock reference and labels does not construct; an export of a
  cue to another project or lane is refused. Tests for both.
- **U4 — Face-state reads joined to timing charts** (after the
  instrument): reaction legibility needs both.

**Pick-up order.** U3 first, U1, U2, U4.
