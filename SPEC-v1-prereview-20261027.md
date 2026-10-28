# Neemu MCP — media extraction for animation and voice (SPEC, 2026-10-27)

Name **neemu** (ネーム — the manga *name*: the rough storyboard
draft where panels, dialogue, and flow are decided before any
finished art exists — the most precise name for what this server
does to a work). Ruled by the Owner 2026-10-27 from a presented
list of ten candidates; the drafting working name was conte.
(The program directory keeps its commissioning path
`~/workspace/conte-mcp-20261027/` until the build starts.) Commissioned by the Owner 2026-10-27, minutes after the
`local-doujin-corpus` skill landed — the skill's §7 graduation
trigger (a corpus-admission decide()-class gate that multiple
clients must share) is the reason this is a server and not only a
skill: extraction is the consequential act, torogao-author and
torori both consume its output, and the gate must be enforced
server-side, not carried as prose in a workflow document.

Normative language per the Tiger Style MCP skill: MUST/MUST NOT
are release-blocking; SHOULD is default; MAY is allowed.

## 1. Domain (one server, one domain)

Neemu turns **screened source works** — doujin page sequences,
video, and images with sound — into **structured production
records** that the estate's builders consume:

- **Faces** → eight-channel face observations, mapped to F-codes
  or registered as new-face candidates.
- **Dialogue** → transcribed, speaker-attributed lines with scene
  beats.
- **Acts** → sex/kink act records at behaviour level: who, what
  contact, what pose/motion, what rhythm — the input from which
  torogao/lumen author animation and models.
- **Voice** → how the voice works in a scene: measured performance
  features + transcript, packaged as **cues for torori**, which
  does the actual synthetic generation.

Neemu also **understands** and **translates** (Owner's amendment,
2026-10-27, minutes after v1 of this spec):

- **Semantic understanding.** Beyond extracting what is on the
  page, neemu segments a work into meaningful units and records
  what is *happening*: scene beats, the hypnosis machinery in
  play and its stage, relationship dynamics, escalation shape
  (§3.6). Understanding records are model analysis — they carry
  the `model-estimate` basis, are labelled advisory wherever they
  travel, and are fenced to authoring time (the D-068 doctrine:
  neural proposers propose, deterministic gates and the Owner
  dispose).
- **Translation.** Understanding is only useful if it lands
  somewhere. Neemu emits three translation products (§3.7–3.9):
  **gap findings** against the animation stack (what the source
  does that W6 / torogao-author / the contact layer cannot yet
  express, with evidence and a proposed feature), **tutorial
  drafts** (how a technique in the source is achieved, written
  for the creator tooling), and **asset adaptations** (per-engine
  asset specs shaped for a named target — torogao, lumen,
  Strobe/Bevy, Hands-On — that the target's own tooling consumes).

Neemu extracts, characterizes, understands, and translates. It
does NOT animate, model, or synthesize — those stay with
torogao-author/lumen and torori — and it never writes into
another system's repo: translation products are records the
owning system ingests through its own gates. It does NOT fetch
from the web: acquisition routes stay where they already live
(the live browser for doujins.com, the legal scraper for
license-verified sources, Owner-supplied files). Neemu receives
local media + a registry entry, nothing else.

## 2. Non-negotiable gates (in server code, not documentation)

1. **Screen gate (decide()-class).** Every work exists in neemu's
   registry with a determination before any extraction tool will
   touch it. Extraction against an unregistered, unscreened, or
   EXCLUDED work returns a typed refusal. The screen reuses the
   legal scraper's canonical `BLOCKLIST_TERMS` (imported, never
   forked) as its floor; the determination itself is per-work,
   evidence-based, fail-closed: school-coded, age-stated or
   age-readable as under 18, loli/shota-coded, or ambiguous on any
   of these → EXCLUDED, reason recorded, nothing extracted.
2. **Tier gate.** Every record carries its work's provenance tier
   (corpus skill §5: Tier A analysis / Tier T training-cleared).
   The export tool refuses to emit Tier A material into a
   training-shaped bundle. Tiers are per work, set only with
   rights-ladder evidence recorded in the registry.
