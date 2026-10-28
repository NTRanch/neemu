# OPST for the adult animation stack — adaptation map (2026-10-27)

Status: ADOPTED (Owner, 2026-10-27) — folded into SPEC.md as §10. Prepared by Pax at the Owner's
direction (2026-10-27 09:09 PDT): OPST was adopted for the phlow
school arm (pre-registration frozen 2026-10-26) but never
specified for the adult animation stack. This document is the
specification candidate. Nothing here is built; adoption folds
into the neemu spec as a new §10 and an S7 stage sketch, and the
animation arm gets its own pre-registration beside the phlow one
before any run.

Source: Sick et al., *On-Policy Self-Training: Data-Free
Self-Improvement in Large Language Models*, arXiv:2610.32770v1
(Google / Ulm University). Estate preregistration of the base
method: ~/workspace/phlow-trials/opst-20261026/PREREGISTRATION.md
— its rules are inherited wholesale, not re-decided here.

## The method in one paragraph

<details>
<summary>OPST mechanics (from the paper, via the frozen prereg)</summary>

Seeds are coarse topic descriptors only — no human-curated
problems, no gold answers in the training loop. The model
generates (question, rationale, candidate answer) triples. The
rationale + answer become PRIVILEGED CONTEXT for an EMA teacher;
the student trains on-policy on its own rollouts against the
teacher's distributions (skewed Jensen–Shannon divergence,
α = 0.8); the evaluated model is the EMA teacher. Two filters
are gates, not options: (1) pseudo-verification — the generator,
in a fresh context WITHOUT the privileged information and at low
temperature, must reproduce the candidate answer, and again on a
paraphrase; (2) consensus gate — samples the student already
answers with full agreement are discarded, so training
concentrates at the capability frontier. Caps from the paper's
ablations: ≤7 rounds, ≤285 samples per round per domain. The
paper demonstrates gains in text and in Visual Question
Answering, with capability retention on untrained domains.

</details>

## Why this stack is a better fit than the school arm

<details>
<summary>The oracle upgrade — the load-bearing observation</summary>

The paper's known weakness (recorded in our prereg): pseudo-
verification measures self-consistency, NOT correctness — a
stable wrong answer passes. The paper has no oracle.

The animation stack HAS oracles everywhere:

- torogao renders faces at known F-code parameters (pupil
  position, heart solidity, crossing are inputs, not guesses).
- lumen-rig-core evaluates rigs at fixed step; the XPBD contact
  layer (D-076) simulates with known contact pairs and spring
  constants.
- The frozen D oracle and the census gates give byte-level
  ground truth for whole classes of questions.

So the privileged information in our variant is not a
model-written rationale — it is MEASURED SIMULATOR/RENDERER
STATE, available at generation time because we generated the
sample, and absent at inference time when the model faces a real
doujin page or reference video. That is exactly the asymmetry
OPST exploits, with the verification upgraded from
self-consistency to oracle-checking wherever a renderer or
simulator produced the sample. Where no oracle exists (narrative
beats), OPST stays in its paper form and is graded accordingly
(§4 below).

</details>

## Adaptation points, graded

<details>
<summary>1. Neemu extraction models — the privileged-simulator lane (STRONGEST)</summary>

Target tools: `neemu_extract_faces`, `neemu_extract_acts`,
`neemu_understand_scene` — specifically the local models behind
them (via rose), and the System-1 triage role defined in the
doujin-corpus skill.

- Topic descriptors = estate vocabularies ONLY: F-codes and the
  NEW-candidate space, ACT registry entries, W6 strata, the
  §3.6 beat vocabulary, §3.12 accent profiles. Descriptors come
  from canon registries, never from corpus works (tier law, §6).
- Generation = rendering/simulation, not text: torogao renders
  faces at known parameters; rig-core + XPBD simulate acts at
  known contact pairs, pose tracks, spring constants; lumen
  renders 3D→2D. Each synthetic sample's privileged trace is the
  true parameter record.
- The student infers parameters from pixels alone; the EMA
  teacher sees pixels + the privileged record. On-policy
  training at the frontier: the consensus gate naturally
  concentrates rounds on the classes the fleet actually fails —
  asymmetric/uneven states (the triptych EDGE row), ghost-heart
  boundary cases, crossed-pupil extremes — because mastered
  classes are discarded by the gate.
- This is the direct attack on the "large swaths of
  undetermined" problem: rather than hoping a general model
  reads screened works, we grow a specialist whose curriculum is
  self-generated from our own canon and verified by our own
  renderers.

</details>

<details>
<summary>2. Reverse-engineering measurements (Strobe study Part 1) (STRONG)</summary>

The RE flow (~/workspace/strobe-animation-study-20261024/STUDY.md
§§1.3/1.7/1.8) infers hidden parameters from reference video:
loop period, timing charts (ones/twos/threes, easing), secondary-
motion spring constants (k/c). Every one of these has a
privileged generator: synthesise clips with KNOWN period,
timing chart, and spring constants; the student learns
measurement→parameter inference where ground truth exists, then
applies it to real reference video where only frame-diff and
flow signals exist. Same shape as point 1, aimed at motion
records instead of face/act records.

</details>

