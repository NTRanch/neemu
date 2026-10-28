# SITES.md — Adult animation/media sources for neemu

Pre-build dossier, 2026-10-27. Companion to `PRIOR-ART.md` in this
directory and to the neemu SPEC (`../SPEC.md`, §1 media types, §3 records).

**Question answered:** which websites/services span adult animation/media
matching neemu's media types (doujin/manga page sequences, video,
image+sound, audio-only) and would be useful to the program — as
acquisition routes, census targets, or vocabulary/metadata references.

**Method:** web search for discovery; `browser.open` only on non-explicit
public pages (about/FAQ/ToS/API-docs/wiki landing pages). No explicit
media or work pages were fetched; no work's content is reproduced —
this is a metadata-level survey of the *sites*. Load-bearing claims were
verified against a primary page actually opened wherever the fetch layer
allowed; anything not so verified is marked **UNVERIFIED**. Two fetch
limits were hit and are recorded rather than papered over: the EHWiki
API page was policy-blocked, and direct fetch of vndb.org /
danbooru.donmai.us was policy-blocked (VNDB claims rest on the opened
VNDB source/dump README mirror and Danbooru-software help pages).

**Standing caution that applies to every aggregator below:** no surveyed
aggregator (doujins.com, nhentai, Hitomi.la, HentaiHaven, hanime.tv,
Rule34Video, IMHentai) yielded an openable primary ToS page. "Scraping
posture" for them is therefore **unstated, not permitted**, and their
material stays Tier A (analysis) with per-work provenance, per the SPEC
§2 tier gate.

---

## Shortlist at a glance

| Cat. | Surveyed | Primary | Standout primaries |
|---|---|---|---|
| (a) Doujin/manga image-sequence | 6 full + 2 noted | 4 | E-Hentai/ExHentai (vocabulary), nhentai (metadata schema), FAKKU (licensed lane), doujins.com (in use) |
| (b) Adult anime video | 4 full + 1 cross-list | 2 | hanime.tv (catalogue + EN tag vocabulary), FAKKU streaming (licensed lane) |
| (c) Voiced doujin / ASMR / voice | 4 | 2 | **DLsite** (survey standout), Ci-en (creator-direct provenance) |
| (d) 3D animation & motion | 6 | 2 | Iwara (MMD video corpus), SmutBase/SFMLab (licensed rigged assets) |
| (e) Adult game / VN media | 4 full + 3 noted unsurveyed | 2 | DLsite (again), itch.io adult (indie lane, real API) |
| (f) Metadata / tag databases | 7 full + 1 mini | 2 | **VNDB** (dump + API + ODbL), E-Hentai/EHWiki (namespaced vocabulary) |

Category honesty: (b) is thin at the quality end (beyond the entries
below it is mostly interchangeable unlicensed tubes/clones); (c) is
genuinely thin outside Japan's DLsite/FANZA/BOOTH/Ci-en quadrangle —
there is no English storefront of comparable structure, and English
discovery largely points back to DLsite's English section.

---

## (a) Doujin/manga image-sequence sources and aggregators

### Doujins.com — PRIMARY (already in use)
- **URL:** doujins.com
- **Media types:** manga/image-sequence (English-translated manga and CG sets).
- **Program use:** acquisition route (analysis-tier only) + census target — the program's existing corpus source (tag-level census, ~1,300 works on one tag).
- **Access:** public browsing; premium membership exists (specifics UNVERIFIED).
- **ToS/scraping:** UNVERIFIED — no primary ToS/FAQ page located or opened.
- **Age verification:** UNVERIFIED — third-party scans note an adult warning; no hard verification evidenced.
- **Why primary:** commissioned and running; listing structure known; aggregator provenance caps it at Tier A permanently.

### nhentai — PRIMARY
- **URL:** nhentai.net
- **Media types:** manga/image-sequence.
- **Program use:** census target + vocabulary/metadata reference. Its gallery JSON model (titles in EN/JP/pretty forms, `num_pages`, upload date, typed tag arrays: artists, characters, parodies, groups, languages, categories, tags) is the cleanest ready-made gallery schema in this category. Also an analysis-tier acquisition route.
- **Access:** public; **no official documented API** — third-party clients describe themselves as clients for nhentai's *undocumented* APIs (the leading Python package describes itself as "made using python webscrapping"), returning full book/image/tag models.
- **ToS/scraping:** UNVERIFIED — no primary ToS page opened; the unofficial-scraping ecosystem implies no sanctioned bulk route.
- **Age verification:** UNVERIFIED (secondary sources claim 18+ confirmation; no hard verification evidenced).
- **License:** none stated — assume none granted. Tag vocabulary is flat/typed, **not** wiki-curated or aliased.
- **Why primary:** best-structured metadata of the free aggregators; vocabulary value is statistical (tag frequency), not definitional.

