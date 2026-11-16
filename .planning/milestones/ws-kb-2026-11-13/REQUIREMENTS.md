# Requirements: Vox Knowledge Base (milestone v1.1, workstream `kb`)

**Defined:** 2026-10-03
**Core Value:** Agents using any model get the right curated knowledge at the right time — measurably, visibly, and without letting unreviewed or model-generated text masquerade as trusted fact.

Research basis: `.planning/workstreams/kb/research/SUMMARY.md` and `docs/src/architecture/knowledge-base-practices-research-2026.md`.

## v1 Requirements

### Research & Practices (DOC)

- [x] **DOC-01**: A contributor or agent can read one research SSOT (`knowledge-base-practices-research-2026.md`) covering KB principles, evaluated techniques (adopt/gate/skip), the pitfall→test matrix and sources; the prior `kb-systems-sota-research-2026.md` is marked superseded
- [ ] **DOC-02**: A contributor can find KB concepts in `where-things-live.md` (indexer, `kb_findings`, `kb_context`, taxonomy contract, eval set) and follow an operator how-to for reindex, inbox review and promotion

### Retrieval Evaluation (EVAL)

- [x] **EVAL-01**: A maintainer can run a deterministic retrieval eval (`vox kb eval`) that reports Recall@5/10, MRR@10 and nDCG@10 computed on doc and chunk IDs, plus the answer-bearing-chunk-within-budget rate
- [x] **EVAL-02**: The golden set lives under `contracts/kb/eval/` in BEIR layout with ≥40 seed queries (origin `human`, or `agent-seed` when authoring is delegated — amended 2026-10-04, see 01-CONTEXT D-02), an LLM-generated expansion, and a held-out split
- [x] **EVAL-03**: A canary-string check proves the golden set is excluded from the indexer roots and the MENS training corpus
- [x] **EVAL-04**: Retrieval metrics are ratcheted against `contracts/kb/retrieval-baseline.v1.json` in PR CI — a regression blocks merge; the baseline may only tighten
- [x] **EVAL-05**: The misnamed word-overlap helpers in `vox-search/src/evaluation.rs` are deleted or renamed so no gate can rely on them
- [ ] **EVAL-06**: Nightly generation evals (faithfulness, citation support) run through the `vox_actor_runtime::llm` facade with n≥3 samples, a judge from a different model family than the generator, and position-swap checks; results are advisory, never PR-blocking

### Library Index (IDX)

- [x] **IDX-01**: Reindexing is incremental — unchanged files are skipped by file hash, chunks are diffed by chunk hash, deleted/renamed docs are pruned, and embeddings survive reindex (no orphaned `source_id`s)
- [x] **IDX-02**: Markdown is chunked by heading with a recorded `heading_path`; headings inside code fences and YAML frontmatter never create or pollute chunks; oversize sections are split by `text-splitter` sized to the embedder tokenizer
- [x] **IDX-03**: Each indexed doc has a summary-card row, and each index build records a manifest (embed model id + revision, dim, chunker version, corpus commit)
- [x] **IDX-04**: `docs/src/archive/` is never indexed (fixture test proves it)
- [x] **IDX-05**: A user can run `vox kb reindex` and `vox kb status`; status reports last-indexed commit, doc/chunk counts, embed model, and per-file errors
- [x] **IDX-06**: General (non-Vox) curated knowledge lives in `docs/src/library/` with a `Library` docs category and sidebar section, indexed alongside Vox docs
- [x] **IDX-07**: The lexical leg is decided by a runtime probe (Turso FTS5 vs Tantivy-with-owning-writer); the writer-less `TantivyDocsIndex` mirror is removed either way

### Retrieval & Tool Surface (RET)

