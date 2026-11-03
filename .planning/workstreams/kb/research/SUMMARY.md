# Project Research Summary

**Project:** Vox Knowledge Base (milestone v1.1, workstream `kb`)
**Domain:** Curated docs-as-code knowledge base as first-class, model-agnostic agent context — git markdown → incremental hybrid index → budgeted injection for any OpenRouter model → CRAG web fallback → reviewed inbox → promotion PR → reindex; surfaced in a Tauri/React GUI (Library / Inbox / Inspector) and feeding the MENS training corpus
**Researched:** 2026-10-03
**Confidence:** MEDIUM-HIGH (repo current-state findings HIGH — read from source; stack HIGH on licences/versions, MEDIUM on self-reported model quality; product survey and UX MEDIUM; pitfalls literature MEDIUM)

Detail files: [STACK.md](STACK.md) · [FEATURES.md](FEATURES.md) · [ARCHITECTURE.md](ARCHITECTURE.md) · [PITFALLS.md](PITFALLS.md). Durable project-facing version: `docs/src/architecture/knowledge-base-practices-research-2026.md` (supersedes `kb-systems-sota-research-2026.md`).

## Executive Summary

Vox already owns most of the retrieval plumbing — `vox-search` hybrid execution with RRF and a CRAG web tier, VoxDb chunk/embedding tables, a chat preamble, MCP tools/resources, a per-turn trace event the GUI renders, and `ContextTrustTier`/`ContextBudget` types. What it lacks is a **curated corpus with a single trustworthy path through it**, a **reviewed inbox** for new knowledge, and **visibility/control over what reaches the model**. At 669 non-archive `.md` files (~2.4M tokens) the corpus is ~12× too large to stuff into the prompt but far below the scale where vector DBs, ANN indexes, or GraphRAG pay off. Experts at this scale build: content-hash-keyed incremental indexing, heading-bounded chunks with a deterministic contextual prefix, BM25 + local dense embeddings fused by RRF, a cross-encoder rerank of the top-20, a small relevance-gated injection preamble plus pull-on-demand tools, and Guru/Notion/KCS-style governance — badge + interval, not gate — with a human-merged PR as the only entry to the curated tier.

The recommended approach is **measure first, close existing leaks second, then build**. A deterministic, ID-based retrieval eval with a ratcheted baseline must precede any tuning, because the existing eval helpers (R4) are word-overlap functions borrowing IR metric names. Four shipped leaks must be fenced before any new ingestion path: web hits persisted into the searchable corpus by default (R1), research syntheses written back with confidence floored at 0.95 (R2), assistant turns auto-accepted into `kb_entries`, and injected context without data/instruction delimiting (R3). The legacy 2026-06-17 VoxKB surface (`KbStore` LIKE search, signal adapters, a writer-less Tantivy docs mirror) is retired into `vox-search`-backed retrieval. Local embeddings run on the Candle 0.10.2 stack already in the workspace (`gte-modernbert-base` + `gte-reranker-modernbert-base`); fastembed is rejected (`ort`, TLS ledger, version skew).

Top risks: (1) **stored prompt injection / poisoning** — web text retrieved weeks later by a tool-capable agent; (2) a **self-consuming loop** — LLM-rewritten pages feeding both retrieval (neural retrievers prefer LLM text) and MENS training; (3) **context dilution** — a per-model budget treated as a target rather than a ceiling; (4) **evals that cannot fail**. Mitigations: trust tiers on every row with spotlighting and tool gating for non-curated text; an `authorship:` field before any LLM rewrite, with LLM-rewritten and web-promoted pages defaulting to `training_eligible: false`; budget-as-ceiling with a relevance floor and an explicit inject-nothing path; deterministic ID-based evals blocking PRs while LLM-judge evals run nightly only (n≥3, cross-family judge, swap-checked).

## Key Findings

### Recommended Stack

No new retrieval engine. Reuse Tantivy 0.22, VoxDb (`turso` 0.6.1) chunk/embedding tables, `rrf.rs`, `crag.rs`, `pulldown-cmark` 0.13, `serde_yaml_ng`, `jsonschema`/`schemars`, `tokenizers` 0.21, `hf-hub` 1.0 and `candle-*` 0.10.2. Only three new dependencies: `text-splitter` 0.33 (`markdown` feature only — the `tokenizers` feature would pull tokenizers 0.23 + onig), plus `react-markdown` 10.1 and `remark-gfm` 4.0 in the GUI. Evals are hand-written Rust (Recall@k / MRR / nDCG@10 ≈ 60 lines); RAGAS/TruLens/ALCE metric *definitions* are ported as judge prompts through `vox_actor_runtime::llm` — no Python.

