---
phase: 02-library-index-and-kb-tool-surface
plan: 06
subsystem: mcp
tags: [mcp, kb-tools, catalog, vox-search, library-index, input-validation]
requires:
  - phase: 02-library-index-and-kb-tool-surface
    provides: "02-01 Library index and search, 02-04 corpus-filtered vox-db readers, library::status and default_index_dir"
provides:
  - "MCP tools vox_kb_search, vox_kb_get, vox_kb_cite, vox_kb_status, vox_kb_reindex (tier core, lane ai)"
  - "Catalog rows kb.search, kb.get, kb.cite, kb.status, kb.reindex with regenerated registries"
  - "kb_tools::library handlers with traversal-proof validate_doc_path / validate_chunk_hash and a reindex single-flight"
  - "kb_tool_surface parity test across catalog, registry, dispatch, input_schemas and read-role governance"
affects: [02-07, 02-08, 02-09, phase-03]
actuals:
  tokens: 11000
  tasks: 3
  commits: 3
plan_head_before: fd4003dba83e3dcb1a3978ea2ee413ce4363d84b
tech-stack:
  added: []
  patterns:
    - "Catalog row, then vox ci operations-sync --target all --write, then dispatch/input_schemas arms; generated registries never hand-edited"
    - "Source-scan guard (include_str! of the handler file, production half only) to prove no file-system API is reachable from tool handlers"
    - "No-parameter write tool: reject every key except dispatch meta keys before running"
key-files:
  created:
    - crates/vox-orchestrator-mcp/src/kb_tools/library.rs
    - crates/vox-orchestrator-mcp/tests/kb_tool_surface.rs
  modified:
    - contracts/operations/catalog.v1.yaml
    - contracts/mcp/tool-registry.canonical.yaml
    - contracts/mcp/http-read-role-governance.yaml
    - contracts/capability/capability-registry.yaml
    - contracts/reports/operations-catalog-inventory.v1.json
    - crates/vox-orchestrator-mcp/src/kb_tools/mod.rs
    - crates/vox-orchestrator-mcp/src/dispatch.rs
    - crates/vox-orchestrator-mcp/src/input_schemas.rs
key-decisions:
  - "validate_doc_path adds only a 512-byte cap on top of kb_ids::is_kb_corpus_path, which already rejects backslash, colon, empty and dot components, traversal and archive; duplicating those checks would be dead code"
  - "vox_kb_reindex tolerates the dispatch meta keys (agent_id, session_id, trace_id, correlation_id, task_id) and rejects every other key, instead of a bare deny_unknown_fields"
  - "Handlers pull the repo root from state.repository.root, never from tool input"
requirements-completed: [RET-01, IDX-05]
status: complete
---

# Phase 2 Plan 06: Library KB MCP tool surface Summary

Five catalog-driven MCP tools now expose the Library index to any host: BM25 search with kb:// URIs, get and cite by validated key from the DB only, a status tool carrying every IDX-05 field, and an argument-free, single-flight reindex that hashes through vox-crypto.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 (tracer) | vox_kb_search from catalog row through dispatch to a ranked hit | f91256c98 |
| 2 | vox_kb_get, vox_kb_cite, vox_kb_status with validated keys | 1951ce6a3 |
| 3 | vox_kb_reindex: fixed roots, vox-crypto hasher, single-flight | 8ef8ec8a2 |

Tracer gate: Task 1's verify (7 tests plus `vox ci operations-verify`) was re-run end to end before Task 2 started. It passed, so expansion went ahead.

## What was built