- [x] **RET-01**: Any MCP host (Vox chat, Claude Code, Cursor) can call `vox_kb_search`, `vox_kb_get`, `vox_kb_cite`, `vox_kb_status` and `vox_kb_reindex`, and read `kb://doc/{path}` / `kb://chunk/{hash}` resources
- [x] **RET-02**: All KB retrieval goes through `vox-search` (single retrieval path); the legacy `KbStore` LIKE search, `KbRouter`, signal adapters, `kb_lines` and the 12 legacy `vox_kb_*` CRUD tools are retired, with colliding names replaced rather than aliased
- [x] **RET-03**: Legacy `kb_entries`/`knowledge_bases` rows are counted before quarantine; non-empty rows are migrated into the inbox as pending findings
- [ ] **RET-04**: Search results carry trust tier, staleness, `scope`, and status; `status: research|roadmap|legacy|deprecated` and superseded pages are down-weighted, not excluded

### Provenance & Trust (TRUST)

- [x] **TRUST-01**: Every indexed row and finding has a required trust tier mapped onto `ContextTrustTier`: SystemVerified = curated + mechanical verification evidence; Trusted = curated (merged to git); Advisory = agent-derived; Untrusted = inbox / live web. Writes without a tier fail to compile or are rejected
- [x] **TRUST-02**: Web hits are no longer written into the searchable corpus; `persist_web_hits` routes them into the `kb_findings` inbox as Untrusted
- [x] **TRUST-03**: Model output never re-enters the KB at elevated trust — the `signal_research` 0.95 confidence floor and the post-turn `accepted=1` chat writeback are removed or routed to the inbox
- [x] **TRUST-04**: Each finding stores summary, short quotes, source URL, fetch date and content hash; a full-text snapshot is kept locally under a gitignored path and is never committed
- [x] **TRUST-05**: Ingested and injected text passes a sanitizer (Unicode tag block, zero-width/bidi characters, HTML comments, off-allowlist markdown images) backed by property tests and a red-team fixture set
- [x] **TRUST-06**: Promotion eligibility fails closed when the trust lookup errors

### Taxonomy & Schema (TAX)

- [x] **TAX-01**: A controlled vocabulary (`contracts/documentation/kb-taxonomy.v1.yaml`, SKOS-lite: prefLabel, altLabel, broader, related, scopeNote, deprecated aliases) defines allowed `topic`/`domain` values; altLabels feed lexical search as synonyms
- [x] **TAX-02**: A schemars-derived facet schema (topic, domain, kb_tier, trust_tier, scope, review_by, verified_at, sources[], authorship, origin, supersedes/superseded_by, sensitivity) is shared by the doc lint, the indexer and the GUI
- [x] **TAX-03**: Every doc has a delimited page-one summary block (40–120 words); the lint enforces it
- [x] **TAX-04**: The facet lint blocks on new and changed files, while existing docs are tracked by a shrinking backlog allowlist (ratchet) rather than failing all at once
- [x] **TAX-05**: `authorship` (human | llm-assisted | llm-generated, plus model) exists and is linted before any LLM rewrite lands
- [x] **TAX-06**: Pages with `origin: web-promoted` or LLM authorship are forced to `training_eligible: false` unless a `human_attested_by` field is set; the MENS corpus builder honors this and strips page-one summary blocks
- [x] **TAX-07**: `sources[]` entries on promoted pages must carry url, date and hash; verbatim quote length is capped by lint

### Embeddings & Ranking (EMB)

- [ ] **EMB-01**: Embeddings are computed locally by default (Candle `gte-modernbert-base`, revision-pinned) behind a feature/plugin boundary that respects `layers.toml`, reachable through `llm_embed` as a `local` provider; API embeddings are opt-in
- [ ] **EMB-02**: Vectors are keyed by (chunk_hash, model_id, revision, dim); similarity across different models is refused
- [ ] **EMB-03**: Chunks are embedded and lexically indexed with a deterministic contextual prefix (title › heading_path + page-one summary)
- [ ] **EMB-04**: A local cross-encoder (`gte-reranker-modernbert-base`) reranks the fused top-20 when the eval shows a gain and the CPU latency benchmark fits the path it is enabled on
- [ ] **EMB-05**: Without model weights, retrieval degrades to lexical-only and `vox doctor` explains how to fetch them
- [ ] **EMB-06**: Any additional technique (LLM-written chunk context, Qwen3 embedding tier) is adopted only with a recorded eval delta

### Context Injection (INJ)