**Core technologies:**
- **Candle 0.10.2 `modernbert`** — local embedding + rerank inference. Pure Rust, meets the build-toolchain invariant, one loader for both models. Lives behind a feature/plugin boundary (candle is not allowed unconditionally in L0–L3) and is reached via `llm_embed` as a `provider: "local"` arm.
- **`Alibaba-NLP/gte-modernbert-base`** (149M, Apache-2.0, 8k ctx, CLS pooling, no prefix) — default embedder, strongest code retrieval in class. Pin the HF revision SHA. Qwen3-Embedding-0.6B is a GPU-only quality tier, adopted only on a ≥+2 nDCG eval gain.
- **`gte-reranker-modernbert-base`** (149M, Apache-2.0) — default cross-encoder over the fused top-20. Benchmark CPU latency before putting it on the auto-preamble path.
- **Tantivy 0.22 / VoxDb FTS** — lexical leg with field boosts (title, heading_path, summary, taxonomy altLabels). *Unresolved conflict — see Gaps*: the VoxDb FTS5 probe likely fails on the Turso engine and silently falls back to `LIKE`.
- **OpenRouter `GET /api/v1/models`** — per-model `knowledge_cutoff` + `context_length` into `ModelCapabilities` (live; no pinned models).
- **`text-splitter` 0.33 `MarkdownSplitter`** — second-stage split for oversize heading sections, sized by the embedder tokenizer via a custom `ChunkSizer`.

**Do not use:** fastembed/`ort`; EmbeddingGemma (Gemma licence, gated, 2k ctx); CC-BY-NC weights (jina-reranker-v2, NV-Embed); Python RAG eval stacks as glue; GraphRAG / LightRAG / RAPTOR trees; ColBERT/SPLADE; bumping `starlight-llms-txt` to 0.12 (requires Astro 7); remote-first embedding (mixes vector spaces).

### Expected Features

Every surveyed product converges on three activation tiers (always / relevance-selected / manual @-mention), scope set per chat rather than globally, verification as badge + owner + interval (Guru, Notion, KCS), and retreat from unreviewed auto-capture (Cursor removed Memories). Staleness is the #1 complaint against tools that don't model it. **No product shows per-turn, per-chunk what was injected, why, and what was dropped — this is Vox's clearest opening.**

**Must have (table stakes — all locked v1):**
- Library: browse/search the whole corpus with facets (category, status, trust_tier, staleness); absorbs `MemoryView` + `DocReader`.
- Rendered reader with provenance header (current DocReader shows raw `<pre>`). Reuse `ResearchReportMarkdown`, `TrustChip`; `react-markdown` without `rehype-raw`.
- Staleness badges from `review_by` plus down-rank — never hide or delete.
- Per-chat controls: `@kb` scope (**remapped onto the Library scope**, with typeahead + chips), injection on/off, trust floor, pin (restrict/prioritize) / exclude, and **one** web control merged into `ResearchModeDropdown`.
- Inspector: per-chunk rows (source, section, tokens, score/backend, tier, staleness, activation reason) with Open/Exclude/Flag; extends `TurnTrace` + `ContextWindowMeter`.
- Inline KB citations to doc#section plus a validator.
- Inbox: agent drafts → Open PR → human merges; diff, sources with TrustChips, similar-docs check, reject-with-reason. **Nothing is injectable pre-merge.**
- Web fallback free-first, then paid under a cap; spend visible in Settings and the Inspector; "capped" shown, never silent.
- Honest index health (last-indexed SHA, error list).
- Scheduled re-verification (minimal: flag overdue + draft re-verify items).

**Should have (differentiators):** dropped-candidates view; usage-weighted verification queue (injection count × days overdue); "Flag it or fix it" and "Save answer to Library" from anywhere via the Inbox; an eval-health panel in the Library; a domain reliability list as a reviewed contract file.

