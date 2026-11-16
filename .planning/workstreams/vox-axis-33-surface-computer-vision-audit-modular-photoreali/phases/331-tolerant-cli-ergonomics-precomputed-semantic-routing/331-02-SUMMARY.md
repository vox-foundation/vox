---
phase: 331-tolerant-cli-ergonomics-precomputed-semantic-routing
plan: 02
status: complete
requirements:
  - CLI-TOL-03
---

# Phase 331 Plan 02 Summary: Dual-Persona & Side-Effect Safety Gate (`CLI-TOL-03`)

## Implemented
- Added `CallerPersona` (`InteractiveHuman`, `NonInteractiveOrAgent`), `ExecutionDecision` (`AutoExecute`, `PromptConfirm`, `RejectWithDiagnostic`), `CliErrorRecovery`, `decide_execution`, `format_recovery_diagnostic`, `lookup_side_effect_class`, and `parse_cli_with_tolerance` in `crates/vox-cli/src/argv_normalize.rs`.
- Wired `vox_cli::argv_normalize::parse_cli_with_tolerance()` in `crates/vox-cli/src/main.rs` replacing bare `VoxCliRoot::parse()`.
- Safety guarantees verified by unit tests & binary execution:
  - `--json`, `CI`, `VOX_MCP`, `VOX_NONINTERACTIVE`, `VOX_CLI_GLOBAL_JSON`, or non-TTY `stdin`/`stdout` classify as `NonInteractiveOrAgent` and never block on stdin or auto-execute.
  - `lookup_side_effect_class` conservatively defaults unannotated paths to `"local_mutation"`, allowing TTY `AutoExecute` only when `side_effect_class == "read_only"` and `kind` is `TokenPermutation | UniqueLeafElision | SynonymRewrite`.
  - Non-interactive rejections emit `aci.error_recovery` JSON or diagnostic text on exit code `2`.

## Verified
- `cargo test -p vox-cli --lib argv_normalize`: 13 passed, 0 failed.
- Binary smoke test `./target/debug/vox status secrets --json`: exited `2` with valid `aci.error_recovery` JSON (`CLI_TOKEN_ORDER_SWAP`, `canonical_suggestion: ["./target/debug/vox", "secrets", "status", "--json"]`, `side_effect_class: "read_only"`, `auto_executed: false`).