- [ ] **INJ-01**: Each chat turn may inject a small KB preamble whose budget is a per-model ceiling derived from the model registry; when no candidate clears a calibrated relevance floor, nothing is injected
- [ ] **INJ-02**: Injection uses one merged ranked list (replacing six separately-capped preamble sections), summary cards before chunks, a per-doc chunk cap, edge-weighted ordering, and static-first prompt ordering that preserves prompt caching
- [ ] **INJ-03**: Model `knowledge_cutoff` (from OpenRouter, labelled "estimated" when derived) is stored in `ModelCapabilities`; cutoff-aware boosting applies only to `scope: external` pages and uses reviewed `verified_at`, never git mtime (whitespace-commit fixture proves ranking unchanged)
- [ ] **INJ-04**: Non-curated tiers are spotlighted with per-turn boundary delimiters, and exfiltration-capable tools are gated while Untrusted content is in context
- [ ] **INJ-05**: Answers cite KB items by ID; a post-generation validator flags citations that don't resolve or don't support the claim
- [ ] **INJ-06**: Each turn emits a `kb_context` event listing injected items (source, section, tokens, score/backend, tier, staleness, reason) and dropped candidates with drop reasons
- [ ] **INJ-07**: `vox_kb_*` tools are pinned so they survive the per-turn tool cap, letting the model pull deeper context on demand
- [ ] **INJ-08**: A `sensitivity` egress check prevents marked-private content from reaching remote models
- [ ] **INJ-09**: Per-chat scope is configurable: `@kb` mentions (remapped onto Library scope), pin (restrict/prioritize), exclude, trust floor, and injection on/off; web fallback is the existing research-mode control (single control, no duplicate toggle)

### Web Fallback, Inbox & Promotion (GROW)

- [ ] **GROW-01**: When local retrieval confidence is below threshold (or the query needs content newer than the model cutoff), research mode permitting, the agent searches the web free-first (SearXNG → DDG) and uses paid providers only under a configurable daily spend cap tracked in a spend ledger; hitting the cap is surfaced, never silent
- [ ] **GROW-02**: Captured pages are summarized in a quarantined, tool-less step and written to `kb_findings` as pending findings with provenance; nothing in the inbox is injected as curated
- [ ] **GROW-03**: A user or agent can list, review, reject (with reason) and propose findings via `vox_kb_propose`, `vox_kb_inbox_list`, `vox_kb_inbox_review`; a similar-docs check flags duplicates
- [ ] **GROW-04**: An agent drafts promotions as a draft PR from an isolated worktree (write → doc-lint → commit → push), with per-claim source excerpts in the PR body; a human merge is the only path into curated docs
- [ ] **GROW-05**: Promotion requires ≥2-domain corroboration (or explicit reviewer override) and passes a promotion lint; `docs/src/library/` has CODEOWNERS
- [x] **GROW-06**: Existing side doors (`publish_research_doc`, GUI `ingest_url`/`ingest_text`, scientia/MEMORY sinks) route through the inbox; MENS `KbSignals` reads `kb_findings`

### Page-One Rewrites (REWR)

- [ ] **REWR-01**: A rewrite guard ships before any batch: referenced identifiers, paths and CLI commands must resolve; retired surfaces must not appear; negations and numbers are preserved; code fences are untouched
- [ ] **REWR-02**: Rewrites touch only the delimited page-one block, ship in PRs of ≤25 pages grouped by category, and stamp `authorship`/`authorship_model` with `training_eligible: false` until attested
- [ ] **REWR-03**: The top-N docs by retrieval usage are rewritten first, then the long tail, with human spot-check sampling recorded per batch
- [ ] **REWR-04**: The docs landing page and hub pages are rewritten summary-first
- [ ] **REWR-05**: A summary-distinctiveness ratchet and a source-bias metric (retrieval share stratified by authorship) are tracked so LLM-written pages can't silently crowd out human pages

### Staleness & Re-verification (STALE)