**Defer (v1.x / v2+):** counterfactual replay; STORM-style outline-first drafting; shared/team scopes; third-party doc-site crawling (likely never).

**Anti-features:** auto-promotion of web content or agent memories; inject-everything budgets; a single numeric trust score; an in-GUI rich editor (a second write path around git); a DeepWiki-style auto-wiki; a second skill/KB index; a separate web toggle beside research mode.

### Architecture Approach

A five-stage pipeline — ingest → index → retrieve → inject → feedback — over **git markdown as the only curated SSOT**, with the DB as a derived index plus the inbox. Everything is keyed on content hashes; every surface goes through one MCP contract; git/forge side effects live at L5 behind a human action. Two things are fixed first: the current reindex DELETE+re-INSERTs AUTOINCREMENT chunk rows (orphaning `embeddings.source_id`), and findings currently land in six sinks, none of them review-tracked. **No new crate edges are taken**: hashing routes through `vox-orchestrator-mcp` (already depends on `vox-crypto`); PR drafting through `vox-cli-review` (already reaches `vox-forge`). `vox-search → vox-crypto` and `vox-cli → vox-forge` are listed only as optional user-authorization proposals.

**Major components:**
1. **Library indexer (`vox-search::library`, L3)** — walks `docs/src/**` (excluding `archive/`, never indexed) plus `docs/src/library/` (general knowledge); frontmatter parse; heading-bounded chunking with `heading_path`; file-hash skip, chunk-hash diff, deleted-doc prune; embeds only new `(chunk_hash, model_id)`; hash fn injected by the caller; one summary-card row per doc; manifest (`embed_model_id`, dim, `chunker_version`, `corpus_commit`).
2. **Schema (`vox-db`, L3)** — `corpus`, `frontmatter_json`, `git_sha`, `chunk_hash`, `heading_path`, `ctx_prefix`, `token_estimate`, `trust_tier` columns; `kb_findings` (inbox) and `kb_index_state` tables; baseline 94 → 95.
3. **KB MCP surface (`vox-orchestrator-mcp`, L3)** — `vox_kb_search` / `get` / `cite` / `propose` / `inbox_list` / `inbox_review` / `reindex` / `status`; `kb://doc/{path}` and `kb://chunk/{hash}` resources. **Replaces** the 12 legacy `vox_kb_*` CRUD tools; colliding `vox_kb_query` / `vox_kb_clip` are replaced, not aliased.
4. **`chat/kb_context.rs`** (new module, kept out of the 2533-line `message.rs`) — two-tier injection (budgeted, relevance-floored, spotlighted preamble + pinned tools), the low-confidence CRAG decision, `kb_findings` writes, and a `kb_context` turn event (contract-checked event kind).
5. **`ModelCapabilities.knowledge_cutoff` (`vox-orchestrator`)** — parsed from OpenRouter; when null, fall back to `released_at` minus a margin, labelled "estimated".
6. **`kb_promote` (`vox-cli-review`, L5) + `vox kb promote` + `kb_open_promotion_pr` Tauri command** — isolated worktree → write → lint → commit → push → draft PR; never writes into the user's tree. Reroutes `publish_research_doc` and the GUI's `ingest_url`/`ingest_text` through the inbox.
7. **GUI Library / Inbox / Inspector (`vox-gui/ui`)** — only `callTool` plus one Tauri command; Playwright coverage with review-bundle screenshots.

### Critical Pitfalls

