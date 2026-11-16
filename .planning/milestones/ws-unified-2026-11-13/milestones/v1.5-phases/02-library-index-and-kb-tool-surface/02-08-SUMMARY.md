---
phase: 02-library-index-and-kb-tool-surface
plan: 08
subsystem: cli
tags: [cli, kb, daemon, catalog, docs, changelog]
requires:
  - phase: 02-library-index-and-kb-tool-surface
    provides: "02-06 vox_kb_status / vox_kb_reindex tools and kb.status / kb.reindex catalog rows; 02-05 Tantivy mirror removal; 02-04 probe-chosen lexical leg"
provides:
  - "`vox kb status [--json]` and `vox kb reindex [--json]`, both routed through the orchestrator daemon"
  - "commands::daemon_tool::{daemon_client, tool_call, unwrap_tool_envelope} shared by dei and kb"
  - "Catalog CLI projections [kb, status] / [kb, reindex] with regenerated command registry"
  - "where-things-live, search SSOT and CHANGELOG records of the Library index, the Tantivy leg, the vox_kb_* tools and the API removal"
affects: [02-09, phase-03, phase-11]
actuals:
  tokens: 8500
  tasks: 3
  commits: 2
plan_head_before: 5a427a9b57990c094827098697bd440881952aaf
tech-stack:
  added: []
  patterns:
    - "CLI never opens a second index writer: status and reindex go through orch.tool_call to the daemon's single-flight handler"
    - "Pure render_* functions over the tool's JSON payload, unit-tested without a daemon"
key-files:
  created:
    - crates/vox-cli/src/commands/daemon_tool.rs
  modified:
    - crates/vox-cli/src/commands/kb.rs
    - crates/vox-cli/src/commands/dei.rs
    - crates/vox-cli/src/commands/mod.rs
    - crates/vox-cli/src/cli_dispatch/mod.rs
    - crates/vox-cli/src/lib.rs
    - contracts/operations/catalog.v1.yaml
    - contracts/cli/command-registry.yaml
    - contracts/capability/capability-registry.yaml
    - contracts/reports/operations-catalog-inventory.v1.json
    - docs/src/reference/cli-command-surface.generated.md
    - docs/src/architecture/where-things-live.md
    - docs/src/architecture/search-retrieval-ssot-2026.md
    - CHANGELOG.md
key-decisions:
  - "kb::run became async (Cli::Kb dispatch awaits it); the eval arm stays synchronous inside it"
  - "FileError entries are rendered from `path` (the real field) with `rel_path` accepted as a fallback, since the plan text said rel_path but the serialized struct uses path"
  - "handler_rust is null on the two new catalog rows as the plan specifies (kb.eval keeps commands::kb)"
requirements-completed: [IDX-05, IDX-07]
status: complete
---

# Phase 2 Plan 08: `vox kb status` / `vox kb reindex` and the KB docs Summary

Humans get the same KB control agents have over MCP: `vox kb status` and `vox kb reindex` call the daemon's `vox_kb_status` / `vox_kb_reindex` through one shared helper, so the CLI never writes a Tantivy index itself, and the docs and CHANGELOG now record the Library index, the probe-chosen Tantivy leg and the removed mirror API.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 (tracer) + 2 | `vox kb status` and `vox kb reindex` through the daemon | ed33ab6d5 |
| 3 | Record the KB index in where-things-live, the search SSOT and the CHANGELOG | 212032acf |

Tracer gate: Task 1's verify (unit and clap tests, `operations-verify`, `cargo check --features dei`, `command-compliance`) passed end to end before the reindex half was treated as done, so expansion went ahead.

## What was built

- **`daemon_tool.rs`.** `daemon_client()` and `tool_call()` moved out of `dei.rs` (which is `cfg(feature = "dei")`) into a non-gated `pub(crate) mod daemon_tool`, with the envelope match factored into `unwrap_tool_envelope` (success returns `data`, failure bails with the error text, a value without `success` passes through). `dei.rs` imports them; its rationale comments moved with the helper.
- **`kb.rs`.** `KbCmd::Status { json }` and `KbCmd::Reindex { json }` beside `Eval`. Both call one `daemon_report(tool, json, render)`: `daemon_client` then `tool_call("vox_kb_status" | "vox_kb_reindex", {})`, then pretty JSON or the rendered report. An "already running" or any other tool error propagates through `anyhow` as a non-zero exit with the message as-is. `render_kb_status` prints last-indexed commit, corpus root, doc/chunk/card counts, chunker version, lexical backend, embed model, trust tier, index dir, error count with one line per file, and consistency (a failing check names both counts and points at `vox kb reindex`); an unindexed library prints "not indexed" plus `vox kb reindex`. `render_index_report` prints docs seen/unchanged/changed/pruned, chunks inserted/deleted/kept, cards, corpus commit, lexical backend, duration and the per-file errors. `reindex` takes no path arguments, so `vox kb reindex docs/src` is a clap error.
- **Catalog.** `kb.status` and `kb.reindex` gained cli blocks (`[kb, status]`, `[kb, reindex]`, active, `ref_cli_required: false`). `vox ci operations-sync --target all --write` regenerated `command-registry.yaml`, `capability-registry.yaml` and the tool registry; `operations-verify` reports 1305 rows OK.
- **Docs.** `where-things-live.md`: new rows for the Library index, the `vox_kb_*` tools and `kb://` resources, and `docs/src/library/`; the existing `vox kb eval` row now reads `vox kb eval|status|reindex` (no duplicate). `search-retrieval-ssot-2026.md`: the "Docs mirror" row is replaced by the KB Library corpus row, the RAG chunk row now says FTS5 is unavailable on turso 0.6.1 and LIKE is what runs, the five tools join the section 6 table, and a line notes the `kb://` resources and the daemon restart after upgrade. `CHANGELOG.md` `[Unreleased]`: Added (tools, resources, CLI, Library category, schema baseline 95), Removed (the Tantivy docs mirror, `SearchExecution::tantivy_doc_lines`, `SearchPolicy::tantivy_index_root`), Notes (restart the daemon).