3. **Voice likeness rule (amended by Owner ruling, 2026-10-27:
   determination first, derivation only).** Neemu never takes a
   voice on resemblance alone. It *determines* clonability per
   voice on two axes (§3.10): **rights** — a recorded basis
   (estate-owned or commissioned voice, the Owner's own voice,
   license terms that explicitly permit cloning, public-domain
   recording) — and **technical sufficiency** (enough clean,
   isolated, consistent speech to clone from). Both axes MUST
   pass; either failing → NOT CLONABLE, reason recorded, and the
   voice stays characterization-only. Where the gate passes,
   cloning is permitted **only as derivation**: the clone is an
   intermediate that MUST be adapted (§3.11 — pitch, formant,
   tempo, accent) into a NEW estate voice identity carrying a
   full provenance chain, and is never shipped, cued, or
   presented as the source performer. Torori cues name estate
   voice identities (locked references), never source performers.
   Honest consequence, recorded not hidden: voices from
   aggregator doujin/anime will essentially always fail the
   rights axis — no evidence, no basis — and remain
   characterization-only. The derivation lane pays off on
   estate-owned, commissioned, licensed, and Owner-supplied
   voices.
4. **Audio silence.** Neemu never plays audio — not locally, not
   through any device. All voice work is file-metric: decode,
   measure, transcribe. (Owner's standing audio rule, 2026-10-26.)
5. **Grounding before recommendation (Owner, 2026-10-27).** Any
   record that recommends a build route — RecreationRecipe (§3.13),
   DistillationProposal (§3.14), and GapFinding proposals by
   reference — must carry `grounding`: at least one checked
   external reference per recommendation, drawn from the reference
   classes: **arXiv** (papers), **HuggingFace** (models/datasets),
   **code forges** (GitLab/GitHub), **game-engine docs** (Bevy,
   Godot, Unity, Unreal), **creator-tool docs** (Krita, Blender,
   Aseprite, OpenToonz, Live2D), and **animation references**.
   Neemu never fetches the web (rule 3 of §1 holds): the calling
   agent performs the sweep and supplies the references; neemu
   validates presence, records each reference's provenance (class,
   identifier, what it contributed, date checked), and the emit
   tools refuse an ungrounded recommendation with a typed
   `grounding_refusal`. The one-time pre-build prior-art check
   becomes a per-recommendation habit.

## 3. Records (the server's data contract)

All records are JSON, content-addressed by the SHA-256 of their
canonical form, stamped with: work_id, tier, extractor name +
version + model pins, input hashes, and creation time. Records
are immutable; a re-extraction mints a new record that supersedes
by reference, never edits in place.

### 3.1 FaceObservation
Per observed face, per panel/frame:
- The eight channels of the estate face language (gaze, pupils,
  crossing, lids, brows, mouth, flush, secondary) as specified in
  `local-nsfw-face-states` (references/FACE-STATES.md).
- `f_code`: an existing F-code when the state genuinely fits,
  else `null` + a `new_candidate` block (full channel description,
  the scene beats it carries, proposed code). New candidates are
  first-class output — never force-fit, never dropped (Owner,
  2026-10-27).
- Measurement basis: grid-read per `local-face-state-measurement`;
  unvalidated automated segmentation is never trusted as a
  measurement. Each observation names its basis (`grid-read`,
  `model-estimate` — the latter is advisory only and is labelled
  as such wherever it travels).

### 3.2 DialogueLine
- `speaker` (character slot or name as evidenced), `text`
  (transcribed exactly as read/heard; illegible spans `[unclear]`,
  never guessed), `beat` (scene context in one line), `locus`
  (page/panel or time range), `face_ref` / `act_ref` where the
  line lands inside a recorded face state or act.

### 3.3 ActRecord — the animation bridge
Per distinct act in a scene:
- `act_code`: from the ACT registry (§6), with its mapping to W6
  strata/primitives where one exists.
- `participants`: character slots and roles.
- `contact_pairs`: body-part → body-part contacts, with the
  contact-layer vocabulary from the D-076 spike (collider pairs,
  compliance class) so records land already shaped for the
  XPBD contact build.