1. **Stored indirect prompt injection (R1, R3)** — web text is persisted into the corpus by default and injected without delimiting. *Avoid:* a required trust tier on every write (enum argument, no default); reroute `persist_web_hits` into the inbox tier; sanitize at ingest **and** injection (Unicode tag block, zero-width/bidi characters, HTML comments, off-allowlist markdown images); spotlight non-curated items with per-turn boundary tokens; gate exfiltration-capable tools whenever an untrusted tier is in context; a nightly red-team ASR ratchet.
2. **Self-consuming loop (R2, R6)** — 59% of docs are `training_eligible: true`, `vox-gamify` awards XP for it, and R2 writes LLM output back at 0.95. *Avoid:* land `authorship:` before any rewrite; rewrite only the delimited page-one block; the MENS builder strips summary blocks and excludes `llm-generated`; remove the 0.95 floor and the post-turn `accepted=1` writeback; LLM-rewritten and web-promoted pages default to `training_eligible: false` until human attestation.
3. **Context dilution** — RRF always returns *something*; today's preamble concatenates six sources each with its own top-k. *Avoid:* budget as a ceiling; a calibrated relevance floor with inject-nothing; summary cards before chunks; a per-doc chunk cap; U-curve ordering (most relevant at the edges); merge sources into one ranked list; static-first prompt ordering so prompt caching survives.
4. **Promotion as a laundering channel** — PoisonedRAG reaches >90% attack success with 5 texts. *Avoid:* per-claim source excerpts in the PR body; lint that `sources[]` carries url + date + hash; fail-closed on trust-lookup errors (invert R5); ≥2-domain corroboration; CODEOWNERS on `docs/src/library/`.
5. **Evals that cannot fail (R4)** — *Avoid:* ID-based deterministic retrieval metrics in PR CI with a ratchet; generation metrics nightly only; the golden set in `contracts/kb/eval/` with a canary string, excluded from the indexer and MENS; a held-out split; a cross-family, swap-checked judge; delete or rename the R4 helpers.

Also: cutoff-aware ranking on the wrong clock (use reviewed `verified_at`, only for `scope: external`, with a whitespace-commit fixture that must leave ranking byte-identical); mixed-model vectors; a `review_by` avalanche and rubber-stamp re-verification (`verified_at` advances only on mechanical evidence); taxonomy sprawl; private text leaking to remote models (`sensitivity` egress check).

## Implications for Roadmap

Suggested structure: **10 phases**. Phases 1, 2 and 4 can overlap; Phase 10's GUI sub-tracks start as soon as their backend contract exists.

### Phase 1: Retrieval Eval Baseline
**Rationale:** Without a trustworthy baseline every later "improvement" is unfalsifiable (Pitfall 8); the existing R4 helpers would report green while retrieval regresses.
**Delivers:** A golden set (~40 human-authored seed questions + an LLM-generated, human-pruned expansion; BEIR layout under `contracts/kb/eval/` — `corpus.jsonl`, `queries.jsonl`, `qrels/test.tsv`; held-out split; canary string); a Rust harness for Recall@5/10, MRR@10, nDCG@10 on doc/chunk IDs plus the answer-bearing-chunk-in-budget rate; a `contracts/kb/retrieval-baseline.v1.json` ratchet; R4 helpers deleted or renamed; a canary scan proving the eval set sits outside the indexer roots and the MENS corpus.
**Addresses:** the eval-health prerequisite. **Avoids:** Pitfall 8 (wrong metrics, leakage, overfit).
**Spike:** the VoxDb FTS5 runtime probe and the candle `modernbert` + tokenizers 0.21 load check, so Phases 2 and 5 start from facts.

### Phase 2: Library Index Foundation and Retrieval Surface
**Rationale:** Stable chunk identity is load-bearing for everything downstream (vectors, citations, Inspector, inbox); the current DELETE-all reindex orphans embeddings (AP3).
**Delivers:** schema columns and `kb_index_state`; a content-hash incremental indexer with a heading-path chunker (replacing the line-based `chunk_markdown_sections_with_size` and fixed 4096-byte chunks); `text-splitter` for oversize sections; archive exclusion; prune on delete/rename; summary cards and the manifest; a `SearchCorpus::Library` leg (lexical leg chosen by the Phase 1 probe); `vox_kb_search` / `get` / `cite` / `status` / `reindex`; `kb://` resources; `vox kb reindex|status`; `docs/src/library/` plus a `Library` category in `VALID_CATEGORIES` and the sidebar contract.
**Uses:** pulldown-cmark, text-splitter, Tantivy/VoxDb, a hash fn injected from `vox-orchestrator-mcp` (vox-crypto). **Avoids:** Pitfall 12 (index drift), AP3, AP6 (building on the dead mirror), AP7 (committed catalog).

