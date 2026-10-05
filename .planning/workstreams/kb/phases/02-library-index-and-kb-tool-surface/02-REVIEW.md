---
phase: 02-library-index-and-kb-tool-surface
reviewed: 2026-10-05T00:00:00Z
depth: standard
files_reviewed: 11
files_reviewed_list:
  - contracts/mcp/http-read-role-governance.yaml
  - contracts/mcp/tool-registry.canonical.yaml
  - contracts/operations/catalog.v1.yaml
  - crates/vox-orchestrator-mcp/src/dispatch_timeout.rs
  - crates/vox-orchestrator-mcp/src/http_gateway/mod.rs
  - crates/vox-orchestrator-mcp/src/kb_resources.rs
  - crates/vox-orchestrator-mcp/tests/kb_tool_surface.rs
  - crates/vox-search/src/library/corpus.rs
  - crates/vox-search/src/library/mod.rs
  - crates/vox-search/tests/kb_library_chunking.rs
  - crates/vox-search/tests/kb_library_index.rs
findings:
  critical: 0
  warning: 8
  info: 9
  total: 17
status: issues_found
---

# Phase 2: Code Review Report (incremental re-review of 02-10 and 02-11)

**Reviewed:** 2026-10-05
**Depth:** standard
**Files Reviewed:** 11
**Status:** issues_found
**Diff:** `0e50058376c1abba4ef7e5ef244a0840c2aed109..HEAD` on the 11 files

## Summary

Closure status of the three targeted findings:

- **CR-01 (prune after incomplete walk): CLOSED.**
  - `walk_corpus` now returns walk errors instead of dropping them.
  - `index_library` fails up front when a corpus root is missing.
  - Pruning is gated on `walk_complete && (docs_seen > 0 || stored chunks == 0)`.
  - `prune_library_documents_not_in` has exactly one production caller, at `mod.rs:156`, and that call is gated. I found no other path that prunes after an incomplete walk.
  - Per-file read errors keep the path in `walked`, so they never prune.
  - The `prune_orphan_kb_chunk_embeddings` call that follows only drops embeddings whose chunk no longer exists, so it is safe when pruning is skipped.
  - A symlink replacing a doc is skipped by the walker and so is pruned. That is intended.
  - Walk-error paths are repo-relative and their text carries no host path.
  - The walkdir I/O error is rendered via `io_error()` Display, which carries no path.
  - Residual hole (IN-07): per-file read errors are not sanitized.
- **WR-08 (`vox_kb_status` in the HTTP read role): CLOSED.**
  - The read role is derived from `TOOL_REGISTRY`, which comes from `tool-registry.canonical.yaml`.
  - Both the governance YAML and the registry now exclude `vox_kb_status`. `catalog.v1.yaml` is consistent.
  - `capability-registry.yaml` has no read-role flag.
  - No hardcoded allowlist in the crates or docs mentions it.
  - Both a unit test (`metadata_read_role_excludes_vox_kb_status`) and an integration test (`kb_tool_surface`) pin it.
  - Per-file errors still embed absolute paths (IN-07), but the tool is no longer reachable by the read role.
- **WR-02 (reindex timeout): PARTIALLY CLOSED.**
  - The explicit 600s timeout is in place and correctly keyed.
  - It lengthens the window rather than closing it. The cross-process lock claim and the cancellation behavior are unchanged (WR-02, WR-03 below).
  - A new mismatch with the CLI client deadline appears (WR-09).

No new critical issues were introduced. Prior findings not addressed by these plans are carried forward unchanged below.

## Warnings

### WR-01 (carried): `rebuild_from_rows` deletes whatever directory is at the index path, and the swap is non-atomic

**File:** `crates/vox-search/src/library/lexical.rs:73-105`
**Issue:** Unchanged from the prior review, and not in this diff. `remove_dir_all(dir)` runs for any existing directory, including one that is not a Tantivy index. Concurrent searches fail between the delete and the rename. If the rename fails, the live index is gone.
**Fix:** Only delete `dir` when it is empty or opens as a Tantivy index. Rename `dir` to `dir.old`, then rename `tmp` to `dir`, then delete `dir.old`.

