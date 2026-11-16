---
phase: 88-mcp-tool-cli-agent-guidance
plan: 03
subsystem: docs
tags: [grammar-card, vox-diagram, agents-md, reference-docs, phase-closure]

requires:
  - phase: 88-mcp-tool-cli-agent-guidance
    provides: "vox_validate_diagram envelope + MCP tool (plan 01), vox diagram CLI (plan 02)"
provides:
  - "Finalized graph + sequence GRAMMAR_CARD (2,101 bytes) feeding the MCP tool description and the reference page"
  - "docs/src/reference/vox-diagram.md: card verbatim, graph + sequence vox-diagram examples, MCP envelope, CLI, Mermaid import, legacy-Mermaid policy"
  - "AGENTS.md subsection directing new docs diagrams to vox-diagram fences"
  - "Phase 88 closure: every generator re-run, every named gate recorded"
affects: [89 docs rendering, 90 fence migration]

status: complete
plan_head_before: 6a9038f4509c8f7fa50c31b163492c5a337f9df7
plan_head_after: 910d8a08c3f56374afcb70fa14e3553dc9f9c063
actuals:
  tokens: 3600
  tasks: 3
  commits: 3

key-files:
  created:
    - docs/src/reference/vox-diagram.md
  modified:
    - crates/vox-diagram/grammar-card.txt
    - crates/vox-diagram/tests/grammar_card.rs
    - crates/vox-diagram/src/lib.rs
    - AGENTS.md
    - docs/src/reference/cli.md
    - docs/src/architecture/where-things-live.md
    - docs/src/api/vox-diagram.generated.md
    - contracts/reports/test-inventory.v1.json

key-decisions:
  - "Page example fences are the card's examples with the inline // comments dropped (the card keeps them as teaching text); both check clean"
  - "Gantt statement kept as 'VoxDiagram has no Gantt kind, so Gantt charts stay Mermaid' per plan; no mermaid fence added"

requirements-completed: [TOOL-03]

duration: ~50min
completed: 2026-10-17
---

# Phase 88 Plan 03: Grammar card, reference page, AGENTS.md directive, phase closure Summary

**The VoxDiagram grammar card now teaches both `graph` and `sequence` with checked examples, is published verbatim on a new reference page (with the MCP envelope, CLI and Mermaid policy), AGENTS.md directs new docs diagrams to `vox-diagram` fences, and every generator and phase gate was re-run on the final tree.**

## Accomplishments

- Task 1 (tracer): card header names both kinds, a six-line `sequence` example (message, `note over`, dashed `reply`) and a sequence rule line were added; the Reserved line and Do-NOT section are untouched. `GRAMMAR_CARD` doc comment no longer calls itself a draft. The card flows through plan 01's `advertised_description` (`tool_list_advertises_the_card`, `grammar_card_fits_600_tokens` pass) and onto the page, which doc lint validates.
- Task 2: page sections (`vox_validate_diagram` input/output tables and repair loop, CLI table and exit codes, Mermaid import, legacy-Mermaid policy), `AGENTS.md` pure insertion (7 added, 0 removed), one cli.md sentence linking the page, two where-things-live rows.
- Task 3: regenerated via the `ssot-regen.vox` generator list plus `crate-api-sync --write`; only `contracts/reports/test-inventory.v1.json` (new tests) and `docs/src/api/vox-diagram.generated.md` (card doc comment) changed, both committed.

## Task Commits

1. Task 1 tracer: `f28914704`
2. Task 2 docs: `28346ddd8`
3. Task 3 regeneration: `910d8a08c`

## TDD record