### Phase 3: Provenance, Trust Tiers, Leak Closure and Legacy Retirement
**Rationale:** Fence the existing unreviewed paths before adding *more* web paths (Pitfall 1); Phases 6–7 assume one sink and one retrieval path.
**Delivers:** a required `trust_tier` on every indexed row, mapped onto `ContextTrustTier`; the `kb_findings` inbox table with provenance (summary, quotes, URL, date, content hash) plus a local gitignored full-text snapshot; `persist_web_hits` rerouted into the inbox tier; the `signal_research` 0.95 floor and `signal_chat` post-turn `accepted=1` writeback removed or rerouted; a sanitizer with property tests; retirement of `KbStore` LIKE search, `KbRouter`, the signal adapters, `kb_lines`, the dead `TantivyDocsIndex` leg and the 12 legacy catalog ops (legacy tables quarantined after a row-count check; non-empty rows migrated into `kb_findings`); MENS `KbSignals` retargeted at `kb_findings`; promotion eligibility fails closed on trust errors.
**Avoids:** Pitfall 1, Pitfall 3 (R2), Pitfall 13 (tiers precede the egress policy), AP1/AP2/AP4.

### Phase 4: Taxonomy Contract and Facet Schema Lint (parallel with 2–3)
**Rationale:** Badges, filters, ranking, staleness, rewrites and promotion all read these fields; `authorship` must exist before Phase 8.
**Delivers:** `contracts/documentation/kb-taxonomy.v1.yaml` (SKOS-lite: prefLabel, altLabel, broader, related, scopeNote, deprecated aliases); a schemars-derived facet schema shared by lint, indexer and GUI (`topic`, `domain`, `kb_tier`, `trust_tier`, `scope` vox-internal|external, `review_by`, `verified_at`, `sources[]`, `authorship`, `origin`, `supersedes`/`superseded_by`, `sensitivity`); `training_eligible` rules (forced false for `origin: web-promoted` and LLM-rewritten pages unless `human_attested_by` is set); a page-one summary-block lint (40–120 words); `doc_type` derived from the directory, not authored; blocking on new/changed files with a shrinking backlog allowlist; facets kept separate from the Astro-nav `category`; removal of the `vox-gamify` XP incentive for `training_eligible`.
**Avoids:** Pitfalls 9, 10, 6 (fields), 3 (authorship ordering); the "block all 670 docs at once" gotcha.

### Phase 5: Local Embeddings, Rerank and Contextual Prefix
**Rationale:** Local embeddings are the locked default; rerank and LLM chunk-context are adopted only on a measured eval gain.
**Delivers:** a Candle `gte-modernbert-base` embedder and `gte-reranker-modernbert-base` behind an existing feature/plugin boundary that respects `layers.toml`, reached via `llm_embed` `provider: "local"`; vectors keyed `(chunk_hash, model_id, revision, dim)` with cross-model similarity refused; a deterministic contextual prefix (title › heading_path + page-one summary) for both the embedding input and the BM25 body; RRF with the vector leg; top-20 rerank after a CPU latency benchmark; lexical-only operation plus a `vox doctor` hint when weights are absent; the remote-first ordering in `embedding_env.rs` replaced.
**Avoids:** Pitfall 12 (mixed vectors), Pitfall 8 (adopting techniques with no eval delta).

### Phase 6: Hybrid Context Injection and Per-Chat Controls (backend)
**Rationale:** Requires indexed cards (2), tiers (3), facets (4) and calibrated scores (5).
**Delivers:** `chat/kb_context.rs` — a per-model budget from the registry `context_length` used as a **ceiling**, a relevance floor and inject-nothing path, cards first, a per-doc cap, U-curve ordering, the six preamble sections merged into one list; spotlighting delimiters for non-curated tiers plus tool gating; `knowledge_cutoff` parsing with cutoff-aware ranking only for `scope: external` and only on `verified_at`; citation IDs plus a post-generation validator; `vox_kb_*` tools pinned so they survive the 40-tool alphabetical cap; a `kb_context` turn event including dropped candidates and their drop reasons; static-first prompt ordering for caching; the `sensitivity` egress check; per-chat scope state (`@kb` remapped onto the Library scope, pin restrict/prioritize, exclude, trust floor, the web control merged with research-mode `off`).
**Avoids:** Pitfalls 5, 6, 7, 13.

