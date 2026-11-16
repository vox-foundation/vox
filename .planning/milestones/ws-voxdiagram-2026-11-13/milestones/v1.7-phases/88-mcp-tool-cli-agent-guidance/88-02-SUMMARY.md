---
phase: 88-mcp-tool-cli-agent-guidance
plan: 02
subsystem: cli
tags: [cli, vox-diagram, clap, operations-catalog, exit-codes, json-envelope]

requires:
  - phase: 88-mcp-tool-cli-agent-guidance
    provides: "vox_diagram::validate envelope (plan 01), reached here through vox_doc_pipeline::diagram"
provides:
  - "vox diagram check|fmt|render|export|import <file|-> with gate-ready exit codes (0 / 1 / 2)"
  - "global --json prints exactly the vox_validate_diagram envelope for every subcommand"
  - "six catalog rows (diagram, diagram.{check,fmt,render,export,import}) with generated registry, surface row, docs and paths baseline"
affects: [88-03 reference page and cli.md link, 90 fence migration (CLI is the file/stdin entry point)]

status: complete
plan_head_before: 5c1aaa21afada6607c7b55d912b3379be2f302da
plan_head_after: 6047fc158cc7934313eb50fd167f5f7106bdeb0e
actuals:
  tokens: 7000
  tasks: 2
  commits: 2

tech-stack:
  added: []
  patterns:
    - "One execute(cmd, json, stdin, out, err) -> i32 over injected streams; errors map to exit 2 in one place"
    - "In-place rewrite through sibling temp file + rename; invalid source never written"

key-files:
  created:
    - crates/vox-cli/src/commands/diagram.rs
    - crates/vox-cli/tests/diagram_cli.rs
  modified:
    - crates/vox-cli/src/lib.rs
    - crates/vox-cli/src/cli_dispatch/mod.rs
    - crates/vox-cli/src/commands/mod.rs
    - crates/vox-cli/tests/fixtures/command_catalog_paths_baseline.txt
    - contracts/operations/catalog.v1.yaml
    - docs/src/reference/cli.md

key-decisions:
  - "Help/doc lines contain no ': ' so they stay plain YAML scalars in the catalog and equal the generated surface text"
  - "fmt without flags prints to stdout; warnings use the same file:line:col: code: message shape (exit code carries severity)"
  - "Catalog side_effect_class set (diagram, fmt, render, export, import: writes_files; check: read_only) because the capability-index side-effect ratchet rejects unclassified cli rows"

patterns-established:
  - "Oversized input is read with take(MAX_SOURCE_BYTES + 1) and decoded lossily so the core's limit-source diagnostic is reported, not a UTF-8 error"

requirements-completed: [TOOL-02]

duration: ~70min
completed: 2026-10-17
---

# Phase 88 Plan 02: vox diagram CLI Summary

**`vox diagram check|fmt|render|export|import <file|->` as a thin clap command over the shared validation envelope, with `file:line:col` diagnostics, exit codes that work as a gate, `--json` identical to the MCP tool, and full catalog/docs/GUI-surface registration with no new crate edge.**

## Accomplishments

- Task 1 (tracer): `vox diagram check` on a file and on stdin goes clap -> `Cli::Diagram` arm -> `commands::diagram::run` -> `vox_doc_pipeline::diagram::validate` -> stderr lines (`<file>:<line>:<col>: <code>: <message>`, `<stdin>` for `-`) and exit 0 / 1 / 2. Warnings print and do not fail. `--json` works as a global flag in both positions.
- Task 2: `fmt` (stdout, `--check`, `--write`), `render`, `export`, `import` with `-o FILE`; `import` prints `<file>:<line>: unsupported: <construct>` (and `dropped:`) lines; a render over the caps exits 1 with the `emit_error` text. `--json` refuses `-o`/`--write`/`--check` before reading input (exit 2). `--write` refuses `-`, writes only a valid, differing canonical form, via a sibling temp file, permission copy and `rename`.
- Registration: six catalog rows, regenerated command registry / capability registry / model manifest / capability index / surface registry (one `cli_group: diagram`, tier `none`, numstat `8 0`) / coverage / test inventory / TS, a `### vox diagram` section, index and reachability rows in `cli.md`, and five new paths in the baseline. `crates/vox-cli/Cargo.toml` is byte-unchanged; `vox ci crate-edges` reports no new edge.

## Task Commits

1. Task 1 tracer: `86f95e444`
2. Task 2 fmt/render/export/import + mutation proof: `6047fc158`

## TDD record

- Task 1 RED: `cargo nextest run -p vox-cli --test diagram_cli` before any production code: 5 of 6 failed (`error: unrecognized subcommand 'diagram'`, exit 2 where 1/0 expected); `no_subcommand_exits_2` passed trivially (clap exit 2). GREEN: 6/6 plus 4 `commands::diagram::` unit tests.
- Task 2 RED: with the Task 1 binary, 11 of 19 `diagram_cli` tests failed (`unrecognized subcommand 'fmt'|'render'|'export'|'import'`); `fmt_write_stdin_is_usage_error` and `json_with_output_is_usage_error` passed trivially because clap also exits 2. GREEN: 19/19 `diagram_cli`, 5 unit tests (incl. `--json` conflict and stdin `--write` refusal over in-memory streams), 1 baseline test.

