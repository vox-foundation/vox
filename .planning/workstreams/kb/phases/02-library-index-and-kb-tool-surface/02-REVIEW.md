---
phase: 02-library-index-and-kb-tool-surface
reviewed: 2026-10-05T00:00:00Z
depth: standard
files_reviewed: 37
files_reviewed_list:
  - crates/vox-db/src/schema/domains/knowledge.rs
  - crates/vox-db/src/store/open.rs
  - crates/vox-db/src/store/ops_memory/library.rs
  - crates/vox-db/src/store/ops_memory/embedding.rs
  - crates/vox-db/src/lib.rs
  - crates/vox-search/src/kb_ids.rs
  - crates/vox-search/src/library/mod.rs
  - crates/vox-search/src/library/chunk.rs
  - crates/vox-search/src/library/corpus.rs
  - crates/vox-search/src/library/lexical.rs
  - crates/vox-search/src/kb_eval/library.rs
  - crates/vox-search/src/kb_eval/mod.rs
  - crates/vox-search/src/execution.rs
  - crates/vox-search/src/policy.rs
  - crates/vox-search/src/rrf.rs
  - crates/vox-search/src/bundle.rs
  - crates/vox-search/src/lib.rs
  - crates/vox-search/Cargo.toml
  - crates/vox-orchestrator-mcp/src/kb_tools/library.rs
  - crates/vox-orchestrator-mcp/src/kb_tools/mod.rs
  - crates/vox-orchestrator-mcp/src/kb_resources.rs
  - crates/vox-orchestrator-mcp/src/dispatch.rs
  - crates/vox-orchestrator-mcp/src/input_schemas.rs
  - crates/vox-orchestrator-mcp/src/server.rs
  - crates/vox-orchestrator-mcp/src/memory_tools/retrieval.rs
  - crates/vox-cli/src/commands/kb.rs
  - crates/vox-cli/src/commands/kb/eval.rs
  - crates/vox-cli/src/commands/kb/gate.rs
  - crates/vox-cli/src/commands/daemon_tool.rs
  - crates/vox-cli/src/commands/dei.rs
  - crates/vox-cli/src/cli_dispatch/mod.rs
  - crates/vox-orchestrator/src/orchestrator/task_dispatch/submit/goal.rs
  - crates/vox-research-shim/src/research/orchestrator/web_gather.rs
  - crates/vox-doc-pipeline/src/pipeline/lint.rs
  - contracts/operations/catalog.v1.yaml
  - contracts/mcp/http-read-role-governance.yaml
  - contracts/mcp/tool-registry.canonical.yaml
findings:
  critical: 1
  warning: 8
  info: 6
  total: 15
status: issues_found
---

# Phase 2: Code Review Report

**Reviewed:** 2026-10-05
**Depth:** standard
**Files Reviewed:** 37
**Status:** issues_found

## Summary

The path-handling surface is sound. `vox_kb_get`, `kb://doc` and `kb://chunk` validate input with `is_kb_corpus_path` / `validate_chunk_hash` before any lookup. They answer from the index only and never touch the file system. Reads are corpus-filtered in the DB layer (`library_chunk_by_hash`, `library_document_by_path`). The schema-95 migration is idempotent: ALTERs are PRAGMA-guarded and the indexes are created after them. The chunk-id and `kb_ids` consistency holds.

The weaknesses are in the reindex lifecycle:

- The prune step trusts an incomplete or empty corpus walk.
- The swap of the Tantivy directory is non-atomic and destructive.
- Single-flight and cancellation safety are weaker than the code comments claim.
- The new `search_document_chunks` rows leak into the generic RAG chunk search.

No injection or traversal defect was found in the KB tools or resources.

## Critical Issues

### CR-01: Reindex over an empty or partially-walked corpus silently prunes the whole Library

**File:** `crates/vox-search/src/library/corpus.rs:34` and `crates/vox-search/src/library/mod.rs:79-141`
**Issue:**
- `walk_corpus` does `for entry in walker.flatten()`, which drops every walkdir error: permission denied, I/O error, a vanished directory. A missing `docs/src` yields no entries and no error.
- `index_library` treats the returned list as the complete tree. `walked` is built only from that list and passed to `prune_library_documents_not_in`. The comment "Every walked path is kept, errored or not: a transient read error never prunes a doc" covers only per-file read errors, not walk errors.
- A transient error on a subdirectory deletes every indexed doc under it. A daemon whose `repository.root` has no `docs/src` deletes all of them. Then `prune_orphan_kb_chunk_embeddings` drops their vectors and Tantivy is rebuilt (possibly empty).
- The report shows `errors: []` and success.
- `kb_reindex` uses `state.repository.root` of a machine-shared daemon, so a wrong root is easy to hit (see WR-07). The `vox kb reindex` CLI also routes there.
- No test covers a missing or unreadable root.

