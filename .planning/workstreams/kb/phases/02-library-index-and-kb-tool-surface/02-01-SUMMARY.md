---
phase: 02-library-index-and-kb-tool-surface
plan: 01
subsystem: search
tags: [vox-db, schema-95, tantivy, kb-ids, chunking, library-index]
requires:
  - phase: 01-retrieval-eval-baseline
    provides: "kb_ids identity SSOT (01-01) and the turso FTS probe (01-03)"
provides:
  - "vox-db baseline 95: library columns on search_documents/search_document_chunks and the kb_index_state table"
  - "VoxDb::apply_library_document / library_chunk_rows / library_chunks_by_hashes (hash-keyed, rowid-stable chunk store)"
  - "kb_ids::frontmatter_range, KbSection, doc_sections (single heading walk; doc_title and doc_heading_paths project from it)"
  - "vox_search::library::{corpus, chunk, lexical} plus index_library and search (indexer-owned Tantivy 0.22 BM25)"
affects: [02-03, 02-04, 02-05, 02-06, 02-07, 02-08, 02-09, phase-03, phase-05]
actuals:
  tokens: 14930
  tasks: 3
  commits: 3
plan_head_before: 7e7b7031871b0a5e6cfb3052291dda4dba41adcb
tech-stack:
  added: []
  patterns:
    - "PRAGMA-guarded ALTER loop, with column-dependent indexes created after the ALTERs in open.rs"
    - "Hash diff with two-pass renumbering through negative temporaries (UNIQUE(document_id, chunk_index))"
    - "Content hasher injected via IndexDeps (vox-crypto is a dev-dependency only, so no crate edge)"
    - "Tantivy rebuilt wholesale from DB rows into a sibling temp dir, then swapped in"
key-files:
  created:
    - crates/vox-db/src/store/ops_memory/library.rs
    - crates/vox-search/src/library/mod.rs
    - crates/vox-search/src/library/corpus.rs
    - crates/vox-search/src/library/chunk.rs
    - crates/vox-search/src/library/lexical.rs
    - crates/vox-search/tests/kb_library_index.rs
  modified:
    - crates/vox-db/src/schema/domains/knowledge.rs
    - crates/vox-db/src/schema/manifest.rs
    - crates/vox-db/src/store/open.rs
    - crates/vox-db/src/store/mod.rs
    - crates/vox-db/src/store/ops_memory/mod.rs
    - crates/vox-db/src/lib.rs
    - contracts/db/baseline-version-policy.yaml
    - crates/vox-search/src/kb_ids.rs
    - crates/vox-search/src/lib.rs
    - crates/vox-search/Cargo.toml
    - Cargo.lock
key-decisions:
  - "schema:approve-as-proposed (trust tier = writer parameter + kb_index_state.trust_tier, no per-row trust column)"
  - "feature:vox-search-default (tantivy-lexical added to vox-search default features)"
requirements-completed: [IDX-01, IDX-02, IDX-07]
status: complete
---

# Phase 2 Plan 01: Schema 95 and library tracer Summary

One markdown file now flows through the Phase 1 identity SSOT (`kb_ids`) into hash-keyed vox-db rows, an indexer-owned Tantivy index, and a ranked `vox_search::library::search` hit, on top of an additive schema-95 migration.

## Decisions

| Decision id | Choice | Approver | Date |
|-------------|--------|----------|------|
| `schema:approve-as-proposed` | Implement schema 95 exactly as proposed. Trust tier is a validated writer parameter (`LibraryDocUpsert.trust_tier`, no `Default`) plus `kb_index_state.trust_tier`; no per-row trust column. | brbrainerd (repo owner) | 2026-10-05 |
| `feature:vox-search-default` | `tantivy-lexical` joins `vox-search` `default` features. A non-gated `tantivy_lexical_enabled_by_default` test in `lib.rs` guards it. | brbrainerd (repo owner) | 2026-10-05 |