### E-Hentai / ExHentai — PRIMARY (vocabulary backbone)
- **URL:** e-hentai.org; sister site exhentai.org. API endpoint: `https://e-hentai.org/api.php` (also on ExHentai).
- **Media types:** manga/image-sequence galleries (doujinshi/manga scanlations, artist CG, cosplay).
- **Program use:** census target + the vocabulary/metadata reference *par excellence* for category (a): the **namespaced tag system** (`artist:`, `group:`, `parody:`, `character:`, `language:`, `female:`, `male:`, `mixed:`, `cosplayer:`, `other:`, `temp`/reclass) is the richest act/attribute ontology available for neemu's ACT/face registries. Tag definitions live on EHWiki. Analysis-tier acquisition via gallery/archive download.
- **Access:** E-Hentai public; **ExHentai requires an existing E-Hentai account** (without one it serves the blank "Sad Panda" page). **Official JSON API:** POST `api.php`, method `gdata`, gid/token list → category, titles (EN/JP), uploader, posted, filecount, filesize, rating, torrentcount, full namespaced tag list; ~25-gallery batch limit reported by client libraries. *(Caveat: the documenting EHWiki page was fetch-policy-blocked in this survey; endpoint and schema are corroborated by multiple independent client codebases — EhViewer, HappyPanda, TachiyomiSY — and by the (f) pass.)*
- **ToS/scraping:** UNVERIFIED as quoted ToS. Documented policy fact with direct screen relevance: in 2010 E-Hentai removed ~30,000 galleries depicting underage characters/bestiality under advertiser pressure and spun that material to ExHentai. **ExHentai-side material must be treated as higher-risk at census, never as pre-screened.**
- **Age verification:** account (ExHentai); E-Hentai side UNVERIFIED.
- **License:** no database license stated in any source opened — assume none granted.
- **Why primary:** the namespaced female/male/other vocabulary is the closest existing thing to neemu's act/content vocabulary for doujin; use via gdata per-gallery, not as a bulk dump.

### Hitomi.la — SECONDARY
- **URL:** hitomi.la
- **Media types:** manga/image-sequence **+ video** (adult-anime videos sourced from Hentai Haven, per the NamuWiki entry opened).
- **Program use:** census cross-check — a large E-Hentai mirror; useful for checking E-Hentai census completeness and for language coverage (KO/ZH/JA/EN/FR/RU/IT/ES…). Tag classification is E-Hentai-derived (`female/male/artist/character/series/language/type`, searched as `category:tag`).
- **Access:** public; no login or API evidenced (third-party downloader tools exist).
- **ToS/scraping:** UNVERIFIED — the opened reference page itself characterises the site as an unlicensed sharing site.
- **Age verification:** UNVERIFIED — none evidenced.
- **Why secondary:** mirror value and language breadth; zero provenance value beyond E-Hentai; analysis-tier only.

