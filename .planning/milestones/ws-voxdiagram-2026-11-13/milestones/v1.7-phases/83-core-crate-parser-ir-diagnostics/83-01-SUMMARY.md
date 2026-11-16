---
phase: 83-core-crate-parser-ir-diagnostics
plan: 01
subsystem: diagram-core
tags: [rust, parser, ir, diagnostics, serde, insta, governance]

requires: []
provides:
  - "vox-diagram L2 crate: check()/parse() over a minimal graph (lexer -> recovering parser -> hoisted resolver -> serde IR -> CheckReport)"
  - "Public data contract: Span/Location, Diagnostic/Fix/Related/Stats/CheckReport, Ir/GraphIr/IrNode/IrContainer/IrEdge/Shape/Direction, ast node types"
  - "codes.rs (UNEXPECTED_TOKEN, UNDECLARED_NODE, LIMIT_SOURCE, ALL_CODES) and limits.rs (all six caps)"
  - "Governance registration: layers.toml L2 row (orphan_exempt), where-things-live rows, crate-graph, generated API page, workspace-hack regen"
affects: [83-02, 83-03, 83-04, 83-05, 83-06, 83-07, 84, 85, 86, 87, 88]

actuals:
  tokens: 12500
  tasks: 2
  commits: 2
plan_head_before: 0f2d8d532ee5be1a9a0d6b83c8aa6720b5201fce
plan_head_after: 950fcca513c24fe36b60386f7f8b8c468a759d46

tech-stack:
  added: []
  patterns:
    - "Panic-free slicing via span::slice (str::get) plus crate-level deny(unwrap/expect/panic)"
    - "Size gate before lexing; LineIndex built once per call"
    - "Hoisted two-pass resolution with BTreeMap scopes (no hash collections)"
    - "Statement-boundary resync (newline or }) so independent errors are all reported"

key-files:
  created:
    - crates/vox-diagram/Cargo.toml
    - crates/vox-diagram/src/lib.rs
    - crates/vox-diagram/src/limits.rs
    - crates/vox-diagram/src/codes.rs
    - crates/vox-diagram/src/span.rs
    - crates/vox-diagram/src/diag.rs
    - crates/vox-diagram/src/ir.rs
    - crates/vox-diagram/src/ast.rs
    - crates/vox-diagram/src/lexer.rs
    - crates/vox-diagram/src/parser/mod.rs
    - crates/vox-diagram/src/resolve.rs
    - crates/vox-diagram/src/check.rs
    - crates/vox-diagram/tests/graph_constructs.rs
    - crates/vox-diagram/tests/snapshots/graph_constructs__tracer_minimal_graph_report.snap
    - docs/src/api/vox-diagram.generated.md
  modified:
    - Cargo.lock
    - crates/workspace-hack/Cargo.toml
    - contracts/ci/crate-graph.v1.json
    - docs/src/api/index.generated.md
    - docs/src/architecture/layers.toml
    - docs/src/architecture/where-things-live.md

key-decisions:
  - "Declared every phase dependency (serde, pulldown-cmark, strsim, insta, proptest, serde_json) in this plan so Cargo.toml/Cargo.lock have one owning plan; pulldown-cmark and strsim are not yet referenced by code"
  - "Tracer lexer treats an unterminated string as Unknown('\"') and has no escape/comment/kebab-ident handling; plan 02 owns those"
  - "Resolver keeps the first declaration on duplicate ids with no diagnostic yet (marked with a ponytail: comment); plan 03 adds duplicate-id/shadow reporting and nested scopes"
  - "Diagnostics echo at most 20 chars of user text in messages"

patterns-established:
  - "Internals are pub(crate) so tdd-guard only gates the thin public API; every module with value-asserting inline tests"
  - "Messages use 'expected X, found Y' with backticked tokens, e.g. expected `:`, found string \"A\""

requirements-completed: []  # DGM-01 and DGM-06 are only partially delivered here (tracer slice); plans 02-03 complete them, so they stay Pending in REQUIREMENTS.md

duration: 70min
completed: 2026-10-16
status: complete
---

# Phase 83 Plan 01: vox-diagram tracer and registration Summary

**A new L2 `vox-diagram` crate whose `check()` takes a minimal `graph` source through size gate, lexer, recovering parser, hoisted resolver and serde IR to a `CheckReport`, with span-precise `unexpected-token`, `undeclared-node` and `limit-source` diagnostics, registered with every governance gate.**

## Performance

- **Duration:** about 70 min, most of it cold-cache builds (workspace-hack drags a full dependency graph into every first build in this worktree)
- **Tasks:** 2 of 2 (tracer + registration)
- **Files modified:** 21

## Accomplishments

