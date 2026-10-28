# Plan 23-03 Summary: Systematic Language Spec & Runtime Gap Closure (GEN-05)

## Overview
Closed the high-priority language specification, runtime documentation, and CLI validation gaps cataloged in `docs/src/api/DOC_GAPS.md`: deterministic verification of `build_catalog()` top-level commands against `contracts/operations/catalog.v1.yaml`, and the retired-syntax prose lint for `@`-prefixed data-layer decorators with line-level migration cue filtering and structured document carve-outs.

## Deliverables
1. **CLI `build_catalog()` Presence Check:**
   - Implemented `check_catalog_top_level_command_presence` in `crates/vox-cli/src/commands/ci/command_compliance/validators.rs`.
   - Wired into `command_compliance::run` in `crates/vox-cli/src/commands/ci/command_compliance/mod.rs`.
   - Validates that every top-level clap command in the CLI catalog exists as a registered operation in `contracts/operations/catalog.v1.yaml`.
2. **Retired Decorator Syntax Prose Lint:**
   - Added 9 retired decorator forms to `contracts/documentation/retired-symbols.v1.yaml`: `@table`, `@query`, `@mutation`, `@server`, `@tool`, `@resource`, `@form`, `@index`, `@endpoint`.
   - Enhanced `crates/vox-cli-ci/src/retired_symbol_check.rs`:
     - Added line-level migration cue filtering (`retired`, `removed`, `deprecated`, `instead`, `→`, `no longer`, `superseded`, `replaced by`, `migration`, `formerly`, `legacy`, `never emit`, `not @`).
     - Restricted scanning to Markdown files (`.md`), excluding JSON data files (`docs/agents/*.json`).
     - Added carve-outs for historical and migration documents (ADRs, case studies, news items, `migration-*.md`, `DOC_GAPS.md`, and generated crate API docs).
     - Fixed retired syntax references across documentation surfaces (`vox-codegen-ts.md`, `mcp-vox-language-exposure.md`, `internal-web-ir-implementation-blueprint.md`).
3. **DOC_GAPS.md Updates:**
   - Marked CLI registry coverage against `build_catalog()` as closed with citations.
   - Marked retired-syntax prose lint coverage as closed with citations.

## Verification Results
- `cargo run -p vox-cli -- ci command-compliance`: passed.
- `cargo run -p vox-cli -- ci retired-symbol-check`: passed with zero violations across the repository.
- `cargo run -p vox-doc-pipeline -- --lint-only --paths api/DOC_GAPS.md`: passed with zero hard errors.
- `rustfmt --edition 2024`: all touched `.rs` files formatted cleanly.