## Mutation proof (exit code is the gate)

Copied `commands/diagram.rs` to the scratchpad, changed `let failed = i32::from(!v.valid || v.emit_error.is_some());` to `i32::from(v.emit_error.is_some())` (grep count of the original line 1 -> 0, mutated line 0 -> 1). `cargo nextest ... binary(diagram_cli)`: 5 failed, including `check_file_reports_every_error_and_exits_1` with ``assertion `left == right` failed: exit code of a broken diagram; stderr: <path>:3:10: diagram/undeclared-node ... left: 0 right: 1``, plus `check_json_prints_the_mcp_envelope`, `fmt_write_leaves_invalid_file_untouched`, `import_lists_unsupported_lines`, `render_emits_the_envelope_svg`. Restored from the copy (`diff` identical, counts back to 1/0), rebuilt, reran green (25/25).

## Verification

- `cargo nextest run -p vox-cli --retries 0 --lib --test diagram_cli --test command_catalog_paths_baseline -E 'test(/^commands::diagram::/) | binary(diagram_cli) | binary(command_catalog_paths_baseline)'`: 25/25 (5 unit, 19 binary, 1 baseline).
- `vox ci command-compliance`, `command-sync`, `crate-api-sync`, `gui-surface-registry`, `crate-edges`, `ssot-drift`: exit 0. `vox-doc-pipeline --lint-only --paths reference/cli.md`: no hard errors.
- Acceptance: vox-cli `Cargo.toml` unchanged vs `82b24d2d4`; one `use vox_doc_pipeline::diagram`; one `Cli::Diagram`; surface registry `8 0`; catalog removed column `0`, `exemptions:` block 7 lines; `diagram.rs` 297 non-blank lines; `fs::rename` present.

## Deviations from Plan

### Auto-fixed / handled

**1. [Rule 2 - Missing critical] `side_effect_class` on the new catalog rows**
- **Found during:** Task 1 generation (`vox ci capability-index --write`)
- **Issue:** the plan said "other keys as in `grammar`" (null), but the capability-index side-effect ratchet fails for unclassified cli rows (`cli.diagram, cli.diagram.check`).
- **Fix:** classified from the handler, not the name: `diagram` and `fmt`/`render`/`export`/`import` = `writes_files` (`--write`, `-o`), `check` = `read_only`. The ratchet baseline was not touched.
- **Files modified:** `contracts/operations/catalog.v1.yaml` and the projections.
- **Commit:** `86f95e444`, `6047fc158`

**2. [Rule 3 - Blocking] Pre-existing paths-baseline drift absorbed**
- **Found during:** Task 1 baseline update
- **Issue:** `command_catalog_paths_match_baseline` already failed at the phase base: the baseline lacked `audit/docs`, `ci/crate-api-sync`, `workflow/signal`, which exist in the clap tree. The generator rewrites the whole file, so those three lines landed in `86f95e444` together with `diagram` and `diagram/check`; the plan's verify requires this test to pass.
- **Commit:** `86f95e444`

**3. [Minor] Generator invocation without PATH override**
- The shell allowlist blocks `$V`-style and inline `PATH=` forms, so the `scripts/ssot-regen.vox` generator list was run command by command via `target/debug/vox --quiet ci ...` (same commands and arguments, no freshness bypass), as in plan 01.

**4. [Minor] Help text and size trimming**
- `Cli::Diagram` help is "Check, format, render, export and import VoxDiagram diagrams from a file or stdin" (no `|` or backticks, which would break the generated markdown tables and plain YAML). To stay under 300 non-blank lines, the unit test `valid_source_exits_0_silently` was dropped (covered by the binary test `check_stdin_valid_exits_0_silently`).
- `crate-api-sync --write` and `docs/src/api/vox-cli.generated.md` needed no change (it lists no CLI items), so that file is not in the commits.

## Known Stubs

None.

## Threat Flags

None beyond the plan's threat model: T-88-06 (temp+rename, valid-only, stdin refused, `fmt_write_leaves_invalid_file_untouched`), T-88-07 (`take(MAX_SOURCE_BYTES + 1)` + `oversized_input_gets_the_size_diagnostic_not_a_read_error`), T-88-09 (Cargo.toml byte-unchanged, crate-edges clean), T-88-SC (no dependency added). Known ceiling recorded in code: a symlinked `--write` target is replaced by a regular file (`ponytail:` comment).

## Self-Check: PASSED

Files `crates/vox-cli/src/commands/diagram.rs`, `crates/vox-cli/tests/diagram_cli.rs` exist; commits `86f95e444` and `6047fc158` are ancestors of HEAD (verified below).