- Tracer verified end to end before anything else was built: the minimal graph yields `valid: true`, nodes a/b, edge a->b, `schema_version: 1`, and its report JSON is snapshot-locked (`graph_constructs__tracer_minimal_graph_report.snap`).
- Error paths each produce exactly one diagnostic and withhold the IR: `diagram/unexpected-token` (line 2, col 12, message names `:` and the found string), `diagram/undeclared-node` (line 5, col 10), `diagram/limit-source` (span starts at byte 65,536, pulled back to a char boundary; a 65,536-byte source is accepted).
- Independent mistakes on separate lines are each reported (resync at newline/`}`), covered by `independent_errors_are_all_reported`.
- 18 tests pass under `INSTA_UPDATE=no`; `cargo check -p vox-diagram --all-targets` and `cargo clippy -p vox-diagram --all-targets -- -D warnings` are clean.
- Governance: `cargo run -p vox-arch-check` exits 0 with no `vox-diagram` finding; `vox ci crate-edges` OK (700 edges within baseline, only the exempt `workspace-hack` edge added); `vox ci crate-api-sync` OK; crate-edges and fan-in baselines untouched.

## Task Commits

1. **Task 1: End-to-end tracer** - `b28f440dc` (feat)
2. **Task 2: Governance registration** - `950fcca51` (chore)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `cargo hakari generate --diff` was not empty (plan expected empty, RESEARCH A17)**
- **Found during:** Task 2 step 4
- **Issue:** Diff added `ppv-lite86`, `rand 0.9` (as `rand-274715c4dabd11b0`) and `rand_chacha 0.9` to both `[dependencies]` and `[build-dependencies]` of `crates/workspace-hack/Cargo.toml` (proptest's rand 0.9 chain, which the new crate's dev-dependency brings into the unified set).
- **Fix:** Ran `cargo hakari generate` as the plan prescribes; `--diff` is now empty.
- **Files modified:** `crates/workspace-hack/Cargo.toml`, `Cargo.lock`
- **Commit:** `950fcca51`

**2. [Rule 3 - Blocking] Installed `vox` was stale, so `vox ci` refused to run**
- **Found during:** Task 2 step 3
- **Issue:** Installed `vox` was built at 79d9cd454; the tree is 13 commits later, and `crate_api_docs.rs` changed in between (drops `workspace-hack` from the dependency table, adjusts relative links), so a stale binary would have generated a page CI rejects.
- **Fix:** Did not use `VOX_SKIP_FRESHNESS_CHECK`. Built `vox-cli` from this worktree (`cargo build -p vox-cli`, 6 min) and ran `target/debug/vox ci affected-crates --regen`, `crate-api-sync --write`, `crate-api-sync` and `crate-edges` from it, so every verdict reflects current source.
- **Files modified:** none beyond the plan's generated outputs

**3. [Rule 1 - Bug] Dead-code warning on `span::slice`**
- **Issue:** `slice` is part of the binding interface but unused by tracer code; hosted clippy runs `-D warnings`.
- **Fix:** `#[allow(dead_code)]` with a comment naming plans 02-06 as consumers.

**4. Test-first note (process deviation)**
- The failing-test (RED) compile run was started with a stub `lib.rs`, but the implementation landed while that background `cargo check` was still compiling dependencies, so cargo checked the finished sources and the RED failure was never observed. Tests were written before the implementation; the RED evidence itself is missing.
- A message-wording assertion in `tracer_reports_unexpected_token` (`':'` vs the backticked form used by every message) was corrected on the first GREEN run.

### Environment adaptations (not plan deviations)

- The `agent-*` branch allow-list in the commit-protocol HEAD assertion was not applied: this run is the sequential executor on branch `milestone/v1.7-voxdiagram` (non-default, not protected), as directed by the orchestrator.

## Auth Gates

None.

## Known Stubs

None that block the plan goal. Deliberate tracer limits owned by later plans: no `direction`/`container`/shape/comment/escape handling, no duplicate-id reporting (first declaration wins, marked with a `ponytail:` comment in `resolve.rs`), no `unterminated-string` recovery, no fuel/diagnostic cap, `stats.containers`/`max_depth` always 0. `pulldown-cmark` and `strsim` are declared but not yet used (Plans 05/06). None were added to `.planning/WINDOWS.md` since each is scheduled work in Plans 02-07.

## Threat Flags

None. T-83-01 (size gate before lexing) and T-83-02 (`str::get` slicing, deny(unwrap/expect/panic), LineIndex multibyte/CRLF tests) are implemented and tested; T-83-03/T-83-SC held (only `workspace-hack` edge, no baseline regenerated, no new third-party crate). Not yet done: mutation proof of the size gate (planned with the Plan 03 adversarial suite).

## Notes for later plans

- Pre-existing warning (not touched): `vox ci crate-edges` reports a stale baseline edge `vox-gui -> vox-db-types`.
- Hosted CI will run the full workspace suite for this branch because `Cargo.toml`/`Cargo.lock` and `crate-graph.v1.json` are sentinel paths (RESEARCH Critical Finding 3).
- Plans 02-07 must edit `codes.rs::ALL_CODES` additively and keep Cargo.toml untouched.

## Self-Check: PASSED

- FOUND: crates/vox-diagram/{Cargo.toml,src/lib.rs,src/check.rs,tests/graph_constructs.rs,tests/snapshots/graph_constructs__tracer_minimal_graph_report.snap}, docs/src/api/vox-diagram.generated.md
- FOUND commits: b28f440dc, 950fcca51 (ancestors of HEAD)
- No `*.snap.new` files; `grep -c 'name = "vox-diagram"' Cargo.lock` = 1; layers.toml row has `layer = 2` and `orphan_exempt = true`
