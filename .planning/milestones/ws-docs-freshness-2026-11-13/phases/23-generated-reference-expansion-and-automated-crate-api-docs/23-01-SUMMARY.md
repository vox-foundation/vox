# Plan 23-01 Summary: Contract Reference Generators (GEN-01, GEN-02, GEN-03)

## Overview
Implemented deterministic generators for environment variables, runtime configuration keys, and MCP tools from their canonical YAML contracts, replacing hand-maintained reference pages and wiring all generators into `vox ci command-sync` with full verification and dead-path test coverage.

## Deliverables
1. **Environment Variables Generator (GEN-01):**
   - Generated `docs/src/reference/env-vars.generated.md` from `contracts/config/env-vars.v1.yaml` (851 variables across all crates).
   - Updated `read_env_vars_ssot_doc` in `crates/vox-cli/src/commands/ci/command_compliance/docs_sync.rs` to read the generated file.
   - Replaced hand-maintained `docs/src/reference/env-vars.md` with a concise pointer to the generated reference.
2. **Configuration Keys & MCP Tools Generators (GEN-02):**
   - Generated `docs/src/reference/config-keys.generated.md` from `contracts/config/registry.v1.yaml` (110 knobs), stating intended readers.
   - Generated `docs/src/reference/mcp-tools.generated.md` from `contracts/mcp/tool-registry.canonical.yaml` (340 tools), stating intended readers.
3. **CI Sync & Verification Integration (GEN-03):**
   - Extended `command_sync::run` in `crates/vox-cli/src/commands/ci/command_sync.rs` to generate and verify all 5 generated reference documents.
   - Created integration tests in `crates/vox-cli/tests/command_sync.rs` testing clean verification, non-empty outputs, and graceful handling of dead/missing contract paths.
   - Updated `AGENTS.md` and verified `.gitattributes` collapse rules for `*.generated.md`.

## Verification Results
- `cargo run -p vox-cli -- ci command-sync --write`: generated all 5 artifacts.
- `cargo test -p vox-cli --test command_sync`: 3/3 tests passed.
- `cargo run -p vox-cli -- ci command-compliance`: passed.
- `cargo run -p vox-doc-pipeline -- --lint-only`: clean on all new and updated docs.