### Phase 7: CRAG Fallback, Inbox Capture and Promotion PR
**Rationale:** Needs the confidence signal (6) and the inbox table (3); every growth path converges here (web, flag, save-answer, re-verify).
**Delivers:** a low-confidence/recency fallback through the existing `research_turn::run_quick`, free-first with paid spend capped by `tavily_budget.rs` and a spend ledger; quarantined, tool-less summarization of captured pages; `vox_kb_propose` / `inbox_list` / `inbox_review`; a similar-docs check; `kb_promote` (worktree → write → doc-lint → commit → push → draft PR, with per-claim source excerpts in the PR body); `publish_research_doc` and the GUI `ingest_url`/`ingest_text` rerouted; a promotion lint, ≥2-domain corroboration, and CODEOWNERS on `docs/src/library/`.
**Avoids:** Pitfall 2 (laundering), Pitfall 1 (inbox side), copyright exposure (excerpts + hashes, not full pages, in git), SEO spam.

### Phase 8: Page-One Summary Rewrites
**Rationale:** Must follow `authorship` (Phase 4) and should follow the indexer and eval (1–2) so source bias is measurable; after Phase 6 is optional but sensible.
**Delivers:** a guard tool shipped before the first batch (identifier/path/CLI resolution, retired-surface check, negation and number preservation, code fences untouched, no `vox:skip` added); rewrites scoped to the delimited summary block only; batches of ≤25 pages per PR, grouped by category; an `authorship` + `authorship_model` stamp with `training_eligible: false` until attested; a summary-distinctiveness ratchet; a source-bias metric stratified by authorship.
**Avoids:** Pitfalls 3, 4.

### Phase 9: Staleness and Scheduled Re-Verification
**Rationale:** Reads the `review_by`/`verified_at` fields (Phase 4) and drafts into the Inbox (Phase 7).
**Delivers:** hash-jittered `review_by` with a per-facet horizon; a durable `@scheduled` re-verify job with a page/token budget per run (expired and high-traffic pages first); `verified_at` advances only on mechanical evidence (paths exist, symbols resolve via `vox graph query`, CLI commands present in the generated surface, links return 200), while an LLM verdict alone only opens an Inbox item; demote rather than delete (logical archive = `status: legacy`).
**Avoids:** Pitfall 11 (avalanche, rubber-stamping, unbounded spend).

### Phase 10: GUI Library, Inbox and Inspector
**Rationale:** The GUI follows the backend contracts it displays; sub-tracks start early: 10a Library browse/search/reader (absorbing Memory + DocReader) after Phase 2; 10b Inspector `KbContextPanel` + composer controls after Phase 6; 10c Inbox + Open-PR + Settings (spend cap, defaults) after Phase 7; 10d a verification-queue tab after Phase 9.
**Delivers:** Playwright visual steppers with `installTauriMock` stubs for each `vox_kb_*` tool and screenshots in `review-bundle/latest/`; assertions that the Inspector item count equals the backend payload, tier and staleness badges render, an unverified-citation badge appears, no remote image auto-loads, and only ~3 primary controls are visible with the rest under advanced.
**Avoids:** opaque context, knob overload, superseded pages shown as current, zero-click image exfiltration.

### Phase Ordering Rationale

- **Measure before changing (1)** — every phase reports deltas against the ratchet.
- **Provenance before generation (3–4 before 7–8)** — LLM text landing without provenance is irreversible without git archaeology and a MENS retrain.
- **Fence R1/R2 before adding web paths (3 before 7)** — every injection defense is moot while the unreviewed path exists.
- **Stable chunk identity before vectors, citations and the Inspector (2 before 5, 6, 10b).**
- **One MCP contract (2)** gates chat, external MCP hosts and the GUI; legacy consolidation sits in 3 so 5–10 never build on `KbStore`.
- **The GUI is designed against the real `kb_context` payload schema**, so the Inspector shows what was actually sent, not a guess.

### Research Flags

Phases likely needing `/gsd-plan-phase --research-phase <N>`:
- **Phase 2:** the lexical-leg decision — a runtime probe of Turso FTS5 vs `USING fts`, `turso` crate feature gating, and the Tantivy-with-writer fallback (the researchers conflict).
- **Phase 5:** candle `modernbert` with tokenizers 0.21; CPU rerank latency for 20 pairs; feature vs L4 plugin placement under `layers.toml`; Qwen3 batch-1 ingest viability.
- **Phase 6:** calibrating the relevance floor (RRF scores are uncalibrated); the per-model ceiling heuristic (ARCHITECTURE suggests `min(1500 tokens, 2% of max_context)`); recency-detection heuristics for the fallback.
- **Phase 7:** the confidence threshold τ; the host crate given `vox-cli-review`'s CodeRabbit lineage; forge-token SecretId reuse; the **copyright posture** for a public FOSS repo (a legal question).
- **Phase 8:** a faithfulness judge choice under the no-pinned-models policy (cross-family with the rewriter).

