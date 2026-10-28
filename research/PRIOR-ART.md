# PRIOR-ART.md — What already exists that neemu should emulate

Pre-build dossier, 2026-10-27. Companion to `SITES.md` in this directory
and to the neemu SPEC (`../SPEC.md`).

**Question answered:** do arXiv, GitHub, HuggingFace, or MCPRush already
have tools, examples, or protocols neemu should emulate before building?

**Method / verification rule:** every item was checked at its primary
page (repo README, HF model/dataset card, official spec/docs, dataset
owner's own terms) unless explicitly marked **UNVERIFIED**. Search
snippets alone were never credited. Licenses are quoted as stated on the
primary page; "not stated" means the fetched primary page carried no
license field. No code was downloaded or run.

**MCPRush — negative result, stated plainly.** `site:mcprush.com`
searches for video / whisper / image servers returned no results, and
general searches surfaced only the MCPRush homepage (mcprush.com — a
marketplace/hosting index with listings imported from public sources,
marked "unclaimed" until ownership is verified; corroborated from the
homepage in this survey) plus a publisher's ship-log (dev.to) documenting
that its MCPRush listing stayed stale for weeks ("mcprush does not
re-read on its own"), showed "No parameter schema on file" for all 19
tools, and printed "Publisher not claimed" for the actual owner. **No
media-extraction, booru, diarization, FER, or voice server was found on
MCPRush that could be verified at a primary repo.** Anything MCPRush-only
is UNVERIFIED / discovery-only per estate ruling D-068, and nothing
below rests on an MCPRush listing.

---

## 1. MCP servers: media extraction / understanding / scene segmentation

### 1.1 aigroup-video-mcp — jackdark425/aigroup-video-mcp
- **URL:** https://github.com/jackdark425/aigroup-video-mcp
- **What:** DashScope-backed MCP video server: `analyze_video`, `summarize_video`, `analyze_video_scenes`, `analyze_video_custom`, `validate_video_source`, plus MCP **Resources** (`config://system`, `models://available`, `status://system`, `stats://usage`).
- **License:** MIT (repo metadata + README, verified).
- **Worth emulating:** (a) a dedicated `validate_video_source` tool *before* any analysis tool — the closest published analogue to neemu's ingest/screen step, validating accessibility/format/size as separate booleans; (b) the tools-vs-resources split (mutable analysis as tools; server state/models/usage as resources); (c) server-side bounds in config — `VIDEO__MAX_FILE_SIZE` (100 MB), `VIDEO__MAX_DURATION` (3600 s), `MCP__MAX_CONCURRENT_REQUESTS`, domain allow/deny lists, rate limiting — enforced in server code, not prompts.
- **Verdict: EMULATE** the tool/resource split and the validate-first shape. Reference-only for the backend (cloud DashScope, free-form prompt analysis, no typed records, no tier/provenance).

### 1.2 gemini-video-mcp-server — moe5445
- **URL:** https://github.com/moe5445/gemini-video-mcp-server
- **What:** 8 tools over Gemini: `analyze_video_file`, `analyze_youtube_video`, `analyze_inline_video`, `transcribe_video`, `compare_videos`, `research_video_concepts`, `generate_video_prompts`, `list_video_files` (1 star, 3 commits, verified README).
- **License:** **not stated** in repo metadata on the primary page.
- **Worth emulating:** the *input-channel* split as three distinct tools (local file / URL / inline base64 <20 MB) with per-channel parameters (`start_offset`/`end_offset` as "1m30s" strings, `fps` sampling, `media_resolution: default|low` as an explicit token-cost knob). Maps cleanly onto neemu's page/video/audio ingest channels.
- **Verdict: REFERENCE-ONLY** — free-form prompt-in/essay-out, no schemas, no gating; only the channel/parameter ergonomics transfer.

### 1.3 Video Context MCP (commercial)
- **URL:** https://www.videocontextmcp.com/ (product site verified; no repo/license found on it)
- **What:** commercial MCP server: video Q&A, structured summarization (key scenes/topics/timeline), frame extraction in 4 modes (timestamp / scene-change / interval / search), timestamp search, transcription with diarization, technical-metadata tool; 6 video + 4 audio providers with automatic fallback chains.
- **License:** not stated (proprietary service).
- **Worth emulating:** (a) frame extraction as *four named modes* rather than one tool with flags; (b) a no-AI metadata tool ("no AI API call needed") — deterministic ffprobe-class data kept separate from model-estimate data, exactly neemu's file-metric vs. model-estimate basis distinction. (c) Its provider fallback is the **anti-pattern** to note: silent fallback changes which model produced a record.
- **Verdict: REFERENCE-ONLY** for the mode taxonomy and the deterministic/model split; **AVOID** silent provider fallback — neemu records must name the exact producing model/version.

### 1.4 PySceneDetect — Breakthrough/PySceneDetect
- **URL:** https://github.com/Breakthrough/PySceneDetect/
- **What:** deterministic shot/scene-cut detection library + CLI (ContentDetector, AdaptiveDetector, ThresholdDetector), with a per-frame **stats file** for threshold auditing.
- **License:** BSD 3-Clause (verified on PyPI project page and repo).
- **Worth emulating:** scene segmentation as a *deterministic, threshold-parameterised* stage whose parameters and stats are recorded, upstream of any semantic scene understanding. Its scene record is just `(start, end, duration)` timecodes — the stable spine neemu's semantic beats should attach to, not replace.
- **Verdict: EMULATE** as neemu's segmentation substrate / record spine.

---

## 2. Booru / gallery metadata MCPs and taggers

### 2.1 gelbooru-mcp — citronlegacy
- **URL:** https://github.com/citronlegacy/gelbooru-mcp
- **What:** 7-tool Python MCP wrapping the Gelbooru API: `search_posts`, `search_tags`, `search_users`, `get_comments`, `get_deleted_posts`, `get_character_tags`, `build_prompt`.
- **License:** MIT (repo metadata + LICENSE, verified).
- **Worth emulating — concrete:** (a) `get_character_tags` returns **frequency-counted, category-bucketed tags** — `{tag, count, frequency}` split into `eye / hair / other`, with the README's own semantics ("frequency near 1.0 = near-universal; below 0.3 = situational") — a ready-made schema pattern for aggregating face/appearance observations across pages of a work; (b) `build_prompt` deliberately fetches only `rating:general solo` posts — a *rating filter baked into the query*, i.e. content-rating as a server-side query constraint, not a client request; (c) `get_deleted_posts(last_id)` as a sync/takedown cursor; (d) explicit cache TTL (24 h) surfaced in every response as `cache_hit` + `posts_analysed`.
- **Verdict: EMULATE** the frequency-bucket record shape and rating-in-query; the server itself has no screening, no provenance, and returns NSFW results unfiltered — do not copy its gate posture (it has none).

### 2.2 gallery-dl — mikf/gallery-dl
- **URL:** https://github.com/mikf/gallery-dl (development now on Codeberg, per README)
- **What:** the canonical gallery/booru/manga extractor framework: per-site extractor modules (Danbooru, Gelbooru, e621, nhentai, exhentai, MangaDex, pixiv…) emitting structured per-post metadata JSON with file-naming/filtering.
- **License:** GPL-2.0 (repo metadata, verified).
- **Worth emulating:** the **extractor contract** — one module per source, each yielding uniform metadata dicts (post id, tags by category, rating, source URL, file hash) regardless of site. That uniform-yield contract is what neemu's ingest should look like internally, with the source-site record preserved as an ingredient.
- **Verdict: REFERENCE-ONLY / do not copy code** — GPL-2.0 in a differently-licensed server is a contamination risk; emulate the extractor *interface* clean-room.

### 2.3 Danbooru-Turso MCP — UNVERIFIED
- **URL (index only):** https://glama.ai/mcp/servers/gamzadongza/danbooru-turso-mcp
- **What (as listed):** collects Danbooru character posts into a Turso DB with pagination + upsert; tools incl. `collect_and_save({tag, max_posts})` returning new-vs-updated counts. Listing claims MIT.
- **License:** MIT *as claimed on the Glama index page only* — underlying repo not confirmed → **UNVERIFIED**.
- **Worth emulating (if verified later):** upsert-with-counts ingest semantics ("N collected, X new, Y updated") and character-tag as the collection key.
- **Verdict: REFERENCE-ONLY, UNVERIFIED.** Do not cite its license or tool surface in the neemu spec until the repo is read.

### 2.4 pixai-tagger-v0.9 — pixai-labs
- **URL:** https://huggingface.co/pixai-labs/pixai-tagger-v0.9
- **What:** anime multi-label tagger predicting ~13.5k Danbooru-style tags (general / character / copyright), fine-tuned head on SmilingWolf/wd-eva02-large-tagger-v3, trained on a Danbooru snapshot (IDs 1–8,600,750, 2025-01).
- **License:** Apache 2.0 for the weights, with the card's explicit note: *"Danbooru content has its own licenses."* (verified on the HF card)
- **Worth emulating:** (a) **per-category thresholds** as first-class parameters (defaults: general 0.30, character 0.75) — character/identity tags held to a higher bar than descriptive tags; directly transferable: identity-bearing face/voice observations in neemu should require higher confidence than descriptive ones; (b) card-level separation of *model license* vs. *training-content license* — the two-line disclosure neemu's tier stamp should mirror.
- **Verdict: EMULATE** the threshold asymmetry and the license-split disclosure.

---

## 3. Voice cloning with rights/consent gating; provenance frameworks

### 3.1 Microsoft Azure Custom Neural Voice / Personal Voice — Limited Access regime
- **URLs:** https://learn.microsoft.com/en-us/azure/ai-foundry/responsible-ai/speech-service/text-to-speech/limited-access and …/disclosure-voice-talent (both verified, Microsoft Learn)
- **What:** the strongest published consent gate in the industry. Custom voice is **Limited Access** (application + approved use case only). Before training: (1) explicit *written* permission from the voice talent; (2) the talent must be given Microsoft's disclosure document *in advance*; (3) a **recorded consent statement** in a pre-defined script, uploaded before training can begin; (4) Microsoft runs **speaker-verification biometrics comparing the consent recording against the training audio** to confirm same speaker; (5) the deployer must disclose the synthetic nature to end users and provide a feedback/reporting channel; (6) consent file and model are *retained* by Microsoft for integrity.
- **License:** proprietary service terms (not a software license).
- **Worth emulating — this is the template for neemu's rights axis:** consent is an *artifact* (recorded statement + written permission + disclosure receipt), verified *against the training material itself*, scoped to an *approved use case*, and retained as part of the record. Note the two-party split: the platform verifies same-speaker; the *customer* remains responsible for actually having permission — a checkbox is explicitly not the permission.
- **Verdict: EMULATE** — the single best gate design found. See Spec Delta 1.

### 3.2 ElevenLabs voice cloning (IVC / PVC)
- **URL:** https://elevenlabs.io/docs (voice-captcha described in ElevenLabs documentation/help material; corroborated in its docs' Voices capability page: "Voice-captcha technology is used to verify that all voice clones are created from your own voice samples")
- **What:** two tiers with *different* gates: Instant Voice Cloning = self-attestation checkbox ("I have the right to clone this voice"); Professional Voice Cloning = fine-tune on ~30 min–3 h audio **plus a voice CAPTCHA** — the claimant reads a shown text aloud live, and the live recording is matched against the uploaded samples. PVC of *another person* is not permitted on your account at all, even with their consent: the owner must create/verify it on their own account and share it privately. Also: C2PA support + imperceptible watermarking on generated audio, and a public AI Speech Classifier for its own outputs.
- **License:** proprietary service terms; free-tier output non-commercial + attribution, commercial rights only on paid plans.
- **Worth emulating:** (a) **gate strength scales with fidelity** — the more faithful the clone, the stronger the proof; (b) the *non-transferability* rule: a verified voice belongs to the verifying account; sharing is a link, not a copy; (c) the honest limitation, stated by ElevenLabs itself, that a CAPTCHA proves *presence at the microphone*, not legal permission — neemu should encode that distinction in its record (`presence_verified` ≠ `rights_basis`).
- **Verdict: EMULATE** the tiered-gate and non-transferability ideas; **AVOID** the IVC checkbox as a sufficient gate for anything training-cleared.

### 3.3 Chatterbox — Resemble AI
- **URL:** https://github.com/resemble-ai/chatterbox
- **What:** open TTS family (Turbo 350M / Nano 110M / Multilingual V3 500M, 23+ languages), zero-shot cloning from ~5–10 s reference, native paralinguistic tags (`[laugh]`, `[cough]`, `[chuckle]`), exaggeration + CFG controls, **PerTh (Perceptual Threshold) neural watermark embedded in every output by default**, with a published extraction snippet (watermark reads 0.0 / 1.0). Single-Language Packs include separate pt-BR and pt-PT finetunes — the same pt-BR/pt-PT distinction neemu's accent registry makes.
- **License:** MIT (repo metadata + LICENSE, verified) — code *and* weights, commercial use permitted.
- **Worth emulating:** (a) watermark-on-by-default with *no off switch in the normal path* — provenance as a property of the derivation pipeline, not an option; (b) the paralinguistic tag vocabulary as part of the text input contract — a ready reference for how neemu's reaction/interjection sets in AccentProfiles will eventually be *consumed* by a TTS; (c) per-language/region finetune packs as the packaging unit for accent conditioning.
- **Verdict: EMULATE** — the leading candidate derivation engine for neemu-cleared voices, precisely because MIT + always-on watermark.

### 3.4 AudioSeal — Meta (facebookresearch)
- **URL:** https://github.com/facebookresearch/audioseal — paper: San Roman et al., "Proactive Detection of Voice Cloning with Localized Watermarking," ICML 2024
- **What:** localized audio watermarking: the generator embeds an imperceptible watermark (optionally a 16-bit message = 65,536 code points); the detector returns **per-sample probability** (every 1/16,000 s), so it localises *which segments* of a long/edited file are synthetic; robust to compression/re-encoding/noise; streaming mode in 0.2.
- **License:** code MIT. **Weights trap, resolved:** early releases listed weights as CC-BY-NC 4.0 (still shown on older PyPI pages); the GitHub README records *"2024-04-02: We have updated our license to full MIT license (including the license for the model weights)!"* — verified on the primary repo. Pin neemu to the post-change release and record which license text shipped with the pinned version.
- **Worth emulating:** (a) **segment-level, not file-level, provenance** — a derived voice line that is only partly synthetic gets a partly-synthetic record; (b) the 16-bit message as a *derivation ID*: embed the neemu VoiceDerivation record id (or a truncated hash of it) as the watermark payload, binding file ↔ record.
- **Verdict: EMULATE.**

### 3.5 Coqui XTTS v2 — license cautionary tale
- **URL:** https://huggingface.co/coqui/XTTS-v2 (model card; license text itself not re-fetched in this survey — **license detail UNVERIFIED at primary text level**)
- **What:** 6-second zero-shot cloning, 17 languages; still the ecosystem default in many pipelines.
- **License (as widely reported, incl. multiple 2026 surveys):** library code MPL-2.0, but **weights under the Coqui Public Model License (CPML), non-commercial only** — and Coqui Inc. shut down in Jan 2024, so there is no one left to sell a commercial license; the CPML page itself now 404s.
- **Worth emulating:** nothing in the license; everything in the *failure mode* — a technically excellent cloner became legally stranded. neemu's clonability assessment needs a third check beyond rights+technical: *is the derivation engine's own license alive and permissive for this use?*
- **Verdict: AVOID** as a derivation engine for anything commercial/training-cleared; cite as the justification for an engine-license field.

### 3.6 Papers (arXiv)
- **"Authenticated Contradictions from Desynchronized Provenance and Watermarking"** — arXiv:2603.02378 — https://arxiv.org/abs/2603.02378. Formalises the **Integrity Clash**: a C2PA manifest and an invisible watermark are independent layers; neither conditions on the other, so an asset can carry a *valid* manifest and a *contradicting* watermark. **Emulate the warning:** neemu must cross-check manifest vs. watermark on ingest and record disagreement as a finding, never silently prefer one. **Verdict: REFERENCE — design constraint.**
- **"Deep Audio Watermarks are Shallow: Limitations of Post-Hoc Watermarking Techniques for Speech"** — arXiv:2504.10782 — https://arxiv.org/abs/2504.10782. Post-hoc watermarking of speech is fragile. Implication: watermark at *generation time* (as Chatterbox/AudioSeal do), not as a neemu afterthought. **Verdict: REFERENCE — design constraint.**
- **"Voice 'Cloning' is Style Transfer"** — arXiv:2605.16578 — https://arxiv.org/abs/2605.16578. Empirically, "cloning" does not faithfully copy identity; it transfers style. Scientific support for neemu's existing rule that a cleared clone is emitted only as a *new* VoiceIdentity via mandatory adaptation, never presented as the source performer. **Verdict: REFERENCE — cite in the spec's rationale.**
- **Barrington et al., "Single and Multi-Speaker Cloned Voice Detection"** — arXiv:2307.07683 — https://arxiv.org/abs/2307.07683. Perceptual + learned detectors for cloned voices. Useful only as an ingest signal (is this aggregator audio already synthetic?). **Verdict: REFERENCE-ONLY.**

---

## 4. Accent identification + accent-conditioned TTS (HF, with licenses)

### 4.1 CommonAccent / Jzuluaga accent-id-commonaccent_ecapa
- **URL:** https://huggingface.co/Jzuluaga/accent-id-commonaccent_ecapa — paper: arXiv:2305.18283, Interspeech 2023
- **What:** ECAPA-TDNN (SpeechBrain) accent classifier over **16 English accents** (african, australia, bermuda, canada, england, hongkong, indian, ireland, malaysia, newzealand, philippines, scotland, southatlandtic, us, wales + one), 87% test accuracy with augmentation; outputs a probability distribution + label; trained on Mozilla Common Voice (CommonAccent).
- **License:** **not stated on the model card** (verified: card carries no license field). SpeechBrain is the framework; the card does not license the weights. Treat weights as unlicensed until clarified.
- **Worth emulating:** (a) the **16-way closed accent taxonomy with a probability vector**, not a single label — neemu's AccentProfile matching should store the full distribution + top-1 + margin; (b) its documented confusion structure (US↔Canada) — expected-confusion pairs belong in the record as a known-limitation field.
- **Verdict: EMULATE the record shape; REFERENCE-ONLY as a model** (unlicensed weights).

### 4.2 dima806/english_accents_classification
- **URL:** https://huggingface.co/dima806/english_accents_classification
- **What:** Wav2Vec2-class classifier, 5 accents (us / england / indian / australia / canada), Common Voice.
- **License:** **not stated on the card** (verified).
- **Worth emulating — as a warning:** its own published classification report shows catastrophic per-class failure hidden by aggregate accuracy: US recall 0.015 (F1 0.029), Australia recall 0.038, while England recall is 0.912; overall accuracy 0.522. A neemu accent record that stored only top-1 label + confidence would launder exactly this.
- **Verdict: AVOID as a model; EMULATE its per-class report as a mandatory disclosure format** (per-accent precision/recall must travel with any accent model neemu registers).

### 4.3 mavleo96/whisper-accent-medium.en
- **URL:** https://huggingface.co/mavleo96/whisper-accent-medium.en
- **What:** Whisper-medium.en fine-tune that *conditions ASR on accent*: accent predicted from encoder states (learned weighted layer sum + attention pooling), accent embeddings injected via **AdaLN in every decoder layer**, encoder/decoder frozen, <10% trainable params; accent accuracy 0.9615, WER 0.0939 on its eval set.
- **License:** not stated in the fetched card — **UNVERIFIED**.
- **Worth emulating:** the conditioning architecture pattern for neemu's TTS side: a *separate, small, trainable accent-embedding table* + frozen backbone — exactly an append-only AccentProfile registry in model form (new accent = new embedding row, backbone untouched).
- **Verdict: REFERENCE-ONLY** (architecture pattern), license unverified.

### 4.4 Accent-conditioned TTS papers (arXiv)
- **Joycent** — arXiv:2606.16417 — https://arxiv.org/abs/2606.16417: diffusion accent TTS via *disentangled* accent vs. speaker factors with **layer-specific conditioning** (different factors injected at different layers).
- **CrossAccent-TTS** — arXiv:2606.25403 — https://arxiv.org/abs/2606.25403: disentangled speaker/accent representations with an explicit **accent-intensity scalar** control.
- **Accent Analogy Guidance** — arXiv:2609.29123 — https://arxiv.org/abs/2609.29123: training-free sampler term that estimates an *accent direction* (same synthetic voice rendered in two languages; voice cancels, accent remains) and subtracts it.
- **License:** papers only (arXiv); no code/models verified.
- **Worth emulating:** three concrete AccentProfile fields — `intensity` (continuous, from CrossAccent), `disentangled_from_speaker: true` as a design requirement (Joycent), and *accent-as-vector-difference* (a profile can be *derived* from paired recordings, with the pair recorded as its provenance — AAG).
- **Verdict: REFERENCE-ONLY**, but they directly shape the AccentProfile schema (Spec Delta 9).

---

## 5. FER / affect datasets & models — LICENSES (critical section)

**Headline finding: there is no widely-used FER dataset that is cleanly commercial, and the most-used model code being Apache/MIT does NOT clean the weights, because the weights are trained on AffectNet/RAF-DB. neemu must treat "model license" and "training-data license" as two separate fields on every face-observation record, and a non-commercial training set must cap the record's tier.**

### 5.1 AffectNet
- **URL:** http://mohammadmahoor.com/wp-content/uploads/2024/06/AffectNet-Agreement-v2.1-10May2024.pdf (owner's official Academic Use License, verified full text)
- **What:** ~1M web faces, ~450k manually annotated: 8 categories (incl. contempt) + continuous valence/arousal + landmarks.
- **License — exact:** **AffectNet™ Academic Use License (University of Denver).** §2: use *"only for noncommercial research and educational purposes."* §4: no provision to third parties (narrow exceptions). Access is gated: a Lead Researcher (faculty-level; trainees may not apply on their own behalf) signs; IRB approval where applicable.
- **Permits:** non-commercial academic research/education by named permitted researchers only. **Forbids:** commercial use, redistribution, casual download.
- **Verdict for neemu: AVOID as a training source** for anything training-cleared or commercial. Its 8-class + valence/arousal schema is, however, the right *shape* reference for neemu's 8-channel face observations (categorical + continuous pair).

### 5.2 RAF-DB / RAF-AU / RAF-ML
- **URL:** http://whdeng.cn/RAF/model1.html (owner's page, PRIS, Beijing University of Posts and Telecommunications; terms also on the dataset page, verified)
- **What:** ~15k (basic) / ~30k in-the-wild faces, 7 basic + compound expressions, 40 annotators per image; RAF-AU adds FACS Action Unit annotations.
- **License — exact terms:** *"The RAF database is available for non-commercial research purposes only… You agree not to reproduce, duplicate, copy, sell, trade, resell or exploit for any commercial purposes, any portion of the images and **any portion of derived data**. You agree not to further copy, publish or distribute any portion."* Download password-gated to university professors/research scientists who email an attestation.
- **Note the derived-data clause:** it reaches *outputs*, not just the images — stricter than AffectNet's wording and directly relevant to neemu's tier logic.
- **Verdict: AVOID** for training-cleared pipelines; the AU-annotated subset is a schema reference for FACS-style channels only.

### 5.3 FER2013 (+ FER+)
- **URL:** Kaggle challenge (ICML 2013 Challenges in Representation Learning); FER+ at https://github.com/microsoft/FERPlus
- **What:** 35,887 48×48 grayscale faces, 7 classes, scraped via Google image search.
- **License — exact:** **no license was ever granted for the images.** (The 2013 contest licensed winners' *code*, not the data; the "open" labels on HF/Kaggle re-uploads belong to third-party re-uploaders, not the dataset owners — this point is sourced from a 2026 secondary licensing analysis, https://www.forasoft.com/blog/article/emotional-analysis-machine-learning, because there is no owner license page.) FER+ (Microsoft) is MIT **over the relabel CSV and code only** — Microsoft never hosted the images.
- **Verdict: AVOID.** The most-copied FER dataset is the least licensed. Any neemu-registered FER model must declare FER2013 in its training lineage as a tier-capping fact.

### 5.4 CREMA-D — the usable contrast case
- **URL:** https://github.com/CheyneyComputerScience/CREMA-D (dataset described in Cao et al. 2014)
- **What:** 7,442 acted audio-visual clips, 91 actors, 6 emotions × 12 sentences — *acted, consented performers*, which is precisely neemu's voice/face situation.
- **License:** **Open Database License (ODbL 1.0) + Database Contents License (DbCL 1.0)** (verified via dataset documentation reproduced in arXiv/HF records).
- **Permits:** commercial use, adaptation, sharing — **with attribution and share-alike on any redistributed derivative database.**
- **Verdict: EMULATE as the model citizen** — consented actors + a licence that permits use with conditions that can actually be complied with and stamped onto records.

### 5.5 RAVDESS
- **URL:** https://zenodo.org/records/1188976 (Livingstone & Russo 2018)
- **What:** 7,356 files (2,452 audio-visual), 24 professional actors, speech + song, categorical + intensity.
- **License:** **CC BY-NC-SA 4.0** — non-commercial, share-alike; the authors sell a separate commercial licence.
- **Verdict: REFERENCE-ONLY** for schema (its filename-encoded metadata — modality/channel/emotion/intensity/statement/repetition/actor — is an excellent compact record-key design); **AVOID** for commercial training without the separate licence.

### 5.6 EmotiEffLib (ex-HSEmotion) — sb-ai-lab
- **URL:** https://github.com/sb-ai-lab/EmotiEffLib
- **What:** EfficientNet-class FER for photo/video: 8-class expression, plus multi-task variants predicting **valence + arousal**, plus AU-detection and compound-expression results in ABAW challenges; ONNX + PyTorch, Python + C++.
- **License:** **Apache-2.0 for the code** — README: *"The code of EmotiEffLib is released under the Apache-2.0 License. There is no limitation for both academic and commercial usage."* (verified)
- **The trap, in its own README:** models were *pre-trained on VGGFace2* and *trained/fine-tuned on AffectNet* (its benchmark table is AffectNet 7/8-class). The Apache grant covers the code; it does not and cannot re-license AffectNet-derived weights.
- **Worth emulating:** the **multi-task output shape** — one forward pass emitting `{8-class probabilities, valence ∈ [-1,1], arousal ∈ [-1,1], AU set}` — is the closest published analogue to neemu's 8-channel FaceObservation and should be its structural template.
- **Verdict: EMULATE the output schema; AVOID the weights** for training-cleared use unless the AffectNet lineage is stamped and tier-capped.

### 5.7 trpakov/vit-face-expression
- **URL:** https://huggingface.co/trpakov/vit-face-expression
- **What:** ViT fine-tuned on FER2013, 7 classes, test accuracy 0.7116; the most-downloaded "just use it" FER checkpoint and base for many derivatives.
- **License:** **not stated on the card** (verified — no license field, no license section).
- **Verdict: AVOID.** Unlicensed card + FER2013 lineage = double defect. Cite in neemu's spec as the canonical example of why an unlicensed checkpoint is a typed refusal, not a warning.

### 5.8 POSTER++ / POSTER V2 (HF: Ammar2000/mf)
- **URL:** https://huggingface.co/Ammar2000/mf
- **What:** FER model reporting RAF-DB 92.21, AffectNet-7 67.49, AffectNet-8 63.77, CAER-S 93.00.
- **License:** card states *"Our research code is released under the MIT license"* — code only; both headline datasets are non-commercial (§5.1/§5.2).
- **Verdict: REFERENCE-ONLY.** Reinforces the split-license rule; nothing else transfers.

---

## 6. Speaker diarization / Whisper pipelines

### 6.1 WhisperX — m-bain/whisperX
- **URL:** https://github.com/m-bain/whisperX (24.9k stars, verified)
- **What:** the reference pipeline: faster-whisper batched ASR (70× realtime, large-v2, <8 GB VRAM) → wav2vec2 forced alignment (**word-level timestamps**) → pyannote diarization → `assign_word_speakers`. VAD preprocessing; `condition_on_prev_text=False` by default to reduce hallucination.
- **License:** **BSD 2-Clause** (repo metadata, verified).
- **Worth emulating — the record shape:** its output segment is `{start, end, text, speaker, words: [{word, start, end, score}]}`. That three-level nesting (segment → word → speaker) is the right spine for neemu's DialogueLine + VoicePerformance records: line-level for dialogue, word-level timings for performance features (pace, pauses), speaker label bound at the *word* level so a mid-line speaker change is representable. Also adopt its honesty section as a template: limitations are enumerated in the README (unaligned words without timings, overlapping speech handled poorly, "diarization is far from perfect") — neemu records should carry a `known_limitations` echo.
- **Verdict: EMULATE** — pipeline and record shape.

### 6.2 pyannote/speaker-diarization-3.1
- **URL:** https://huggingface.co/pyannote/speaker-diarization-3.1
- **What:** segmentation + embedding pipeline; ingests mono 16 kHz (auto-downmix/resample); outputs an `Annotation` exportable as **RTTM**; `num_speakers` / `min_speakers` / `max_speakers` controls; benchmarked DER across 9 corpora (7.8% REPERE → 50.0% AVA-AVD — a 6× spread by domain).
- **License / access:** **gated.** The card requires accepting user conditions for *both* `pyannote/segmentation-3.0` and `pyannote/speaker-diarization-3.1` and creating an HF access token. No license field was shown in the fetched card text — treat as conditions-gated, not as freely licensed.
- **Worth emulating:** (a) **RTTM as an interchange export** alongside neemu's native record; (b) speaker-count bounds as *caller-supplied priors recorded in the output* (a doujin scene's known cast size is a legitimate prior, but it must be stamped as a prior, not discovered truth); (c) **per-domain DER disclosure** — diarization confidence in neemu should be domain-tagged (clean dialogue vs. overlapping/acted audio), because the same model is 7.8% and 50% DER depending on domain.
- **Verdict: EMULATE** (with the gated-access fact recorded in neemu's model registry).

### 6.3 Speech Audio MCP — andretisch — UNVERIFIED
- **URL (index only):** https://www.pulsemcp.com/servers/gh-andretisch-speech-audio
- **What (as listed):** STT + TTS MCP with faster-whisper, Silero VAD, speaker diarization; has a `server.json`.
- **License:** not stated; primary repo not confirmed in this survey → **UNVERIFIED**.
- **Verdict: REFERENCE-ONLY, UNVERIFIED.** Its existence shows the obvious MCP wrapping of §6.1/§6.2 is already commoditised — neemu's differentiation must be its records and gates, not "Whisper behind MCP."

---

## 7. Protocol patterns: annotations, gating, provenance standards

### 7.1 MCP Tool Annotations — official spec + MCP blog
- **URLs:** https://blog.modelcontextprotocol.io/posts/2026-03-16-tool-annotations/ and the 2025-03-26 / 2025-06-18 specification (ToolAnnotations)
- **What:** `ToolAnnotations { title?, readOnlyHint (default false), destructiveHint (default true), idempotentHint (default false), openWorldHint (default true) }`.
- **License:** specification text (Apache-2.0 / MIT dual for MCP specs); treat as a standard, not code.
- **The load-bearing sentence, verified from the official blog:** every property is a **hint**; annotations *"are not guaranteed to faithfully describe tool behavior, and clients **must** treat them as untrusted unless they come from a trusted server."* SDK docs repeat: clients must **never** make security decisions from annotations.
- **Worth emulating:** set annotations *correctly and completely on all 16 neemu tools* (extraction reads: `readOnlyHint:true, openWorldHint:false`; derivation-creating tools: `readOnlyHint:false, destructiveHint:false, idempotentHint:true` — idempotent because content-addressed creation with the same inputs is a no-op) — while keeping every actual screen/tier/clonability refusal in server code, exactly as neemu already plans. The spec *validates* that architecture; annotations are UX/policy vocabulary, never the gate.
- **Verdict: EMULATE** (and cite the untrusted-hint clause in neemu's spec as the reason typed refusals are server-side).

### 7.2 C2PA — Content Credentials Technical Specification (v2.4)
- **URL:** https://spec.c2pa.org/specifications/specifications/2.4/specs/ContentCredentials.html
- **What:** signed provenance manifests for media: an **assertion store + claim + claim signature**; a **hard binding** (`c2pa.hash.data` / `c2pa.hash.bmff.v2`) hashing the asset; a **soft binding** (fingerprint/watermark) for manifest recovery after stripping; `c2pa.actions` (each action: `action`, `when`, `softwareAgent{name,version}`, `digitalSourceType`, `parameters.ingredients[]`, and v2 `changes` region-maps scoping an action to a *portion* of an asset); `c2pa.ingredient` assertions with `relationship: parentOf | componentOf`; **Update Manifests** that add assertions *without* changing content and therefore must not contain a hash or actions assertion.
- **License:** C2PA specification (open standard; © C2PA, published for implementation).
- **Worth emulating — directly mappable onto neemu:** source media = ingredient (`parentOf`); extracted records = actions with `softwareAgent` = neemu version + model version; a voice derivation = a new manifest whose ingredient is the source voice record and whose action parameters carry the adaptation (pitch/formant/tempo/accent deltas); `digitalSourceType` values (`trainedAlgorithmicMedia` etc.) as the vocabulary for "this output is synthetic"; **region-scoped actions** for segment-level claims (pairs with AudioSeal localisation, §3.4); Update Manifests as the pattern for *appending a tier re-assessment or a GapFinding to an existing record without mutating it* — exactly neemu's immutable-record + append-only-registry discipline, in standard form.
- **Verdict: EMULATE** — adopt C2PA's assertion/action/ingredient vocabulary for neemu's provenance chains even if full JUMBF signing is a later stage.

### 7.3 W3C PROV — PROV-O / PROV-DM
- **URL:** https://www.w3.org/TR/prov-o/ (W3C Recommendation, 2013)
- **What:** the generic provenance model: three classes — **Entity, Activity, Agent** — and relations `wasDerivedFrom` (entity→entity), `used` / `wasGeneratedBy` (activity↔entity), `wasAttributedTo` (entity→agent), `wasAssociatedWith` (activity→agent), `startedAtTime/endedAtTime`.
- **License:** W3C Document License (specification).
- **Worth emulating:** use PROV as the *abstract* chain grammar underneath C2PA's media-specific one: every neemu record is an Entity; every extraction/derivation is an Activity with start/end, a named Agent (neemu build + model id + operator), and a `used` set = the exact content hashes consumed. This gives neemu chains that are auditable without inventing vocabulary, plus a clean export (PROV-JSON) for the estate's other programs.
- **Verdict: EMULATE** (as the chain grammar; C2PA as the media binding of it).

### 7.4 Content-gating design pattern — negative survey result
No published MCP server was found that implements **typed, code-level content refusals** (screen-fail / tier-fail / rights-fail as distinct, machine-readable refusal types) — the media MCPs in §1 either have no gate (§2.1), only size/format/domain limits (§1.1), or rely on the upstream cloud provider's policy (§1.2, §1.3). The only mature typed-gate designs found are in *voice platforms*, not MCP: Azure's Limited Access + consent-artifact verification (§3.1) and ElevenLabs' tiered CAPTCHA (§3.2). **neemu's typed-refusal gate has no MCP prior art to copy; its prior art is Azure's consent pipeline translated into MCP tool code.** That is a genuine novelty claim the spec can make, with those two citations as the lineage.

---

## Top 5 standouts

1. **Azure Custom Neural Voice Limited Access regime (§3.1)** — the only complete, published, *enforced* consent pipeline: written permission + advance disclosure + recorded consent statement + biometric same-speaker verification against the training audio + scoped approved use + retention. It is the design neemu's rights axis should be a translation of.
2. **C2PA actions/ingredients + W3C PROV (§7.2, §7.3)** — together they hand neemu a finished provenance vocabulary (ingredient/`parentOf`, region-scoped actions, Update Manifests for append-only re-stamping; Entity/Activity/Agent underneath) so neemu invents *no* chain grammar of its own.
3. **WhisperX + pyannote record shapes (§6.1, §6.2)** — segment→word→speaker nesting, RTTM export, recorded speaker-count priors, per-domain DER honesty. The DialogueLine/VoicePerformance spine is solved prior art (BSD-2 code).
4. **Chatterbox + AudioSeal (§3.3, §3.4)** — MIT-licensed cloning with an *always-on* watermark (PerTh), plus segment-localised watermark detection with a 16-bit payload slot for a derivation ID. Together: a derivation engine neemu can actually ship, whose outputs self-identify.
5. **The FER license trap, as a system (§5.1–5.3, §5.6)** — AffectNet (non-commercial, signed academic agreement), RAF-DB (non-commercial *including derived data*), FER2013 (no license ever granted), and Apache-2.0 FER *code* whose weights are AffectNet-trained. The standout is negative but it is the finding most likely to save neemu from a fatal tier error.

---

## SPEC DELTAS for neemu (each traceable to a find above)

1. **Consent-artifact object on the rights axis** (from §3.1 Azure): `ClonabilityAssessment.rights` must reference a `ConsentArtifact {written_permission_ref, disclosure_version_given, consent_recording_hash, consent_vs_training_speaker_match: verified|failed|not_run, approved_use_scope, retained_until}` — a boolean or checkbox-style attestation is a typed refusal (`RIGHTS_ARTIFACT_MISSING`), not a pass.
2. **Presence ≠ permission as separate fields** (from §3.2 ElevenLabs): add `presence_verified` (live/CAPTCHA-class proof) distinct from `rights_basis`; neither may substitute for the other in the deny-by-default conjunction.
3. **Gate strength scales with derivation fidelity** (from §3.2): neemu's derivation classes (characterisation-only → adapted derivation → high-fidelity) each name a minimum rights-evidence class; requesting a higher class than the evidence supports returns `REFUSAL_GATE_TIER`, not a downgraded result.
4. **Engine-license axis** (from §3.5 XTTS/CPML): add a third clonability check, `engine_license {engine, license, license_alive: bool, commercial_ok: bool}`, evaluated at derivation time; an engine whose weights are non-commercial or whose licensor no longer exists fails closed regardless of the voice's own rights/technical scores.
5. **Split license fields on every model-derived record** (from §5, §2.4): every FaceObservation / tag / accent record carries `code_license`, `weights_license`, and `training_data_licenses[]` as three separate fields; the record's tier is capped by the *most restrictive* of the three. A model with any field "not stated" (e.g. §5.7, §4.1) is registrable for analysis-tier experimentation only.
6. **FER training-lineage tier cap** (from §5.1–5.3): if a face model's lineage includes AffectNet, RAF-DB, or FER2013, its outputs are hard-capped at Tier A (analysis) and can never contribute to a Tier T (training-cleared) record; RAF-DB lineage additionally flags `derived_data_restricted: true`. CREMA-D-style consented-actor sources (§5.4) are the named positive example of a clearable lineage.
7. **FaceObservation output shape = multi-task tuple** (from §5.6 EmotiEffLib, §5.1 AffectNet schema): adopt `{categorical_8ch: probabilities, valence: [-1,1], arousal: [-1,1], au_set: [...]}` as the emitted shape, with neemu's 8-channel expression language as the categorical layer — plus a mandatory `basis: model-estimate` stamp and the producing model's lineage fields from Delta 5.
8. **Dialogue/VoicePerformance record = WhisperX nesting** (from §6.1): adopt `{segment{start,end,speaker}, words[{word,start,end,score,speaker}]}`; VoicePerformance features (pace, pause distribution, pitch/formant stats) are computed *from* the word level, and speaker is bound per-word so mid-line changes are representable. Add a `known_limitations` echo per record.
9. **AccentProfile schema additions** (from §4.1, §4.3, §4.4): profiles and matches store `distribution{accent: p}` + `top1` + `margin` (never top-1 alone), `expected_confusions[]` (e.g. US↔Canada), `intensity: 0..1` conditioning scalar, and `derivation: paired-recordings vector-difference` as a permitted profile provenance type. Any accent *model* registered in neemu must publish per-accent precision/recall in the dima806 report format (§4.2) — aggregate accuracy alone is a registration refusal.
10. **Identity-vs-descriptive confidence asymmetry** (from §2.4 pixai-tagger): two threshold classes in neemu's extraction config — identity-bearing observations (character identity, voice identity, accent top-1) require a higher confidence floor than descriptive observations (expression channel, dialogue act, tags); the floors are named constants in server config, recorded per record.
11. **Frequency-bucket aggregation for repeated observations** (from §2.1 gelbooru-mcp): when neemu aggregates observations across pages/clips of one work, emit `{feature, count, frequency}` buckets with the ≥~1.0 "near-universal" / <0.3 "situational" interpretation written into the schema doc — for faces, appearance, and recurring acts alike.
12. **Deterministic segmentation spine before semantics** (from §1.4 PySceneDetect, §1.3): scene records are created by a deterministic detector with recorded parameters + stats; semantic scene understanding (beats/machinery/dynamics) attaches to those scene IDs and may not create, merge, or split scenes. A separate no-model metadata record (duration/fps/codec/hash) is always emitted, basis `file-metric`, never merged with model-estimate fields.
13. **Provenance chain grammar = PROV, media binding = C2PA** (from §7.2, §7.3): every neemu record is a PROV Entity; every tool run is a PROV Activity (`used` = input content hashes, agent = neemu build + model ids); derivations add a C2PA-style ingredient assertion (`parentOf`) and an action whose parameters enumerate the mandatory adaptation deltas. Tier re-assessments and GapFindings are appended as C2PA **Update-Manifest analogues** — new linked records, never edits — preserving immutability.
14. **Watermark-at-derivation, with payload = derivation ID** (from §3.3 PerTh, §3.4 AudioSeal, §3.6 arXiv:2504.10782): a VoiceDerivation is not complete until its output carries an embedded watermark whose payload encodes the derivation record ID; post-hoc watermarking after the fact is prohibited by spec. On *ingest*, neemu runs watermark detection and C2PA-manifest reading as independent checks and records any disagreement as an `IntegrityClash` finding (arXiv:2603.02378) rather than resolving it silently.
15. **Annotations complete, gates in code — and say so in the spec** (from §7.1): all 16 tools publish full `ToolAnnotations` (reads: `readOnlyHint:true, openWorldHint:false`; content-addressed creators: `idempotentHint:true, destructiveHint:false`), and the spec cites the MCP blog's untrusted-hint clause as the normative reason every screen/tier/clonability refusal is a typed server-side refusal that no annotation, prompt, or client may override.
16. **Validate-first tool** (from §1.1): neemu's ingest tool returns a structured pre-flight record (accessibility, format, size/duration, hash, screen result) as a *separate* step whose record ID is a required input to every extraction tool — extraction tools refuse (`REFUSAL_NOT_SCREENED`) on a missing or failed pre-flight, making the screen un-skippable by construction rather than by convention.

**Gaps / unverified, for the record:** Danbooru-Turso MCP (§2.3) and andretisch Speech Audio MCP (§6.3) are index-listing-only and marked UNVERIFIED throughout. XTTS v2's CPML terms (§3.5) are consistently reported but the primary license text was not re-fetched (the licensor's page is reported 404). Several HF cards (CommonAccent ECAPA §4.1, mavleo96 whisper-accent §4.3, trpakov §5.7) genuinely carry **no license field** — that absence is itself a verified finding and is load-bearing in Deltas 5 and 9.


---

## 8. Enrichment prior art — the System-0 pass for thin census records (added 2026-10-27, Owner's follow-up)

**The problem this section answers.** The Mind Control census (chunks 1–2: `~/workspace/hypnosis-doujin-corpus-20261027/census/`) captured **512 of 700 works (73%) as AMBIGUOUS at listing level**, because doujins.com listing cards carry only title + series + artist + a cover thumbnail — no per-work tags. Under the screen law, the determination of record happens when a work is opened, so the ambiguity is honest; but opening 512 works blind is the expensive path. **Enrichment** is the cheap first pass that tries to attach full tag metadata to a thin record *before* any work is opened, so the open-queue is prioritised and many determinations arrive pre-loaded with cited evidence. Every candidate below was verified from primary documentation in this pass; the section ends with the pass design (§8.9) and its spec deltas (§8.10).

**The headline finding, stated once:** no surveyed tool returns determination-grade tags from the census's keys directly. Every working path is **two-hop** — a *lead* (an ID, a similarity score, a candidate match) resolved against a *source record* (a booru post, a gallery, a catalogue entry) — and only the source record is citable evidence. Two of the seven candidates cannot consume the census's keys at all. The corroboration rule in §8.9 is built around exactly this shape.

### 8.1 SauceNAO API — the only cover-image lead generator

- **Where:** https://saucenao.com (API at `search.php`; docs at `user.php?page=search-api`). Verification note: the fetch layer policy-blocked saucenao.com itself in this pass; the terms text below comes from the SauceNAO Legal page served on a mirror host (`saucenao.executeme.world/legal.html` — the operator link between hosts is **UNVERIFIED**), and the API shape is corroborated by multiple open-source wrappers whose code implements and cites the official API page.
- **Match key: image upload or image URL — exactly those two.** No title, text, hash, or work-URL mode exists. A `db`/`dbmask` bitmask selects indexes; `db=999` = all.
- **What a hit returns: source IDs, not tags.** Per result: `similarity`, a temporary thumbnail URL, `index_id`/`index_name`, external source URLs, title/author strings, and per-index IDs — booru family: `danbooru_id`, `gelbooru_id`, `e621_id` (+ yande.re/konachan/sankaku/anime-pictures/idol-complex IDs); doujin/manga indexes: MangaDex `md_id`, MangaUpdates `mu_id`, `mal_id`, `part`, `date` (H-Magazines, Madokami); **FAKKU, H-Misc (nH), H-Misc (eH), and 2D-Market return only a `source` string — no work ID at all**; anime indexes return `anidb_aid`/`anilist_id`/`mal_id` + episode. The *only* tag-adjacent payload anywhere is on booru hits: flat `material` (franchise) and `characters` strings — corroboration material, not a tag set. The full index list (~44 indexes, incl. #0 H-Magazines, #9 Danbooru, #16 FAKKU, #18 H-Misc nH, #25 Gelbooru, #29 e621, #36 Madokami, #37 MangaDex, #38 H-Misc eH) is a **2022 snapshot from a wrapper's index file; the current list is UNVERIFIED** (official page blocked).
- **Limits:** two windows (short per 30 s, long per 24 h), reported in every response header; wrapper-embedded examples for the entry account type show **4 / 30 s and 100 / 24 h** — treat SauceNAO as a *rationed* resource in pass design. The official free-vs-paid tier table is **UNVERIFIED**.
- **Terms:** usage limits by account type; multiple accounts/IPs to bypass limits "may result in a ban"; query images stored <~30 min then deleted; and the load-bearing clause — **"Do not rely on the validity of any results or information provided."**
- **Grade: advisory / lead only.** By its own terms, a SauceNAO hit is not evidence. It becomes determination-grade only after hop 2 resolves the returned ID at the source site (§8.2) and *that* record is cited.
- **MCP servers: verified negative.** No dedicated, credible SauceNAO MCP server exists. (The many "sauce-*-mcp" hits are Sauce *Labs*, a different company — a naming-collision trap.) The sole real repo, `CloudWaddie/osint-mcp` (MIT asserted in README, **no LICENSE file**; created 2026-03, 0 stars), carries one `reverse_image_search_anime` tool among ~40 OSINT tools — a reference implementation to read, not infrastructure to adopt. Neemu would call the API directly.

### 8.2 Hop 2 — resolving an ID at the source (the determination-grade step)

These are the records a lead resolves *into*; all verified in SITES.md (§a/§f) and cross-cited here rather than re-derived:

- **E-Hentai gdata API** (SITES §a): batch lookup by gallery id → the gallery's full namespaced tag set (the richest doujin vocabulary in the survey). A SauceNAO H-Misc (eH) hit yields only a `source` string, so reaching gdata requires locating the gallery on E-Hentai itself — the hop is real but not automatic.
- **nhentai gallery JSON** (SITES §a): gallery id → namespaced tags incl. the `tag` type. Same locating caveat.
- **Danbooru / Gelbooru post APIs** (SITES §f): post id → the post's complete tag set (Danbooru's categorised by namespace). This is the clean hop: `danbooru_id`/`gelbooru_id` from §8.1 resolve in one call.
- **MangaDex by `md_id`** (§8.4): id → the closed-taxonomy record.
- **Grade:** a resolved source record, on an identity-pinned match, is **determination-grade for that record** — it is the source site's own curated/community record of that work/post, cited with its match evidence. It never speaks beyond what the source records (a booru *post* is one page/image, not the whole work — see §8.6).

### 8.3 gallery-dl in metadata mode — the URL-keyed bulk resolver

- **Where / license:** https://github.com/mikf/gallery-dl (development now on Codeberg, §2.1), **GPL-3.0**.
- **Metadata-only operation — verified from its own options doc:** `-j/--dump-json` and `-J/--resolve-json` print JSON per item; `-s/--simulate` "simulate data extraction; do not download anything"; `--no-download` suppresses file downloads; selection options `--tags-blacklist`/`--tags-whitelist` filter by tags during extraction. The enrichment recipe is therefore `gallery-dl -j --no-download <work-or-post URL>` — the site's own metadata record, no media fetched.
- **Roster — verified from the current supported-sites document (fetched 2026-10-27):** Danbooru + instances, Gelbooru + its 0.1.11/0.2 instance families (**including Hypnohub**, hypnohub.net — a hypnosis-specific booru, directly on-census-theme), e621 family, rule34.xxx, Sankaku (incl. Books), MangaDex (Authors/Chapters/Covers/Manga), **the IMHentai mirror family** (imhentai.xxx, HentaiEra, HentaiRox, HentaiFox, HentaiEnvy, HentaiZap — capabilities: Galleries, Search Results, **Tag Searches**), nozomi.la (Posts, Tag Searches), Nijie (Doujin).
- **Verified absences (as of the document fetched):** **doujins.com is not a supported site**, and **the exhentai, hitomi, and nhentai extractors do not appear** in the current supported-sites document. Consequence: gallery-dl enriches at *cousin and mirror* sites once a work is located there (by title search on an IMHentai-family site, a nozomi lookup, or a booru post URL) — never at the census site itself, and its E-Hentai/nhentai lanes must go through §8.2's APIs instead. Extractor rosters change between releases; re-check before relying on either direction.
- **Match key: a URL on a supported site.** It does not search across sites by title or image; a human/process must supply the located URL (from §8.1's `source` strings, a mirror's own search, or a census run at the mirror).
- **Grade:** the JSON it emits *is* the source site's record for that gallery/post — determination-grade for that record, under the same identity-pinning requirement as §8.2.

### 8.4 MangaDex API — the title-keyed closed taxonomy

- **Where:** https://api.mangadex.org (docs at api.mangadex.org/docs). **Match key: title** (`GET /manga?title=…`, with tag/demographic/status/`contentRating` filters) **or a known UUID**. No image/hash matching; the cover is fetched *from* a matched record, not matched *by*.
- **Returns (verified against the live API):** per-title `contentRating` (`safe`/`suggestive`/`erotica`/`pornographic`), `publicationDemographic`, `status`, year, external `links` (AniList/MangaUpdates/MAL/…), and `tags[]` from a **closed taxonomy of exactly 77 tags** (`GET /manga/tag`) in four groups — `content` (warnings only: Sexual Violence, Gore), `format` (incl. **Doujinshi**, Self-Published, Oneshot, Anthology), `genre`, `theme` (incl. **Loli** and **Shota** as theme tags — screen-relevant flags in the site's own vocabulary). Coarse and mainstream by design: no per-act vocabulary.
- **Coverage honesty:** Doujinshi/Self-Published formats and `pornographic` ratings exist in the schema, but an entry exists only if someone created and uploaded it; the catalogue is overwhelmingly serialised mainstream manga/manhwa/manhua. A one-shot circle doujin census will mostly **miss** — and **a miss proves nothing** (absence = not catalogued, not safe).
- **Limits/terms:** ~5 requests/second/IP global cap (429 → temp ban on persistence); Acceptable Usage Policy: entirely free, **must credit MangaDex**, must credit scanlation groups and honour their removal requests, **no ads or paid services** on API-derived products. A service under an AUP — not a licensed dataset.
- **Grade:** a title hit is a **lead** (titles collide across circles/remakes); once identity is pinned by corroboration (alt-titles + author/artist + year + cover), the entry's `contentRating` + tags are **determination-grade for that entry**.

### 8.5 Hydrus + the Public Tag Repository — exact-hash only; a post-acquisition tool

- **Where:** https://hydrusnetwork.github.io/hydrus/ (docs: `client_api.html`, `PTR.html`); repo hydrusnetwork/hydrus. License: GitHub reports **Other (NOASSERTION)**; the README invites forking freely and distro metadata reports **WTFPL-2** — treat WTFPL-2 as corroborated, the LICENSE file text itself **UNVERIFIED** in this pass.
- **The PTR, verbatim posture from its guide:** a community-managed repository holding **only SHA-256 hash → tag mappings** ("The PTR only store the sha256 hash and tag mappings of a file, not the files themselves or any non-tag metadata"; ~54 million files tagged at writing). Clients sync the *entire* PTR locally; a local file whose hash matches a PTR entry displays that entry's tags (namespaces: `creator:`, `character:`, `series:`, `title:`, `meta:`, …). Additions pass automatically; deletions/siblings need janitor action — crowd tags, unreviewed on the way in.
- **Match key: the exact file's SHA-256 — confirmed, with no alternative.** No title, filename, fuzzy, perceptual, or thumbnail lookup exists in the documented mechanism. The guide's own FAQ is decisive for the census: tags exist only if another user uploaded them *for that hash*, and "Imgur, Reddit, Discord, and many other sites and services recompress images… which might give it a different hash even if it looks indistinguishable" — the PTR is unreachable from a cover thumbnail, which is a different (recompressed) file.
- **Client API:** localhost-only by recommendation (default port 45869, off by default, per-program access keys, active only while the client runs); `file_metadata` by `hashes` returns dimensions/mime/`known_urls`/ratings and `tags` split per tag-service into `storage_tags`/`display_tags` — a PTR hit surfaces through it as that file's tag set under the PTR service key.
- **Grade:** **unusable at census stage** (no file, no hash — lead value zero, not merely advisory). *If* the actual file is later lawfully acquired, a PTR hit is **determination-grade about that exact file's community tag set** (cryptographic file identity + the verbatim tag list; proving the record, with the crowd-tag caveat). Note the unit mismatch: PTR is **per-file** (per page image); a multi-page work has no single PTR identity.

### 8.6 Booru MCP servers — post-ID resolvers, not census tools

- **citronlegacy/gelbooru-mcp** (§2.1, re-verified at repo this pass; **MIT**, LICENSE file present): 7 tools. `get_character_tags` returns the §2.1 `{tag, count, frequency}` buckets; `search_posts` takes a full Gelbooru tag query **or a single post `id`**; `search_tags` returns tag name + post `count`. Match keys: **tag query / Gelbooru post ID / character tag** — no image, title, or work-URL input. Two documented traps: unauthenticated Gelbooru allows only **2 tags per query**, and `build_prompt`/`get_character_tags` **always filter `rating:general`** — the two tag-aggregating tools are blind to the adult tail by design.
- **gamzadongza/danbooru-tags-mcp** — verified this pass via its **Smithery registry record** (tool schemas), **GitHub repo still not opened; license UNVERIFIED**: 4 tools, of which **`get_post_tags` (Danbooru post ID or post URL → that post's full categorised tag set, live from the Danbooru API)** is the single cleanest hop-2 implementation in this whole section. `get_character_tags` is frequency analysis (advisory); `get_wiki_info` resolves a tag's meaning/aliases (vocabulary support for §2.2's implication discipline); `get_post_count` is counts only.
- **gamzadongza/danbooru-turso-mcp** — **re-checked: the GitHub repository could not be found or opened** (searches return only index listings; the Glama page renders its README without linking a repo). Index-level facts only: one tool, `collect_and_save` (character tag + max_posts → counts only: `collected`/`new_posts`/`updated_posts`), persisting `posts(id, tags, rating, score, fav_count, file_url, pixiv_id)` into the *user's own* Turso DB. A bulk-collector pattern (private tag DB, SQL-queryable afterwards), returning no evidence to the caller; its MIT claim remains **index-only, UNVERIFIED**.
- **Grade:** all three are **single-image-post** tools. A multi-page doujin *work* has no booru post identity in a thin census record, so none is even lead-generating until a work→post mapping exists (which is what §8.1's `danbooru_id`/`gelbooru_id` provide — for the specific page SauceNAO matched, usually the cover). Post-level tags then describe *that page*, and generalising them to the whole work is an inference the record must mark as such.

### 8.7 trace.moe — the video lane's identity hop

- **Where / license:** https://trace.moe; API docs `soruly.github.io/trace.moe-api`, repo soruly/trace.moe-api, **MIT**. The estate already holds a connector (`custom.trace-moe`): URL-only input, and its standing rule — similarity under ~0.9 is a lead to check by eye, not an identification — is adopted by this dossier unchanged.
- **Match key: an anime screenshot (upload, image URL, or an MPEG-7 ColorLayout vector).** Returns per result: `anilist` (AniList ID), `filename`, `episode`, `from`/`to`/`at` timestamps, `similarity` (0–1), and expiring preview image/video URLs. **No content tags, genres, or ratings exist in the schema.** The optional `anilistInfo` expansion adds only AniList's own identity fields (`id`, `idMal`, titles, synonyms, `isAdult`) — the docs' own instruction is to "make a second query to AniList API" for more. The docs also warn: "Similarity lower than 90% are most likely incorrect results."
- **Limits:** 100 requests/min/IP global; search quota by sponsor tier — **free 100/day** (no key, by IP), rising with sponsorship; `/me` reports usage. **Service ToS: UNVERIFIED** (no terms document in the docs set; the limits doc is the operative contract).
- **Grade:** determination-grade only for the narrow fact it proves (this frame = AniList work X, episode Y, moment T, at ≥0.90 similarity); **advisory for everything content-related** — tags come from resolving the AniList ID at AniList/AniDB. Domain limit: anime frames only; it cannot enrich a manga/doujin census record at all.

### 8.8 Grade summary

| Tool | Census key it consumes | Returns tags itself? | Grade |
|---|---|---|---|
| SauceNAO API | cover image / image URL | No — source IDs + URLs (booru hits add `material`/`characters` strings) | Lead; its own ToS disclaims reliance |
| Source APIs (gdata, nhentai, Danbooru/Gelbooru, MangaDex-by-ID) | an ID from hop 1 | Yes — the source's own record | **Determination-grade for that record**, identity pinned |
| gallery-dl `-j --no-download` | a located URL on a supported mirror/booru | Yes — the site's metadata record | **Determination-grade for that record**; doujins.com/E-Hentai/nhentai unsupported (verified) |
| MangaDex API | title (+ corroboration) | Yes — closed 77-tag set + `contentRating` | Lead → determination-grade for the entry once identity is pinned; a miss proves nothing |
| Hydrus PTR | **none** (needs the file / its SHA-256) | Yes — per-file tag set | Unusable at census stage; determination-grade per file post-acquisition |
| Booru MCP servers | post ID / tag query (not census keys) | Per-post yes; aggregates are frequencies | Post-level determination-grade via `get_post_tags`-class tools; aggregates advisory; Turso repo UNVERIFIED |
| trace.moe | a video frame (not a manga cover) | No — AniList ID + episode + timestamps | Provenance-grade for frame identity; advisory for content; video lane only |

### 8.9 The System-0 enrichment pass — proposed order and corroboration rule

Run per thin census record (title + series + artist + cover thumbnail URL), cheapest-and-broadest first, rationed resources last-resort per record:

0. **Normalise.** Record title variants (romaji/original/translated), series, artist/circle, and the cover URL as the pass's working keys. Every artefact the pass produces is stored as an `EnrichmentBundle` on the work record — per-source sub-records `{source, match_key, match_evidence, retrieved_at, payload}`, basis-stamped `enrichment`. Nothing in the bundle is a determination.
1. **MangaDex title search** (cheap, ~5 rps, no rationing pressure). On a candidate hit, pin identity by corroboration (step 6's rule) using alt-titles + author/artist + year + the record's own cover. Attach `contentRating` + the 77-tag set.
2. **SauceNAO on the cover thumbnail** (rationed ~100/day at entry tier → batch by priority; dbmask restricted to the doujin/booru/manga indexes). Collect every returned source ID/URL as a *lead* — `danbooru_id`, `gelbooru_id`, `e621_id`, `md_id`, `mu_id`, `source` strings — with its similarity.
3. **Resolve each lead at its source** (§8.2): booru post APIs for full post tags; MangaDex by `md_id`; E-Hentai/nhentai gallery located from the `source` string/title, then gdata/JSON. Each resolution is stored as its own cited sub-record — **the lead is never the citation; the resolved record is**.
4. **Mirror sweep.** Title-search the gallery-dl-supported mirrors (IMHentai family, nozomi.la) and any booru tag search for the work; for each located gallery/post URL, one `gallery-dl -j --no-download` run attaches that site's metadata record. (Video records take the parallel lane instead: frame still → trace.moe → AniList ID → AniList/AniDB record, §8.7.)
5. **Aggregate, don't average.** Merge sub-records into the bundle preserving provenance and contradictions verbatim. Map source vocabularies onto the screen's working categories only as *annotations* (E-Hentai namespaces ↔ booru namespaces ↔ MangaDex groups), never by overwriting the source payload.
6. **Corroboration rule (proposed, for the SPEC):** **two independent sources to pin identity; one source suffices to escalate; no enrichment result suffices to clear.** Concretely: (a) a tag set enters the bundle as *identity-pinned* only when two independent sources agree on the work's identity (e.g. MangaDex entry + a resolved booru/gallery record sharing title+artist, or a ≥threshold SauceNAO lead + its resolved record); a single-source match stays marked `unpinned`. (b) **Escalation is single-source:** any identity-pinned *or* unpinned-but-credible record carrying school-coding evidence in its own vocabulary (MangaDex `theme: Loli/Shota`, booru age/school tags, E-Hentai namespaces) raises an escalation flag on the work — enrichment can move a work *toward* EXCLUDE-proposed or mandatory work-open review, and flags are never cancelled by silence from other sources. (c) **Clearance is never enrichment's to give:** the bundle may re-prioritise the open-queue (AMBIGUOUS → CANDIDATE when identity is pinned, a full tag record is attached, and no escalation flag stands) but the determination of record still happens at work-open under the §2 screen gate, and enrichment never overrides an EXCLUDE.
7. **Post-acquisition tail (outside the census pass):** when a work is later lawfully acquired as files, Hydrus PTR lookup by SHA-256 (§8.5) attaches per-file community tags — recorded with the crowd-tag caveat — as a second, file-level enrichment layer.

Expected effect, stated honestly: steps 1–4 will resolve the mainstream, mirrored, and booru-posted fraction of the 512 ambiguous works and pre-load their determinations; the untranslated, unmirrored tail will stay AMBIGUOUS after the pass — that residue is the genuine work-open queue, now smaller and prioritised, not a failure of the pass.

### 8.10 Spec deltas — enrichment (continuing the §7 numbering)

17. **System-0 enrichment stage + `EnrichmentBundle` record** (from §8.1–§8.4, §8.9): the SPEC gains an enrichment stage between census and screening, and a `conte_enrich` operation (sibling to delta 16's validate-first pre-flight) that runs the §8.9 pass and stores the bundle — per-source sub-records with `source`, `match_key`, `match_evidence`, `retrieved_at`, and the verbatim payload, basis `enrichment` — on the WorkRecord. The bundle is advisory input to screening; no field of it is a determination, and the screen gate's typed refusals are unchanged by anything a bundle contains.
18. **The corroboration rule as gate text** (from §8.9 step 6): identity pinning requires two independent sources; escalation (school-coding evidence in any source's own vocabulary) is single-source and un-cancellable by other sources' silence; enrichment absence is never exculpatory (MangaDex/PTR miss semantics, §8.4/§8.5) and enrichment never clears a work or overrides an EXCLUDE — clearance remains the work-open screen's alone.
19. **Lead-vs-citation discipline for similarity services** (from §8.1, §8.7, extending delta 8's basis discipline): similarity scores, source IDs, and `material`/`characters` strings from SauceNAO-class services are stored as *leads*; a tag may enter a neemu record only from the resolved source record, which is what the record cites. SauceNAO's own no-reliance terms clause is quoted in the spec as the normative reason. trace.moe results carry the estate's existing threshold rule (≥0.90 identification, below = lead).
20. **Hydrus PTR as the sanctioned post-acquisition tag source — and only that** (from §8.5): when neemu lawfully holds a file, PTR lookup by SHA-256 via a local Hydrus client API is the specified per-file tag enrichment, recorded with its caveats (crowd-sourced, additions unreviewed, per-file not per-work). The spec expressly bars PTR from census-stage use (no file, no hash, no path in) so the hash-exactness limit is a designed boundary, not a discovered one. Likewise, booru tag lookups inside neemu are implemented against the source APIs directly (the §8.6 servers are read as protocol references — `get_post_tags`' shape, `get_character_tags`' frequency buckets — not adopted as dependencies; the Turso server's repo remains UNVERIFIED).

**§8 verification gaps, for the record:** saucenao.com was fetch-blocked in this pass — its API shape is wrapper-corroborated and its terms come from a mirror-hosted legal page whose operator link is UNVERIFIED; the current SauceNAO index list and paid tier table are UNVERIFIED. Hydrus's LICENSE file text was not opened (WTFPL-2 is distro-corroborated). gamzadongza's two servers remain GitHub-unopened (Smithery/Glama index records only; licenses UNVERIFIED). trace.moe publishes no service ToS in its docs set. gallery-dl's roster findings are as of the supported-sites document fetched 2026-10-27 and will drift with releases.