- `pose_track`: keyframe descriptors at behaviour level (joint
  configuration classes, support/base, orientation) — enough for
  torogao-author or lumen to author rig motion from, without
  pretending to be captured motion data.
- `motion`: rhythm (beats per cycle, tempo curve), intensity
  curve over the act, entry/exit transitions.
- `face_timeline`: F-code / FaceObservation refs per beat.
- `sfx_hooks`: torori catalogue pair/programme refs where the act
  implies sound (shape per the torori GAP5 spec, e.g.
  `pair.torori.*`, `program_id: torori.conditioning-kit`).
- `props`: scene props the act depends on (restraints, furniture,
  devices) as named items, for set/prop modelling.

### 3.4 VoicePerformanceRecord — "how the voice works here"
Per voice segment (a line or a continuous non-speech stretch):
- `transcript` where speech exists (whisper.cpp, pinned model —
  the D-072 primary), `[non-speech: <class>]` where it doesn't
  (breath, moan-pattern class, cry, laugh — classified by measured
  features, labelled as classification, not fact).
- Measured features, all computed from the file: f0 median/range
  and contour class, tempo (syllables/sec), pause structure,
  breath events (count, placement), spectral proxies for
  breathiness/roughness, loudness curve summary.
- `scene_function`: what the voice is *doing* dramatically —
  e.g. giving a trigger phrase, counting, chanting, pleading,
  narrating — recorded as the cue's intent, distinct from the
  transcript.
- `source_kind`: `video` / `image+audio` / `audio-only`, with
  time ranges into the source media.

