# English-Core + Latin Alias Migration Ledger

## Phase 0: Baseline & Inventory Lock

This ledger captures the frozen baseline state of the Vox workspace prior to initiating the English-Core nomenclature migration.

### T001-T005: Core Metadata & Contract Hashes
- **Workspace Members**: 58 packages enumerated under `crates/*` (excluding `crates/vox-py`).
- **Command Registry Hash (`command-registry.yaml`)**: Locked.
- **Operations Catalog Hash (`catalog.v1.yaml`)**: Locked.
- **Capability Registry Hash (`capability-registry.yaml`)**: Locked.
- **Dependency Graph Snapshot**: `cargo metadata --locked --no-deps > migration_cargo_metadata_baseline.json` executed successfully.

### T006-T007: Canonical Concept Domain Map
The following explicit mapping table forms the 1:1 binding between canonical English concepts and Latin aliases:
- `orchestrator` ↔ `dei`
- `skills` ↔ `ars`
- `forge` ↔ `fabrica`
- `database` ↔ `codex`
- `secrets` ↔ `clavis`
- `speech` ↔ `oratio`
- `ml` ↔ `populi`
- `gamification` ↔ `ludus`
- `tutorial` ↔ `schola`
- `package_manager` ↔ `arca`

### T008-T010: CLI Dispatch & Alias Inventory
- **clap-visible aliases (`crates/vox-cli/src/lib.rs`)**: Currently using explicit `visible_alias` strings (e.g., `visible_alias = "secrets"` for `clavis`).
- **Nested Latin Commands (`crates/vox-cli/src/latin_cmd.rs`)**: Contains enums `FabricaCmd`, `DiagCmd`, `ArsCmd` mapping directly to underlying English args structures (`BuildArgs`, `CheckArgs`, etc.).
- **Dispatch Routes (`crates/vox-cli/src/cli_dispatch/mod.rs`)**: Uses `cli_top_level_into_fabrica_or_self` and `run_*_cmd` functions to route aliases to canonical workflows.

### T011-T013: Ecosystem SSOT & CI Baseline
- **CI Checks (`.github/workflows/ci.yml`)**: Includes explicit guards for `codex-ssot`, `check-docs-ssot`, `command-compliance`, `clavis-parity`.
- **Nomenclature Rules (`nomenclature-migration-map.md`)**: Currently positions English as canonical text but Latin as primary CLI structure (`latin_ns`).
- **Orphan Surface Inventory (`orphan-surface-inventory.md`)**: Reflects `vox-dei` as a minimal member, with `vox-orchestrator` handling heavy lifting.

### T014-T018: API & Crate Dependency Baseline
- `vox-dei` currently acts as a slim structural member.
- `vox-ars` exports skill registries and workflows.
- `vox-orchestrator` holds canonical orchestration APIs.
- API exports and paths are logged for safe forwarding shim construction in Phase 3 & 4.

### T019-T023: Build & CI Performance (pre-migration)
- Build timings: Stable.
- Test pass set (`vox-cli`, `vox-mcp`, `vox-orchestrator`): Green.
- Command compliance: Passing.
- Capability sync: Clean.

---

## Migration Risk Log (T024)

### Identified Risks & Mitigations
1. **Dangling Docs Links**: Renaming concept structures might invalidate `docs/src` markdown paths.
   *Mitigation*: Automated doc-inventory verification and link-checker in `.github/workflows/ci.yml`. Phase 6 handles bindings before Phase 7 does any physical directory moves.
2. **LLM Context Disruption**: AI agents are currently heavily context-biased toward `vox-dei` and `vox-ars`. Removing the terms abruptly will degrade code generation accuracy.
   *Mitigation*: Header bindings in `lib.rs` and `Cargo.toml` keywords (Phase 6), plus a deprecated forwarding shim with Tombstone warnings (Phases 3/4).
3. **Broken CI Workflows**: Cargo paths and features inside `.github/workflows/ci.yml` that rely on `vox-dei` (e.g., `ci no-vox-dei-import`).
   *Mitigation*: Phase 5 enforces renaming rules, and we will update all CI scripts iteratively alongside crate logic updates.
4. **Collision of Latin/English CLI arguments**: Passing English args to a Latin alias and causing parse errors, or vice versa.
   *Mitigation*: CLI Interchangeability (Phase 2) builds 1:1 mapping directly in the parsing layer, tested for deterministic output.

---

## Phase 1: Canonical English Naming in Contract Layer (Completed)

This phase systematically verified and extended the `catalog.v1.schema.json` and its projections.

### T025-T040: Contract Schema and Base Mapping
- Safely extended `catalog.v1.schema.json` inserting `canonical_name` and `latin_aliases` safely without breaking downstream JSON tooling.
- Populated `catalog.v1.yaml` with explicit bounds mapping `dei -> orchestrator`, `ars -> skills`, `fabrica -> forge`, `codex -> database`, etc.

### T041-T044: Projections 
- Automatically generated capabilities and CLI representations mapping via synchronous pipeline updates.

### T045-T054: Built-in Tests & CI Verifiers
- Authored rigid CI safeguards covering T045..T050 directly deeply within `commands::ci::operations_catalog`. Extracted verification checks into `verify_catalog_nomenclature()`.
- Wrote unit tests confirming the system actively rejects structural/alias collisions, retired boundaries, missing core aliases, and enforces `^[a-z]+(-[a-z]+)*$` nomenclature string grammar checks.

### T055-T066: Status
- All compliance checks are actively gated inside `ci command-compliance` and `ci operations-verify` respectively.
- Phase locked and green.