**Fix:** Make the walk fallible and refuse to prune on an incomplete one.
```rust
// corpus.rs: return (Vec<CorpusFile>, Vec<String /*walk errors*/>)
for entry in walker { match entry { Ok(e) => ..., Err(err) => walk_errors.push(err.to_string()) } }

// mod.rs
if !walk_errors.is_empty() || (report.docs_seen == 0 && prior_doc_count > 0) {
    // record in report.errors, skip prune + tantivy rebuild
}
```
At minimum, skip pruning when `docs_seen == 0` or any walk error occurred. Also assert that the `repo_root/docs/src` directory exists before indexing, and fail the call otherwise.

## Warnings

### WR-01: `rebuild_from_rows` deletes whatever directory is at the index path, and the swap is non-atomic

**File:** `crates/vox-search/src/library/lexical.rs:73-105`
**Issue:**
- `if dir.exists() { remove_dir_all(dir) }` runs for any existing directory. The index dir is `<VOX_SEARCH_TANTIVY_ROOT>/library`. If that path holds non-index data (misconfigured root, or a root that already has a `library/` folder), `Index::open` fails, `current` is `None`, a rebuild is forced and the directory is wiped.
- Between `remove_dir_all(dir)` and `rename(tmp, dir)` concurrent `vox_kb_search` / `vox_kb_status` calls fail to open the index. Searches at that moment return "library search failed".
- On Windows, `remove_dir_all` fails while any reader holds an mmap, so reindex errors.
- If `rename` fails after the delete, the live index is gone.

**Fix:**
- Only delete `dir` if it is empty or opens as a Tantivy index. Otherwise error out.
- Prefer a versioned layout: build `library.<gen>`, then atomically flip a small pointer file or symlink. Alternatively rename `dir` to `dir.old` first, then rename `tmp` to `dir`, then delete `dir.old`. This shrinks the window to one rename and keeps a rollback.

### WR-02: Single-flight is per-process only; the claimed cross-process Tantivy lock does not exist

**File:** `crates/vox-orchestrator-mcp/src/kb_tools/library.rs:123-125`, `crates/vox-search/src/library/lexical.rs:78-84`
**Issue:**
- The doc comment says "Tantivy's own writer lockfile guards other processes".
- The writer is created inside `<name>.tmp`, and every run starts with `remove_dir_all(&tmp)`. A second process (the HTTP gateway has its own `ServerState` and calls `handle_tool_call`) deletes the first process's temp index mid-build. No lock protects that.
- `vox_kb_reindex` is not in `dispatch_timeout::EXPLICIT_TIMEOUTS`, so it gets the 120s `DEFAULT_TIMEOUT`.
- On timeout, the handler future is dropped. The static `KB_REINDEX_LOCK` guard is released while the `spawn_blocking` Tantivy rebuild keeps running. A retry then enters the same `.tmp` concurrently.

**Fix:**
- Take an exclusive file lock (for example `fs2`/`flock` on `<dir>.lock`) around the whole of `index_library`'s Tantivy phase.
- Give `vox_kb_reindex` an explicit timeout entry.
- Hold the lock inside the blocking task so it outlives a cancelled handler.

### WR-03: Per-document transactions are not cancellation-safe and not isolated on the shared connection

**File:** `crates/vox-db/src/store/ops_memory/library.rs:156` using `crates/vox-db/src/facade/migrations.rs:98-113`
**Issue:**
- `VoxDb::transaction` issues `BEGIN`, awaits the body, then `COMMIT`/`ROLLBACK`, with no drop guard.
- `GuardedConnection` serializes single statements, not transactions. Other tasks on the same `Arc<VoxDb>` (daemon tool calls) interleave inside the open transaction. A rollback discards their writes, and any other `transaction()` caller gets a nested-`BEGIN` error.
- The new tool makes cancellation reachable: the 120s dispatch timeout (WR-02), or a client disconnect, drops the future between `BEGIN` and `COMMIT`. The connection is left in an open transaction for the daemon's lifetime. Later transactions fail and plain writes stay uncommitted.
- 671 docs are indexed one transaction at a time, so this is plausible on a loaded machine.

