---
title: "Turso Indexing Capabilities & All-TursoDB Persistence Audit (2026)"
description: "Technical research on Turso (tursodatabase/turso) indexing trade-offs (composite covering B-trees, partial indexes, FTS, vector/DiskANN, MVCC) and a full codebase persistence audit enforcing the All-TursoDB doctrine."
category: "Architecture SSOTs"
status: "current"
topic: "agent-orchestrator"
domain: "orchestration"
authorship: "llm-generated"
authorship_model: "claude-opus-4-7"
---

# Turso Indexing Capabilities & All-TursoDB Persistence Audit (2026)

<!-- kb:summary -->
This document evaluates `tursodatabase/turso` (`turso = 0.6.1`) indexing capabilities across composite covering B-tree indexes, partial indexes (`WHERE ...`), native Full-Text Search (`USING fts`), vector similarity indexing, and MVCC concurrency, cross-referenced against the 118 tables and 131 indexes in `crates/vox-db`. It also presents a complete workspace persistence audit classifying all 17 storage and index surfaces into three categories: canonical TursoDB stores, sidecar JSON/external stores to migrate into TursoDB (`vox-db`), and documented positive-tradeoff exceptions (`tantivy` derived lexical index, `sqlx` user-app Postgres/MySQL runtime, dev-only `rusqlite` benchmarks, `.vox-cache` L1 AST caches, and MENS `.safetensors`/`.jsonl` training artifacts).
<!-- /kb:summary -->

## 1. Is Turso Indexing Better If We Support It?

### 1.1 Composite & Covering B-Tree Indexes — **Immediate High-Leverage Win**
`turso = 0.6.1` (`turso_core 0.6.1`) implements full B-tree composite and covering index scans with range-scan pushdown. Cross-referencing `crates/vox-db/src/schema/domains/*.rs` against the query hot-paths in `crates/vox-db/src/store/**/*.rs` revealed **10 concrete index optimizations** across our 118 tables:

| # | Domain File | Table | Query Hot-Path / Issue | Recommended Turso Index |
|---|---|---|---|---|
| 1 | `agents.rs` | `llm_interactions` | `ops_scientia.rs:420` filters `WHERE task_category = ?1 AND trace_id IS NOT NULL ORDER BY created_at DESC LIMIT 1` (currently full table scan + sort) | `CREATE INDEX IF NOT EXISTS idx_llm_interactions_cat_created ON llm_interactions(task_category, created_at DESC, trace_id);` |
| 2 | `agents.rs` | `llm_interactions` | `trials.rs:630` (`partition_llm_spend`) sums `cost_usd` partitioned by `trial_run_label IS NOT NULL` vs `IS NULL` across multi-KB `prompt`/`response` rows | Upgrade `idx_llm_interactions_trial` to covering index `ON llm_interactions(trial_run_label, cost_usd)` (100% index-only scan) |
| 3 | `agents.rs` | `llm_attempts` | Lookups by `trace_id` and `attempt_number` currently have zero secondary indexes | `CREATE INDEX IF NOT EXISTS idx_llm_attempts_trace ON llm_attempts(trace_id, attempt_number);` |
| 4 | `publish_cloud.rs` | `scientia_external_intelligence` | Queue polling by `ingest_status` and `ingested_at_ms` currently has zero secondary indexes | `CREATE INDEX IF NOT EXISTS idx_scientia_ext_intel_status ON scientia_external_intelligence(ingest_status, ingested_at_ms DESC);` |
| 5 | `mens_intelligence.rs` | `corpus_pairs` | Filtering by `(label, origin)` currently has zero secondary indexes | `CREATE INDEX IF NOT EXISTS idx_corpus_pairs_label_origin ON corpus_pairs(label, origin);` |
| 6 | `scientia.rs` | `scientia_harness_decisions`, `scientia_harness_fix_proposals` | Foreign keys `REFERENCES scientia_harness_issues(id) ON DELETE CASCADE` lack indexes on `issue_id`, forcing full table scans on parent delete | Add `idx_scientia_harness_decisions_issue(issue_id)` and `idx_scientia_harness_fix_proposals_issue(issue_id)` |
| 7 | `agents.rs`, `knowledge.rs`, `foundation.rs` | `memories`, `behavior_events`, `learned_patterns`, `knowledge_edges`, `search_document_chunks`, `user_preferences`, `account_config` | 8 single-column indexes (`idx_memories_agent`, `idx_behavior_user`, `idx_learned_patterns_user`, `idx_knowledge_edges_src`, `idx_knowledge_edges_dst`, `idx_search_chunks_doc`, `idx_user_preferences_user`, `idx_account_config_account`) are strict left-prefixes of existing composite PKs or composite indexes | Drop the 8 redundant prefix indexes to eliminate unnecessary B-tree write amplification on every `INSERT`/`UPDATE` |

