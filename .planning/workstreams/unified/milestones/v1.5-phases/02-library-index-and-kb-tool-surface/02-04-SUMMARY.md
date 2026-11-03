---
phase: 02-library-index-and-kb-tool-surface
plan: 04
subsystem: search
tags: [vox-db, library-index, incremental-reindex, kb-index-state, tantivy, prune]
requires:
  - phase: 02-library-index-and-kb-tool-surface
    provides: "02-01 hash-keyed chunk store, index_library, Tantivy rebuild; 02-03 CHUNKER_VERSION and card chunks"
  - phase: 01-retrieval-eval-baseline
    provides: "turso_fts_probe (IDX-07 engine characterization)"
provides:
  - "index_library skips unchanged files by hash, prunes deleted/renamed docs and orphan kb_chunk vectors, and writes a kb_index_state manifest every run"
  - "library::{LibraryStatus, status}: manifest plus a live DB-vs-Tantivy consistency check"
  - "lexical::{resolve_index_dir, default_index_dir} (VOX_SEARCH_TANTIVY_ROOT or <data_dir>/kb/library)"
  - "vox-db: KbIndexState, LibraryDocRow, library_document_file_hash, library_chunk_count, prune_library_documents_not_in, prune_orphan_kb_chunk_embeddings, put/get_kb_index_state, library_document_by_path, library_doc_chunks, library_chunk_by_hash"
affects: [02-05, 02-06, 02-07, 02-09, phase-05]
actuals:
  tokens: 14000
  tasks: 3
  commits: 3
plan_head_before: aae5451df1dcd474cf630fc1d6256d18fc7fa80e
tech-stack:
  added: []
  patterns:
    - "Hash skip is sound only with a manifest whose chunker_version matches; no manifest means re-chunk everything"
    - "Prune keeps every walked path, errored files included, so a transient read error never deletes a doc"
    - "Tantivy is rebuilt from DB rows on any change, prune, or count mismatch (AP-16)"
key-files:
  created:
    - crates/vox-db/tests/kb_library_store.rs
  modified:
    - crates/vox-db/src/store/ops_memory/library.rs
    - crates/vox-db/src/store/ops_memory/mod.rs
    - crates/vox-db/src/store/mod.rs
    - crates/vox-db/src/lib.rs
    - crates/vox-search/src/library/mod.rs
    - crates/vox-search/src/library/lexical.rs
    - crates/vox-search/tests/kb_library_index.rs
key-decisions:
  - "Hash skip requires a stored kb_index_state whose chunker_version equals CHUNKER_VERSION; a missing manifest forces a full re-chunk (a pre-02-04 DB has rows of unknown provenance)"
  - "Trust tier is validated at the top of index_library, because skipped files never reach the store's own validation"
  - "indexed_at is assigned by the store (datetime('now')); the value on KbIndexState passed to put is ignored"
  - "library_document_by_path takes no corpus argument and is hard-filtered to the library corpus; library_chunk_by_hash likewise"
requirements-completed: [IDX-01, IDX-03, IDX-05, IDX-07]
status: complete
---

# Phase 2 Plan 04: Incremental reindex, prune and manifest Summary

Reindexing is now incremental and self-describing: an unchanged tree writes nothing, one edited section costs one delete plus one insert, deleted or renamed docs and their orphan `kb_chunk` vectors are pruned, and every run records a `kb_index_state` manifest that `library::status` compares against the live Tantivy index.

## Commits

- `ac33dd51a` feat(02-04): skip unchanged library files by hash and prove one-section rewrites (Task 1, tracer)
- `77568f45c` feat(02-04): prune deleted docs and orphan vectors, record the kb_index_state manifest (Task 2)
- `49609d3e6` feat(02-04): corpus-filtered library readers and store-level renumber test (Task 3)

## What was built

- **File-hash skip (IDX-01).** `index_library` hashes the normalized text before chunking and, when it equals `VoxDb::library_document_file_hash`, counts the file in `IndexReport.docs_unchanged` and does no DB writes for it. A changed `CHUNKER_VERSION` in the stored manifest (or no manifest) disables the skip.
- **Identity-level proof.** `kb_reindex_unchanged` (3 docs unchanged, snapshot of every `(rowid, chunk_hash)` identical), `kb_reindex_one_section` (exactly 1 delete + 1 insert, every other pair survives, docs A and C untouched), `kb_reindex_insert_middle_section` (1 insert, 0 deletes, `chunk_index` contiguous and in document order) and `kb_kept_hash_keeps_vector`.
- **Prune.** `prune_library_documents_not_in` deletes chunk rows explicitly and then the doc row, in one transaction, only inside the requested corpus. `prune_orphan_kb_chunk_embeddings` deletes `kb_chunk` vectors whose `source_id` is no longer any `chunk_hash`; other `source_type`s are untouched. The indexer passes every walked path as the keep list.
- **Manifest (IDX-03, IDX-07).** `put_kb_index_state` (upsert) / `get_kb_index_state`. Each run records corpus root and commit, doc/chunk/card counts, chunker version, `lexical_backend = "tantivy-0.22"`, trust tier, NULL embed fields (AP-07), `errors_json` and duration. Doc count is distinct docs with chunks; cards are `chunk_kind = 'card'` rows.
- **Status.** `library::status(db, lexical_dir)` returns `LibraryStatus { state, db_chunk_count, lexical_docs, consistent }`. `consistent` needs a manifest at the current chunker version and Tantivy `num_docs == DB chunk count`. Deleting the Tantivy directory flips it to false; the next run rebuilds it with no doc changes.
- **Index location.** `resolve_index_dir` (pure) and `default_index_dir`: `<VOX_SEARCH_TANTIVY_ROOT>/library` when the secret is non-blank, else `<data_dir>/kb/library`. The `SecretId` and its registry entry are unchanged.
- **Readers.** `library_document_by_path`, `library_doc_chunks` (chunk_index order, card first), `library_chunk_by_hash` (corpus-filtered). `library_chunks_by_hashes` now delegates to `library_chunk_by_hash`.