Phases with standard patterns (skip research-phase):
- **Phase 1** — BEIR layout and IR metrics are textbook.
- **Phase 3** — in-repo patterns (quarantine mechanism, catalog ops sync).
- **Phase 4** — extends the `vox-doc-pipeline` lint and the ratchet pattern.
- **Phase 9** — the durable `@scheduled` runtime already exists.
- **Phase 10** — existing component patterns (`DiscoveryInbox`, `TurnTrace` exclude chip, `TrustChip`, Playwright mocks).

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | MEDIUM-HIGH | Crate versions, licences and candle module APIs verified against crates.io, HF cards and source; embedding/reranker quality figures are vendor self-reported — the Phase 1/5 eval decides |
| Features | MEDIUM | Vendor docs cross-checked with ≥1 independent source; several forum/blog claims LOW; the existing-Vox-asset inventory is HIGH |
| Architecture | HIGH (current state) / MEDIUM (target) | Current-state map read from source; target checked against `layers.toml` and the crate-edge contract |
| Pitfalls | MEDIUM (literature) / HIGH (R1–R7) | Peer-reviewed, corroborated; the repo pitfalls were confirmed in code |

**Overall confidence:** MEDIUM-HIGH

### Resolved Conflicts (operator decisions applied)

| Conflict | Resolution |
|----------|------------|
| `@kb` mentions — ARCHITECTURE recommended dropping | **Remap** onto the Library scope; a locked v1 control |
| Separate web toggle vs `ResearchModeDropdown` `off` | **One control** — the web fallback merges with research-mode "off" |
| `persist_web_hits` — delete vs default-off vs reroute | **Reroute into the inbox tier**; remove/reroute the `signal_research` 0.95 floor and the post-turn `accepted=1` writeback |
| PITFALLS suggested extending `KbStore` with a tier | **Retire** `KbStore` LIKE search, the signal adapters and the dead Tantivy mirror into `vox-search`-backed retrieval; replace the colliding `vox_kb_*` names |
| New crate edges `vox-search → vox-crypto`, `vox-cli → vox-forge` | **Not taken** — hashing via `vox-orchestrator-mcp`, PR drafting via `vox-cli-review`; both listed only as optional user proposals |
| Embedding runtime (fastembed vs Candle) | **Candle** `gte-modernbert-base` + reranker behind a feature/plugin boundary; fastembed rejected |
| ARCHITECTURE "lexical-first, vectors optional" vs STACK "local embeddings default" | **Local embeddings are the default**; lexical-only is the graceful degradation when weights are absent |
| Golden-set size (30 / 30–50 / 50–100) | ~**40 human-authored seed** + an LLM-generated, human-pruned expansion; BEIR under `contracts/kb/eval/`, excluded from the indexer and MENS |
| Trust vocabulary (PITFALLS proposed curated/inbox/web-live/agent-derived) | Map onto the existing **`ContextTrustTier`** — proposed mapping (to confirm in requirements): SystemVerified = curated + mechanical evidence; Trusted = curated (merged git); Advisory = agent-derived; Untrusted = inbox and web-live |
| Budget (fixed 2% vs per-model) | A per-model **ceiling** with a relevance floor and an inject-nothing path |
| Clock for cutoff-aware ranking | Only `scope: external`, on reviewed `verified_at`, never git mtime |

### Gaps to Address

