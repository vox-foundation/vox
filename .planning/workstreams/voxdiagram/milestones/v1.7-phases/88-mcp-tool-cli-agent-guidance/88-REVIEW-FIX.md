---
phase: 88-mcp-tool-cli-agent-guidance
fixed_at: 2026-10-17T21:30:00Z
review_path: /Users/brbrainerd/dev/vox-voxdiagram/.planning/workstreams/voxdiagram/phases/88-mcp-tool-cli-agent-guidance/88-REVIEW.md
iteration: 1
findings_in_scope: 9
fixed: 9
skipped: 0
status: all_fixed
---

# Phase 88: Code Review Fix Report

**Fixed at:** 2026-10-17
**Source review:** 88-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 9 (WR-01..WR-06 plus the IN-01, IN-02 and IN-04 items the orchestrator asked for)
- Fixed: 9
- Skipped: 0

**Where it ran:** `workflow.use_worktrees` is `false`, so edits and commits landed directly in the `/Users/brbrainerd/dev/vox-voxdiagram` checkout on `milestone/v1.7-voxdiagram`. No worktree, temp branch or recovery sentinel was created. Verification ran in that same checkout, so every number below is reproducible from it.

Two commits, because the code and docs hunks overlap in the same files:
- `d36720b86` covers the docs, the AGENTS.md subsection and the grammar card (WR-05, WR-06 docs, IN-04).
- `4be9a44bd` covers the CLI code, its tests and the test inventory (WR-01..04, WR-06 test rename, IN-01, IN-02).

Each fix started red. `diagram_cli_files.rs`, the updated `diagram_cli.rs` assertions and `grammar_card_states_the_render_caps` failed against the old code. Then the fix went in and they went green.

## Fixed Issues

### WR-01: `fmt --write` replaces a symlink with a regular file

**Files modified:** `crates/vox-cli/src/commands/diagram.rs`, new `crates/vox-cli/src/commands/diagram/write.rs`, `crates/vox-cli/tests/diagram_cli_files.rs`
**Commit:** 4be9a44bd
**Applied fix:** I chose write-through over refusal. `write_atomic` canonicalizes the path first, so the temp file and the rename land next to the real target. The link survives and the real file is formatted. A dangling symlink is an error and is never replaced. The reference page documents this. Regression test: `fmt_write_writes_through_a_symlink`.

### WR-02: predictable, non-exclusive temp file

**Files modified:** `crates/vox-cli/src/commands/diagram/write.rs`
**Commit:** 4be9a44bd
**Applied fix:** The temp file is created with `create_new(true)` (O_EXCL, so a planted symlink is never followed). On Unix it is created with the source's mode (`& 0o7777`), so a 0600 source is never briefly wider. The final `set_permissions` is done on the open handle before the rename. A cleanup runs only after our own create succeeded, so a foreign file at the temp path is never removed. Regression tests: unit `a_planted_temp_symlink_is_never_followed` and integration `fmt_write_keeps_the_source_mode`.

I checked the guard by mutation: switching to `create(true).truncate(true)` made the planted-symlink test fail. The mutation was reverted, with a presence check before and after.

### WR-03: `fmt --write` rewrites read-only files

**Files modified:** `crates/vox-cli/src/commands/diagram/write.rs`
**Commit:** 4be9a44bd
**Applied fix:** `write_atomic` bails with "file is read-only" (exit 2) when the existing target is read-only. Regression test: `fmt_write_refuses_a_read_only_file`, which skips itself when run as root.

### WR-04: `-o FILE` can destroy the input and is not atomic

**Files modified:** `crates/vox-cli/src/commands/diagram.rs`, `crates/vox-cli/src/commands/diagram/write.rs`, `crates/vox-cli/tests/diagram_cli_files.rs`
**Commit:** 4be9a44bd
**Applied fix:** `ensure_not_input` compares canonicalized paths before the input is read. `-o` onto the input exits 2 for `render`, `export` and `import`, and it catches `./` spellings and symlink aliases. `-o` now goes through the same `write_atomic`. It also follows symlinks and refuses read-only targets, and it creates new files. Regression tests: `output_may_not_be_the_input`, `output_refuses_a_read_only_target_and_follows_symlinks`, `output_file_is_written_whole_without_temp_litter`, and unit `output_equal_to_input_is_refused_by_any_spelling`.

### WR-05: AGENTS.md names a CLI path that never prints `fixes[]`