## Verification

| Check | Result |
|-------|--------|
| `cargo nextest run -p vox-search -E 'test(/kb_\|library::\|resolve_index_dir/)'` (default features include tantivy-lexical) | 85/85 pass after Task 2 |
| `cargo nextest run -p vox-db -E 'test(/kb_store_\|library_document_by_path\|library_doc_chunks\|library_chunk_by_hash\|library::\|kb_index_state\|prune_/)'` | 16/16 pass |
| `cargo nextest run -p vox-db --test turso_fts_probe` (Phase 1, required precondition) | 5/5 pass; no second probe added |
| Combined `-p vox-db -p vox-search` over `kb_\|library\|prune_\|resolve_index_dir` | 101/101 pass |
| `cargo clippy -p vox-search -p vox-db --all-targets -- -D warnings` | clean |
| `cargo check -p vox-search -p vox-db` | clean |
| `VOX_SKIP_FRESHNESS_CHECK=1 vox ci crate-edges` | OK, 689 edges; no `Cargo.toml` touched, no exception written |
| Acceptance greps | `docs_unchanged`, `prune_orphan_kb_chunk_embeddings`, `resolve_index_dir`, `library_chunk_by_hash` present; `VoxSearchTantivyRoot` still in `missing.rs` |

The Task 1 red run failed on `docs_unchanged` before the skip existed (2 of 4 failing; the middle-insert and kept-vector tests already passed, which shows 02-01's two-pass renumber needed no fix, Pitfall 3).

## Mutation proofs (T-02-10, T-02-11, orphan cleanup)

Each row records presence counts before the edit and after restore (restored from a backup copy, then re-grepped; restored runs green).

| Guard | Mutation | Result |
|-------|----------|--------|
| keep list includes errored files (`walked.push` before the read) | moved the push to after the read succeeds (presence 1, 1) | `kb_status_reports_errors` FAIL (`docs_pruned` 1, expected 0) |
| `prune_orphan_kb_chunk_embeddings` call in `index_library` | commented out (presence 1, 0, 1) | `kb_no_orphan_embeddings` FAIL (4 orphan rows left) |
| corpus filter in `library_chunk_by_hash` | `d.corpus = ?1` replaced by `?1 = ?1` (presence 1, 0, 1) | `kb_store_readers_never_return_other_corpora` and `library_chunk_by_hash_is_corpus_filtered` FAIL |

## Deviations from Plan

**1. [Rule 1 - Bug] Bad trust tier no longer rejected once unchanged files skip the store**
- **Found during:** Task 1 verification (`library::tests::index_then_search_round_trip_and_missing_index_errors` failed).
- **Issue:** the store was the only trust-tier validator; a skipped file never reaches it.
- **Fix:** `vox_db::validate_trust_tier_wire(deps.trust_tier)?` at the top of `index_library`.
- **Commit:** `ac33dd51a`

**2. Plan shape adjustment, chunker-version guard lands in Task 2**
- The plan's Task 1 text mentions the stored chunker version, which comes from `kb_index_state`. That table has no writer until Task 2, so Task 1 shipped the hash-only skip and Task 2 added the manifest guard (`skip_ok`). I also made a missing manifest force a re-chunk, which the plan says only "when present". Reason: a DB indexed by 02-01 or 02-03 has no manifest but may hold rows from an older chunker, and skipping them would pin stale chunks forever. Added `kb_chunker_version_change_forces_rechunk` for it.

**3. Plan shape adjustment, readers test names**
- `kb_store_readers_never_return_other_corpora` is an extra integration test beyond the plan's `kb_store_diff_renumber`, covering the T-02-10 leak check at the integration level.

## Notes for maintainers

- `schema_extensions.rs` holds dead FTS5 code for other tables; it is out of scope and untouched. `turso_fts_probe` pins that this engine has no FTS5 module, which is why the lexical leg is Tantivy.
- `docs_changed` counts documents with an insert or delete. An edit that only changes a document's own `file_hash` without changing any chunk (for example whitespace outside any section body, if the chunker drops it) is stored but not counted as changed and does not trigger a Tantivy rebuild.
- `index_error` on `search_documents` is not written for errored files; errors live only in `kb_index_state.errors_json` (per the plan). A doc that is currently unreadable keeps its last good rows.
- The installed `vox` is stale (built at 6382, tree at 6571); `crate-edges` needed `VOX_SKIP_FRESHNESS_CHECK=1`. It also still warns about the stale baseline edge `vox-gui -> vox-db-types` (`--tighten` would remove it), pre-existing and unrelated.
- Known simplification kept from 02-01: `library_chunks_by_hashes` is one query per hash (`ponytail:` comment).

## Known Stubs

None.

## Threat Flags

None. New surface is DB reads/writes inside the existing library corpus and a local index directory path. T-02-10, T-02-05, T-02-11 and T-02-12 are mitigated as planned (corpus-filtered readers, consistency check plus rebuild, keep-all-walked prune, manifest with root, commit and `indexed_at`).

## Self-Check: PASSED

- Files exist: `crates/vox-db/tests/kb_library_store.rs`, `crates/vox-db/src/store/ops_memory/library.rs`, `crates/vox-search/src/library/{mod,lexical}.rs`, `crates/vox-search/tests/kb_library_index.rs`.
- Commits exist: `ac33dd51a`, `77568f45c`, `49609d3e6` (`git rev-list --count aae5451df..HEAD` = 3 before this SUMMARY commit).
