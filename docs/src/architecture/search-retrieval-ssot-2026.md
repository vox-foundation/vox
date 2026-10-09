---
title: "Search & Retrieval SSOT (2026)"
description: "Single baseline for how agent-facing retrieval works across vox-db contracts, vox-search execution, orchestrator MCP tools, and dashboard transport — including the two distinct \"VoxDB\" surfaces."
category: "Architecture SSOTs"
sort_order: 52
status: "current"
training_eligible: true
training_rationale: "Prevents drift between FTS, embeddings, policy knobs, and GUI wiring."
schema_type: "TechArticle"
audience: ["contributors", "agents"]
related:
  - docs/src/architecture/data-storage-ssot-2026.md
  - crates/vox-db/src/retrieval.rs
  - crates/vox-search/src/execution.rs
  - crates/vox-search/src/policy.rs
---

# Search & Retrieval SSOT (2026)

## 1. Scope

This document is the **baseline for agent-facing search and retrieval**: planning, execution, storage contracts, policy, MCP tools, and dashboard integration.

It does **not** replace app-author documentation for generated Convex-style schemas (`voxdb/server`); see Section 4.

## 2. Canonical pipeline

| Layer | Crate / surface | Responsibility |
|-------|-----------------|----------------|
| Contracts & fusion | `vox-db` (`retrieval.rs`, `store/ops_memory.rs`) | `SearchPlan`, `RetrievalResult`, `fuse_hybrid_results`, heuristic planner `heuristic_search_plan`, FTS / embeddings SQL |
| Execution | `vox-search` | `execute_search_plan`, `run_search_with_verification`, corpora routing, optional Tantivy / Qdrant / web |
| Policy | `vox-search` (`policy.rs`) | `SearchPolicy`: fusion weights, budgets, backend toggles — resolved via **vox-secrets** (`resolve_secret`), not raw `std::env::var` in consumers |
| Entry points | `vox-cli`, `vox-orchestrator` | CLI research paths; MCP tools `vox_memory_search`, `vox_knowledge_query`; chat preamble uses the same retrieval bundle as `vox_memory_search` |

**Rule:** CLI, orchestrator, MCP, and chat preamble must **not** fork retrieval logic; they call into `vox-search` with a `SearchRuntimeContext` and `SearchPolicy`.

## 3. Two distinct “VoxDB” surfaces (do not conflate)

### 3.1 Rust `vox-db` (Turso / libSQL)

Used by **`vox-search`** for durable corpora:

- `search_documents` / `search_document_chunks` (+ FTS5 shadow table when available)
- `embeddings` (vector blobs; brute-force similarity for modest tables)
- `knowledge_nodes` (graph / FTS)

Hybrid chunk retrieval merges lexical hits (`chunk_id` = `search_document_chunks.id` as string) with vector hits (`embeddings.source_id` for `source_type = "search_document_chunk"`) via **`fuse_hybrid_results`**. **Stable `chunk_id` alignment** between ingest, embedding storage, and hybrid query is mandatory — regressions produce two parallel ranked lists with no overlap.

### 3.2 Generated `voxdb/server` (TypeScript schema)

The compiler emits **`defineSchema` / `searchIndex(...)`** from `@search_index` on `@table` declarations (see `crates/vox-codegen-ts/src/schema/from_hir.rs`). That targets the **app author's** Convex-compatible runtime and is **not** invoked by `vox-search`.

## Language surface (decision)

- **Declarative only** for app data: `@search_index`, `@vector_index`, `@index` on `@table`; emitted `searchIndex(...)` / `vectorIndex(...)` in `voxdb/server`.
- **No global `search()` Vox builtin** in `eval`/stdlib — agent retrieval stays on the orchestrator path (`vox_memory_search`, chat preamble, CLI).
- Optional future work: an `@retrieval` decorator that lowers to planner overrides — deferred until a concrete consumer exists.

## 4. Corpus → backend matrix (authoritative per corpus)

