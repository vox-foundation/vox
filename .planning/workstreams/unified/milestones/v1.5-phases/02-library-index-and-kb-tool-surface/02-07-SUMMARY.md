---
phase: 02-library-index-and-kb-tool-surface
plan: 07
subsystem: mcp
tags: [mcp, resources, kb-uri, daemon-routing, vox-db, sql-surface-guard]
requires:
  - phase: 02-library-index-and-kb-tool-surface
    provides: "02-06 vox_kb_get / vox_kb_status / vox_kb_search handlers, validate_doc_path, validate_chunk_hash"
provides:
  - "kb://doc/{path} and kb://chunk/{hash} readable through resources/read, served by the daemon's vox_kb_get"
  - "resources/templates/list advertising both templates; resources/list with one kb://doc entry per corpus file"
  - "kb_resources::{KbUri, parse_kb_uri, kb_resource_templates, list_kb_doc_resources, read_kb_resource}"
  - "VoxDb::library_chunk_positions and VoxDb::embedding_count (sql-surface-guard fix)"
affects: [02-08, 02-09, phase-03]
actuals:
  tokens: 8000
  tasks: 2
  commits: 3
plan_head_before: 233d905781cabf58bb2356da79fb17c6d758bb0f
tech-stack:
  added: []
  patterns:
    - "Resource reads reuse the tool path: validate URI, call_tool_via_daemon(vox_kb_get), unwrap the ToolResult envelope"
    - "Resource listings derive only from walk_corpus, never from request input"
    - "Test DB inspection goes through VoxDb methods so tests stay outside the sql-surface-guard allowlist"
key-files:
  created:
    - crates/vox-orchestrator-mcp/src/kb_resources.rs
  modified:
    - crates/vox-orchestrator-mcp/src/lib.rs
    - crates/vox-orchestrator-mcp/src/server.rs
    - crates/vox-orchestrator-mcp/tests/stdio_daemon_routing_tests.rs
    - crates/vox-db/src/store/ops_memory/library.rs
    - crates/vox-db/src/store/ops_memory/embedding.rs
    - crates/vox-db/src/store/ops_memory/mod.rs
    - crates/vox-db/src/store/mod.rs
    - crates/vox-db/src/lib.rs
    - crates/vox-search/tests/kb_library_index.rs
key-decisions:
  - "Resource names for concrete kb://doc entries are the rel_path (no per-file title read); the stdio process stays DB-free and file-read-free"
  - "Parse errors map to invalid_params and daemon/tool errors to internal_error by parsing in read_resource before the daemon call, so read_kb_resource keeps the plan's Result<(String,String),String> shape"
requirements-completed: [RET-01]
status: complete
---

# Phase 2 Plan 07: KB resources over the daemon path Summary

An MCP host can now read `kb://chunk/{hash}` and `kb://doc/{path}` from the stdio server and discover them by template or concrete listing; every read is validated locally and then served by the daemon's `vox_kb_get`, and the KB tools are proven on the real stdio-to-daemon path.

## Orchestrator-assigned blocking fix (done first)

**[Rule 3 - Blocking] sql-surface-guard failure at HEAD** (commit `f691aa05b`).
`crates/vox-search/tests/kb_library_index.rs` (from 02-01/02-04) used raw `connection().query/execute`, so `vox ci sql-surface-guard` failed for the whole repo.

- Moved, not allowlisted. Moving was possible, so no `docs/agents/sql-connection-api-allowlist.txt` entry was added.
- Reused existing readers: `library_document_by_path` (one-document check), `get_kb_index_state` / `put_kb_index_state` (the `chunker_version = 'old'` fault injection becomes read-modify-write), `library_chunk_count` (vs manifest chunk_count), `library_chunk_rows`.
- Two minimal new `VoxDb` methods, each with its own test in the owning file:
  - `library_chunk_positions(corpus) -> Vec<LibraryChunkPos{rowid, rel_path, chunk_index, chunk_hash}>` in `ops_memory/library.rs` (needed because the stability tests assert rowids survive a reindex and `chunk_index` stays contiguous; `LibraryChunkRow` carries neither). Exported with `LibraryChunkRow`.
  - `embedding_count(source_type, Option<source_id>)` in `ops_memory/embedding.rs`.