- RED (Task 1): `cargo nextest run -p vox-diagram --test grammar_card` failed to compile: `couldn't read .../docs/src/reference/vox-diagram.md: No such file or directory` (the page-sync test's `include_str!`). Because that is a compile error, the whole binary was RED; the sequence-example test had no separate runtime RED (the card had no `sequence {`, so it would have panicked "no sequence example"). GREEN: 6/6 (4 existing + 2 new).
- No separate mutation proof was required for this plan; T-88-10 is covered by `reference_page_publishes_the_card_verbatim` (any divergence between page and constant fails it) and the unchanged-example/Do-NOT tests.

## Gate results (final tree)

| Gate | Result |
|------|--------|
| `vox ci ssot-drift` | `nested SSOT guards OK` (re-run after the last commit) |
| `command-sync` | OK |
| `command-compliance` | OK (1033 operations) |
| `crate-edges` | OK (702 live edges within baseline) |
| `crate-api-sync` | OK (130 crates) |
| `affected-crates --check` | exit 0 |
| `check-links` | PASS (5368 internal links) |
| `gui-surface-registry` | registry and generated TS up to date |
| `cargo run -p vox-arch-check` | exit 0 (pre-existing LoC/budget WARN lines only) |
| `cargo hakari generate --diff` | empty |
| `vox ci pre-push` (fast) | `profile fast: all checks passed` (drift findings are warnings on unrelated files) |
| `cargo fmt -p vox-diagram -p vox-orchestrator-mcp -p vox-cli --check` | clean |
| scoped doc lint (vox-diagram.md, cli.md, where-things-live.md) and `--mermaid-only` | no hard errors |
| `nextest -p vox-diagram` | 578 passed, 3 skipped |
| `nextest -p vox-mcp-registry` | 85/85 |
| `nextest -p vox-doc-pipeline pipeline::diagrams::` | 10/10 (committed_docs_diagram_fences_are_valid sees the new page) |
| plan 01 MCP filter | 39 passed, 2 failed (both pre-existing, below) |
| plan 02 CLI filter | 25/25 |
| `vox ci profile-parity` | FAIL, pre-existing (below) |

## Phase-wide checks

- `contracts/ci/crate-edges.allow.v1.json`: `7 0` vs `82b24d2d4`; `fan-in-snapshot.v1.json`, `crates/vox-cli/Cargo.toml`, root `Cargo.toml` unchanged; only `crates/vox-orchestrator-mcp/Cargo.toml` changed among manifests.
- Catalog: `exemptions:` block 7 lines, `mcp_tools: []` count 1, numstat `184 0` (append-only), 7 `- id: diagram*` rows.
- `git status --porcelain -- . ':(exclude).planning'` empty after the commits.
- `git diff --quiet 82b24d2d4 -- contracts/documentation/kb-facet-backlog.v1.yaml` clean; no `mermaid` fence in the new page.

## Pre-existing failures (out of scope, logged in deferred-items.md)

1. `registry::profile_backcompat_tests::{snapshot_is_todays_default_feature_registry, dev_profile_advertises_the_snapshot}`: advertised-tools fixture lacks 14 tools already in the registry at the phase base (see plan 88-01 summary).
2. `vox ci profile-parity`: `lean: forbidden crate vox-gamify present in lean build`. Proof it is unrelated: vox-gamify enters the lean `vox-cli` tree through `vox-cli-core` (optional `db` feature, line 15 of its Cargo.toml at `82b24d2d4`), `vox-lsp` and `vox-orchestrator`; none of those changed since the base; `cargo tree -p vox-diagram` has zero gamify matches; the phase's sole manifest change is the leaf edge `vox-orchestrator-mcp -> vox-diagram`. I did not run the gate on a clean checkout of `82b24d2d4` (no extra worktree built), so the proof is by dependency path, not by a base-commit run.

## Deviations from Plan

**1. [Rule 3 - Blocking] Generator invocation without PATH override.** The shell policy blocks `PATH=... vox run scripts/ssot-regen.vox` (exit 126, as in plan 88-01). Ran the identical generator list from `scripts/ssot-regen.vox` via `target/debug/vox ci ...` (script in the session scratchpad), after rebuilding `vox-cli` because the freshness guard keys on commit count. No freshness bypass.

**2. [Scope] GUI typecheck deferred.** `crates/vox-gui/ui/node_modules` is absent, so `pnpm typecheck` was not run; no UI source changed in this plan (generated data unchanged this run) and the hosted `ui` job covers it.

**3. profile-parity not green** (pre-existing, see above); every other named gate passes.

Page example fences drop the card's inline `//` comments; otherwise the plan was executed as written.

## Known Stubs

None.

## Threat Flags

None. No new endpoints, auth paths or schema changes; T-88-10 (card/page drift) is mitigated by the sync test, T-88-11 by the 7-line pure AGENTS.md insertion.

## Self-Check: PASSED

- FOUND: `docs/src/reference/vox-diagram.md`, `crates/vox-diagram/grammar-card.txt`, `crates/vox-diagram/tests/grammar_card.rs`
- FOUND commits: `f28914704`, `28346ddd8`, `910d8a08c`