| Corpus | Primary backend | Notes |
|--------|-----------------|-------|
| Agent markdown memory | In-process BM25 + optional embeddings + `fuse_hybrid_results` | Policy weight: `memory_vector_fusion_weight` |
| Knowledge graph | SQLite FTS / LIKE on `knowledge_nodes` | Exposed as MCP `vox_knowledge_query` |
| Ingested doc chunks (RAG) | LIKE + `embeddings` hybrid; FTS5 is unavailable on turso 0.6.1 (probe recorded in KB Phase 2), so LIKE is what runs in practice | Fusion weight: `chunk_vector_fusion_weight`, passed into `query_search_document_chunks_hybrid` (not hard-coded in `vox-db`) |
| Repo inventory | Bounded filesystem walk + token overlap | Interim until persistent code indexes exist |
| Symbol proximity | `vox-search::symbol_proximity` (`scan_symbol_proximity`) | Contracts-aware lexical proximity vs optional query vectors; planner corpus `SymbolProximity` |
| Semantic FS (intent paths) | `vox-search::semantic_fs` (`discover_files_for_intent`, `retrieve_evidence_for_intent`) | Same inventory ranker as repo paths; MCP exposes `semantic_fs_discover` for AgentOS intent-shaped discovery |
| KB Library (`docs/src/**` accepted by `kb_ids::is_kb_corpus_path`, no archive; includes `docs/src/library`) | Tantivy 0.22 index owned by the indexer (`vox_search::library`), under `<data_dir>/kb/library` (`VOX_SEARCH_TANTIVY_ROOT` overrides the root) | Heading chunks keyed by `chunk_hash`; rebuilt from DB rows on change; chosen over FTS5 by the Phase 2 probe |
| Sidecar ANN (optional) | Qdrant (`qdrant-vector` feature) | Parallel to DB chunk search |
| Web | SearXNG / DDG / optional Tavily | Policy-gated |
| Graphify search log (virtual) | Turso `graphify_search_hit` nodes | Virtual corpus tracking queries and FNV-64 hashes. Always fresh; bypassed by lexical check |

## 5. Policy knobs (vox-secrets / env)