Task 1 was the `checkpoint:decision` (blocking-human). It was resolved by the human in chat and produced no commit.

## Accomplishments

- **Schema 95 (Task 2).** `BASELINE_VERSION` is now 95, and `baseline-version-policy.yaml` carries the new digest `0xaaa2601b...a9130`.
  - `search_documents` gained `corpus`, `rel_path`, `frontmatter_json`, `indexed_commit` and `index_error`.
  - `search_document_chunks` gained `chunk_hash`, `chunk_kind`, `heading_path`, `anchor` and `token_estimate`.
  - `kb_index_state` is new.
  - `open.rs` adds the columns to a pre-95 database through a PRAGMA-guarded loop, then creates `idx_search_documents_corpus`, the UNIQUE `idx_search_chunks_doc_hash` and `idx_search_chunks_hash`. These are not in the baseline text, because a v94 database lacks the columns when that text runs.
  - `kb_library_migration_from_94` builds a real v94 file with a legacy row, opens it through `VoxDb::open`, and checks that the row survives with defaults.
- **Library store (Task 2).** `apply_library_document` upserts the document and diffs chunks by hash inside one transaction.
  - Kept rows keep their rowid and body and are renumbered in two passes.
  - Duplicate hashes in one input and invalid trust tiers are rejected.
  - `library_chunk_rows(corpus)` and `library_chunks_by_hashes` are the read side. The by-hash lookup preserves input order.
- **kb_ids extension (Task 3).**
  - Added `frontmatter_range`, `KbSection { heading_path, level, range }` and `doc_sections`.
  - `doc_title` and `doc_heading_paths` now come from the same byte-offset line walk, and every existing Phase 1 kb_ids and kb_eval test passes unchanged.
  - Added tests: `kb_sections_project_heading_paths`, `frontmatter_range_covers_the_closed_block_only` and `sections_split_at_headings_and_h1_is_root`.
- **`vox_search::library` (Task 3).**
  - `corpus::walk_corpus` uses `kb_ids::is_kb_corpus_path` as its only filter and skips symlinks.
  - `chunk::chunk_document` takes title, sections and heading_path only from `kb_ids`. The hash is `H(rel_path 0x00 heading_path 0x00 body)` with a `#n` ordinal for duplicates. Token estimate is `ceil(bytes/4)`. `CHUNKER_VERSION` is `sections-v1+tok-ceil4`.
  - `lexical::LibraryLexicalIndex` rebuilds from DB rows through a temp dir and swap. Fields carry en_stem and boosts (title 2.0, heading 1.5), the query parse is lenient, and ties break on hash.
  - `index_library` runs one DB transaction per document, then rebuilds Tantivy when anything changed, the index is missing, or the counts diverge. The rebuild runs in `spawn_blocking`. `search` hydrates Tantivy hashes from the DB.
- **Tracer tests** (`crates/vox-search/tests/kb_library_index.rs`).
  - `kb_tracer_one_doc_end_to_end`: the first hit for `turbofrobnicate` is `Guide › Install` with that row's 64-hex hash.
  - `chunk_h1_not_duplicated`: there is no `Guide › Guide`, and a renamed H1 still yields the root path.
  - `kb_tracer_reindex_is_stable`: a second run inserts 0, deletes 0, and leaves ids and hashes unchanged.

## Verification

| Check | Result |
|-------|--------|
| `cargo nextest run -p vox-db -E 'test(/kb_library_migration_from_94\|library_store_\|trust_tier_wire\|baseline_policy_matches_compiled_schema\|library::/)'` | 6/6 pass |
| `cargo nextest run -p vox-db --test turso_fts_probe` | 5/5 pass |
| `cargo nextest run -p vox-search -E 'test(/kb_tracer\|chunk_h1_not_duplicated\|kb_sections_project_heading_paths\|tantivy_lexical_enabled_by_default\|kb_ids\|kb_eval\|corpus::\|chunk::\|lexical::/)'` (default features, no `--features`) | 68/68 pass |
| `cargo clippy -p vox-search -p vox-db --all-targets -- -D warnings` | clean |
| `VOX_SKIP_FRESHNESS_CHECK=1 vox ci crate-edges` | OK (689 edges, no new vox-search edge) |
| Acceptance greps | `blake3` is absent from `library/`, `pulldown-cmark` is absent, `parse_query_lenient` is present, `vox-crypto` appears only under `[dev-dependencies]`, `idx_search_chunks_doc_hash` is absent from `knowledge.rs` |