## Verification

| Check | Result |
|-------|--------|
| `cargo nextest run -p vox-cli -E 'test(/unwrap_tool_envelope\|render_kb_status\|render_index_report\|kb_cli/)'` | 8/8 pass |
| `cargo nextest run -p vox-cli --features dei -E 'test(daemon_status_json_deserializes)'` | 1/1 pass |
| `cargo check -p vox-cli --features dei` | exit 0 |
| `cargo clippy -p vox-cli --all-targets -- -D warnings` | clean |
| `vox ci operations-verify` | exit 0 (1305 rows) |
| `cargo run -p vox-cli -- ci command-compliance` | exit 0 after regenerating the command-surface doc |
| `cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/where-things-live.md,architecture/search-retrieval-ssot-2026.md` | no hard errors |
| `grep "fn tool_call" dei.rs` / `daemon_tool.rs` | none / one match |
| `grep "Docs mirror" search-retrieval-ssot-2026.md` | prints nothing |
| `vox ci` freshness | installed `vox` is stale; every `vox ci` call used `VOX_SKIP_FRESHNESS_CHECK=1` |

No mutation proof was run: this plan adds no security guard of its own. T-02-25 (no second writer) holds structurally, since `kb.rs` has no `vox-search` indexing call and both handlers go through `tool_call`.

## Deviations from Plan

**1. [Rule 3 - Blocking] `cli-command-surface.generated.md` regenerated and committed**
- **Found during:** Task 1 verify. `command-compliance` exits non-zero ("is stale") once the new CLI paths exist, and the plan's verify requires exit 0, while the plan also says to leave `docs/src/reference/*.generated.md` to the CI job.
- **Fix:** ran the generator (`vox ci command-sync --write`, not a hand edit) and committed its output. The verify gate is satisfied; ssot-autoregen will find no drift.
- **Commit:** ed33ab6d5

**2. [Rule 3 - Blocking] Test added to `dei.rs`**
- **Found during:** Task 1 commit. The `tdd-guard` pre-commit hook scans every staged file and flagged `dei.rs` (15 pub fns, no `#[test]`) once it was staged for the helper move.
- **Fix:** added `daemon_status_json_deserializes`, which pins the CLI's read-side `DaemonOrchestratorStatus` mirror against the daemon's JSON. No suppression entry written.
- **Commit:** ed33ab6d5

**3. Tasks 1 and 2 share one commit**
- The Status and Reindex variants, both renderers and both catalog cli blocks were built and verified together (one `kb.rs`, one generator run), so they landed in a single commit rather than two. The plan's per-task verify commands all ran against the combined state.

**4. `kb::run` is now async**
- The daemon call needs `.await`; `cli_dispatch/mod.rs` awaits it. The `Eval` arm is unchanged.

**5. Error-entry field name**
- Plan text says `rel_path`; the serialized `FileError` is `{path, error}`. Renderers read `path` and fall back to `rel_path`.

## Notes and Deferred Issues

- The `Cli::Kb` doc comment in `lib.rs` (and the `commands/mod.rs` mod comment) were updated, which is what `cli-command-surface.generated.md` picked up.
- `toestub` is not on this shell's lean-ctx allowlist, so the hook's finding was read from the git hook's own output rather than a direct run.
- `vox ci ssot-drift` was not run in full; the targeted gates above were.

## Known Stubs

None.

## Threat Flags

None. T-02-25 mitigated (daemon-only path); T-02-26 accepted (existing token auth unchanged); T-02-27 mitigated by the CHANGELOG note and the search SSOT restart line.

## Self-Check: PASSED

- Files exist: `crates/vox-cli/src/commands/daemon_tool.rs`, this SUMMARY.
- Commits exist: ed33ab6d5, 212032acf (`git rev-list --count 5a427a9b5..HEAD` = 2 before this SUMMARY commit).