**Files modified:** `AGENTS.md`, `docs/src/reference/vox-diagram.md`
**Commit:** d36720b86
**Applied fix:** I took the smaller change, docs rather than a text-mode fix printer. AGENTS.md now says to use `vox --json diagram check <file|->` and notes that `fixes[]` appear only in the JSON envelope. The subsection stays at 6 body lines. The reference page's repair loop and CLI table say the same. Regression test: `docs_send_agents_to_the_json_envelope_for_fixes`.

### WR-06: docs claim the CLI `--json` output is the MCP output

**Files modified:** `docs/src/reference/cli.md`, `docs/src/reference/vox-diagram.md`, `crates/vox-cli/src/commands/diagram.rs` (module doc), `crates/vox-cli/tests/diagram_cli.rs`
**Commit:** d36720b86 (docs), 4be9a44bd (test and module doc)
**Applied fix:** I chose to correct the docs rather than change the CLI output. The docs now say the CLI prints the bare envelope, which is the MCP tool's `data` field. The tool wraps it as `{success, data}`, or `{success:false, error}` for invalid arguments. The module doc says the same. The test `check_json_prints_the_mcp_envelope` was renamed to `check_json_prints_the_envelope`. The MCP side already asserts `data == validate()` in `validate_diagram_tool.rs`, so no new parity test was needed. Status: fixed, requires human verification of the wording, since this is a doc-accuracy call.

### IN-01: `fmt --check -` hint named an unsupported command

**Files modified:** `crates/vox-cli/src/commands/diagram.rs`
**Commit:** 4be9a44bd
**Applied fix:** On stdin the message now reads "run `vox diagram fmt -` to print the canonical form". File inputs keep the `--write` hint. Regression test: `fmt_check_on_stdin_suggests_a_command_that_exists`.

### IN-02: text-mode diagnostics hid severity

**Files modified:** `crates/vox-cli/src/commands/diagram.rs`, `crates/vox-cli/tests/diagram_cli.rs`
**Commit:** 4be9a44bd
**Applied fix:** The format is now `file:line:col: error|warning: code: message`. The existing exact-line assertions were updated, and `check_warning_only_exits_0` now asserts `warning:`. The reference page documents the format.

### IN-04: grammar card hid the SVG render caps

**Files modified:** `crates/vox-diagram/grammar-card.txt`, `docs/src/reference/vox-diagram.md`, `crates/vox-diagram/tests/grammar_card.rs`
**Commit:** d36720b86
**Applied fix:** Rule 6 gains a continuation line: "SVG render caps: 100 nodes+containers, 200 edges (sequence: 100 actors, 200 events)." The reference page carries the card verbatim, so the page states the caps next to the parse caps. The card is 2189 bytes against the 2600 budget. The new test `grammar_card_states_the_render_caps` ties the numbers to `MAX_RENDER_NODES` and `MAX_RENDER_EDGES`. The optional `maxLength` schema change was not made.

## Verification

All of it ran in the main checkout:
- `cargo test -p vox-cli --test diagram_cli --test diagram_cli_files`: 19 + 8 passed.
- `cargo test -p vox-cli --lib commands::diagram`: 7 passed.
- `cargo test -p vox-diagram`: all suites green, including `grammar_card`.
- `cargo test -p vox-orchestrator-mcp --test validate_diagram_tool`: 10 passed, including the card-size and tool-list tests.
- `cargo clippy -p vox-cli -p vox-diagram --all-targets`: no findings in touched files. I also removed one pre-existing `needless_question_mark` in `read_source`.
- `cargo fmt -p vox-cli -p vox-diagram`: applied.
- Scoped doc lint (`vox-doc-pipeline --lint-only --paths reference/vox-diagram.md reference/cli.md`): no hard errors.
- Generators, with `target/debug/vox` rebuilt after the last source edit:
  - `ci test-inventory --output contracts/reports/test-inventory.v1.json` changed the file, which went into the second commit.
  - `ci crate-api-sync --write` reported OK with no diff.
- No Cargo or lock edits. No crate-edges changes. All new and edited files are under 400 lines (the largest is `diagram.rs` at 311).

## Skipped Issues

None.

## Not in scope, unchanged

IN-03, IN-05 and IN-06 were not part of this request. `.planning/active-workstream` and `.planning/workstreams/voxdiagram/milestone.lock` were already modified before this run and were left unstaged. This file is not committed.

---

_Fixed: 2026-10-17_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