- **Catalog and registry.** Five rows (`kb.search`, `kb.get`, `kb.cite`, `kb.status`, `kb.reindex`) copy the `kb.query` shape and use new ids, so the 12 legacy `vox_kb_*` tools are untouched (AP-08). `kb.reindex` is `side_effect_class: writes_files`, `http_read_role_eligible: false`. Registries (`tool-registry.canonical.yaml`, `capability-registry.yaml`, `command-registry.yaml`) and `operations-catalog-inventory.v1.json` came from `vox ci operations-sync` / `operations-verify`, not hand edits. The read-role list gained `vox_kb_cite`, `vox_kb_get`, `vox_kb_search`, `vox_kb_status` and not `vox_kb_reindex`.
- **`kb_tools/library.rs`.** Self-contained (no import from `handlers.rs` / `params.rs`, which Phase 3 deletes).
  - `kb_search`: trims the query and rejects empty input or more than 1024 bytes (no truncation). `limit` defaults to 10 and clamps to 1..=50. A missing DB or index returns an error envelope with a remediation, never "Unknown tool".
  - `kb_get`: exactly one of `path` or `chunk_hash`. By path it returns doc metadata, the chunk list and the markdown rebuilt from section bodies in `chunk_index` order (card excluded). By hash it returns the chunk with its body. An unindexed but valid key returns `not indexed`; an invalid key returns `invalid path` / `invalid chunk_hash`.
  - `kb_cite`: returns `link` (`doc_path#anchor`, or just the path when there is no anchor), `citation` (`[heading_path](link)`), `kb://chunk/<hash>` and `indexed_commit`.
  - `kb_status`: `indexed:false` before any run. After one it returns corpus root and commit, doc, chunk and card counts, chunker version, `tantivy-0.22`, `embed: "none (lexical-only)"`, trust tier, errors array, `consistent`, `index_dir`.
  - `kb_reindex` / `reindex_at`: a static `tokio::sync::Mutex::try_lock` returns `reindex already running` on contention. The hasher is `vox_crypto::hex_encode(secure_hash(..))`, the commit is `git rev-parse HEAD` through `GitExec` (None on failure), the trust tier is `trusted`, and the corpus root is the repository root. The result is the index report as JSON.
- **Wiring.** Dispatch arms and `input_schemas` arms for all five. `vox_kb_reindex` runs `reject_reindex_args` first, so `{"root": "/etc"}` is an error before anything happens.

## Verification

| Check | Result |
|-------|--------|
| `cargo nextest run -p vox-orchestrator-mcp -E 'test(/kb_\|validate_doc_path\|validate_chunk_hash\|uri_shapes\|every_registry_tool_has_static_dispatch\|yaml_registry_tools_have_dispatch_match_arms/)'` | 29/29 pass |
| `cargo nextest run -p vox-orchestrator-mcp -p vox-mcp-registry` (whole crates) | 732/732 pass |
| `cargo clippy -p vox-orchestrator-mcp --all-targets -- -D warnings` | clean |
| `vox ci operations-verify` | exit 0 (1304 catalog rows) |
| `vox ci crate-edges` | OK, 689 edges; no `Cargo.toml` touched, no exception written |
| `grep "std::fs" library.rs` | prints nothing |
| `grep vox_kb_reindex http-read-role-governance.yaml` | prints nothing |
| `id: kb.query` / `id: kb.clip` | still present once each (lines 26914, 26934) |

## Mutation proofs

Each run recorded presence counts before the edit, after the edit, and after restore (restored from a backup copy; the single-flight file was `cmp`-identical to the backup).

| Guard | Mutation | Result |
|-------|----------|--------|
| `..` component rule in `kb_ids::is_kb_corpus_path` (the check `validate_doc_path` relies on); presence 1, 0, 1 | removed ` \|\| *c == ".."` | `validate_doc_path_accepts_and_rejects` FAIL on `docs/src/../../etc/shadow.md`; green after restore |
| single-flight early return in `reindex_at` (`return Err("reindex already running"`); presence 1, 0, 1 | replaced the `let ... else { return Err }` with `let _guard = KB_REINDEX_LOCK.try_lock().ok();` | `kb_reindex_single_flight` FAIL (`unwrap_err` on an `Ok`, 3 of 3 tries); green after restore |