- **Memory hybrid:** `VOX_SEARCH_MEMORY_VECTOR_WEIGHT` → `SearchPolicy.memory_vector_fusion_weight` (default `0.55`).
- **Chunk hybrid:** `VOX_SEARCH_CHUNK_VECTOR_WEIGHT` → `SearchPolicy.chunk_vector_fusion_weight` (default `0.60`), threaded into `VoxDb::query_search_document_chunks_hybrid` as a scalar so `vox-db` stays free of `SearchPolicy`.
- **BM25 (memory index):** `VOX_SEARCH_BM25_K1`, `VOX_SEARCH_BM25_B` → clamped into `MemorySearchEngine` via `SearchPolicy` (`memory_bm25_k1` / `memory_bm25_b`).
- **RRF fusion:** `VOX_SEARCH_RRF_K` → reciprocal-rank constant for `rrf_merge_line_lists`.
- **Web hit capture:** `VOX_SEARCH_PERSIST_WEB_HITS_DISABLED` — when unset/false and a `WebFindingCapture` sink is injected, `execute_search_plan` and the Tavily CRAG path in `bundle.rs` hand SearXNG/DDG/Tavily-style web hits to the findings inbox as pending `Untrusted` `kb_findings` rows (deduplicated by content hash). The flag no longer writes `search_documents`: web text never enters the corpus (see [Trust tiers and the findings inbox](#trust-tiers-and-the-findings-inbox)).
- **Embedding model name:** `VOX_EMBEDDING_MODEL` (`SecretId::VoxEmbeddingModel`) — resolved in `embedding_env.rs` per provider (HF / OpenAI / OpenRouter defaults remain at the call site when unset).

**Dependency slimming:** consumers that only need `EmbeddingService` (e.g. `vox-scientia-ingest`, `vox-plugin-publication`) should depend on `vox-search` with **`default-features = false`** and enable only `tantivy-lexical` / `web-scrape` as needed — avoids pulling default `qdrant-vector` + `tavily`.

## 6. MCP tools and GUI

| Tool | Purpose |
|------|---------|
| `vox_memory_search` | Full retrieval bundle (memory, knowledge, chunks, repo, optional RRF / web paths per plan) |
| `vox_knowledge_query` | Narrow query against `knowledge_nodes` only |
| `vox_research_run` | Orchestrator `run_research` pipeline: `vox-search` web tier (SearXNG → DDG → Tavily) + optional CRAG hops + synthesis/judge when LLM env is configured — see [`deep-research-prior-art-and-vox-roadmap-2026.md`](deep-research-prior-art-and-vox-roadmap-2026.md) |
| `vox_kb_search` | BM25 search over the KB Library index; hits carry `kb://` URIs |
| `vox_kb_get` | Fetch a Library document (by `path`) or chunk (by `chunk_hash`) from the DB; never reads files |
| `vox_kb_cite` | Citation link and `kb://chunk/<hash>` for a chunk |
| `vox_kb_status` | Index state: corpus root and commit, counts, chunker version, lexical backend, embed model, trust tier, errors, consistency |
| `vox_kb_reindex` | Rebuild the Library index from the repository `docs/src` tree (no path arguments, single-flight; the daemon is the single writer). CLI: `vox kb status` / `vox kb reindex` |
| `vox_graphify_search` | Lexically search a graphify corpus graph on disk (read-only) |
| `vox_graphify_query` | BFS neighbor expansion from seed node IDs (read-only; layered navigation) |
| `vox_graphify_path` | Shortest path between two node IDs in a graphify corpus (read-only) |
| `vox_graphify_compare` | Diff two graphify corpus manifests for node/edge/community delta (read-only) |

The Library is also served as `kb://doc/<path>` and `kb://chunk/<hash>` MCP resources. After upgrading, restart a running `vox-orchestrator-d` so it serves the new `vox_kb_*` tools.

**Dashboard:** use `POST /v1/tools/call` (`voxTransport.callTool`). No client-side BM25/vector index unless there is an explicit offline product requirement.

## Trust tiers and the findings inbox

KB Phase 3 gives every stored and injected retrieval line a trust tier, routes model- and web-derived text into a review inbox instead of the corpus, and sanitizes text at ingest and again at injection. It also retired the legacy VoxKB surface (the LIKE-search store, the keyword router, the model-output signal adapters and the 12 legacy CRUD tools).

### Four-way tier mapping

| Tier (`ContextTrustTier`) | Wire string | What carries it |
|---|---|---|
| `SystemVerified` | `system_verified` | Curated content plus mechanical verification evidence (`tier_for_curated(true)`). |
| `Trusted` | `trusted` | Curated content merged to git and indexed by the Library indexer. |
| `Advisory` | `advisory` | Agent-derived content built from curated inputs only; knowledge-graph nodes of type `fact` or `concept`. |
| `Untrusted` | `untrusted` | Inbox findings, live web hits, legacy web rows (re-tiered, never purged), `external_research` nodes, every other or missing node type, and anything derived from `Untrusted` input. |

The type lives in `vox-orchestrator-types::trust` and has no `Default`: Turso cannot enforce a CHECK, so every writer names the tier at the type boundary, and parsing a stored string is fail-closed (anything unrecognised is `Untrusted`). Variants are ordered by ascending trust, so `min` picks the least trusted.

**Taint rule (D-06).** An LLM synthesis of `Untrusted` input is `Untrusted`, never `Advisory`. Only the knowledge-node types `fact` and `concept` map to `Advisory` (`tier_for_knowledge_node_type`); research-source bookkeeping nodes and every other type are `Untrusted`. A finding can only be `Untrusted` or `Advisory` (`FindingTier`); it can never be `Trusted` or `SystemVerified`.

**Where the tier is stored.** `search_documents.trust_tier` (`NOT NULL DEFAULT 'untrusted'`, baseline 96) is set by every writer. For Library corpus rows the single source is `kb_index_state.trust_tier`, aligned onto the rows by `put_kb_index_state` in one transaction. A static test pins the set of code allowed to write `search_documents` or `kb_findings`.

### Injection rules

- The automatic chat preamble (`AutoChatPreamble`) drops `Untrusted` hits (`SearchPolicy::exclude_untrusted_hits`). Explicit tool queries (`ExplicitToolQuery`) return them, labelled with a `[tier:<wire string>]` line prefix.
- `RetrievalEvidenceEnvelope::min_trust_tier` is the minimum of the kept hits and is `Untrusted` when there are none; there is no blanket stamp.
- Vector-sidecar lines carry no stored tier and count as `Untrusted`.
- Known gap: Tavily `web_lines` appended in `bundle.rs` bypass the tier filter. Nothing consumes them today; Phase 7 must treat them `Untrusted` before any consumer reads them.

### The findings inbox

Table `kb_findings` holds pending findings until a human reviews them. Every capture path goes through one chokepoint: web hits implement `WebFindingCapture` (`vox-search` `capture`), the MCP sink calls `kb_tools::findings::capture_finding`, and the row lands in `kb_findings` as pending and `Untrusted`, after secret redaction and sanitization, with a BLAKE3 content hash. A content-addressed snapshot is written under `.vox/cache/kb-findings/`; snapshots are never committed. Research turns, deep-research gathers and the GUI side doors (Ingest URL, Ingest Text, Publish) file the same way; `vox research publish` writes a local draft under `.vox/cache/research-drafts/` instead of `docs/src`.

Legacy VoxKB rows are copied into the inbox as pending `Untrusted` findings on every database attach (`migrate_legacy_kb_entries`, idempotent and copy-only). The legacy tables are quarantined, not dropped: fresh databases no longer create them, existing databases keep their rows, and `vox kb status` prints the legacy counts and the pending/total inbox counts until the Phase 12 Inbox GUI exists. MENS `KbSignals` reads only reviewed findings and emits DPO pairs only; SFT data already exported from legacy rows cannot be recalled.

Phase 3 has no automatic promotion from the inbox into the corpus. `vox_search::trust::promotion_eligibility` is the fail-closed verdict a later promotion step must consult (an unavailable Crossref lookup is ineligible).

### Sanitization

One KB-text sanitizer, `vox_db_types::kb_text::sanitize` (re-exported as `vox_db::kb_text`), neutralizes the Unicode tag block, zero-width and bidi characters, variation selectors, HTML comments, images whose host is not on `IMAGE_HOST_ALLOWLIST` (including reference-style images), and chat role tokens, iterating to a bounded fixpoint and failing closed to the empty string when the input will not converge. It runs at ingest (the inbox writer takes a `SanitizedText`) and at injection: every `run_retrieval_bundle` leg, numbered research sources, and every `vox_kb_*` tool result.

It is the only sanitizer on the KB path. Two prompt helpers stay outside it and are separate sanitizers, not KB-text sanitizers: `vox_config::sanitize_chatml` and a private copy in the `vox-corpus` arca replay.

## 7. Telemetry

Orchestrator wraps retrieval in **`RetrievalEvidenceEnvelope`** (`crates/vox-orchestrator/src/mcp_tools/memory_tools/retrieval.rs`). New retrieval paths must preserve this envelope.

## 8. Sandbox tiers

`vox-search` pulls native/async/network dependencies. **Interp / WASI tiers must not link retrieval inline**; scripts use host tools / orchestrator bridges.

## 9. Related crates

- `vox-scientia-ingest` depends on `vox-search` (e.g. `EmbeddingService` in `deduplicator.rs`); do **not** remove `vox-search` from that crate.

## 10. Change checklist

- After retrieval / persistence changes: `cargo test -p vox-db`, `cargo test -p vox-search`, and **`vox ci data-storage-guard`** when contracts shift.
- After new Secrets-backed env: `SecretId` + `SecretSpec`, bump [`contracts/config/env-vars.v1.yaml`](../../../contracts/config/env-vars.v1.yaml), run **`vox ci secrets-contracts`**.
- After new architecture pages: set frontmatter so the Starlight sidebar lists them; add a row to [`where-things-live.md`](where-things-live.md) if the page introduces a new concept→crate mapping. Link from root [`AGENTS.md`](../../../AGENTS.md) only when the page is a new always-loaded policy pointer.