**Fix:**
- Make `transaction` cancellation-safe: run the body on a spawned task or a drop-guard that issues `ROLLBACK`.
- Run the reindex DB phase in a `tokio::spawn`ed task detached from the request future.
- Give the reindex tool a timeout that matches the workload.

### WR-04: DB and Tantivy can diverge undetected after a failed rebuild

**File:** `crates/vox-search/src/library/mod.rs:132-155`
**Issue:**
- The Tantivy rebuild runs after the DB commits. If it fails (the `?` after `spawn_blocking`), the DB has the new chunks and Tantivy has the old ones. The manifest is not updated.
- On the next run the changed docs are now `docs_unchanged` (the stored file hash matches). `docs_changed` and `docs_pruned` are 0, and the only fallback is `current != Some(rows.len())`.
- If the edit replaced chunks one-for-one (a typical single-section edit: one hash out, one in), the counts match and no rebuild happens. The new chunk is unsearchable, and stale hashes are silently dropped by `search()`.
- `status()` compares counts only, so it also reports `consistent: true`.

**Fix:** Persist an index generation or content digest. For example, store a hash over the sorted chunk hashes in `kb_index_state`, and also in a `meta` file next to the Tantivy index. Rebuild when they differ. Use that same digest in `status()`.

### WR-05: `source_uri` hard-codes the library corpus, so a second corpus with the same `rel_path` clobbers the first

**File:** `crates/vox-db/src/store/ops_memory/library.rs:155`
**Issue:**
- `source_uri = format!("{KB_LIBRARY_CORPUS}:{}", doc.rel_path)` ignores `doc.corpus`.
- The upsert is `ON CONFLICT(source_uri) DO UPDATE SET corpus = excluded.corpus, ...`. Writing the same `rel_path` under another corpus re-labels the library doc to that corpus and replaces its chunks. Library data silently moves out of the library.
- Existing tests avoid this by using a different path for the "other" corpus.

**Fix:** `format!("{}:{}", doc.corpus, doc.rel_path)`, and add a test that writes the same `rel_path` under two corpora.

### WR-06: Library chunks leak into the generic RAG chunk search with no corpus or trust filter

**File:** `crates/vox-db/src/store/ops_memory/search.rs:257-279` (consumer: `crates/vox-search/src/execution.rs`, DocumentChunks corpus)
**Issue:**
- `query_search_document_chunks_like` / `_fts` select from `search_document_chunks JOIN search_documents` with no `corpus` predicate. A `vox_kb_reindex` therefore injects the whole `docs/src` library into every `DocumentChunks` retrieval.
- It appears as `chunk_lines`, Socrates evidence, `vox://chunk` research hits and the gui web-count, mixed with untrusted web-ingested documents.
- The trust tier is not stored per row (an approved decision), so downstream code cannot tell them apart.
- This is the reverse of the T-02-10 corpus filtering done on the new readers.

**Fix:** Add `AND d.corpus = ''` (or `<> 'library'`) to the legacy readers, or migrate them to an explicit corpus argument. Add a test asserting generic search never returns a library chunk.

### WR-07: `vox kb reindex` / `vox kb status` operate on the shared daemon's repository root, not the caller's

**File:** `crates/vox-cli/src/commands/kb.rs:48-58`, `crates/vox-orchestrator-mcp/src/kb_tools/library.rs:211-212`
**Issue:**
- The CLI sends no root. The machine-shared daemon uses its own `state.repository.root`.
- With multiple worktrees or checkouts, `vox kb reindex` from worktree A indexes (and, per CR-01, prunes against) whichever tree the daemon started in. The report is printed as if it were A's.
- The stdio MCP `resources/list` uses its local `state.repository.root`, while `resources/read` goes to the daemon's, so listed URIs may be "not indexed".

**Fix:**
- Print the daemon's `corpus_root` prominently in the reindex output.
- Optionally refuse when it differs from the CLI's `git rev-parse --show-toplevel`.
- Make the stdio resource listing come from the daemon, or note the mismatch.

