---
phase: 447-before-audit-baseline-and-continuum-contracts
plan: 02
subsystem: research-storage-and-embedder
tags:
  - scientia
  - vox-db
  - baseline-v99
  - pagination
  - embedder
  - disc-01
completed: "2026-11-14"
files_modified:
  - crates/vox-db-types/src/research.rs
  - crates/vox-db-types/src/lib.rs
  - crates/vox-db/src/schema/domains/scientia.rs
  - crates/vox-db/src/schema/domains/agents.rs
  - crates/vox-db/src/schema/manifest.rs
  - crates/vox-db/src/store/open.rs
  - crates/vox-db/src/codex_legacy.rs
  - crates/vox-db/src/research_session_pages.rs
  - crates/vox-db/src/lib.rs
  - contracts/db/baseline-version-policy.yaml
  - crates/vox-scientia/src/embedder.rs
  - crates/vox-scientia/src/lib.rs
requirements_completed:
  - DISC-01
---

# Phase 447 Plan 02 Summary — Additive VoxDb v99 Schema, Cursor Pagination & L3 Shared Embedder

## Delivered Artifacts

1. **Pure L0/DTO Session Pagination Types (`crates/vox-db-types/src/research.rs`):**
   - Added `NewResearchSessionSource`, `ResearchSessionSourceRecord`, `SessionClaimPageItem`, `SessionClaimsPage`, and `SessionSourcesPage` with serde round-trip unit test `test_session_page_dtos_serde_roundtrip`.
   - Re-exported all five DTOs from `crates/vox-db-types/src/lib.rs` and `crates/vox-db/src/lib.rs`.

2. **Additive `VoxDb` Baseline `v99` Schema, Migration & Policy Parity:**
   - Added `continuum_tier TEXT` and `escalated_from TEXT` columns to `scientia_research_sessions` (`crates/vox-db/src/schema/domains/scientia.rs`) and `research_sessions` (`crates/vox-db/src/schema/domains/agents.rs`).
   - Added `CREATE TABLE IF NOT EXISTS research_session_sources` (`UNIQUE(session_id, url)`) and covering cursor indexes `idx_research_session_sources_cursor` on `research_session_sources(session_id, id)` and `idx_scientia_claims_session_cursor` on `scientia_claims(session_id, id)` in `crates/vox-db/src/schema/domains/scientia.rs`.
   - Added PRAGMA-guarded `ensure_v76_research_continuum_columns` in `crates/vox-db/src/store/open.rs`, invoked inside `VoxDb::migrate` prior to `apply_schema_extensions` to idempotently `ALTER TABLE ... ADD COLUMN ... TEXT` on pre-v99 databases.
   - Bumped `BASELINE_VERSION: i64 = 99` in `crates/vox-db/src/schema/manifest.rs`, synced `contracts/db/baseline-version-policy.yaml` (`repository_baseline_integer: 99`, `repository_baseline_digest_hex: "0x57e677717da73a3007e8abbb919427cec9c106ee012152b95f1602e22a7e2b9b"`), and added `"research_session_sources"` in alphabetical order immediately before `"research_sessions"` in `LEGACY_EXPORT_TABLES` (`crates/vox-db/src/codex_legacy.rs`).

3. **Cursor-Paginated Session Queries (`crates/vox-db/src/research_session_pages.rs` — 391 LoC, `<400` budget):**
   - Exported `pub mod research_session_pages;` from `crates/vox-db/src/lib.rs`.
   - Implemented `upsert_session_source`, `update_session_continuum_metadata`, `get_session_continuum_metadata`, `list_session_claims_page`, and `list_session_sources_page` (clamping `limit` to `1..=200`, fetching `clamped + 1` rows ordered by `id ASC` via `trim_page`, computing `next_cursor` and `total_count`).
   - Unit tests (`cursor_pagination_55_sources_and_45_claims`, `upsert_session_source_deduplicates_session_url`, `upgrade_pre_v99_sqlite_adds_continuum_columns_without_data_loss`) use `DbConfig::Memory` or `tempfile::tempdir()`.

4. **L3 Shared Embedder (`crates/vox-scientia/src/embedder.rs` — 324 LoC, `<400` budget):**
   - Exported `pub mod embedder;` in `crates/vox-scientia/src/lib.rs`.
   - Implemented `Embedder`, `cosine_similarity`, `embed_cache_key` (exclusively via `vox_crypto::interop_sha256` and `vox_crypto::hex_encode` over `model + NUL + text`), `LocalHashEmbedder` (`MODEL_ID = "vox-local-hash-v1"`, `DEFAULT_DIMS = 256`, `vox_crypto::fast_hash` feature hashing over word unigrams, word bigrams, and char 3-grams, L2-normalized), `EmbedderBackend`, and `CachedLlmEmbedder`.
   - Routes external embeddings exclusively through `vox_actor_runtime::llm::llm_embed`, resolves config via `vox_search::embedding_env::embedding_config_from_env()`, caches vectors in `VoxDb` (`get_cached_embedding` / `put_cached_embedding`), and provides `from_env_or_local_fallback` for keyless offline operation.