The traversal test deliberately includes `.md`-terminated traversal paths (`docs/src/../../etc/shadow.md`, `docs/src/../Cargo.md`). The plan's list, `docs/src/../../etc/passwd`, would still be rejected by the `.md` suffix rule with the `..` check removed, so it could not catch this mutation.

## Deviations from Plan

**1. Plan shape adjustment, `validate_doc_path` is not a separate layered check**
- The plan asks to layer length, backslash, colon, and empty or `.` component checks on top of `is_kb_corpus_path`. That function already rejects all of them (and `#`, archive, and absolute paths), so only the 512-byte cap was added. The mutation therefore targets the `..` rule in `kb_ids.rs`, which was restored immediately.

**2. Plan shape adjustment, `vox_kb_reindex` argument rejection**
- A plain `deny_unknown_fields` struct would also reject `agent_id` / `session_id` / `trace_id` / `correlation_id` / `task_id`, which the dispatch layer reads from `args` for any tool. `reject_reindex_args` allows exactly those meta keys and rejects everything else. No root or path parameter exists. A rejected call returns `Err` from dispatch, like a failed `serde_json::from_value` on the other arms.

**3. [Rule 3 - Blocking] `catalog.v1.yaml` YAML parse error**
- **Found during:** Task 2, `operations-sync`.
- **Issue:** the `kb.status` description contained `: ` inside a plain scalar.
- **Fix:** reworded to "index state, covering corpus root ...". Commit 1951ce6a3.

**4. Test-file note**
- `kb_get_never_reads_files` sets `state.repository.root` to a temp repo and drops a valid `docs/src/late.md` there after indexing, then expects `not indexed`. It also source-scans the production half of `library.rs` for `std::fs`, `tokio::fs`, `File::open` and `read_to_string`. The scan is built with `concat!` so the literal `std::fs` never appears in the file (the plan's acceptance grep).

## Notes and Deferred Issues

- **Pre-existing, out of scope:** `vox ci ssot-drift` fails its `sql-surface-guard` on `crates/vox-search/tests/kb_library_index.rs` (raw `query`/`execute`, from 02-01 and 02-04). It is logged in `deferred-items.md`. It is not caused by this plan and hosted CI will hit it.
- **`vox ci` freshness:** the installed `vox` is stale (built at 6382, tree at 6578). All `vox ci` calls used `VOX_SKIP_FRESHNESS_CHECK=1`. Refresh with `cargo install --locked --path crates/vox-cli --force` before the PR.
- **Test isolation:** library tests that touch `VOX_SEARCH_TANTIVY_ROOT` are `#[serial]` and call `vox_config::snapshot::bump` after `set_var`. They run in the default (non-nextest-sharded) crate test binary alongside the existing `#[serial]` tests.
- **Not in scope here:** CLI projection and `vox kb` commands (02-08), `kb://` resolution beyond URI strings, deprecating the legacy `vox_kb_*` tools (Phase 3).

## Known Stubs

None.

## Threat Flags

None. T-02-15..T-02-21 are mitigated as planned: path and hash validation plus corpus-filtered readers and no fs access (T-02-15, T-02-20), no path parameter on reindex (T-02-16), single-flight (T-02-17), query cap and limit clamp (T-02-18), reindex absent from the read-role list and asserted by `kb_tool_surface` (T-02-19), vox-crypto hasher with the `kb_reindex_uses_vox_crypto_hasher` test (T-02-21).

## Self-Check: PASSED

- Files exist: `crates/vox-orchestrator-mcp/src/kb_tools/library.rs`, `crates/vox-orchestrator-mcp/tests/kb_tool_surface.rs`, this SUMMARY.
- Commits exist: f91256c98, 1951ce6a3, 8ef8ec8a2 (`git rev-list --count fd4003dba..HEAD` = 3 before this SUMMARY commit).