- [ ] **STALE-01**: Docs carry hash-jittered `review_by` dates with per-facet horizons; overdue docs show a badge and are down-ranked, never hidden or deleted
- [ ] **STALE-02**: A durable `@scheduled` job re-verifies expired and high-traffic docs within a per-run page/token budget
- [ ] **STALE-03**: `verified_at` advances only on mechanical evidence (paths exist, symbols resolve via `vox graph query`, CLI commands exist in the generated surface, links resolve); an LLM verdict alone only opens an inbox item
- [ ] **STALE-04**: Superseded or failed docs are demoted (`status: legacy`), not deleted

### GUI (GUI)

- [ ] **GUI-KB-01**: A user can browse and search the whole KB in a Library surface with facets (category, status, trust tier, staleness, domain), absorbing Memory and DocReader browsing
- [ ] **GUI-KB-02**: A user can read a rendered doc with a provenance header (tier, authorship, sources, verified/review dates) — rendered with `react-markdown` + `remark-gfm`, no raw HTML, no remote image auto-load
- [ ] **GUI-KB-03**: A user can open a per-turn Inspector showing exactly the `kb_context` payload (item count matches backend), why each item was chosen, what was dropped, and Open/Exclude/Flag actions
- [ ] **GUI-KB-04**: A user can set per-chat KB controls from the composer (`@kb` typeahead with chips, injection on/off, strict sources/pin) with ~3 primary controls and the rest under Advanced
- [ ] **GUI-KB-05**: A user can review the inbox (diff, sources with trust chips, similar-docs warning), reject with reason, and open a promotion PR via the `kb_open_promotion_pr` command
- [ ] **GUI-KB-06**: A user can configure KB defaults in Settings: per-model budget override, web spend cap and current spend, trust floor default, embedding provider
- [ ] **GUI-KB-07**: A user can see index health (last indexed commit, errors) and a verification queue of overdue docs ordered by usage × days overdue
- [ ] **GUI-KB-08**: Inline citations in chat link to doc#section and show an unverified badge when the validator rejects them
- [ ] **GUI-KB-09**: Every new surface has Playwright visual steppers with `installTauriMock` stubs for each `vox_kb_*` tool and screenshots in `review-bundle/latest/`, passing `test:e2e` and `typecheck`

## v2 Requirements

- **KB2-01**: Counterfactual replay — re-run a turn without a chosen source and compare answers
- **KB2-02**: Eval-health panel in the Library with trend charts
- **KB2-03**: Domain reliability list as a reviewed contract file feeding web-source ranking
- **KB2-04**: "Save answer to Library" from chat via the inbox
- **KB2-05**: Mesh (Populi) sync of curated index and inbox across nodes (schema already carries origin/hash)
- **KB2-06**: STORM-style outline-first article drafting for new library pages
- **KB2-07**: HippoRAG-2-style link/taxonomy graph ranking, if evals show multi-hop gaps
- **KB2-08**: Shared/team KB scopes

## Out of Scope