### 1.2 Partial Indexes (`CREATE INDEX ... WHERE ...`) — **Nuanced Tradeoff in Turso `0.6.1`**
- **Parser vs. Bytecode Support:** While `turso 0.6.0-pre.8` rejected partial index `WHERE` clauses (`Parse error: Partial indices are not supported yet`), `turso 0.6.1` parses and executes simple partial indexes (`vox-db` currently ships 4 partial indexes in `conversations.rs:93`, `scientia.rs:431`, and `vox_mesh.rs:58,154`).
- **The Mutable-Nullable `IdxDelete` Hazard:** As documented in `crates/vox-db/src/schema/domains/scientia.rs:361-364`, Turso `0.6.1` has an `IdxDelete` bytecode bug (`no matching index entry`) when a row's indexed column transitions from `NULL` to non-`NULL` on `UPDATE`.
- **Policy:**
  - Use `CREATE UNIQUE INDEX ... WHERE ...` **only** on immutable/insert-time columns requiring partial uniqueness (`conversations`, `scientia_harness_issues`).
  - Use **composite covering B-tree indexes** (e.g. `(trial_run_label, cost_usd)`) instead of partial indexes on nullable lifecycle or telemetry columns: they avoid the `IdxDelete` hazard and simultaneously accelerate both `IS NOT NULL` and `IS NULL` aggregations via index-only scans.

### 1.3 Full-Text Search: Turso `USING fts` vs. `vox-search` `tantivy`
- **Finding:** `crates/vox-db/src/schema_extensions.rs` attempts `PRAGMA compile_options` + `CREATE VIRTUAL TABLE ... USING fts5(...)` + `CREATE TRIGGER`. As proven by `crates/vox-db/tests/turso_fts_probe.rs`, Turso does not implement C-SQLite's `fts5` virtual table module (`schema_extensions.rs` is dead code on Turso).
- **Turso's Native FTS (`CREATE INDEX ... USING fts`):** Turso's native FTS is itself a wrapper around `tantivy` inside `turso_core` (`fts = ["dep:tantivy"]`), gated behind `--experimental-index-method`.
- **Tradeoff Verdict:** Keep standalone `tantivy 0.22` in `vox-search::library::lexical` as a **derived index rebuilt from `VoxDb` rows**. Enabling `turso`'s experimental `fts` feature would pull a duplicate `tantivy 0.26` into the dependency graph, cannot index joined columns across `search_documents.title` + `search_document_chunks.body_text`, and lacks the custom BM25 field boosts (`TITLE_BOOST = 0.25`, `HEADING_BOOST = 0.25`) that lifted Vox's KB retrieval nDCG@10 from `0.358` to `0.549`.

### 1.4 Vector Indexing: Turso `embeddings` BLOB vs. External `Qdrant`
- **Finding:** At Vox's corpus scale (`< 25,000` chunks × 768 dims), brute-force `f32` cosine similarity over Turso's `embeddings.vector` `BLOB` (`crates/vox-db/src/store/ops_memory/embedding.rs:70-138`) executes in `< 5 ms` in-process with zero external network hops.
- **Optimization:** `scientia_embedding_cache` (`crates/vox-db/src/schema/domains/scientia.rs:331-336`) currently stores vectors as JSON `TEXT` (`vector_json`). Converting `vector_json` to little-endian `f32` `BLOB` cuts storage 3.1× (`9.5 KB -> 3.0 KB` per 768-d vector) and eliminates JSON float-parsing overhead.

---

## 2. Codebase-Wide Persistence Audit ("All TursoDB Unless Positive Tradeoff")