### 3.5 TororiCue
The hand-off record. One cue = one intended synthetic line/event:
- `text` (from DialogueLine), `delivery` (the measured feature
  targets from the matching VoicePerformanceRecord, normalized to
  torori's conditioning parameters), `intent` (scene_function),
  `target_voice` (estate character voice identity ref — REQUIRED,
  never a source performer), `sfx_pairs` (torori catalogue refs),
  `tier` (inherited; torori-side training use checks it again).
- Emitted as a bundle file torori ingests; neemu does not call
  torori directly in v1 (file hand-off keeps the servers
  decoupled and the cue inspectable by the Owner before any
  generation happens).

### 3.6 SceneUnderstanding — the semantic layer
Per scene (a segment from `neemu_segment_scene`, refined by
meaning rather than by page breaks alone):
- `beats`: the scene's beat sequence, each beat typed from a
  small estate vocabulary — setup, induction, deepening, trigger
  installation, trigger use, testing, act, escalation, aftermath,
  transition — with the locus (pages / time range) per beat.
- `machinery`: the hypnosis/control mechanism in play (device,
  app, eye contact, drug, magic, voice, other-as-evidenced), its
  stage model (how control is established, deepened, exercised),
  and its reset/wakener handling — present or absent; absence is
  a finding, recorded, exactly as in the corpus skill's
  PLOT-DEVICES discipline.
- `dynamics`: relationship and power structure as evidenced
  (roles, who directs, who responds), stated as observation, not
  interpretation beyond the evidence.
- `links`: the FaceObservation / DialogueLine / ActRecord /
  VoicePerformanceRecord refs that evidence each beat.
- Basis is `model-estimate` throughout: advisory, authoring-time
  only, never cited as measurement.

### 3.7 GapFinding — feeding the animation stack
Emitted whenever extraction or understanding meets something
the estate's stack cannot currently express:
- `kind`: `unmapped_act` / `unmapped_face` / `missing_primitive`
  / `missing_contact_class` / `missing_voice_feature` /
  `other_inexpressible`.
- `evidence`: the record refs + locus showing the thing itself.
- `proposal`: the feature that would close the gap, shaped for
  its addressee (a W6 stratum/primitive candidate, a contact-layer
  collider/compliance class, a torogao-author verb, a torori
  conditioning parameter).
- `addressee`: the owning program (W6 lab, contact layer,
  torogao-author, torori, lumen). Neemu proposes; the addressee's
  own gates and the Owner dispose. A gap finding is a backlog
  item with evidence attached, not a change order.

### 3.8 TutorialDraft
When a source demonstrates a *technique* worth teaching the
creator tooling (a face construction, a page rhythm, a voice
delivery pattern):
- `technique`, `evidence` (record refs), `steps` (the method at
  behaviour level, in the estate's own vocabulary), `target`
  (which tooling/tutorial series it belongs to), `fidelity_note`
  (what the draft could not determine from the source).
- Drafts enter the owning program's doc pipeline as drafts; the
  Owner's review is the publication gate.

### 3.9 AssetAdaptation
One source scene/act, re-expressed as an asset spec for ONE
named target engine:
- `target`: `torogao` / `lumen` / `strobe-bevy` / `hands-on`.
- `spec`: the asset in the target's own record shape as far as
  neemu can determine it (for torogao: action + face-timeline +
  contact pairs; for lumen: model/rig feature list; for
  strobe-bevy: scene/animation clip structure; for hands-on: rig
  + state set) — built from the §3 records, never from the raw
  source.
- `losses`: anything in the source the adaptation could not
  carry, named explicitly. A silent loss is a defect; a named
  loss is engineering.
- Adaptations are proposals into the target's intake, gated
  there. Tier travels: a Tier A source yields Tier A adaptations
  (design reference), never shippable-asset claims.

### 3.10 VoiceClonabilityAssessment — "can this voice be cloned?"
The determination record behind §2.3, one per voice:
- `rights_axis`: PASS only with a named basis + its evidence ref
  (contract, license terms, ownership record, PD status).
  `unknown` is a FAIL — absence of evidence is not a basis.
- `technical_axis`: measured from the media — isolated-speech
  seconds, SNR/clipping profile, speaker consistency across
  segments. Thresholds are spec constants, recorded in the
  record, not tuned per call.
- `verdict`: CLONABLE (both pass) / NOT-CLONABLE (either fails),
  with the failing axis and reason. Immutable once made; new
  evidence mints a new assessment that supersedes by reference.
- A voice with no assessment is treated as NOT-CLONABLE by every
  downstream tool (deny by default).

### 3.11 VoiceDerivation — clone, then adapt into an original
The only path from a source voice to a usable estate voice:
- Input: a CLONABLE assessment + the source characterization
  (VoicePerformanceRecord set) + an **adaptation plan**: target
  pitch placement, formant shift, tempo character, and an accent
  target (§3.12) — adaptation is mandatory, not optional; a
  derivation with an identity adaptation plan is refused.
- Output: a new **VoiceIdentity** record — the estate's locked
  reference for a character — carrying: the derived conditioning
  bundle ref (executed via torori's generation lane, the
  D-084-admitted Qwen route), its design targets (the
  attractiveness parameters as designed: warmth, breathiness
  balance, pitch placement, articulation clarity — stated as
  design values, with the Owner's ear as the admission gate,
  exactly as generated pools are admitted today), and the
  **provenance chain**: source voice → assessment → adaptation
  plan → identity. The chain is part of the record wherever the
  identity is used.
- A derived identity is an estate voice from birth: it can
  itself be cued, adapted further, and retired — and its
  provenance chain retires with nothing hidden.

### 3.12 AccentProfile — the accent system for voice gen
Accents are first-class conditioning, applied at cue/derivation
time — to speech AND to reactions (gasps, laughter, cries, and
interjections carry accent too; a reaction set is part of the
profile, not an afterthought).
- Registry: append-only, like the ACT registry (§6); each entry
  = accent id, variety, phonetic feature set (segment
  substitutions/shifts), prosody profile (rhythm class,
  intonation contours, stress habits), reaction/interjection
  set, and the conditioning parameters for torori's lane.
- **Seed coverage (Owner, 2026-10-27):** native English
  varieties of the English-speaking nations — en-US (general,
  Southern, Northeast), en-GB (RP, Northern), Scottish, Welsh,
  Irish, Canadian, Australian, New Zealand, South African,
  Caribbean — and English spoken with French, Italian, Spanish,
  German, Brazilian Portuguese, Japanese, Russian, and European
  Portuguese accents. Brazilian and European Portuguese are
  distinct profiles (the Owner named both). The registry is
  built extensible: new varieties enter as candidates from
  evidence, Owner ratifies canon.
- Source-side: VoicePerformanceRecords gain an `accent_heard`
  field (the profile the source voice matches, basis
  `model-estimate`) so what a scene's voice is doing accent-wise
  is part of its characterization, and derivations can keep,
  shift, or replace it deliberately.

### 3.13 RecreationRecipe — "how would we remake this?"
For anything observed in a screened work — a character, an
animation, an asset, an object, an effect, a place — neemu
records how the estate could recreate it (Owner, 2026-10-27).
- `target_kind`: character / animation / asset / object /
  effect / environment; `observed_in` (work + locus refs).
- **Decomposition**: the parts and techniques that compose the
  thing — for a character: silhouette and proportions, palette,
  face system, rig structure, motion set; for an animation: key
  poses, timing and easing, secondary motion; for an object:
  geometry, materials, construction steps. Decomposition is
  observation, stated in the estate's own vocabularies where
  they exist.
- **Recreation routes**: per estate lane (torogao, lumen,
  strobe-bevy, hands-on, and the AI-accessible drawing stack —
  Krita / Aseprite / Blender / Inkscape / OpenToonz / Synfig,
  §9), the concrete build path, the primitives it needs
  (existing vs missing — a missing primitive cross-references a
  GapFinding), a difficulty reading, and a determinism note
  (can this be made deterministic under estate doctrine, and
  what would it take).
- **Grounding is mandatory** (§2 rule 5): references checked
  BEFORE the route is recommended — arXiv, HuggingFace, code
  forges, engine docs, creator-tool docs, animation references.
  An ungrounded route cannot be emitted.

### 3.14 DistillationProposal — where the knowledge goes
A recipe (or finding) becomes estate capability through exactly
one output class per proposal:
- `skill` — a candidate skill: name, trigger, what it teaches
  the fleet to do.
- `mcp_tool` — a new tool for an EXISTING MCP: named server,
  tool name, schema sketch, why it belongs there.
- `new_mcp` — a new server proposal; the record must pass the
  skill-first / server-on-evidence test in writing (the state,
  enforcement, daemon, or multi-client need, named).
- `app_addition` — an addition to an existing app: named app,
  slice shape.
Addressee is Pax/Owner: neemu proposes, addressees dispose, and
neemu never writes into other repos (the §3.7 law). Grounding
mandatory (§2 rule 5).

## 4. Tool surface (18 tools — inside the 25-tool budget)

Every tool ships with a full object `inputSchema` (explicit
`properties`), an `outputSchema`, and behaviour annotations from
birth (Tiger MCP Rules 3 and 5). Counts and enumerations are
computed by server code, never by the calling model (Rule 4).

| Tool | Does | Annotations |
|---|---|---|
| `neemu_register_work` | Register a work: source provenance, media paths + hashes, tier claim. Idempotent by content hash. | write, idempotent |
| `neemu_screen_work` | Run/record the §2.1 determination with its evidence. The gate every extraction checks. | write, idempotent |
| `neemu_segment_scene` | Split a work into scenes/segments (pages→scenes; video→shots by cut detection + duration bounds). Bounded, paginated output. | read-only |
| `neemu_extract_faces` | FaceObservation records for a scene's panels/frames. | write (records), idempotent by input hash |
| `neemu_extract_dialogue` | DialogueLine records (OCR/transcription per source kind). | write, idempotent |
| `neemu_extract_acts` | ActRecord(s) for a scene. | write, idempotent |
| `neemu_analyze_voice` | VoicePerformanceRecord(s) from audio-bearing media. File-metric only; never plays audio. | write, idempotent |
| `neemu_assess_voice_clonability` | The two-axis §3.10 determination for a voice. No assessment → nothing downstream treats the voice as clonable. | write, idempotent |
| `neemu_derive_voice` | Clone + adapt a CLONABLE voice into a new estate VoiceIdentity (§3.11) under a mandatory adaptation plan (pitch/formant/tempo/accent). Refuses without a passing assessment. | write, idempotent |
| `neemu_emit_torori_cue` | Assemble a TororiCue bundle from dialogue + voice records for a named target voice. | write, idempotent |
| `neemu_understand_scene` | SceneUnderstanding record for a scene: beats, machinery, dynamics, evidence links. Basis `model-estimate`, advisory. | write, idempotent |
| `neemu_emit_gap_findings` | GapFinding records for a work or scene, addressed to their owning programs. | write, idempotent |
| `neemu_adapt_asset` | AssetAdaptation spec of a scene/act for ONE named target engine, with named losses. | write, idempotent |
| `neemu_draft_tutorial` | TutorialDraft from a demonstrated technique in a scene. | write, idempotent |
| `neemu_list_records` | Bounded, paginated listing of a work's records by kind; server-computed counts. | read-only |
| `neemu_export` | Tier-checked bundle export. Refuses Tier A → training-shaped exports with a typed error naming the blocking works. | read-only (emits a bundle file; no registry change) |
| `neemu_analyze_recreation` | Build a RecreationRecipe (§3.13) for an observed character / animation / asset / object / effect: decomposition, per-lane routes, grounding recorded. | write |
| `neemu_emit_distillation` | Route a recipe or finding into a DistillationProposal (§3.14) of exactly one output class (skill / mcp_tool / new_mcp / app_addition). Refuses ungrounded recipes (typed `grounding_refusal`). | write, idempotent |

Role-shaped groupings (Rule 8): **intake** (register, screen),
**extraction** (segment, faces, dialogue, acts, voice),
**understanding** (understand, gap findings), **voice**
(analyze, assess, derive, cue), **translation** (adapt, tutorial,
export, list). Session plan: neemu co-loads realistically with
torogao-author or torori, not both plus the full fleet —
18 tools keeps any pairing well inside the ~40 ceiling (Rule 2).

## 5. Error, timeout, and security contract (Rule 6)

- Gate refusals (unscreened/excluded work, tier breach, missing
  target voice) are execution results with `isError` and a typed
  `error.kind` (`screen_refusal`, `tier_refusal`,
  `missing_target_voice`, `media_unreadable`, `bound_exceeded`) —
  never protocol errors, never silent empty results.
- Every extraction tool declares a per-call work bound (max
  pages/frames/seconds) in its schema; exceeding it returns a
  partial result that says it is partial and how to continue
  (Rule 4). No tool waits without limit; long media is processed
  in segment-sized units.
- Inputs are validated server-side (paths confined to registered
  media, hashes verified against the registry before decode).
  Outputs are records, not raw media: neemu never returns source
  images/audio through the protocol.

## 6. The ACT registry

A versioned, append-only registry file (estate-kept, alongside
the corpus skill): each entry = act code, behaviour-level
definition, W6 stratum/primitive mapping (or `unmapped` — an
honest state, not a defect), contact-pair vocabulary, and the
works it was first observed in. New acts observed during
extraction enter as candidates, exactly like new faces: the
registry grows from evidence, and the Owner ratifies canon.
Seeding: the first tranche is derived from the corpus census +
the W6 strata already in the lab; the seed list is a build-stage
deliverable, not invented in this spec.

## 7. Build stages (each gated, none skippable)

- **S1 — registry + gate + image lane.** Server skeleton under
  Tiger MCP rules; register/screen with the imported blocklist;
  segment/faces/dialogue for page-sequence works. Gate: contract
  gate (Inspector + rose + phlow, per `local-mcp-contract-testing`)
  + adversarial set: extraction attempts on unscreened and
  excluded works MUST refuse; a school-coded fixture MUST screen
  out even with a clean blocklist pass on its title.
- **S2 — acts + understanding.** ActRecord extraction + ACT
  registry seed, and the SceneUnderstanding layer over the same
  scenes. Gate: records for a known scene are complete against a
  human read of the same scene (faces timeline + contact pairs
  present), every unmapped act is labelled `unmapped`, none
  silently dropped, and every understanding claim traces to an
  evidence record ref — an unlinked beat fails the gate.
- **S3 — voice + torori cues + derivation.** Video/audio lane:
  segmentation, whisper transcription, feature measurement,
  TororiCue emission; the clonability gate (§3.10), derivation
  (§3.11), and the accent registry seed (§3.12). Gates: cue
  bundle ingested by torori's conditioning programme shape
  (GAP5) without hand-editing; silence rule verified (no
  playback path exists in the codebase — checked, not assumed);
  adversarial set: a derivation attempt on an unassessed voice
  and on a rights-axis-fail voice MUST refuse, and a derivation
  with an identity adaptation plan MUST refuse; every seeded
  accent profile carries its reaction set, none speech-only.
- **S4 — hardening + EFLO.** Rate bounds, fuzzed inputs, the
  per-release EFLO record (Rule 9) built from gate evidence only.
- **S5 — translation emitters.** GapFinding emission wired to
  the ACT/face registries (a registry miss MUST produce a finding
  candidate, never a silent skip), AssetAdaptation emitters for
  the four named targets, TutorialDraft emission. Gate: for a
  scene with a known inexpressible element, the finding exists
  and names its addressee; for each target, an adaptation of the
  reference scene carries an explicit `losses` list (empty only
  when genuinely nothing was lost, with the check recorded).
- **S6 — recreation + distillation.** RecreationRecipe and
  DistillationProposal emitters (§3.13–3.14). Gates:
  `neemu_emit_distillation` refuses a recipe with zero grounding
  references (typed `grounding_refusal`); a `new_mcp` proposal
  without the server-on-evidence test filled MUST refuse; every
  emitted proposal names its addressee and output class.

Determinism: same input hashes + same extractor pins → identical
records, byte-for-byte, for every deterministic stage (screening,
segmentation, feature measurement, transcription with pinned
model/settings). Model-estimate stages are labelled with their
basis (§3.1) and excluded from the byte-determinism claim.

## 8. Open decisions for the Owner (at spec review)

1. **Name** — RESOLVED: **neemu** (Owner, 2026-10-27).
2. **Voice samples** — RESOLVED by Owner ruling 2026-10-27:
   the two-axis clonability determination (§3.10) with
   derivation-only cloning (§3.11) replaces the v1 blanket-off.
   The rights axis is unchanged in effect for uncleared
   performers: no basis, no clone.
3. **Cue transport** — file bundle (recommended, inspectable) vs
   direct server-to-server call in a later version.
4. **Build order vs torori** — torori's own build-start is a
   separate standing item (its trigger was met at D-084); neemu
   S3 depends on torori's conditioning shape being real, so the
   natural order is torori build confirmed first, or neemu S1–S2
   first (they don't touch torori) with S3 after.

## 9. The drawing stack (recreation lanes; Owner criterion 2026-10-27)

The Owner's criterion for any drawing/animation tool in the
estate: **fully AI/MCP/A2A-accessible, with a CLI workflow
usable from Neovim and headless operation.** Verified on primo
2026-10-27 (installed versions + access surface):

- **Blender 5.2** — full access: bpy Python API, true headless
  (`blender -b`), a Blender MCP package already installed on
  primo (`blender-mcp-git`), plus the estate's Blender
  connector. Grease Pencil covers drawn 2D in the same tool.
- **Krita 6.0.4** — full access: PyKrita plugin API,
  `kritarunner` for headless script execution, batch-export
  CLI, and community MCP bridges (at least two; one CC0,
  tested against Krita 6.0.4). Community bridges must pass the
  estate's MCP contract testing before adoption; the fallback
  is a thin in-house bridge plugin (the tackle pattern), NOT a
  new editor.
- **Aseprite 1.3.18** — full access, estate-proven: Lua API +
  batch CLI, already runs headless in estate pipelines.
- **File-driven lanes** (no live API; the file format is the
  interface, which suits code-first): Synfig (.sif is XML,
  generatable; CLI renders headless), OpenToonz (`tcomposer`
  headless renderer; scene files generatable), Inkscape (SVG
  is text; CLI actions for batch ops/export).
- **Pencil2D** — export CLI only, no scripting API: a hand
  tool, not an automation lane.

Verdict recorded: **no clean-room drawing editor is needed.**
The stack exists, is installed, and meets the criterion; the
only contingent build is the Krita bridge fallback above.