### WR-08: `vox_kb_status` discloses host paths to read-role (HTTP) callers

**File:** `crates/vox-orchestrator-mcp/src/kb_tools/library.rs:411-437`; `contracts/mcp/http-read-role-governance.yaml`
**Issue:**
- `vox_kb_status` is added to the HTTP read role. It returns the absolute `index_dir`, the absolute `corpus_root`, and per-file `errors`. The error text is `format!("{e:#}")` from the file reader, which typically embeds absolute paths.
- Every other new read-role tool returns only index-derived content.

**Fix:** For read-role callers, drop `index_dir` and `corpus_root`, or return only the repo-relative form. Sanitize error strings to the repo-relative path before storing them in `errors_json`. Alternatively mark `vox_kb_status` `http_read_role_eligible: false`.

## Info

### IN-01: `indexed_commit` labels uncommitted content with HEAD

**File:** `crates/vox-orchestrator-mcp/src/kb_tools/library.rs:189-197`, `crates/vox-search/src/library/mod.rs:124`
**Issue:** Working-tree text (dirty or untracked files) is indexed and stamped with `git rev-parse HEAD`. `vox_kb_cite` then reports that commit for content that is not in it.
**Fix:** Stamp the commit only for files clean in `git status`, or add a `dirty` flag to the report and citation.

### IN-02: `walk_corpus` follows a symlinked corpus root

**File:** `crates/vox-search/src/library/corpus.rs:24-27`
**Issue:** The walkdir default `follow_root_links(true)` means `docs/src` as a symlink to an outside tree is indexed and then served by `vox_kb_get`. The per-file `path_is_symlink` check does not cover the root.
**Fix:** `.follow_root_links(false)`, or check `symlink_metadata` on the root.

### IN-03: `index_error` is never written, and `FileError`'s doc overstates behavior

**File:** `crates/vox-search/src/library/mod.rs:28-33,123`
**Issue:**
- `LibraryDocUpsert.index_error` is always `None`, so `vox_kb_get`'s `index_error` is always null.
- `FileError` says "could not be read or indexed; indexing continues past it", but a DB error from `apply_library_document` aborts the run (`?`), while only read errors continue.

**Fix:** Either persist read errors per document or drop the field. Fix the comment.

### IN-04: Resource and tool edge cases

**File:** `crates/vox-orchestrator-mcp/src/server.rs:281,349`, `crates/vox-orchestrator-mcp/src/kb_tools/library.rs:388`
**Issue:**
- `list_kb_doc_resources` runs a blocking recursive walkdir inside an async `resources/list` handler on every call.
- A not-indexed `kb://` read returns `internal_error`; MCP's "resource not found" would be accurate.
- `vox_kb_cite` builds `[{heading_path}]({link})` without escaping `]`, `)` or spaces. A heading such as "Foo (bar)" or a path with spaces produces a malformed link.
- `kb_cite` swallows the doc-lookup error with `Err(_) => None`.

**Fix:** Use `spawn_blocking` for the listing. Map not-indexed to `ErrorData::resource_not_found`. Percent-encode the link destination and escape the link text.

### IN-05: Tantivy schema is unversioned and uses `.expect` on field lookup

**File:** `crates/vox-search/src/library/lexical.rs:86,120`
**Issue:**
- The on-disk index has no schema or version marker. A future schema change, or a stale or foreign index at the path, would panic on `schema.get_field(...).expect(...)` instead of triggering a rebuild.
- `CHUNKER_VERSION` does not cover the Tantivy schema or the boost constants. A boost change alone never rebuilds, because rebuild is gated on `docs_changed`, `docs_pruned` or count mismatch.

**Fix:** Map a missing field to an error. Include a `LEXICAL_SCHEMA_VERSION` in the manifest and rebuild when it differs.

### IN-06: Unbounded YAML expansion in frontmatter parsing

**File:** `crates/vox-search/src/library/chunk.rs:73-84`
**Issue:** `serde_yaml::from_str::<serde_json::Value>` expands anchors and aliases, so a pathological frontmatter block in a working-tree doc (not only committed ones, since untracked files are indexed per IN-01) can blow up memory. The corpus is "trusted" by assumption only.
**Fix:** Reject frontmatter blocks containing `&`/`*` aliases above a small size, or cap the frontmatter size before parsing.

---

_Reviewed: 2026-10-05_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