## Static & Structural Inspection Receipts

- `wc -l crates/vox-db/src/research_session_pages.rs` -> `391` (`< 400` LoC)
- `wc -l crates/vox-scientia/src/embedder.rs` -> `324` (`< 400` LoC)
- `rg -n "use (sha2|sha3|blake3)" crates/vox-scientia/src/embedder.rs` -> exit `1` (0 matches)
- `rg -n "\"/(tmp|var|home)"` across all 12 modified/created files -> exit `1` (0 matches)

## Verification Receipts (from `@rust-verifier` board post `#27`)

### 1. `timeout 900s cargo test -p vox-db-types` — PASS (exit 0)

```text
     Running unittests src/lib.rs (target/debug/deps/vox_db_types-a6105ebe5948b9be)

running 43 tests
test research::tests::test_misguidance_reporter_as_str_and_from_str ... ok
test research::tests::test_research_defect_class_as_str_and_from_str ... ok
test research::tests::test_session_page_dtos_serde_roundtrip ... ok
...
test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s

     Running tests/serde_uniformity.rs (target/debug/deps/serde_uniformity-6846522e57833bc1)

running 1 test
test all_row_types_implement_serde ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests vox_db_types

running 1 test
test crates/vox-db-types/src/kb_text.rs - kb_text::SanitizedText (line 319) ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

### 2. `timeout 900s cargo test -p vox-db baseline_policy_matches_compiled_schema` — PASS (exit 0)

```text
     Running unittests src/lib.rs (target/debug/deps/vox_db-3116bbe0fbf81ccb)

running 1 test
test schema::manifest::baseline_digest_policy::baseline_policy_matches_compiled_schema ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 312 filtered out; finished in 0.01s
```

### 3. `timeout 900s cargo test -p vox-db legacy_export_covers_all_baseline_tables` — PASS on default features (0 executed; feature-gated)

```text
     Running unittests src/lib.rs (target/debug/deps/vox_db-3116bbe0fbf81ccb)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 313 filtered out; finished in 0.00s
```

*(Note: `local_tests::legacy_tests::legacy_export_covers_all_baseline_tables` is gated behind `#[cfg(feature = "legacy-import")]` in `crates/vox-db/src/local_tests.rs:461`; `"research_session_sources"` presence in `LEGACY_EXPORT_TABLES` in `crates/vox-db/src/codex_legacy.rs:262` was directly verified by inspection, and `task-345` was queued for `--features legacy-import` at the time of `#27`.)*

### 4. `timeout 900s cargo test -p vox-db research_session_pages` — PASS (exit 0)

```text
     Running unittests src/lib.rs (target/debug/deps/vox_db-3116bbe0fbf81ccb)

running 3 tests
test research_session_pages::tests::upgrade_pre_v99_sqlite_adds_continuum_columns_without_data_loss ... ok
test research_session_pages::tests::upsert_session_source_deduplicates_session_url ... ok
test research_session_pages::tests::cursor_pagination_55_sources_and_45_claims ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 310 filtered out; finished in 13.55s
```

### 5. `timeout 900s cargo test -p vox-scientia --lib embedder` — PASS (exit 0)

```text
     Running unittests src/lib.rs (target/debug/deps/vox_scientia-952f5115a344e3a4)

running 4 tests
test embedder::tests::cosine_similarity_handles_edge_cases_and_unit_bounds ... ok
test embedder::tests::embed_cache_key_matches_known_sha256_vector_and_separates_boundary ... ok
test embedder::tests::local_hash_embedder_determinism_unit_norm_and_discrimination ... ok
test embedder::tests::cached_llm_embedder_round_trips_through_voxdb_embedding_cache ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 347 filtered out; finished in 9.94s
```

### 6. `timeout 900s cargo test -p vox-research-shim --lib types` — PASS (exit 0)

```text
     Running unittests src/lib.rs (target/debug/deps/vox_research_shim-779ae962181339e3)

running 4 tests
test research::types::tests::domain_mode_parses_wire_labels ... ok
test research::types::tests::research_stage_as_str_round_trips ... ok
test research::types::tests::research_stage_ordered_first_and_last ... ok
test research::types::tests::scholarly_domain_mode_serde_round_trip ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 153 filtered out; finished in 0.04s
```