- The orphan-embedding assertion got stronger, not weaker: it now checks per hash that a vector survives iff its chunk is live, plus the kb_chunk total and the untouched `memory` row.
- `kb_reindex_insert_middle_section` first failed on my new helper (rowid order is not chunk order); the test helper now sorts by `chunk_index`, matching the old query.
- Verification: `VOX_SKIP_FRESHNESS_CHECK=1 vox ci sql-surface-guard --all` prints `sql-surface-guard OK` (the installed `vox` is stale, so this used the stale binary's guard against the current tree and allowlist; the guard regex is `.connection().query|execute(` and the file no longer contains it). `cargo nextest run -p vox-search -E 'test(/kb_/)'` 73/73. `cargo nextest run -p vox-db` new tests pass. `cargo clippy -p vox-db -p vox-search --all-targets -D warnings` clean. deferred-items.md entry marked resolved.
- The full `vox ci ssot-drift` was not run end to end (stale binary); only the sql-surface-guard sub-check was.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| fix | Move kb_library_index test SQL behind VoxDb methods | f691aa05b |
| 1 (tracer) | Read kb://chunk and kb://doc through the stdio path into the daemon's vox_kb_get | 225d42ff7 |
| 2 | Resource templates, concrete kb://doc listing, mutation-proven URI validation | 8814b8d11 |

Tracer gate: Task 1's verify (daemon-path test plus the two in-file `kb_resources::` tests) passed, and the verify is automated-only, so expansion went ahead. The whole `vox-orchestrator-mcp` suite later passed (712/712).

## What was built

- `kb_resources.rs`: `parse_kb_uri` reuses `validate_doc_path` / `validate_chunk_hash` from 02-06. `read_kb_resource` parses first, then calls `call_tool_via_daemon(daemon, "vox_kb_get", {path | chunk_hash})`, and unwraps the envelope (`data.markdown` for docs, `data.chunk.body` for chunks; an error envelope becomes `Err(message)`, so an unindexed path reports `not indexed` from the daemon). `kb_resource_templates` returns the two templates, `list_kb_doc_resources` maps `walk_corpus` to `kb://doc/<rel_path>`.
- `server.rs`: `read_resource` gets a `kb://` branch before the workspace branch (parse error -> `invalid_params`, daemon/tool error -> `internal_error`); new `list_resource_templates`; `list_resources` appends the concrete kb://doc entries (AP-17).
- `kb_resources_and_tools_via_daemon` (in `stdio_daemon_routing_tests.rs`, `#[serial]`, reusing the file's `IsolatedHomeEnv` / `wait_ready` and the token-file daemon setup): indexes a temp corpus into the daemon's DB with the vox-crypto hasher into `default_index_dir()` under `VOX_SEARCH_TANTIVY_ROOT`, then asserts `vox_kb_status` is `indexed: true`, `vox_kb_search("turbofrobnicate")` returns the `Guide > Install` hit, `kb://chunk/<hash>` returns `text/markdown` with the body, `kb://doc/docs/src/guide.md` returns markdown containing `## Install`, and an unindexed valid path returns an error mentioning `not indexed`.

## Mutation proof (T-02-22 guard)

- Guard line recorded: `kb_resources.rs:31` `validate_doc_path(path).map(|p| KbUri::Doc(p.to_string()))`.
- Mutated to `Ok::<_, String>(path).map(...)` (grep of `validate_doc_path(path)` returned 0, confirming the mutation applied). `parse_kb_uri_rejects_unsafe_and_malformed` went RED: `must reject kb://doc/../../etc/passwd` (1 passed, 1 failed).
- Restored from a saved copy; line 31 is back; the same filter then ran 6/6 green (`parse_kb_uri`, `kb_resource_templates`, `list_kb_doc_resources`, `kb_resources`).

## Verification

| Check | Result |
|-------|--------|
| `cargo nextest run -p vox-orchestrator-mcp -E 'test(/kb_resources_and_tools_via_daemon\|kb_resources::/)'` | 3/3 |
| `cargo nextest run -p vox-orchestrator-mcp -E 'test(/parse_kb_uri\|kb_resource_templates\|list_kb_doc_resources/)'` | 4/4 |
| `cargo nextest run -p vox-orchestrator-mcp` (whole crate) | 712/712 |
| `cargo clippy -p vox-orchestrator-mcp --all-targets -- -D warnings` | clean |
| `grep 'starts_with("kb://")' server.rs`, `grep call_tool_via_daemon kb_resources.rs`, `grep 'fn list_resource_templates' server.rs` | all match |

## Deviations from Plan

**1. [Rule 3 - Blocking] sql-surface-guard failure** — assigned by the orchestrator, described above. Commit `f691aa05b`.

Otherwise the plan ran as written. In Task 1 the plan's "red first" step was folded into one pass because the test and module only compile together; the test was written before the implementation and passed on first run. The in-file test count for the plan's `fewer than 3 tests` gate was met (4 + 3).

## Threat model

T-02-22 and T-02-23 mitigated as planned (URI validated before any daemon call, mutation-proven; the daemon reads DB rows only). T-02-24 accepted: `resources/list` carries one entry per corpus file; add pagination if a host chokes.

## Known Stubs

None.

## Threat Flags

None. No new network endpoint or trust boundary; the stdio server only forwards validated arguments to the existing daemon tool call.