### FAKKU — PRIMARY (the licensed lane)
- **URL:** fakku.net
- **Media types:** manga/image-sequence **+ video** (licensed anime streaming) + games/books store.
- **Program use:** **acquisition route — the only category-(a) source whose provenance can support per-work rights-ladder / Tier-T evaluation** instead of permanent analysis tier. Post-2015 it publishes only officially licensed manga (unlicensed scanlations removed), holds a Wani Magazine partnership (Wani's catalogue in English), and a Kitty Media/Media Blasters streaming licence.
- **Access:** login + paid subscription for manga/anime; individual purchase for books/games; no public API found (UNVERIFIED — none surfaced).
- **ToS/scraping:** UNVERIFIED — no primary ToS page opened; the licensing posture above is the load-bearing fact and is well documented.
- **Age verification:** account (payment-backed subscription); releases rated 18+.
- **Why primary:** the provenance counterweight to every aggregator in this dossier.

### IMHentai — REFERENCE-ONLY
- **URL:** imhentai.xxx (domain per third-party snippets; not opened)
- **Media types:** manga/image-sequence.
- **Program use:** census cross-check only if a gap appears; duplicates nhentai/E-Hentai/Hitomi coverage with weaker documentation. Its footer (quoted in snippets) states it "offers fictional drawn artwork… does not create or sell any of the artwork."
- **Access / ToS / age verification:** public (UNVERIFIED in detail); no API; no primary page opened; snippet-level 18+ boilerplate only.
- **Why reference-only:** nothing it offers is unique; documentation is the weakest in the category.

*Also noted:* **Pururin / Tsumino** — legacy aggregators; Pururin largely unavailable since June 2020, successors unorganised tagging. **Avoid** as a primary source (stale, weak metadata). **DLsite** also sells manga/image-sequence works and belongs to the (a) acquisition lane — full record under (c)/(e).

---

## (b) Adult anime video (streaming/download)

### hanime.tv — PRIMARY (for b)
- **URL:** hanime.tv
- **Media types:** video (English-translated adult anime, streaming).
- **Program use:** census target + vocabulary/metadata reference for video. Its full tag list (reproduced in the opened NamuWiki entry) maps directly onto neemu's act/scene vocabulary — it includes Mind Control, Mind Break, NTR, POV, Vanilla, Censored/Uncensored, Filmed, Plot, plus character/presentation tags; some tags (e.g. loli/rape tags) are listed as removed. Sort by favourites/views/date/A–Z; tag blacklist and Any/All filtering documented. Acquisition: streaming only, analysis-tier.
- **Access:** public streaming; account exists (favourites/bookmarks account-linked); premium tiering **contested** (one current review says free/ad-supported with no paid tier, another lists ~$4.99/mo premium) — premium UNVERIFIED.
- **ToS/scraping:** UNVERIFIED — a Terms of Use page exists (referenced by site-statistics mirrors) but was not opened; a mirror's claim that it "does not block any known AI crawler" is secondary and UNVERIFIED.
- **Age verification:** UNVERIFIED in mechanism — listed as an adult site in the opened reference; no hard verification evidenced.
- **Why primary:** the dominant dedicated hentai-streaming catalogue with the most usable English video tag vocabulary; unlicensed, so analysis-tier only.

### HentaiHaven — SECONDARY
- **URL:** hentaihaven.xxx
- **Media types:** video (streaming tube, English-subbed series, since 2014).
- **Program use:** census cross-reference — the upstream source whose videos also surface on Hitomi.la and hanime.tv; useful for cross-site work-identity/dedup in a video census.
- **Access:** public, no signup/subscription ("completely free to browse"); **streaming only, usually no official download**; no API evidenced.
- **ToS/scraping:** UNVERIFIED. Domain instability and clone proliferation flagged by reviewers — a provenance hazard in itself; verify the canonical domain before any census.
- **Age verification:** UNVERIFIED — none evidenced.
- **Why secondary:** dedup value only; not an acquisition route.

### Rule34Video — SECONDARY / REFERENCE-ONLY (also surveyed under (d))
- **URL:** rule34video.com
- **Media types:** video only — short animated clips: 3D renders, SFM/MMD-style and 2D hentai-style parody animation from independent/fan animators (secondary claim: ~286,514 videos). Not licensed studio anime; no model/motion assets.
- **Program use:** census target + vocabulary reference for the **fan-animation stratum** specifically — organisation is character + franchise keyed ("a lookup engine for a single character's animated appearances", per the opened AdultRankly review), which maps to neemu character/face-identity census for fan works, a different population from commercial hentai. In (d) terms: motion/act-staging reference for the wider non-MMD 3D scene.
- **Access:** public streaming, no account needed to watch; upload requires registration; no public API documented (UNVERIFIED).
- **ToS/scraping:** ToS;DR's case index (opened, https://edit.tosdr.org/cases/315) quotes its terms permitting users to share/view adult content; no scraping clause verified (UNVERIFIED). Secondary sources report AI-generated content was banned in 2025 and split to rule34gen.com.
- **Age verification:** UNVERIFIED — no age-gate page opened.
- **Why secondary:** huge English volume and a distinct population, but flat tag-only organisation and aggregator/re-upload provenance is the weakest in the survey — heaviest screening load, analysis-tier only.

### FANZA (DMM) — video arm — SECONDARY
- **URL:** dmm.co.jp (adult brand FANZA; video arm video.dmm.co.jp)
- **Media types:** video (commercial adult video incl. anime titles); doujin manga/games/voice in sibling sections (see (c)).
- **Program use:** licensed acquisition route (JP) + census target — Japan's largest commercial adult platform; its catalogue structure (video / PC games / doujin) is a census spine for *commercial* Japanese works, complementing the unlicensed Western tubes.
- **Access:** DMM account + purchase/subscription; Japanese-first UX; no public API evidenced (UNVERIFIED).
- **ToS/scraping:** UNVERIFIED — no primary ToS page opened (help pages exist under help.dmm.co.jp).
- **Age verification:** account-based R18 gating (R18 sections are a separate branded area); hard ID verification UNVERIFIED.
- **Why secondary:** licensed and comprehensive, but JP-only UX and no evidenced API make it a manual acquisition lane, not a programmatic census source.

### FAKKU streaming — PRIMARY as (b)'s licensed lane (cross-listing)
Full record under (a). Its Kitty Media streaming licence makes it the **only licensed English adult-anime streaming route** found in (b); subscription-gated, 18+. Secondary to hanime.tv on catalogue size, primary on provenance.

---

## (c) Voiced doujin / image+sound / adult ASMR & voice-work sources

### DLsite — PRIMARY (standout of the whole survey)
- **URL:** dlsite.com / dlsite.jp (adult doujin arm historically "DLsite Doujin R18" / maniax)
- **Media types:** **all four neemu types as purchasable downloads** — audio-only + image+sound (voice/ASMR works, typically WAV, often with cover illustration and sometimes scripts), manga/image-sequence, video, and games (VN/eroge incl. animated CG).
- **Program use:** **acquisition route (licensed purchase) + census target + vocabulary/metadata reference — the centre of category (c) and a primary in (a) and (e) as well.** Work records natively carry the exact fields neemu's voice lane needs: `product_id` (RJ/BJ/VJ-prefixed codes), `site_id`, circle/maker, `age_category` with explicit **R18** value, `work_type` including **`VOICE_ASMR` ('SOU')**, **`voice_actor[]`** credits, genre list (Healing / Dirty Talk / **Binaural / ASMR**…), `file_format` (WAV), file size, page count. RJ-code + circle + Work Format + Genre is the cleanest structured product metadata of any adult store surveyed.
- **Access:** public catalogue browsing; **account + purchase to download** (guest purchasing abolished per secondary buying guides — that specific claim is secondary-sourced). **No official public developer API found (UNVERIFIED)**; however a product JSON endpoint pattern is documented by third-party clients: `https://www.dlsite.com/maniax/api/=/product.json?workno={rjcode}&locale={locale}` (the `dlsite-async` PyPI client's page was opened and verifies the record shape above).
- **ToS/scraping:** UNVERIFIED as quoted ToS — no primary terms page opened. Two documented platform facts: (i) DLsite/Ci-en **banned AI-generated manga/CG/video works in 2023**; (ii) **Visa/Mastercard/Amex payments suspended since April 2024** — JCB / viviON Points are the working acquisition routes. Both are real lane constraints.
- **Age verification:** account + R18 sectioning (works carry an explicit R18 age category; storefront separates All-Ages / Adults (R18) / Otome-BL entrances); hard ID verification UNVERIFIED.
- **Why primary:** the only source that is simultaneously a lawful purchase route, a structured census target, and strong on animation **and** voice; the Voice/ASMR catalogue is the richest legal source of adult voice works for neemu's voice-characterisation lane; purchase-based provenance is exactly the rights posture the program requires.

### Ci-en — PRIMARY for provenance (secondary for scale)
- **URL:** ci-en.net (DLsite-linked creator-support platform; creator pages also under ci-en.dlsite.com)
- **Media types:** audio-only + image+sound + video as subscription perks (short voice works, situation voice, free-talk audio, "NG"/blooper voice, Ci-en-exclusive radio voice, birthday voice/video messages, broadcast archives), alongside manga/game perks.
- **Program use:** acquisition route (creator-direct — licensed by definition) + provenance reference. **Verified on Ci-en's official creator page (opened):** DLsite's fan-support platform; monthly free/paid plans and one-off articles; its model cases are voice-first (¥500 plan: short voice distribution; ¥1,000: situation-voice requests + main audio; ¥3,000: exclusive radio voice); sales settle monthly (bank transfer or viviON-point conversion into the creator's DLsite account); fee table explicitly splits all-ages vs **R18** (monthly plans 10% / **13% R18**; crowdfunding 10% / 15%; one-off 15% / 15%).
- **Access:** account + per-creator subscription/one-off purchase; deep DLsite account linkage; no API evidenced.
- **ToS/scraping:** UNVERIFIED as quoted ToS (the opened page is the creator/about page, not terms). R18 is a first-class, fee-differentiated platform category — verified above.
- **Age verification:** UNVERIFIED (R18 plans exist as a category; mechanism not on the opened page).
- **Why primary-for-provenance:** smallest catalogue in (c), but every work is creator-direct and attributable — the strongest provenance tier available in (c) and the best candidate pool for rights-cleared voice work (see §"Program implications", point 3).

### FANZA Doujin (DMM) — voice — SECONDARY
- **URL:** dmm.co.jp (doujin section)
- **Media types:** audio-only / image+sound (doujin voice), plus doujin manga/games/video.
- **Program use:** licensed acquisition route (JP) + census cross-check — named with DLsite and BOOTH as one of the three ESDs through which doujin voice is mainly distributed ("especially DLsite", per the opened "doujin voice" reference). Main value: cross-validation for works released on both DLsite and FANZA, and FANZA-only circles.
- **Access:** DMM account + purchase; no API evidenced (UNVERIFIED).
- **ToS/scraping:** UNVERIFIED — no primary page opened. One secondary comparison notes FANZA Doujin permits AI-generated works **with disclosure and a cap of 3/month per owner** — the opposite posture from DLsite's ban; worth recording in per-source rights notes.
- **Age verification:** account-based R18; details UNVERIFIED.
- **Why secondary:** the #2 licensed voice store; census value is mostly DLsite cross-validation.

### BOOTH (pixiv) — SECONDARY
- **URL:** booth.pm
- **Media types:** audio-only / image+sound (download voice/drama/music files), plus manga/image-sequence downloads, 3D models, physical goods.
- **Program use:** creator-direct acquisition route + long-tail census target. Verified on the opened BOOTH reference entry: consignment/creator-market e-commerce operated by Pixiv Inc.; individuals open shops without complicated procedures; membership necessary; JA/ZH/KO/EN; low fees (5.6% + ¥45/order vs the 30–40% attributed to DLsite/DMM) — why small voice circles often sell on BOOTH first or exclusively.
- **Access:** pixiv/BOOTH account + per-item purchase (download delivery); no API evidenced.
- **ToS/scraping:** UNVERIFIED as quoted ToS — **but a material policy change is documented on the opened page**: as of September 2025 censorship was strengthened and several R-18 creators had accounts suspended; an October 2025 section describes R-18 products suddenly taken down or rejected; as of May 2026 PayPal is unusable for R-18 works (PayPal adult-content policy).
- **Age verification:** account (pixiv R-18 settings model); hard verification UNVERIFIED.
- **Why secondary:** good creator-direct provenance and a real long-tail voice catalogue, but the 2025 R-18 enforcement wave makes it an unstable primary route — census it, don't depend on it.

---

## (d) Adult 3D animation & motion communities (MMD / Blender / SFM-class)

### Iwara — PRIMARY (motion reference / census)
- **URL:** iwara.tv
- **Media types:** video (MMD / 3D animation, MP4/HLS per secondary downloader FAQ); image galleries also reported. No native PMX/VMD distribution — models/motions are linked externally by creators.
- **Program use:** motion reference + census target — the canonical R18 MMD video corpus; useful for pose/motion, camera and act-staging reference, and for censusing which models/motions a video credits. Not a clean acquisition route (re-upload rights are creator-by-creator).
- **Access:** public browsing; login for upload and some restricted items (secondary). No public API documented — UNVERIFIED.
- **ToS/scraping:** UNVERIFIED from a primary page — no Iwara ToS page opened. Secondary (NamuWiki): uploads must be complete original works, explicit content requires R18 tags, creators retain ownership; a reported 2026 policy change moves AI content to a separate subdomain and prohibits depictions of real people.
- **Age verification:** separate SFW / "Ecchi" sides switched on-site; 18+ with age confirmation at registration per secondary sources — primary wording UNVERIFIED.
- **Why primary:** largest dedicated R18 MMD video community; rights and ToS must be re-verified on-site before any acquisition use.

### SmutBase / SFMLab / Open3DLab network — PRIMARY (reference / licence-checked asset route)
- **URL:** smutba.se; sfmlab.com (about page opened: https://sfmlab.com/about/); operator news at open3dlab.com
- **Media types:** motion/model **assets**, not finished video — "models, textures, sceneries, HDRis and other NSFW resources for machinima filmmakers" (SmutBase); SFMLab FAQ (opened): creators "can upload 3D resource files, such as FBX, Blender, Maya or 3ds Max files… free to download for anyone"; complete, often rigged, textured characters.
- **Program use:** motion/model reference and — **only with per-project licence compliance** — asset acquisition. This is the one (d) source that yields usable rigs rather than pixels. Also a vocabulary source: its 4-type tag system (General/Misc, Character, Property/Universe, Software incl. version) is documented in the opened FAQ.
- **Access:** public download; account required to upload and to use the sites at all. No API documented.
- **ToS/scraping — EXPLICITLY ANTI-SCRAPING, verified on the operator's own post (opened):** a crawler "did not respect our robots.txt, (which it did retrieve), ignored our Terms of Service, (which forbid scraping), and defied the creators' licenses: 423 of the 712 projects are licensed for non-commercial use only." Licence is per-project (creator-chosen), never site-wide.
- **Age verification — verified (SFMLab FAQ, opened):** "No. You must be at least 18 years old to use this site"; all three sites adults-only per operator post; nude underage-appearing character models prohibited.
- **Why primary:** real rigged assets with per-project licences and provenance — and a standing warning example of what *not* to crawl. Manual, licence-checked acquisition only.

### BowlRoll — SECONDARY
- **URL:** bowlroll.net
- **Media types:** motion/model assets — 3D models, motion data, accessories, tools, VSQs and other VOCALOID-related resources (MMD Guide, opened: "Probably the most popular service… a Japanese website allowing creators to share their 3D models, motions, VSQs…"). In MMD practice: PMX/PMD models, VMD motions (format-level claim is community-standard but UNVERIFIED on a BowlRoll page itself).
- **Program use:** motion reference / asset census — the main Japanese distribution point for the MMD motions and models Iwara videos are built from; essential for tracing a video's motion/model provenance.
- **Access:** public, Japanese-only interface; free downloads, account needed for some files; many files password-gated; files marked "Expired" have had distribution ended by the creator.
- **ToS/scraping:** UNVERIFIED — no terms page opened; per-file creator readme terms govern most MMD assets in practice.
- **Age verification:** UNVERIFIED — none documented; the site is general MMD, not adult-only, so adult-coded screening applies per file.
- **Why secondary:** indispensable provenance/motion census target, but Japanese-only, password/expire mechanics, and readme licences make it a poor automated acquisition route.

### Niconico (Nico Nico Douga) — REFERENCE-ONLY
- **URL:** nicovideo.jp (user RSS and `ext.nicovideo.jp/api/getthumbinfo` endpoints cited in ArchiveTeam wiki)
- **Media types:** video (the original MMD publishing venue; model-distribution videos; Seiga is the sister still-image service).
- **Program use:** vocabulary/provenance reference only — its **content tree** is a genuine provenance graph (model → motion → video) that neemu's provenance model can learn from; also the historical census root for MMD.
- **Access:** public (account no longer required to watch, per ArchiveTeam, 2018); per-user RSS and a thumbinfo API exist.
- **ToS/scraping:** UNVERIFIED for scraping. Material changes verified in search results: from January 2025 stricter content regulation has removed certain MMD and R-18 videos; October 2024 brought foreign-viewing/region restrictions.
- **Age verification:** UNVERIFIED in sources opened; R-18 handling has tightened rather than stabilised.
- **Why reference-only:** provenance-graph model and historical value, but R-18 MMD is being removed and region-restricted — no longer a reliable adult acquisition or census route.

### Patreon (creator-platform lane) — SECONDARY
- **URL:** patreon.com (Community Guidelines opened: https://www.patreon.com/en-GB/policy/guidelines)
- **Media types:** whatever a creator posts behind a paywall — animation video files, image+sound, audio-only, downloadable asset packs (models/motions) as patron rewards. No standard format; delivery is per-creator.
- **Program use:** acquisition **only by subscribing/buying from a specific creator** (a real rights transaction with that creator) + census of who produces adult 3D animation. Not a scrape target.
- **Access:** login + paid membership per creator; no content API for third-party ingestion.
- **ToS/scraping posture (guidelines verified on the opened page):** Adult/18+ works "must reside behind a Patreon subscription paywall"; Adult/18+ creators complete a verification process attesting they are at least 18; animated/illustrated adult subjects must be unmistakably adults. **Direct program-relevance flag:** the guidelines prohibit depictions of participants/characters who cannot give consent, expressly including characters "under the influence of alcohol, drugs, **hypnosis** or blackmail," and ban "consensual non-consent" depictions — a large slice of mind-control-coded work is off-platform there, which bounds Patreon's usefulness for this program's core theme.
- **Age verification:** creator-side age attestation/verification for the Adult/18+ category; public-facing pages must be free of explicit imagery.
- **Why secondary:** legitimate per-creator purchase/commission route, but no catalogue structure, no API, and consent-policy lines that exclude some target themes.

*(Rule34Video is recorded in full under (b) — in (d) terms it is the wider non-MMD 3D-animation census/motion reference; verdict there: secondary.)*

---

## (e) Adult game / visual-novel media sources (animation/voice-bearing)

### DLsite — PRIMARY (see full record under (c))
In (e) terms: games (VN/eroge incl. animated CG) are one of its four media lanes, with the same RJ-code metadata discipline. It is the only (e) source that is simultaneously a lawful purchase route, a structured census target, and strong on animation **and** voice.

### itch.io (adult section) — PRIMARY (indie lane)
- **URL:** itch.io (Creator FAQ opened: https://itch.io/docs/creators/faq; API docs at itch.io/docs/api/oauth)
- **Media types:** downloadable/browser games (incl. VNs with animation and voice), plus comics, books, soundtracks, asset packs. Creator FAQ (opened): classifications include "tools, comics, books, soundtracks, assets, and physical games"; "you can upload any kind of project for any kind of platform… users download the files exactly as you uploaded them."
- **Program use:** per-title purchase/download acquisition route (files unmodified, **no DRM imposed by itch.io** — verified in the opened FAQ) + census target for indie adult VNs/games absent from DLsite/Nutaku.
- **Access:** public pages; login to buy/download restricted items. **Real API:** OAuth 2.0 (authorization-code + PKCE, device/QR grant) and a server-side API, with scopes incl. `game:view:ownership` and `game:view:uploads` ("download files for games the user owns and free games") — per itch.io's own developer update.
- **ToS / adult policy (verified on the opened Creator FAQ):** "We don't allow hosting content that includes sexualized images or videos of real-life humans. Fictional, illustrated, and rendered content is generally fine, assuming it's legal… Depictions of minors, minor-presenting, or suggested minors in a sexual context are not allowed and will result in account suspension," plus a payment-processor prohibited-themes list for **paid** works (non-consensual implied, incest/pseudo-incest, rape/coercion, bestiality, underage/"barely legal", etc.). **Volatility warning (verified in search results):** in July 2025 itch.io de-indexed NSFW titles en masse under payment-processor pressure and later re-indexed free NSFW works first — itch.io census data can vanish from search without the works being deleted.
- **Age verification:** none documented in the opened FAQ — UNVERIFIED; NSFW flagging is creator-applied and user display settings gate visibility (secondary).
- **Why primary (indie lane):** lawful, DRM-free downloads and a real API; census by direct project URLs, not by (unstable) search.

### Nutaku — SECONDARY
- **URL:** nutaku.net
- **Media types:** games only in neemu's terms — browser and downloadable adult games/VNs (animation and voice inside the games; no standalone manga, video, or audio-only catalogue). Publisher copy (quoted in search results): "the world's largest English gaming platform dedicated to 18+ titles… over 150+ games" (dated publisher copy).
- **Program use:** census target for the English commercial adult-game market + a developer/publishing route; weak as an extraction source (browser builds/clients, not downloadable media files).
- **Access:** login (free account, freemium "Gold"). No public API documented — UNVERIFIED.
- **ToS/scraping:** UNVERIFIED — no terms page opened.
- **Age verification:** 18+ platform; users must confirm legal age before accessing content (secondary) — primary wording UNVERIFIED.
- **Why secondary:** important English-market census node and VNDB cross-check; closed delivery makes it census/reference only for extraction.

### F95zone — REFERENCE-ONLY (avoid for acquisition)
- **URL:** f95zone.to *(no primary page opened — all facts from secondary guides, flagged accordingly)*
- **Media types:** forum threads linking adult games/VNs (Ren'Py, RPGM, Unity…), mods, comics — games with animation/voice, delivered via third-party file hosts, not hosted media. UNVERIFIED at primary level.
- **Program use:** census/identification reference **only** — its thread taxonomy (engine, development status, genre/feature tags, version numbers) is genuinely useful for knowing what exists and which version is current. **Not an acquisition route:** secondary guides describe user "request/share" practices and one guide flatly calls it a pirated site; provenance there does not clear neemu's rights ladder.
- **Access:** forum, registration for most sections (secondary). No API.
- **ToS/scraping:** UNVERIFIED. **Age verification:** 18+ forum with age confirmation on registration (secondary) — UNVERIFIED at primary level.
- **Why reference-only:** best-in-class discovery/version census for Western indie adult games; disqualified as a file source by sharing/pirated provenance.

*Category (e) gap note (honest, not thinness):* VNDB-listed commercial stores beyond the above — **JAST USA, Kagura Games, Johren** — were **not surveyed / UNVERIFIED** in this pass. The category is not thin; they are the known remainder if the Owner wants full (e) coverage.

---

## (f) Adult-media metadata / tag databases (census & vocabulary)

> **Enrichment cross-reference (added 2026-10-27):** how these databases (and SauceNAO, the MangaDex API, gallery-dl's metadata mode, and the Hydrus PTR) combine into a **System-0 enrichment pass** that attacks the census's thin-record problem — listing cards carry title + series + cover only, leaving 512 of 700 works (73%) AMBIGUOUS at listing level — is designed in **PRIOR-ART.md §8**, including the per-tool match keys, the lead-vs-determination-grade analysis, the query order, and the proposed corroboration rule (two sources to pin identity, one to escalate, none to clear). The records below remain the per-source reference for the databases that pass resolves into.

### VNDB — The Visual Novel Database — PRIMARY
- **URL:** vndb.org (dump page https://vndb.org/d14); API at https://api.vndb.org/kana *(both URLs verbatim from sources; direct fetch of vndb.org was policy-blocked — claims rest on the opened VNDB source/dump README via the code.blicky.net mirror and on client libraries)*
- **Media types:** metadata only — visual novels, releases, producers, staff, characters, **tags** (content/sexual-content tags on VNs) and **traits** (on characters).
- **Program use:** vocabulary/metadata reference **and** census target — the strongest single source in (f): VN tags/traits are a curated, hierarchical, community-moderated vocabulary with spoiler levels, directly mappable to neemu's face/act/character fields; release entries link works to the stores in (e).
- **Access — both, verified:** (i) a real-time public JSON **Kana API** (api.vndb.org/kana, filters/fields documented, referenced by multiple client libraries); (ii) a full **database dump** — dump README (opened): "Each table is exported as a separate file in PostgreSQL's COPY format… a script is provided to load the data into a PostgreSQL database."
- **ToS/scraping posture:** use the dump/API rather than scraping — the dump exists precisely for bulk use. Dump privacy clause (opened README): republishers should "synchronise regularly and remove data that is not present anymore" (users can opt lists/votes out).
- **Age verification:** none for the database/API itself — UNVERIFIED for the website's adult-image display settings; metadata incl. sexual-content tags is not age-gated in the dump.
- **License — VERIFIED (dump README, opened):** database under the **Open Database License (ODbL)**; individual contents under the **Database Contents License (DbCL)**. Exceptions: Wikidata data CC0; **anime data obtained from AniDB is CC-BY-NC-SA 4.0**; VN/character descriptions gathered from online sources "may be subject to separate license conditions."
- **Why primary:** machine-readable dump + API, curated/aliased tag+trait vocabulary, and a stated open licence. The model answer for (f), and the spine for (e) census cross-referencing.

### E-Hentai / EHWiki — PRIMARY (vocabulary) — see also (a)
- **Program use in (f):** vocabulary/metadata reference — the namespaced tag system is the de-facto standard for doujin content description; the official **gdata API** returns tags in `namespace:tag` form (namespaces verified across sources: artist, group, parody, character, female, male, mixed, language, cosplayer, other, temp/reclass). Tag definitions live on EHWiki (landing page not opened — curation-workflow details UNVERIFIED).
- **Access:** official JSON API (POST api.php, method `gdata`). No bulk dump found — UNVERIFIED (none documented).
- **ToS/scraping:** UNVERIFIED — no terms page opened; the gdata API is the sanctioned machine route.
- **Age verification:** UNVERIFIED in sources opened.
- **License:** none stated — assume none granted.
- **Why primary (vocabulary):** use via gdata per-gallery, not as a bulk census dump.

### Danbooru — REFERENCE-ONLY (but first-class reference)
- **URL:** danbooru.donmai.us *(direct fetch policy-blocked; claims rest on the Danbooru-software help page opened via a sakugabooru clone of the help text and on client docs)*
- **Media types:** metadata only **for this dossier** — per the survey brief, Danbooru is assessed for tag vocabulary only.
- **Program use:** vocabulary/metadata reference — the gold-standard **curated** tag system: typed tags (artist / character / copyright / general / meta), a per-tag **wiki** with definitions, **aliases** and **implications**. Verified on the opened help page: "Danbooru tries to fix this issue by using tag aliases. You can alias one or more tags to one reference tag… Tags are normalized before they are saved to the database… While anyone can suggest an alias, only an administrator can approve it," with implications for non-equivalent broader relations.
- **Access:** **REST JSON API** (e.g. `https://danbooru.donmai.us/posts.json?tags=…`); anonymous reads work, login + API key raises limits. No official bulk dump verified — UNVERIFIED.
- **ToS/scraping:** UNVERIFIED from a primary terms page (fetch-blocked). Client-ecosystem evidence shows operators actively block scraper/bot traffic on the image CDN — assume API-with-key + rate limits is the only sanctioned machine route.
- **Age verification:** ratings system (s/q/e in API docs) rather than age verification — site wording UNVERIFIED.
- **License:** none stated in sources opened — assume none granted (tag *names* as factual data are one thing; wiki prose should be treated as unlicensed until checked).
- **Why reference-only (first-class):** adopt its alias/implication/wiki-definition **governance model** and cross-walk neemu's vocabulary to its general tags; do not build on it as an adult census source.

### Gelbooru — SECONDARY
- **URL:** gelbooru.com (API documentation mirrored at telegra.ph/Gelbooru-Posts-08-09, opened)
- **Media types:** metadata only for neemu (the site hosts images incl. explicit-rated posts; some video posts — video claim UNVERIFIED).
- **Program use:** vocabulary/metadata reference + census target for the explicit-rated booru corpus that Danbooru's posture excludes — where Danbooru-style vocabulary is actually applied to adult works at scale.
- **Access — DAPI, verified on the opened documentation mirror:** `/index.php?page=dapi&s=post&q=index`, hard limit 100 posts per request, `pid` paging, `json=1` for JSON; parallel comment/tag endpoints. Full use requires `api_key` + `user_id` (client comparison table); unauthenticated search limited (2 tags per an MCP wrapper's notes — see PRIOR-ART §2.1).
- **ToS/scraping:** the DAPI is the sanctioned route; no separate scraping clause verified — UNVERIFIED. Vocabulary is **flatter and less curated** than Danbooru's: "only three types: artist, character, and copyright" (plus general) on the opened page; alias mechanics UNVERIFIED.
- **Age verification:** ratings-based (general/questionable/explicit in search syntax); no age verification documented — UNVERIFIED.
- **License:** none stated — assume none granted.
- **Why secondary:** real API + adult-applied tags make it a usable census/vocabulary cross-check, but neemu should normalise against Danbooru/E-Hentai, not against Gelbooru.

### nhentai — SECONDARY (in (f); full record under (a))
- **In (f):** census target + lightweight vocabulary cross-check — typed tags (`tag, category, artist, parody, character, group, language`, verified from the Dart client library) give an independent tagging of the same works E-Hentai carries. Machine access is an unofficial scrape-based API ecosystem; no licence stated; vocabulary is **not** wiki-curated/aliased. Value is statistical (what tags are actually used), not definitional.

### AniDB — REFERENCE-ONLY
- **URL:** anidb.net (API documented at wiki.anidb.net/w/API; HTTP API base `http://api.anidb.net:9001/httpapi` — verbatim from client docs)
- **Media types:** metadata only — anime (incl. adult anime) titles, episodes, files, groups, characters/tags, at episode- and **file-level** granularity (codecs, hashes) that MAL-style databases lack.
- **Program use:** vocabulary/metadata reference for the **video lane** — identifies the exact release/file and episode structure when neemu ingests an adult anime work; also the upstream source of VNDB's anime data.
- **Access:** two registered-client APIs (verified from client documentation): UDP API (bulk, authenticated, registered client name/version + API key) and HTTP API (`request=anime&client=…&clientver=…&protover=1&aid=…`), plus a downloadable anime-titles dump for title resolution. Registration is free but mandatory — AniDB "will not authenticate an unregistered client."
- **ToS/scraping — strictest in (f), verified from multiple client docs:** flood protection ≈1 request / 2 s (UDP ≤0.5 packets/s after burst, ~4 s spacing), mandatory caching (shortest cache 1 day); violations earn a **temporary IP ban**, not an error. Web scraping is not the intended route at all.
- **Age verification:** none for the API/database — UNVERIFIED for the website; adult titles are in the same database, flagged by category/tag.
- **License:** via the opened VNDB dump README, AniDB-sourced data is **CC-BY-NC-SA 4.0** — non-commercial, share-alike; that licence travels with the data into any neemu derivative. AniDB's own site licence statement was not separately opened — UNVERIFIED.
- **Why reference-only:** indispensable for identifying adult **anime video** releases; useless for doujin/manga or act/face vocabulary; its rate regime makes it a lookup service, never a census crawl.

### The Doujinshi & Manga Lexicon — REFERENCE-ONLY
- **URL:** doujinshi.org (now doujinshi.wiki as backup; Fanlore page opened: https://fanlore.org/wiki/The_Doujinshi_%26_Manga_Lexicon)
- **Media types:** metadata only — "a site intended to collect and disseminate information about doujinshi and other doujin works… **It is intended purely to be a resource, not a site for downloading or sharing doujinshi themselves**" (Fanlore, quoting the site).
- **Program use:** census/authority reference for **bibliographic identity** — exactly neemu's provenance pain: "the same author may use multiple names; the same Doujin Circle may undergo name changes…" Circle/author authority control for works E-Hentai/nhentai tag inconsistently. Claimed scale "over 1,500,000 items."
- **Access:** public website search. No API or dump found — UNVERIFIED (none documented).
- **ToS/scraping:** UNVERIFIED. **Currency risk, verified (Fanlore):** dates "2005 – July 23, 2022" for doujinshi.org, with doujinshi.wiki as "backup, currently online."
- **Age verification:** none documented (bibliographic database covering pornographic and non-pornographic doujinshi) — UNVERIFIED for any on-site gate.
- **License:** none stated — assume none granted.
- **Why reference-only:** unmatched for circle/author disambiguation in provenance records, but frozen/archived since 2022 and API-less — a manual authority check, not a pipeline source.

### WholesomeList — REFERENCE-ONLY / NICHE
- **URL:** wholesomelist.com
- **What:** a curated, tagged/tiered list of "wholesome vanilla" doujin (site description via HypeStat). No API, no licence, no terms verified.
- **Why niche:** curation is one-note (vanilla only) by design — useful only as a small hand-curated positive-example set, not a vocabulary or census source.

---

## Program implications (what this survey changes or confirms for neemu)

1. **Vocabulary strategy = three-way crosswalk, not one donor.** E-Hentai namespaces for act/content description; VNDB tags + traits for VN/character semantics; Danbooru's alias/implication/wiki model as the *governance pattern* for neemu's own registries (the pattern, not its data — Danbooru's database licence is unstated). Gelbooru and nhentai serve as tag-frequency cross-checks. AniDB is lookup-only for anime-video identity (registered client, ~1 req/2 s, ban-on-flood; its CC-BY-NC-SA data must be licence-stamped wherever it lands).
2. **Acquisition lanes ranked by rights strength.** Tier-T-evaluable: DLsite, FAKKU, Ci-en, itch.io purchases (licence/purchase on record). Licence-checked assets: SmutBase/SFMLab per-project licences (motion lane). Tier-A-ceiling forever: every aggregator and tube (doujins.com, nhentai, Hitomi, hanime.tv, HentaiHaven, Rule34Video, Iwara, IMHentai). F95zone never supplies files.
3. **The voice lane has a shape and a home.** DLsite's product records (`VOICE_ASMR` work type, `voice_actor[]`, binaural/ASMR genres, WAV, R18 category, RJ codes) are effectively the input schema for neemu's VoicePerformanceRecord census. The clonability rights axis will, on this evidence, only ever clear on creator-direct or licence-on-record lanes (Ci-en first, then DLsite/FANZA/BOOTH purchases with terms recorded, FAKKU) — every aggregator/tube voice stays characterisation-only, exactly as SPEC §2.3 predicts.
4. **Scraping posture diverges sharply — encode it per source.** Sanctioned machine routes: VNDB (dump/API), E-Hentai (gdata API), Gelbooru (DAPI), itch.io (OAuth API), AniDB (registered client, lookup only). Explicitly forbidden: SmutBase/SFMLab (ToS + robots, operator-stated). Unstated (≠ permitted): all remaining aggregators. A per-source access-posture field belongs in neemu's source registry (see PRIOR-ART spec deltas for the record-shape consequence).
5. **Platform-volatility risk is real and recent.** itch.io's July 2025 NSFW de-indexing, BOOTH's September–October 2025 R-18 enforcement wave, Niconico's 2025 R-18 removals/region restrictions, DLsite's payment-rail suspension (Visa/MC/Amex since April 2024) — census data and acquisition lanes must be snapshotted with dates, never treated as standing infrastructure.
6. **Theme-availability flag.** Patreon's guidelines expressly prohibit hypnosis/non-consent-coded depictions — the creator-commission lane for this program's core mind-control theme is narrower there than a generic "adult Patreon" assumption suggests. Ci-en/DLsite (JP doujin lanes) carry no equivalent surfaced restriction in this survey (their terms are UNVERIFIED as quoted text — see gaps).

## Verification gaps (need a live-browser pass by an eligible agent for primary grade)
- Actual ToS texts: doujins.com, nhentai, hanime.tv, DLsite.
- The EHWiki API page (fetch-policy-blocked in this survey; API facts rest on multiple independent client codebases).
- DLsite's own API existence (product-JSON pattern is third-party-documented; no official developer API was evidenced).
- VNDB website age/display settings and Danbooru's primary terms page (both fetch-blocked; substance verified via official mirrors/help pages as noted).