## Deviations from Plan

**1. [Rule 1 - Bug] Clippy lints on new test code**
- **Found during:** Task 3 verification (`cargo clippy --all-targets -D warnings`)
- **Issue:** `cloned_ref_to_slice_refs` in the Task 2 library tests, and `assertions_on_constants` on the `cfg!` guard test.
- **Fix:** Used `std::slice::from_ref`, and added a scoped `#[allow(clippy::assertions_on_constants)]` on the guard test, because the constant is the assertion.
- **Commits:** 22ed3fd85 (vox-db), d9c6221ba (vox-search)

**2. [Rule 3 - Blocking] tdd-guard on `library/mod.rs`**
- **Found during:** Task 3 commit
- **Issue:** The pre-commit `skeleton/untested-pub-api` rule needs a `#[test]` in the same file as `index_library` and `search`. The tracer tests live in `tests/`.
- **Fix:** Added an in-file `library::tests::index_then_search_round_trip_and_missing_index_errors`. It also covers the missing-index error and the invalid-trust-tier path.
- **Commit:** d9c6221ba

**3. Plan shape adjustment, `tantivy_lexical_enabled_by_default`**
- The test sits in `crates/vox-search/src/lib.rs` (`#[cfg(test)]`, not feature-gated) instead of the integration file. That file is gated by `#![cfg(feature = "tantivy-lexical")]`, and a guard inside a gated file could never fail.

## Notes and Deferred Issues

- **`vox ci crate-edges` needed `VOX_SKIP_FRESHNESS_CHECK=1`.** The installed `vox` binary is stale relative to the tree (built at 6382, tree at 6559). The guard output was OK. Re-run it with a refreshed binary (`cargo install --locked --path crates/vox-cli --force`) before the PR. No `contracts/ci/crate-edges.allow.v1.json` exception was written.
- **Pre-existing, not caused by this plan:** `cargo hakari generate --diff` reports a `reqwest` feature-unification diff in `workspace-hack` (`default-tls`, `default-features = false`). It does not involve tantivy or vox-crypto, and `workspace-hack` was left untouched. The same command also reported a stale baseline edge `vox-gui -> vox-db-types`, which `vox ci crate-edges --tighten` would remove. Both belong to whoever owns those changes.
- **Known simplifications** (marked `ponytail:` in code):
  - `library_chunks_by_hashes` issues one query per hash (callers cap at about 50).
  - `LibraryLexicalIndex::search` over-fetches 3x before the tie-break sort.
  - `index_library` neither prunes files removed from the tree nor skips unchanged files yet (02-04).
- `Cargo.lock` changed by one line: the `vox-crypto` dev-dependency on `vox-search`.
- Mutation-proving the H1 guard was not done. The H1 rule is pinned by `chunk_h1_not_duplicated` and `sections_split_at_headings_and_h1_is_root`, not by a break-and-restore check.

## Known Stubs

None.

## Threat Flags

None. New surface is local file reads under `docs/src` through the shared `kb_ids` filter, plus a DB migration covered by T-02-01.

## Self-Check: PASSED

- Created files exist: `library.rs`, `library/{mod,corpus,chunk,lexical}.rs`, `tests/kb_library_index.rs`, and this SUMMARY.
- Commits exist: a85ffa271 (Task 2), 22ed3fd85 (lint fix), d9c6221ba (Task 3).