| Feature | Reason |
|---------|--------|
| Auto-promotion of web content or agent memories into curated docs | Poisoning/laundering risk; human merge is the only entry (locked) |
| Indexing `docs/src/archive/` | Archive is tombstoned for agents by repo policy |
| In-GUI rich editor for docs | Creates a second write path around git; edit via PR |
| GraphRAG / LightRAG / RAPTOR / ColBERT / SPLADE | No gain at ~2.4M-token scale per research; complexity |
| fastembed / ONNX Runtime | Pulls `ort` with a prebuilt binary + TLS stack needing a user-only transport ledger entry |
| Python eval stacks (RAGAS, DeepEval, TruLens) as glue | VoxScript-first / no new Python glue; metrics ported to Rust |
| Committed generated catalogs / research index | Retired by `2026-09-06-research-index-obsolete` plan |
| Third-party doc-site crawling into the KB | Staleness complaints dominate in tools that do it; curate via library pages instead |
| DeepWiki-style auto-generated wiki | Unreviewed generated text; violates curation model |
| A single numeric trust score | Tiers + provenance are more honest and actionable |
| Removing `vox-gamify` XP for `training_eligible` beyond what TAX-06 requires | Touches another crate's surface; record as follow-up |
| New crate edges `vox-search → vox-crypto`, `vox-cli → vox-forge` | User-authorized-only; routed around. Proposed separately for the user to decide |
| Legal determination of copyright posture | Needs human/legal review; interim policy = excerpts + URL + hash, paraphrase with attribution, quote-length lint |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| DOC-01 | Phase 1 | Complete |
| DOC-02 | Phase 11 | Pending |
| EVAL-01 | Phase 1 | Complete |
| EVAL-02 | Phase 1 | Complete |
| EVAL-03 | Phase 1 | Complete |
| EVAL-04 | Phase 1 | Complete |
| EVAL-05 | Phase 1 | Complete |
| EVAL-06 | Phase 7 | Pending |
| IDX-01 | Phase 2 | Complete |
| IDX-02 | Phase 2 | Complete |
| IDX-03 | Phase 2 | Complete |
| IDX-04 | Phase 2 | Complete |
| IDX-05 | Phase 2 | Complete |
| IDX-06 | Phase 2 | Complete |
| IDX-07 | Phase 2 | Complete |
| RET-01 | Phase 2 | Complete |
| RET-02 | Phase 3 | Complete |
| RET-03 | Phase 3 | Complete |
| RET-04 | Phase 5 | Pending |
| TRUST-01 | Phase 3 | Complete |
| TRUST-02 | Phase 3 | Complete |
| TRUST-03 | Phase 3 | Complete |
| TRUST-04 | Phase 3 | Complete |
| TRUST-05 | Phase 3 | Complete |
| TRUST-06 | Phase 3 | Complete |
| TAX-01 | Phase 4 | Complete |
| TAX-02 | Phase 4 | Complete |
| TAX-03 | Phase 4 | Complete |
| TAX-04 | Phase 4 | Complete |
| TAX-05 | Phase 4 | Complete |
| TAX-06 | Phase 4 | Complete |
| TAX-07 | Phase 4 | Complete |
| EMB-01 | Phase 5 | Pending |
| EMB-02 | Phase 5 | Pending |
| EMB-03 | Phase 5 | Pending |
| EMB-04 | Phase 5 | Pending |
| EMB-05 | Phase 5 | Pending |
| EMB-06 | Phase 5 | Pending |
| INJ-01 | Phase 7 | Pending |
| INJ-02 | Phase 7 | Pending |
| INJ-03 | Phase 7 | Pending |
| INJ-04 | Phase 7 | Pending |
| INJ-05 | Phase 7 | Pending |
| INJ-06 | Phase 7 | Pending |
| INJ-07 | Phase 7 | Pending |
| INJ-08 | Phase 7 | Pending |
| INJ-09 | Phase 7 | Pending |
| GROW-01 | Phase 9 | Pending |
| GROW-02 | Phase 9 | Pending |
| GROW-03 | Phase 9 | Pending |
| GROW-04 | Phase 9 | Pending |
| GROW-05 | Phase 9 | Pending |
| GROW-06 | Phase 3 | Complete |
| REWR-01 | Phase 10 | Pending |
| REWR-02 | Phase 10 | Pending |
| REWR-03 | Phase 10 | Pending |
| REWR-04 | Phase 10 | Pending |
| REWR-05 | Phase 10 | Pending |
| STALE-01 | Phase 11 | Pending |
| STALE-02 | Phase 11 | Pending |
| STALE-03 | Phase 11 | Pending |
| STALE-04 | Phase 11 | Pending |
| GUI-KB-01 | Phase 6 | Pending |
| GUI-KB-02 | Phase 6 | Pending |
| GUI-KB-03 | Phase 8 | Pending |
| GUI-KB-04 | Phase 8 | Pending |
| GUI-KB-05 | Phase 12 | Pending |
| GUI-KB-06 | Phase 12 | Pending |
| GUI-KB-07 | Phase 12 | Pending |
| GUI-KB-08 | Phase 8 | Pending |
| GUI-KB-09 | Phase 12 | Pending |

**Coverage:**

- v1 requirements: 71 total (1 already complete: DOC-01)
- Mapped to phases: 71/71
- Unmapped: 0

---
*Requirements defined: 2026-10-03*
*Last updated: 2026-10-03 after roadmap creation*
