---
phase: 12-safety-floor-lowering-paths
plan: 05
type: execute
status: completed
date: 2026-10-14
requirements: [FLOOR-01, FLOOR-02]
files_modified:
  - contracts/orchestration/permission-modes.v2.yaml
  - crates/vox-orchestrator-mcp/src/permission_modes.rs
  - crates/vox-orchestrator-mcp/tests/pending_approvals_tests.rs
  - contracts/index.yaml
  - docs/src/reference/ref-decorators.md
  - docs/src/api/DOC_GAPS.md
  - contracts/dei/rpc-methods.schema.json
  - crates/vox-orchestrator-mcp/src/dispatch.rs
  - crates/vox-orchestrator-mcp/src/approval_allowlist.rs
---

# Phase 12 Plan 05 Summary: Permission Modes Contract v2, Always-Ask Deploy & Floor Categories Parity

## 1. Plan Overview
Plan 12-05 migrated the human-in-the-loop permission modes contract to v2, made `vox_deploy` unconditionally always-ask, recorded the safety floor across all categories and tools in the contract, and enforced bidirectional parity with Rust tables and classifiers.

---

## 2. Decisions
- **Rename vs Side-by-Side (Task 1 Checkpoint):** Option `"rename"` chosen:
  > `git mv v1 → v2, single file, migration_window: none (no runtime readers) noted in the v2 header (recommended)`
  - Recorded verbatim. The file was moved to `contracts/orchestration/permission-modes.v2.yaml`, setting `version: 2` and `x-vox-version: 2`, with all repo references updated to v2. GUI references (`transport.ts`, `ApprovalsView.tsx`) remain deferred to Phase 18.

---

## 3. Key Changes & Verifications

### Task 1 & 2: Contract v2 Bump & `vox_deploy` Always-Ask
- **Implementation:**
  - Moved `permission-modes.v1.yaml` to `permission-modes.v2.yaml`.
  - Updated header: `version: 2`, `x-vox-version: 2`, lifecycle stable, and migration note `migration_window: none — no runtime reader, Rust mirrors via RISK_CLASSES`.
  - In `contracts/orchestration/permission-modes.v2.yaml`: set `vox_deploy` `always_requires_approval: true` and documented that it cannot be auto-approved under any mode or allowlist.
  - In `crates/vox-orchestrator-mcp/src/permission_modes.rs`:
    - Updated `RISK_CLASSES` entry for `vox_deploy` to `always_requires_approval: true`.
    - Updated module docs and comments to reference `permission-modes.v2.yaml`.
  - Pointed `risk_classes_yaml_matches_rust_table` in `pending_approvals_tests.rs` at `permission-modes.v2.yaml`.
- **Verification:**
  - `risk_classes_yaml_matches_rust_table` passed.
  - All 10 unit tests in `permission_modes` passed (including `accept_all_approves_mutating_and_destructive_except_always_requires_approval`).

### Task 3: Floor Categories in Contract, Floor Parity Test & Reference Updates
- **Implementation:**
  - Extended `YamlRiskClass` in `crates/vox-orchestrator-mcp/tests/pending_approvals_tests.rs` with `#[serde(default)] floor: Option<String>`.
  - Added new integration test `floor_categories_yaml_matches_rust_floor`:
    - Validates that YAML `floor_categories` ids match `FloorCategory::ALL` (`history_rewrite`, `policy_grant`, `external_effect`, `secrets_write`).
    - Validates that sorted `tools` match `tools_for(c)` and sorted `arg_rules` match `arg_rule_ids(c)`.
    - Asserts that every non-null `floor` row in `risk_classes` has `always_requires_approval: true` and matches `classify_floor(tool, {})`.
  - Confirmed `floor_categories_yaml_matches_rust_floor` failed initially when `floor_categories` was absent from YAML (`Error("missing field floor_categories")`).
  - Updated `contracts/orchestration/permission-modes.v2.yaml`:
    - Added `floor` field to all 7 `risk_classes` rows (`vox_add_approval_allowlist_entry` -> `policy_grant`, `vox_deploy` -> `external_effect`, others `null`).
    - Added `floor_categories` section with tool lists and arg rules for each category.
  - Updated references across the repository:
    - `contracts/index.yaml`: updated entry `orchestration-permission-modes-v2` with path `contracts/orchestration/permission-modes.v2.yaml` and description.
    - `docs/src/reference/ref-decorators.md`: updated link target to `v2.yaml`.
    - `docs/src/api/DOC_GAPS.md`: updated path text to `v2.yaml`.
    - `contracts/dei/rpc-methods.schema.json`: updated description path to `v2.yaml`.
    - `crates/vox-orchestrator-mcp/src/dispatch.rs`: updated comment at line 740 to `v2.yaml`.
    - `crates/vox-orchestrator-mcp/src/approval_allowlist.rs`: updated comment at line 12 to `v2.yaml`.
- **Verification:**
  - Parity tests passed: `risk_classes_yaml_matches_rust_table` and `floor_categories_yaml_matches_rust_floor`.
  - CI contracts-index check passed: `contracts-index OK (schema v1, 197 entries)`.
  - CI SSOT drift check passed: `ssot-drift: nested SSOT guards OK`.
  - Daemon RPC method parity test passed: `orch_daemon_method_schema_parity ... ok`.
  - Documentation linter passed: `vox-doc-pipeline: linting ./docs/src/reference/ref-decorators.md ... lint complete — no hard errors`.
  - Clippy passed with zero warnings: `cargo clippy --no-deps -p vox-orchestrator-mcp --all-targets -- -D warnings`.