| # | Storage Surface | Location | Decision | Rationale / Action |
|---|---|---|---|---|
| 1 | **Main Codex Store (`store.db`)** | `crates/vox-db/` (`turso 0.6.1`) | **KEEP (Canonical TursoDB)** | Primary workspace database (118 active tables). |
| 2 | **Clavis Secret Vault (`clavis_vault.db`)** | `crates/vox-secrets/src/backend/vox_vault.rs` (`turso 0.6.1`) | **KEEP (TursoDB)** | Uses TursoDB; separate `~/.vox/clavis_vault.db` file isolates encrypted DEKs from project repos. |
| 3 | **Populi Legacy JSON Stores (`a2a-store.json`, `exec-lease-store.json`, `dispatch-store.json`)** | `crates/vox-populi/src/transport/store/mod.rs:171-328` | **MIGRATE TO TURSO** | No positive tradeoff. `VoxDbMeshStore` (`vox_mesh.rs`) already exists in `vox-db`; retire the JSON file fallback. |
| 4 | **Mesh Replay State (`mesh-replay-state.json`)** | `crates/vox-populi/src/transport/mesh_replay.rs:25-136` | **MIGRATE TO TURSO** | Whole-file JSON rewrite on every JWT `jti` check. Move to `mesh_replay_nonces` table in `vox_mesh.rs`. |
| 5 | **Populi Local Node Registry (`local-registry.json`)** | `crates/vox-populi/src/node_registry.rs:23-103` | **MIGRATE TO TURSO** | Whole-file JSON rewrite on heartbeats. Consolidate into `node_identities` / `mesh_heartbeats` in `vox-db`. |
| 6 | **Telemetry Upload Queue Spool (`pending/*.json`)** | `crates/vox-cli/src/telemetry_spool.rs:1-170` | **MIGRATE TO TURSO** | Directory of `<uuid>.json` files causes inode/stat churn. Replace with `telemetry_upload_spool` table in `vox-db`. |
| 7 | **External Qdrant Vector Client (`qdrant-vector`)** | `crates/vox-search/src/vector_qdrant.rs:1-130` | **MIGRATE TO TURSO** | External non-Turso server is unnecessary below 200k chunks; route `CodeKnnIndex` through `VoxDb::search_similar_embeddings`. |
| 8 | **Model Catalog Cache (`model-catalog.v1.json`)** | `crates/vox-orchestrator/src/models/registry/catalog_seed.rs:163` | **MIGRATE TO TURSO** | Store cached catalog payload inside `VoxDb` (`model_pricing_catalog` / `user_preferences`) to eliminate file/DB split-brain. |
| 9 | **Unused `bincode` Dependencies** | `crates/vox-db/Cargo.toml:20`, `crates/vox-package/Cargo.toml:19` | **REMOVE DEAD DEPS** | Zero call sites in `vox-db` or `vox-package`. |
| 10 | **Tantivy Lexical Index (`tantivy 0.22`)** | `crates/vox-search/src/library/lexical.rs` | **KEEP (Positive Tradeoff)** | 100% derived cache rebuilt from Turso `VoxDb` rows; custom multi-table BM25 boosts (`+53%` nDCG@10). |
| 11 | **Markdown Memory (`MEMORY.md`)** | `crates/vox-orchestrator/src/memory/manager.rs` | **KEEP (Positive Tradeoff)** | Human-editable, git-diffable agent memory dual-written into Turso `VoxDb` (`memories` + `knowledge_nodes`). |
| 12 | **`vox-sql` `sqlx` (`postgres`/`mysql`)** | `crates/vox-sql/Cargo.toml` | **KEEP (Positive Tradeoff)** | Optional runtime adapter for *user-authored* Vox programs targeting external cloud DBs; isolated by `run_sqlx_isolation_guard`. |
| 13 | **`rusqlite 0.32` in `vox-db`** | `crates/vox-db/Cargo.toml:70` (`[dev-dependencies]`) | **KEEP (Positive Tradeoff)** | Dev-only benchmark reference for comparing Turso vs. C-SQLite; never linked into production binaries. |
| 14 | **L1 AST & Build Caches (`.vox-cache/`)** | `crates/vox-drift-check/src/cache.rs`, `crates/vox-package/src/artifact_cache.rs` | **KEEP (Positive Tradeoff)** | Synchronous pre-commit L1 lint and compiler output directories that cannot depend on async `vox-db` per `layers.toml`. |
| 15 | **MENS Corpora & Weights (`.jsonl`, `.safetensors`)** | `crates/vox-corpus/`, `crates/vox-quantize/` | **KEEP (Positive Tradeoff)** | Native tensor weight and streaming dataset formats required by Candle, QLoRA, and HuggingFace Hub. |
