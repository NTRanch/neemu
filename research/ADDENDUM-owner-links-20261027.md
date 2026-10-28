# Addendum — Owner-supplied links review (2026-10-27)

Three links supplied by the Owner (2026-10-27 09:15 PDT) and
reviewed firsthand by Pax against the question: what can neemu
extract or adapt? Verdicts are per link; new spec deltas continue
the PRIOR-ART.md numbering (21+). SPEC.md was not modified.

## 1. eh-index-mcp (github.com/RichardGuan1/eh-index-mcp) — ADAPT

<details>
<summary>What it is</summary>

Read-only MCP server for E-Hentai/ExHentai search and metadata:
30 tools, TypeScript/Node ≥20.3, MIT licence, 65 commits,
5 releases, 0 stars, created 2026-08. Strictly read-only by
design (no account mutation, no archive purchase, no gallery
download, no file upload). It is, in effect, a working
implementation of the hop-2 E-Hentai resolver that PRIOR-ART §8
specified abstractly: gdata metadata (single call ≤25; batch
≤500), gallery detail with grouped tags, version chains, page
resolution, EHWiki tag definitions, EhTagTranslation lookup.

</details>

<details>
<summary>Patterns worth taking (mechanism, not code)</summary>

- **Works/variants/series grouping with evidence taxonomy**
  (`eh_search_gallery_works`): every grouping states its basis —
  official version chain / heuristic normalised-title-and-creator
  match / standalone fallback — plus a confidence level, with
  full source-gallery references preserved, and low-confidence
  items kept SEPARATE rather than merged. This is the missing
  schema for census series-clustering and for EnrichmentBundle
  (delta 17) grouping.
- **Version chains + tag diff** (`eh_get_gallery_chain`,
  `eh_find_latest_gallery_version`, `eh_compare_gallery_versions`):
  ordered deduplicated chains and a version-to-version comparison
  (titles, dates, page counts, sizes, TAG CHANGES). Census
  determinations currently treat each listing as an atom; a
  chain-aware enrichment pass can inherit tags across versions
  and escalate on a tag diff.
- **EhViewer similar-gallery strategy**: structural title
  extraction (strip event/group/language/edition wrappers) →
  exact-phrase search → first-artist fallback → uploader
  fallback, with the chosen strategy REPORTED in the response.
  A concrete, field-tested title-normalisation algorithm for the
  System-0 pass's normalise step.
- **Batch discipline**: order-preserving batches with per-item
  errors preserved (metadata ≤500, resolve ≤500); page-budgeted
  scans (≤10 pages) returning a RESUME CURSOR when truncated —
  the same shape as neemu §5's partial-result law, proven in a
  neighbouring server.
- **Untrusted-text marking as a safety-model row**: titles,
  tags, comments, and wiki text are marked untrusted data in
  outputs. Neemu's enrichment-sourced text fields should carry
  the same explicit flag (delta 23).
- **Rate-limit configuration with real numbers**: serialised
  request classes; search interval 3000 ms, API 1250 ms, page
  1000 ms; tiered cache TTLs (30 s / 60 s / 300 s); bounded
  retries (2) with Retry-After taking priority. A sane starting
  config for any estate E-Hentai client under shared-IP limits.
- **Hash search without upload** (`eh_search_by_file`): hashes
  one explicit local file (SHA-1), searches by digest, rejects
  relative paths/directories/wildcards/oversize — the §5 input
  discipline mirrored, and a second exact-hash lane beside
  Hydrus PTR (post-acquisition only, same as delta 20).
- **Contract-smoke pattern**: `pack:smoke` installs a tarball in
  a temp dir and verifies stdio initialisation, tool list,
  output schemas, and read-only annotations — a lightweight echo
  of the estate's local-mcp-contract-testing gate.

</details>

<details>
<summary>Caveats</summary>

- Single author, 0 stars, young repo: patterns are adoptable on
  their merits; the server itself is a DEPENDENCY candidate only
  behind the estate contract-testing gate (see delta 24).
- EhTagTranslation data is CC BY-NC-SA 3.0 CN, fetched at
  runtime, not bundled: translated-tag text entering neemu
  records inherits a non-commercial licence — fine for analysis
  records, a flag for anything feeding commercial artefacts.
- ExHentai access rides browser identity cookies via env vars;
  the estate's credential discipline (pass / vault, never in
  config files) would govern any adoption.
- Its Dockerfile packaging is irrelevant to the estate
  (container rule); the npx/stdio route is the usable one.

</details>

## 2. Hentaidoujinshi MCP (mcpbundles listing) — NO-FIT

The name promises a doujinshi content server; the probed tool
list (13 tools) is a Shopify storefront MCP implementing the UCP
shopping protocol: carts, checkouts, orders, catalog search.
There is no content, metadata, or extraction surface at all.
Nothing transfers to neemu (neemu has no commerce surface, and
the estate's shopping needs are already served). Sole residual
value: a worked example that directory listings mislabel servers
— probe the tool list before believing a name, the same
discipline that produced the MCPRush verified negative.

## 3. Shyft "AI Anime Generator" (shyft.ai listing) — NO-FIT / UNVERIFIED

An aggregator stub: the page's own review text is AI-generated,
and its limitations section concedes no information on output
consistency, style control, usage limits, export formats, or
commercial licensing. It claims "MCP Ready" with no server, no
schema, no API, and no vendor documentation linked. There is
nothing technical to extract — no tool surface, no workflow, no
licence posture. Recorded as a directory stub, not a tool.

## New spec deltas (for the Owner's spec review)

<details>
<summary>Deltas 21–24</summary>

- **21 — Grouping evidence in EnrichmentBundle.** Any grouping
  of census/enrichment items into works/series carries
  `grouping_basis` (official_chain / heuristic_match /
  standalone) + confidence; low-confidence items remain
  separate. (From eh-index-mcp's works organiser.)
- **22 — Version-chain awareness in enrichment.** Enrichment
  resolves a work's version chain where the source supports it;
  tag inheritance across versions is recorded per version, and
  a tag DIFF between versions is itself an escalation signal
  for the determination pass.
- **23 — Untrusted flag + batch shape.** Text fields sourced
  from enrichment (titles, tags, comments, wiki definitions)
  carry an explicit untrusted marker in records; enrichment
  resolvers use order-preserving batches with per-item errors
  and page budgets with resume cursors.
- **24 — eh-index-mcp adoption decision — RULED (Owner,
  2026-10-27 09:18 PDT): route (b).** "Any adaptations are still
  Rust." No TypeScript co-load, interim or otherwise: the gdata
  resolver and every pattern in §1 land as Rust inside neemu,
  per PRIOR-ART delta 20 and the standing Rust-first doctrine.
  eh-index-mcp remains a pattern reference only.

</details>
