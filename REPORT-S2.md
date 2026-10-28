# Neemu S2 — Build Report (acts + understanding + System-0 enrichment)

- **Stage:** S2 per SPEC §7, on top of the gated S1 (REPORT-S1.md).
- **Crate:** `neemu` 0.1.0, `~/workspace/conte-mcp-20261027/`. Deps
  unchanged: serde / serde_json / sha2 only. Hand-rolled
  newline-delimited JSON-RPC over stdio, as S1.
- **Served surface:** 11 tools (S1's 8 + 3 new). S3+ tools remain
  absent, not stubbed.
- **Built:** 2026-10-27, sandbox. No commits, no pushes, no web
  fetching. SPEC.md / REVIEW-20261027.md / research/ / the SPEC-v1
  backup were not edited.

## New tools

### `neemu_extract_acts` (§3.3, §6)

Full S1 gate chain (registered → determination `cleared`, else
`screen_refusal` → passing work-matching pre-flight, else
`refusal_not_screened`), then a deterministic scene resolution:
`scene_id` must resolve against `Neemu::segment_pages` under the
caller's `max_pages_per_scene` (default 24) — an unknown id is an
invalid-arguments error, never a minted scene (Δ12).

Each act mints one immutable, content-addressed `act_record`
(prefix `act_`): act_code; a `registry` snapshot (see below);
participants (slot + role); contact pairs; pose track (label,
joint_configuration, support, orientation); motion (beats_per_cycle,
tempo_curve, intensity_curve, entry/exit); face timeline; sfx
hooks; props; the scene locus + per-page input hashes. Stamped
`basis: "grid-read"` — like S1's faces/dialogue, S2 records are
caller-measured; the server has no eyes of its own.

- **Contact pairs** validate against the closed D-076 vocabulary:
  colliders `sphere | capsule | surface`, compliance classes
  `hard_non_penetration | neighbour | attachment |
  falloff_weighted`. Unknown terms are argument errors.
- **Face timeline** entries accept a canonical F-code or a
  FaceObservation record id of the same work (verified to be a
  `face_observation` record). A dangling ref is a typed
  `evidence_refusal`.
- **Registry discipline:** a known family code records `mapped`; a
  known non-family code records its disposition and `unmapped`; an
  unknown code mints anyway as `registry_status: "candidate"`,
  `mapping: "unmapped"`. Nothing is dropped; nothing is presented
  as canon that is not. The response carries server-computed
  `unmapped_act_codes` / `candidate_act_codes` lists.

### `neemu_understand_scene` (§3.6, Δ12)

Same gate chain + scene resolution. Mints one `scene_understanding`
record (prefix `und_`), server-stamped `basis: "model-estimate"`,
`advisory: true`. Contents: beats (type from the closed §3.6
ten-word vocabulary; page locus required to lie **inside** the
scene's pages), machinery (mechanism from the §3.6 vocabulary,
stage model, and the reset/wakener present-or-absent finding), and
dynamics (roles, who_directs, who_responds). **Every claim block —
each beat, machinery, and dynamics — must carry ≥1 evidence ref,
and every ref must resolve to a record of the same work**; missing
or dangling refs are the typed `evidence_refusal`. The record
carries the sorted union as `links`. A test proves segmentation is
byte-identical before and after understanding calls: the layer
attaches, it never creates/merges/splits.

### `neemu_enrich` (§3.15, Δ17–Δ23)

Mints `enrichment_bundle` records (prefix `enrich_`), one per work
per call. **Registered works only** — no screen state is required
and none is touched: enrichment sits between census and screening
(Δ17), so an unscreened or excluded work may carry a bundle. The
only work-record mutation is the `enrichment_bundle_id` pointer;
the determination field is never read for gating and never
written. Neemu never fetches: callers supply candidate source
records, neemu validates, applies the corroboration discipline,
and records.

Per source: `{source, match_key, match_evidence, retrieved_at,
basis, similarity?, tags?, escalation_flags?, translated?,
payload}` stored as a sub-record with `untrusted_text: true` and a
server-computed `citable` verdict.

- **Corroboration (Δ18)** is computed server-side and the rule text
  rides on every bundle: two independent sources pin identity, one
  suffices to escalate, silence never cancels escalation,
  enrichment never clears or overrides an EXCLUDE. Identity is
  `pinned` only when ≥2 distinct **citable** sources assert the
  same match_key. Escalation flags = caller-declared flags ∪
  canonical-blocklist hits over candidate tags ∪ version-chain tag
  diffs (Δ22). `queue_hint: "candidate"` is emitted only when
  identity is pinned, citable tags exist, and no escalation flag
  stands — it is advisory text on the bundle, touching no state.
- **Leads, not citations (Δ19):** tags enter `bundle.tags` only
  from citable (resolved-record) sources, each stamped with its
  source + `untrusted_text: true`. Similarity-service sources
  (SauceNAO-class, trace.moe-class, matched by name class) are
  forced to leads even when the caller claims `resolved_record`;
  **any** source claiming resolved with `similarity < 0.90` is
  forced to a lead, with the reason recorded on the sub-record.
- **Grouping (Δ21):** caller `{basis: official_chain |
  heuristic_match | standalone, confidence, group_ref}` recorded
  with `applied = confidence ≥ 0.5` (floor constant recorded on
  the bundle; see interpretation calls).
- **Version chains (Δ22):** per-version tag lists recorded;
  consecutive diffs computed; any change adds an escalation flag.
- **Rate limits (Δ23):** when a source name matches the E-Hentai
  lane, the bundle records the caller's resolver config — search
  3000 ms / API 1250 ms / page 1000 ms, retries 2, Retry-After
  priority. Neemu itself performs no fetching.
- **Translated tags (Δ24):** tags from a `translated: true`
  resolved source are stamped `non_commercial: true` (EhTagTranslation
  CC BY-NC-SA 3.0 CN), and the bundle carries
  `non_commercial_tags: true`.
- **Batches (Δ23):** `items[]` form processes at most
  `MAX_ENRICH_ITEMS_PER_CALL = 4` items per call, **in order**,
  each item's outcome independent (`{work_id, ok, bundle_id |
  error{kind, message}}`); the response declares `partial` and a
  `resume_cursor {next_index}`; `batch_id` is content-derived from
  the item list, so a resumed call recognises its own batch.
- Payload bound: 64 KiB serialized per source payload
  (`bound_exceeded`).

`neemu_list_records` kinds extended: `act_record` and
`scene_understanding` join the extraction kinds (refuse on
non-cleared works); `enrichment_bundle` is listable regardless of
determination (advisory trail, like preflight/screen). Export picks
the new record kinds up through the existing all-records path;
tier rules unchanged.

New bounds constants (lib.rs, S1 style): MAX_ACTS_PER_CALL 16,
MAX_PARTICIPANTS_PER_ACT 8, MAX_CONTACT_PAIRS_PER_ACT 32,
MAX_POSE_KEYS_PER_ACT 32, MAX_BEATS_PER_SCENE 32,
MAX_EVIDENCE_REFS 32, MAX_ENRICH_SOURCES_PER_CALL 16,
MAX_ENRICH_ITEMS_PER_CALL 4, MAX_ENRICH_PAYLOAD_BYTES 65,536,
MAX_VERSIONS_PER_CALL 16, GROUPING_CONFIDENCE_FLOOR 0.5.

## ACT registry seed — provenance

Carrier: `src/acts.rs`, versioned `act-registry/1`, 83 coded
entries + 7 uncoded candidates. Sources (all read directly this
build; nothing invented):

| slice | count | grounded in |
|---|---|---|
| B-canon codes B-01..B-80 | 80 | `~/workspace/ntr-corpus-skeleton/BDSM-TYPES.md` |
| Lab-local codes KISS, M-01, SOLO (no B-code) | 3 | `~/workspace/adult-audio-research-20261026/TORORI-CATALOGUE.md` [ACT] entries, coded in `night-20261026/w6/W6-SPEC.md` §1.1.1 |
| Family grouping (37 families: PEN 10, ORAL 4, DEV 4, IMP 6, RST 8, CONTACT 5) + physics classes P-A/P-B/P-C | — | W6-SPEC §1.1.1, §1.1.2 |
| Non-family dispositions for the other 46 B-codes (context 15, register 11, regime 6, reaction_overlay 4, modifier 7, section 3) | — | W6-SPEC §1.1.3 |
| Stratum links — **only where a source states them**: B-48 → S-18 (primitives `mv.limb.grasp`, `mv.tors.breast-bounce`, `mv.flud.dribble` milk form); M-01 → S-17 + S-18 (primitive `mv.flud.spurt-to-dribble`); B-60 → S-12 | 3 codes | `night-20261026/w6/ATLAS.md` §4 |
| Uncoded candidates (JOI; Liquid events; Sex-toy play, general; Thrusting/friction loop; Vore/swallowing foley; Wet friction/contact layer; the second masturbation entry W6 merged into SOLO) | 7 | TORORI-CATALOGUE [ACT]; W6-SPEC §1.1.3 (layer/umbrella/foley) |

Mapping rule, applied uniformly: `mapped` ⟺ the entry carries a
W6 family group (§1.1.1) **or** a source-stated stratum. Result:
**38 mapped** — the 37 families (B-48 and M-01 among them, strata
stated as well) plus B-60, a section-disposition code whose S-12
stratum ATLAS states — and **45 coded entries `unmapped`** (the
remaining non-family dispositions), plus the 7 uncoded candidates
`unmapped`. The seed test counts families (37) and strata-linked
codes (exactly B-48, M-01, B-60). The W6 strata
S-01..S-26 are reaction/fluid/hypnosis strata, **not** per-act
strata — no per-act stratum mapping exists in the sources for the
other codes, so none was fabricated.

Per-code provenance strings ride in every entry (`source` field,
embedded in each ActRecord's registry snapshot).

Contact vocabulary grounding: collider kinds from the CONTACT-SPIKE
brief's candidates (XPBD surface samples → `surface`; authored
sphere/capsule collider sets → `sphere`/`capsule`; wrist-anchored
sphere → `sphere`), compliance classes from the spike report §4–§5
+ DECISIONS D-076 (hard non-penetration α=0; neighbour α_n;
attachment α_a; falloff-as-attachment-weighting).

## Human-read fixture + completeness

Fixture: `tests/fixtures/human_read_scene.md` + the test
`human_read_scene_completeness` (validation_s2). **The human read
is builder-authored — stated in the fixture document itself and
here — and Pax will re-check it.** The fixture work is six
synthetic pages forming three deterministic scenes; the read covers
the middle scene (pages 2–4): 4 face rows, 1 dialogue line, the
B-16 act (2 contact pairs, 2 pose entries, motion curves, 3-entry
face timeline), and a 3-beat understanding (eye_contact machinery,
reset/wakener **absent**).

Comparison result: **complete, exact.** The test asserts the face
records match the read's (page, slot, F-code) rows with no extras
(list total 4), the dialogue text + face link match, the ActRecord's
contact pairs match endpoint-for-endpoint (collider + compliance
included) and its registry snapshot reads B-16 / PEN / P-A /
mapped, the face timeline resolves to the read's records + F-code,
and the SceneUnderstanding's beats/machinery/dynamics match the
read with all four cited evidence records present in `links`.

## Gate checklist (SPEC §7 S2), item by item

| gate item | evidence | verdict |
|---|---|---|
| Known-scene completeness vs human read (faces timeline + contact pairs present and matching) | `human_read_scene_completeness` (validation_s2) + fixture doc | PASS (builder-authored read; Pax re-check owed) |
| Every unmapped act labelled `unmapped`, none dropped (adversarial: act with no registry mapping) | `acts_unmapped_code_kept_and_labelled` (B-13), `acts_unknown_code_becomes_candidate_not_canon` (B-99), `registry_seed_counts_and_provenance` | PASS |
| Every understanding claim traces to an evidence ref (adversarial: beat with no/dangling ref → fail) | `understand_beat_without_evidence_refs_refused`, `understand_beat_dangling_evidence_ref_refused`, `understand_ref_from_another_work_refused` — all typed `evidence_refusal`, nothing minted | PASS |
| Enrichment cannot clear an EXCLUDE / override the screen (adversarial: benign bundle for an excluded work → screen stands) | `enrich_cannot_clear_exclude` (pinned benign bundle lands; determination stays `excluded`; extraction still `screen_refusal`), `enrich_cannot_override_cleared_screen_either` (escalating bundle leaves `cleared` standing) | PASS |
| S1 gates keep passing | full suite below | PASS |
| New tools: explicit properties, outputSchema, all four annotations | S1 `tools_list_surface_contract` (now 11 tools) asserts the shape for every served tool incl. the 3 new; `tools_surface_s2` asserts presence + S3 absence | PASS |
| Tools/list token budget | measured, itemised below | **OVER — stated plainly: 18,905 bytes ≈ 4,726 tokens vs the ≤2,500-token estimate** |
| Contract gate (Inspector + rose + phlow) | — | DEFERRED to primo mirror (not claimed) |
| Diver gate | — | DEFERRED to primo mirror (not claimed) |

## Test counts

Full suite: **99 passed, 0 failed** (`cargo test --offline`).

| suite | S1 | S2 new | total |
|---|---|---|---|
| unit (lib) | 8 | 3 (acts.rs seed tests) | 11 |
| validation | 20 | 16 (validation_s2) | 36 |
| adversarial | 29 | 23 (adversarial_s2) | 52 |

Behavioural split: 36 validation / 52 adversarial — the same ratio
S1 shipped (its 20/29); S2's own additions are 16/23, matching it.
Two S1 assertions were updated for the grown surface (tool count
8 → 11 in `tools_list_surface_contract` and the stdio handshake
test); no other S1 test was touched.

- **clippy:** `cargo clippy --all-targets -- -D warnings` — clean.
- **fmt:** `cargo fmt --check` — clean.
- Toolchain note: builds ran with `RUSTC_WRAPPER=` (sccache
  bypass) and `--offline`, per brief.

## Tools/list payload — honest accounting

Served payload (compact JSON of the tools array, live-measured
against the built binary): **18,905 bytes ≈ 4,726 tokens** by the
bytes/4 estimate — **over** the ≤2,500-token Rule 1 estimate that
S1 met at 9,908 bytes ≈ 2,477. Itemised:

| tool | bytes |
|---|---|
| neemu_register_work | 1,455 |
| neemu_validate_ingest | 604 |
| neemu_screen_work | 1,171 |
| **neemu_enrich (new)** | **2,831** |
| neemu_segment_scene | 1,089 |
| neemu_extract_faces | 2,472 |
| neemu_extract_dialogue | 1,353 |
| **neemu_extract_acts (new)** | **3,238** |
| **neemu_understand_scene (new)** | **2,870** |
| neemu_list_records | 930 (grew 55 B: kind enum) |
| neemu_export | 880 |
| **total** | **18,905** |

The weight is structural, not prose: the three new tools carry
§3.3/§3.6/§3.15's nested shapes (contact-pair endpoints ×2,
pose/motion, beat/machinery/dynamics objects, the batch+grouping+
versions enrich surface) under the explicit-properties rule.
Shrinking it means either dropping schema explicitness (against
Rule 3 / Δ15) or moving detail out of band; both are Owner/Pax
calls, so the overage is reported, not silently trimmed.

## Interpretation calls (all flagged, none hidden)

1. **Evidence links apply to machinery and dynamics too**, not
   only beats. §3.6's link text names beats; the S2 gate says
   "every understanding claim traces" — machinery/dynamics are
   claims, so they carry the same requirement.
2. **New typed kind `evidence_refusal`.** §5's kind list is
   treated as extensible (the task gate itself demands "a typed
   refusal/failure" for unlinked beats); S3+ should ratify or
   rename it.
3. **Unknown scene id = invalid arguments**, not a gate refusal
   (mirrors S1 interpretation call 4: a failed lookup against a
   pure function is a caller error, −32602 class).
4. **Act face_timeline dangling ref = `evidence_refusal`**, where
   S1's dialogue used an arguments error for a bad `face_ref`. S2
   treats all evidence links uniformly; the divergence is
   deliberate and recorded here.
5. **Registry carrier:** the seed ships as versioned in-crate data
   (`act-registry/1`) instead of a separate estate file; candidate
   growth lands as ActRecords, canon changes are Owner-ratified
   edits to the table. §6's content and versioning discipline are
   honoured; the carrier choice is flagged for Pax.
6. **`mapped` definition** (above): family group or stated
   stratum. The 46 non-family B-codes are carried with their §1.1.3
   dispositions rather than forced into families.
7. **Body-part and slot names are free short strings** in contact
   pairs; only collider + compliance vocabularies are closed —
   no closed body-part list exists in the D-076 sources.
8. **Enrich requires registration only** (no pre-flight, no
   cleared determination) — System-0 sits between census and
   screening per Δ17/§3.15; requiring a screen would invert the
   pipeline order.
9. **SauceNAO-class detection is by source-name class**
   (`sauce*` / `trace.moe*`) plus the similarity rule; a caller
   naming a similarity service differently and claiming resolved
   with no similarity score would pass as citable — a known seam,
   flagged rather than papered over.
10. **Grouping confidence floor 0.5 is builder-chosen** (Δ21 gives
    no number); it is a named constant recorded on every bundle.
11. **Δ23 numbers are recorded config for the caller's resolver**,
    triggered by E-Hentai-lane source names — neemu never fetches,
    so there is nothing in neemu to rate-limit.
12. **`#![recursion_limit = "512"]`** added at the crate root: the
    eleven-tool schema literal outgrew the default macro limit.
    No behavioural effect.
13. **ActRecord `basis: "grid-read"`** — S2 extraction remains
    caller-measured (S1 posture); when real extractors land, the
    basis/lineage fields follow the Δ7 pattern S1 set for faces.

## Files

- New: `src/acts.rs` (registry seed + vocabularies),
  `src/s2.rs` (the three tools), `tests/validation_s2.rs` (16),
  `tests/adversarial_s2.rs` (23), `tests/fixtures/human_read_scene.md`.
- Edited: `src/lib.rs` (module wiring, S2 bounds, `evidence_refusal`
  kind, scene/record helpers, `opt_f64`, `parse_string_list`),
  `src/tools.rs` (3 definitions, dispatch, list kinds),
  `tests/common/mod.rs` (per-page-dimension fixtures),
  `tests/validation.rs` (two tool-count assertions 8 → 11).
