---
phase: 331-tolerant-cli-ergonomics-precomputed-semantic-routing
plan: 01
status: complete
requirements:
  - CLI-TOL-01
  - CLI-TOL-02
---

# Phase 331 Plan 01 Summary: Prefix Unfolding, Global Flag Hoisting & Deterministic Argv Normalizer

## Implemented
- Enabled `infer_subcommands = true` and `infer_long_args = true` on `VoxCliRoot` in `crates/vox-cli/src/lib.rs`, preserving `0 ms` happy-path clap parsing for unambiguous prefixes (`vox sec stat`, `vox comm --rec`) while rejecting ambiguous prefixes (`vox c`, `vox comp`).
- Updated ML delegation in `crates/vox-cli/src/argv_normalize.rs::build_ml_delegation_args` and `crates/vox-cli/src/main.rs` so leading global flags (`--json`, `-q`, `-v`, `--color <WHEN>`) placed before ML subcommands (`mens`, `oratio`, `speech`, `populi`, `mesh`, `train`, `quantize`, and `ext <ml>`) are preserved alongside trailing flags.
- Created `crates/vox-cli/src/argv_normalize.rs` with `OnceLock<RecoveryIndex>` over `CommandCatalog` and `CapabilityIndex::bundled_index()`:
  - Rule 2a (`TokenPermutation`): resolves verb↔noun order permutations (`vox status secrets --json` -> `vox secrets status --json`, `vox remove catalog demo` -> `vox catalog remove demo`).
  - Rule 2b (`UniqueLeafElision`): expands globally unique leaf subcommands (`vox command-compliance` -> `vox ci command-compliance`).
  - Rule 2c (`WrongSubcommandFlag`): redirects wrong-subcommand long flags to sibling commands in the same `source_group` (`vox build --release app.vox` -> `vox compile --release app.vox`).
  - Rule 2d (`FuzzyShortlist`): falls back to `vox_capability_registry::resolve` for shortlist diagnostics without any network or synchronous LLM call.
  - Guards: caps tokens (`MAX_RECOVERY_TOKENS = 16`) and chars (`MAX_RECOVERY_CHARS = 256`), and leaves empty / `-h` / `--help` / `-V` / `--version` inputs untouched.

## Verified
- `cargo test -p vox-cli --lib argv_normalize`: 13 passed, 0 failed (`0.08s`, including 100-iteration warm loop `<150 ms` `<1.5 ms/call` budget check).
- `cargo test -p vox-cli-core --lib`: 97 passed, 0 failed.
- Binary smoke test `./target/debug/vox comm --rec --json` exited `0`.