### WR-02 (carried, partially mitigated): single-flight is per-process only, and the lock comment is still wrong

**File:** `crates/vox-orchestrator-mcp/src/kb_tools/library.rs:123-125`, `crates/vox-search/src/library/lexical.rs:78-84`
**Issue:**
- The explicit 10 min timeout closes the "120s default drops a normal reindex" half.
- The comment at `library.rs:123` still says "Tantivy's own writer lockfile guards other processes", which is false. The writer is created in `<name>.tmp`, and each run starts with `remove_dir_all(&tmp)`.
- A second process (the HTTP gateway has its own `ServerState`) deletes the first process's temp index mid-build.
- On timeout, `KB_REINDEX_LOCK` is released while the `spawn_blocking` rebuild keeps running. A retry then enters the same `.tmp`.
- The new doc comment in `dispatch_timeout.rs` says "cross-process single-flight is deferred" but does not retract the false claim at the lock site.

**Fix:**
- Take an `fs2`/`flock` exclusive lock on `<dir>.lock` for the whole Tantivy phase, held inside the blocking task.
- Fix the comment at `library.rs:123`.

### WR-03 (carried, window widened rather than closed): per-document transactions are not cancellation-safe

**File:** `crates/vox-db/src/store/ops_memory/library.rs:156`, `crates/vox-db/src/facade/migrations.rs:98-113`
**Issue:**
- `VoxDb::transaction` has no drop guard.
- Any future drop between `BEGIN` and `COMMIT` leaves the shared connection in an open transaction. Triggers include the now-600s dispatch timeout and a client disconnect.
- Raising the timeout makes this rarer but does not remove it. The `dispatch_timeout.rs` comment presents the bump as addressing WR-03, which overstates it.

**Fix:** Run the reindex DB phase in a detached `tokio::spawn`ed task, or give `transaction` a drop-guard `ROLLBACK`.

### WR-04 (carried): DB and Tantivy can diverge undetected after a failed rebuild

**File:** `crates/vox-search/src/library/mod.rs:166-190`
**Issue:**
- If the Tantivy rebuild fails after the DB commit, the next run sees `docs_unchanged`. With a one-for-one chunk swap the row count equals the index count, so no rebuild happens.
- `status()` also reports `consistent: true`.
- The new prune-skipped path adds a related case. If pruning is skipped while the index was built from a different row set, the count comparison is still the only guard.

**Fix:** Persist a digest of the sorted chunk hashes in `kb_index_state` and next to the Tantivy index. Rebuild and report inconsistency when they differ.

### WR-05 (carried): `source_uri` hard-codes the library corpus

**File:** `crates/vox-db/src/store/ops_memory/library.rs:155`
**Issue:** `format!("{KB_LIBRARY_CORPUS}:{}", doc.rel_path)` ignores `doc.corpus`. The same `rel_path` under another corpus re-labels the library doc.
**Fix:** `format!("{}:{}", doc.corpus, doc.rel_path)`, plus a two-corpus test.

### WR-06 (carried): library chunks leak into generic RAG chunk search

**File:** `crates/vox-db/src/store/ops_memory/search.rs:257-279`
**Issue:** The `search_document_chunks` readers have no `corpus` predicate, so library rows mix with untrusted web-ingested documents.
**Fix:** Add `AND d.corpus = ''` to the legacy readers, and add a test.

### WR-07 (carried): `vox kb reindex` / `vox kb status` use the shared daemon's repository root

**File:** `crates/vox-cli/src/commands/kb.rs:48-58`, `crates/vox-orchestrator-mcp/src/kb_tools/library.rs:211-212`
**Issue:** The CLI sends no root, so a reindex from worktree A runs against whichever tree the daemon started in.
- The new `index_library` root check reduces the damage. A daemon rooted at a tree without `docs/src` now errors instead of pruning.
- A daemon rooted at a different checkout that has `docs/src` still reindexes and prunes that checkout's content as if it were the caller's.

**Fix:** Print the daemon's `corpus_root` prominently in the reindex output. Refuse when it differs from the CLI's `git rev-parse --show-toplevel`.

### WR-09 (new): the 600s server timeout is unreachable from the CLI, which gives up at 195s

