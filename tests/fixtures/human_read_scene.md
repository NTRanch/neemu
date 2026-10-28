# Human read — fixture scene (S2 gate evidence)

**Provenance, stated plainly: this human read is BUILDER-AUTHORED.**
The S2 builder wrote both the fixture work and this read of it; the
read is the checklist the completeness test asserts against, and
Pax will re-check it independently at the S2 gate. It is not an
independent human's read of third-party material.

## The fixture work

"Harbour Wife" — six synthetic pages (header-valid PNG fixtures,
the S1 lane hashes bytes and sniffs headers; no pixels are read):

| page | dimensions | deterministic scene |
|------|-----------|---------------------|
| 0 | 800×1200 | scene A |
| 1 | 800×1200 | scene A |
| 2 | 1024×768 | **fixture scene** |
| 3 | 1024×768 | **fixture scene** |
| 4 | 1024×768 | **fixture scene** |
| 5 | 800×1200 | scene C |

Segmentation is the S1 deterministic spine (boundary on dimension
change, max_pages_per_scene 24): three scenes, the fixture scene
covering pages 2–4. Its scene id is content-derived and discovered
through `neemu_segment_scene` at test time, never hard-coded.

## The read of the fixture scene (pages 2–4)

Two participants throughout: slot `lead`, slot `partner`.

### Faces (grid-read, per page)

| page | slot | F-code |
|------|------|--------|
| 2 | lead | F-02 |
| 3 | lead | F-03 |
| 3 | partner | F-08 |
| 4 | lead | F-05 |

### Dialogue

| page | speaker | text |
|------|---------|------|
| 3 | partner | Look at me. |

The line lands in the partner's page-3 face (its FaceObservation
record is the line's `face_ref`).

### Act

One act spans the scene: **B-16** (Missionary, silhouette partner —
PEN family, physics class P-A).

- Participants: `lead` (role `receiver`), `partner` (role `giver`).
- Contact pairs (D-076 vocabulary):
  1. a: partner / hand / **sphere** → b: lead / cheek / **surface**,
     compliance **attachment** (the wrist-anchored sphere against a
     surface sample — the D-076 spike's gesture shape).
  2. a: partner / pelvis / **surface** → b: lead / pelvis /
     **surface**, compliance **hard_non_penetration**.
- Pose track:
  1. `entry` — joint configuration "supine, knees bent", support
     "bed surface", orientation "face-to-face".
  2. `cycle` — joint configuration "hips raised, legs wrapped",
     support "bed surface", orientation "face-to-face".
- Motion: beats_per_cycle 2, tempo_curve "steady then rising",
  intensity_curve [0.3, 0.6, 0.9], entry "from setup hold",
  exit "hold".
- Face timeline: beat `setup` → the page-2 lead face record;
  beat `act` → the page-3 lead face record; beat `escalation` →
  F-code **F-05** (the page-4 read, cited by code).
- SFX hooks: `pair.torori.conditioning-kit` (the SPEC §3.3 example
  shape). Props: `bed`.

### Understanding (beats over the same scene)

| beat type | pages | evidenced by |
|-----------|-------|--------------|
| setup | 2–2 | page-2 lead face record |
| act | 3–3 | the B-16 ActRecord + the dialogue record |
| escalation | 4–4 | page-4 lead face record |

- Machinery: mechanism **eye_contact**; stage model "held gaze
  deepens compliance across the scene"; reset/wakener **absent**
  (a finding, recorded as such); evidenced by the dialogue record.
- Dynamics: roles lead = subject, partner = director;
  who_directs `partner`, who_responds `lead`; evidenced by the
  B-16 ActRecord.

## Completeness claim (what the test asserts)

1. Face records exist for exactly the read's four (page, slot,
   F-code) rows — the faces timeline is complete, nothing extra.
2. The dialogue record matches the read's line and links the
   page-3 partner face.
3. The ActRecord's contact pairs equal the read's two pairs,
   endpoint-for-endpoint, collider and compliance included; its
   face timeline resolves to the read's records/code; its registry
   snapshot says B-16 / PEN / P-A / mapped.
4. The SceneUnderstanding's beats equal the read's three beats
   (type + locus), every beat's evidence refs resolve, and the
   machinery/dynamics blocks match the read — reset/wakener
   `absent` included.
