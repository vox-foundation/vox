---
phase: 51-unified-intellisense-dynamic-completions
plan: "01"
subsystem: vox-lsp, vox-cli, vox-gui
tags: [intellisense, completions, nushell, nucleo, fuzzy-ranking, discovery]
provides:
  - Context-aware vox-lsp::CompletionEngine with trigger, cursor, and AST symbol extraction
  - vox completions nushell (and nu alias) with valid extern "vox" declarations
  - vox doctor verification for Nushell config.nu
  - Multi-source nucleo-ranked discovery_suggest in vox-gui
key-files:
  modified:
    - crates/vox-lsp/src/completions.rs
    - crates/vox-lsp/src/server.rs
    - crates/vox-lsp/src/symbols.rs
    - crates/vox-cli/src/lib.rs
    - crates/vox-cli/src/command_catalog.rs
    - crates/vox-cli/src/cli_dispatch/mod.rs
    - crates/vox-cli/src/commands/diagnostics/doctor/checks_standard/completions.rs
    - crates/vox-gui/src/commands/discovery.rs
completed: 2026-10-15
---

# Phase 51 Plan 01: Context-Aware `vox-lsp::CompletionEngine`, `vox completions nushell`, and `nucleo` Multi-Source `discovery_suggest` Summary

## Accomplishments
1. **Context-Aware `vox-lsp::CompletionEngine` (`crates/vox-lsp/src/completions.rs` & `server.rs`)**:
   - Upgraded `completions_for_document` to inspect trigger character (`@` -> decorator-only completions with docs, `:` / `->` -> `SURFACE_TYPE_NAMES` + user types), word prefixes, and document AST symbols.
   - Built resilient `extract_symbols_from_source` in [`crates/vox-lsp/src/symbols.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-lsp/src/symbols.rs) that handles incomplete buffer states via full parse, script parse, cursor-line blanking, and token fallback.
   - Wired `self.documents` in LSP completion handler so active document text is passed.
2. **Nushell Completion Generator & Doctor Check (`vox completions nushell`)**:
   - Added `render_nushell_completions` in [`crates/vox-cli/src/command_catalog.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-cli/src/command_catalog.rs) emitting valid `extern "vox"` and `extern "vox <cmd>"` definitions with `--help(-h)`, typed parameters, and descriptions.
   - Extended `CompletionShell` with `nushell` / `nu` visible alias in [`crates/vox-cli/src/lib.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-cli/src/lib.rs) and wired dispatch.
   - Added Nushell (`~/.config/nushell/config.nu` and `%APPDATA%\nushell\config.nu`) check to `vox doctor` shell completion inspection in [`crates/vox-cli/src/commands/diagnostics/doctor/checks_standard/completions.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-cli/src/commands/diagnostics/doctor/checks_standard/completions.rs).
3. **`nucleo`-Powered Multi-Source `discovery_suggest` (`crates/vox-gui/src/commands/discovery.rs`)**:
   - Enriched `Suggestion` with `kind`, `category`, `signature`, and `score` fields.
   - Upgraded `match_catalog` to rank terminal `/slash` commands, `!nu` Nushell pipelines, `--flag` completions, and subcommands via `FuzzyMatcher` (`nucleo`), with exact prefix and path subsequence boosts.

## Verification
- `cargo test -p vox-lsp` -> 18 unit tests + 31 integration tests passed (`test completions::tests::test_at_trigger_returns_decorators_with_doc ... ok`, `test completions::tests::test_type_position_prioritizes_surface_type_names ... ok`, `test completions::tests::test_document_ast_symbol_included_in_completions ... ok`).
- `cargo test -p vox-cli --lib command_catalog` -> 30 passed (`test command_catalog::tests::render_nushell_completions_emits_valid_signatures ... ok`).
- `cargo test -p vox-gui --bin vox-gui commands::discovery::tests` -> 8 passed (`fuzzy_match_ranks_typos_and_subsequences ... ok`, `flag_completion_suggests_command_flags ... ok`, `slash_and_nu_completions_work ... ok`).