- **Lexical leg (unresolved researcher conflict):** ARCHITECTURE says use VoxDb FTS5 and delete Tantivy; STACK says the FTS5 probe likely fails on the Turso engine (FTS is Tantivy-backed `USING fts`), so lexical search silently degrades to `LIKE`, and recommends a Tantivy index rebuilt from git markdown. *Recommendation:* run the Phase 1 probe; plan for a Tantivy Library index **with an owning writer** (the indexer) unless the probe proves FTS works. The writer-less mirror is deleted either way.
- **Legacy table row counts:** check `kb_entries` / `knowledge_bases` on real user DBs before quarantine; migrate non-empty rows into `kb_findings(pending)`.
- **`ContextTrustTier` mapping:** confirm the four-way mapping above in requirements, especially what earns `SystemVerified`.
- **Control count:** FEATURES lists a budget slider, trust floor and pin modes; PITFALLS warns to keep ~3 primary controls. Resolve in the UI-SPEC — primary: KB on/off, web (via research mode), `@kb` scope, "strict sources"; the rest under advanced/Settings.
- **Are architecture/ADR/research docs injectable by default?** Recommendation: index all non-archive docs; down-weight `status: research|roadmap|legacy|deprecated` and superseded pages.
- **`vox-cli-review` lifecycle:** if maintainers retire it with CodeRabbit, the `vox-cli → vox-forge` proposal becomes necessary.
- **Copyright posture** for promoted web knowledge needs a human/legal decision. Interim: excerpts + URL + hash in the DB, paraphrase with attribution in git, verbatim-quote length capped by lint.
- **Direct `blake3` use in `vox-search`** is an existing policy violation the `crypto_ban` detector misses — don't copy the pattern into new code; report the detector gap.

## Sources

### Primary (HIGH confidence)
- Local repo, read 2026-10-03: `crates/vox-search/src/{ingest,execution,policy,evaluation,trust,embeddings,embedding_env,lexical_tantivy}.rs`; `crates/vox-db/src/{schema/domains/knowledge.rs,schema_extensions.rs,capabilities.rs,research.rs}`; `crates/vox-orchestrator-mcp/src/{chat_tools/chat/message.rs,memory_tools/retrieval.rs,kb_tools/*}`; `crates/vox-orchestrator/src/{context_envelope.rs,models/spec.rs,catalog.rs}`; `crates/vox-gui/{src/commands/*,ui/src/components/surfaces/*}`; `docs/src/architecture/layers.toml`; `contracts/ci/crate-edges.allow.v1.json`
- Candle `modernbert.rs`, `qwen3.rs`, `models/mod.rs`; HF model cards for gte-modernbert-base, gte-reranker-modernbert-base, Qwen3-Embedding-0.6B, granite-embedding-r2, EmbeddingGemma
- crates.io / npm / PyPI registry APIs (text-splitter 0.33, fastembed 7.1, ort rc.13, tantivy, react-markdown 10.1, remark-gfm 4.0, starlight-llms-txt, ragas, deepeval)
- Anthropic, Contextual Retrieval — https://www.anthropic.com/engineering/contextual-retrieval
- OpenRouter models API (`knowledge_cutoff`, `context_length`) — https://openrouter.ai/docs/api/api-reference/models/get-models
- MCP spec 2025-06-18 (resources vs tools); W3C SKOS; Diátaxis; DITA 2.0; llmstxt.org

### Secondary (MEDIUM confidence)
- Liu et al., Lost in the Middle; Chroma, Context Rot; Cuconasu et al., Power of Noise; Sufficient Context; ClashEval; HoH; Dated Data
- Greshake et al.; Spotlighting (Hines et al.); Design Patterns for Securing LLM Agents; AgentDojo; PoisonedRAG; OWASP LLM Top 10 2025
- Shumailov et al. (Nature 2024); Gerstgrasser et al.; Alemohammad et al.; Dai et al. (retriever source bias); FABLES; StorySumm
- MT-Bench judge bias; Panickssery et al. (self-preference); GraphRAG-Bench; HippoRAG 2; late chunking
- Cursor rules/@Docs, Claude Code memory, Copilot Spaces, Guru verification, Notion verified pages, NotebookLM, ChatGPT deep research, KCS v6, Wikipedia RSP
- Continue indexing, Cursor secure indexing, LlamaIndex IngestionPipeline, Sourcegraph Cody

### Tertiary (LOW confidence)
- Cursor forum threads on stale @Docs; embedding/reranker leaderboard aggregators (they contradict primary sources on licences); an HF practitioner "best practices failed us" post; MinHash thresholds; PKM vocabulary drift; an OpenRouter caching-order blog; a Ropes & Gray web-scraping note (needs legal review)

---
*Research completed: 2026-10-03*
*Ready for roadmap: yes*