<details>
<summary>3. Face-state reading as VQA (STRONG — the paper's own multimodal arm)</summary>

Face reading IS visual question answering with an eight-channel
answer schema; the paper's VQA gains transfer in kind. OPST's
paraphrase filter earns its keep here: "where are the pupils"
and "describe the gaze" must agree — the cross-phrasing check
catches the grid-read drift behind the F-05 ghost-heart rounds.
The deterministic grid measurement stays the standard of record
(face-state-measurement skill); the OPST model remains
`model-estimate` basis, advisory — it proposes readings, the
measurement tool disposes.

</details>

<details>
<summary>4. SceneUnderstanding beats and machinery (MODERATE — no oracle)</summary>

Descriptors = the §3.6 beat vocabulary (setup / induction /
deepening / trigger installation / trigger use / testing / act /
escalation / aftermath / transition). No simulator generates
narrative ground truth, so this lane runs paper-form OPST:
self-generated Q/A + privileged reasoning traces,
pseudo-verification only. Consistent-error amplification risk is
highest here (a misread of hypnosis machinery can self-confirm).
Mitigations inherited from the prereg: the S2 gate's human-read
reference scene is the external battery; per-round spot audits;
retention set scored every round. Records stay `model-estimate`
regardless of training.

</details>

<details>
<summary>5. Voice characterization (MODERATE-STRONG — synthesis as oracle)</summary>

Privileged = torori/Qwen synthesis parameters. We synthesise
lines from estate VoiceIdentities with KNOWN accent profile, f0,
tempo, breathiness; the student learns to CHARACTERIZE a
performance (accent_heard, §3.4 feature set) from audio alone.
Trains the §3.4/§3.12 reading lane and gives torori an
accent-discrimination check beside the Owner's ear gate. Rights
axis untouched: the training voices are estate identities by
construction; nothing here clones or derives from an uncleared
performer (§3.10/§3.11 law unchanged).

</details>

<details>
<summary>6. GapFinding / RecreationRecipe drafting (WEAK-OPTIONAL)</summary>

OPST-shaped self-consistency filtering on recipe drafting:
generate in one context, re-derive in a fresh context, keep what
survives — layered on the existing §2 rule 5 grounding refusal.
Marginal over what S6 already gates; listed for completeness,
not recommended as a build item.

</details>

## What OPST must NOT touch

<details>
<summary>Hard exclusions</summary>

- **Tier A corpus material as training seed.** Tier A is
  analysis-only; OPST seeded from Tier A works would launder
  them into training data and defeat `neemu_export`'s tier
  refusal. The OPST descriptor set is drawn from estate canon
  registries and estate-owned/commissioned material only. The
  tier refusal inherits into any OPST tooling as a typed gate.
- **The screen.** The §2.1 determination of record stays
  deterministic, deny-by-default, blocklist-overrides-model
  (doujin-corpus skill §3 law). A self-improving screener that
  can drift is precisely the failure mode the screen exists to
  prevent. OPST never trains, tunes, or advises the gate of
  record; System-1 proposals remain advisory.
- **Latent-capability ceiling.** The paper's own limitation:
  OPST amplifies latent capability, it does not create it. A
  base model that cannot segment a pupil at all will not learn
  to from self-training alone; a flat result there is the
  predicted outcome, recorded as a finding, not retried louder.

</details>

## Gates and economics (inherited, not re-invented)

<details>
<summary>Prereg rules applied to the animation arm</summary>

- External gold battery, frozen before any run, never entering
  generation/filtering/training: the Owner-ruled keeper sets —
  the cast rig battery (25/25), the F-05 incumbent standard
  (seed 22355 line), the S2 human-read reference scene, the
  census-gate fixtures.
- Retention set from untrained domains (general faces, general
  VQA) scored every round; a retention regression is a
  first-class result even if in-domain scores rise.
- Caps: ≤7 rounds, ≤285 samples/round/descriptor-domain, dedup
  across rounds.
- Adoption rule: a configuration is adopted only if it BEATS
  the single-model baseline at equal budget on the external
  battery. Self-consistency metrics are diagnostics, never
  adoption triggers.
- Moneyball stat-card economics: bytes, peak RAM, CPU tok/s,
  production-per-unit-cost reported per round; subjects are
  small local models on primo (A6000 preferred, good-neighbour
  rules apply).

</details>

## Cost and sequencing

<details>
<summary>What adoption actually commits</summary>

- No new server (skill-first law): OPST enters as neemu spec
  §10 + an S7 stage sketch (builds after S6; depends on S1–S2 —
  the registries and record types must exist first) plus a
  sibling pre-registration for the animation arm reusing the
  phlow arm's harness (same EMA/JSD training code, different
  descriptors and oracles).
- Voice lane (point 5) sequences with torori's build, like S3.
- Cost: Pax drafting (this document + prereg), primo GPU windows
  at run time. No new infrastructure, no new dependencies beyond
  the school arm's.

</details>

## Recommendation

Adopt points 1–5 as neemu spec §10 (point 6 recorded, not
built), add open decision #5 to spec §8 ("OPST animation arm —
adopt per OPST-ADAPTATION.md"), and pre-register the animation
arm beside the phlow prereg before the first OPST run anywhere
in the estate, so both arms share one harness and one frozen
scorecard discipline.