**File:** `crates/vox-orchestrator-mcp/src/dispatch_timeout.rs:70-80` (consumer: `crates/vox-cli/src/commands/kb.rs:44-52`)
**Issue:**
- The new comment acknowledges that the client read deadline stays at 195s, but treats it as acceptable.
- `vox kb reindex` goes through the generic daemon client, so for a run longer than 195s the CLI errors out while the server keeps indexing for up to 600s. The user sees a failure for an operation that is still running or later succeeds.
- A retry then hits "reindex already running", or, once the lock is released by a timeout, races the live rebuild (WR-02).
- The first full index of about 13.4k chunks is the likely trigger.

**Fix:** Give the reindex CLI path a client deadline at least as long as `KB_REINDEX_TIMEOUT`, or make reindex return a job id and poll. At minimum, print "reindex may still be running; check `vox kb status`" on a client timeout.

## Info

### IN-01 through IN-06 (carried unchanged)

- **IN-01:** `indexed_commit` labels uncommitted content with HEAD. File: `library.rs:189-197` and `library/mod.rs:124`.
- **IN-02:** `walk_corpus` follows a symlinked corpus root. File: `corpus.rs`. The new `is_dir()` check in `index_library` also follows symlinks, and `WalkDir::new(root)` still has `follow_root_links(true)`.
- **IN-03:** `LibraryDocUpsert.index_error` is always `None`. The `FileError` doc was reworded but still says "indexing continues past it", while a DB error aborts the run via `?`.
- **IN-04:**
  - `list_kb_doc_resources` does a blocking walk inside an async handler.
  - Not-indexed `kb://` reads map to `internal_error`.
  - `vox_kb_cite` does not escape link text or destinations.
  - The new `.0` in `kb_resources.rs` silently discards walk errors. The comment justifies this, but a partial listing is invisible to the caller.
- **IN-05:** Tantivy schema is unversioned and uses `.expect` on field lookup (`lexical.rs:86,120`).
- **IN-06:** Unbounded YAML alias expansion in frontmatter parsing (`chunk.rs:73-84`).

### IN-07 (new, residual of WR-08): per-file read errors still embed absolute host paths

**File:** `crates/vox-search/src/library/mod.rs:96-103`
**Issue:**
- `format!("{e:#}")` over `read_utf8_path_capped` yields text such as `stat /abs/repo/docs/src/x.md` (`vox-bounded-fs/src/lib.rs:23-34`). This text is persisted in `errors_json` and returned by `vox_kb_status` and `vox_kb_reindex`.
- Walk errors were made repo-relative with a test asserting no host path. Read errors were not, so the contract is inconsistent.
- Exposure is now limited to write-role and stdio callers.

**Fix:** Store `file.rel_path` plus the `io::Error` kind, or replace `file.abs.display()` in the message with `rel_path`. Extend the no-host-path assertion in `kb_reindex_unreadable_subdir_keeps_rows` to an unreadable file.

### IN-08 (new): a deliberately emptied corpus can never be pruned

**File:** `crates/vox-search/src/library/mod.rs:151-169`
**Issue:**
- With a complete walk, `docs_seen == 0` and stored chunks > 0, pruning is permanently skipped. The run reports a "prune skipped" error forever.
- This is the intended trade-off, but there is no override. The only escape is a manual DB reset.

**Fix:** Optional `force_prune` flag on `vox_kb_reindex`, or document the reset path in the tool description.

### IN-09 (new): permission-based tests pass vacuously when permissions are bypassed

**File:** `crates/vox-search/tests/kb_library_index.rs` (`kb_reindex_unreadable_subdir_keeps_rows`) and `crates/vox-search/src/library/corpus.rs` (`walk_corpus_reports_unreadable_dir`)
**Issue:** When the process is root, or the filesystem ignores mode bits, the tests `return` early and report as passed with nothing asserted. In a root-run CI container the main CR-01 regression test never executes.
**Fix:** Mark the test ignored, or emit a visible skip message. Alternatively stage the walk failure without permissions, for example with a directory replaced by a dangling entry mid-walk.

---

_Reviewed: 2026-10-05_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
