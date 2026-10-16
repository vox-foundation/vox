# Phase 12 Plan 01 Summary: Typed Safety Floor & Dispatch Enforcement

**Execution Mode:** Autonomous (`/gsd-autonomous`)  
**Status:** Completed & Verified  

---

## 1. Decisions

- **Task 1 Decision:** `accept-park`
  - Floor applies to every origin; human clicks on floored tools park and are approved in the Approvals view until an authenticated human-origin channel lands in Phase 14 (FLOOR-04). Matches P12-D1, P12-D2, and P12-D7 locked decisions without creating spoofable wire-level bypasses.

---

## 2. Implemented Components

### A. Core Action-Only Safety Floor (`vox-orchestrator::oversight::floor`)
- Created `crates/vox-orchestrator/src/oversight/mod.rs` and registered `pub mod oversight;` in `crates/vox-orchestrator/src/lib.rs`.
- Implemented `crates/vox-orchestrator/src/oversight/floor.rs`:
  - `FloorCategory`: `HistoryRewrite`, `PolicyGrant`, `ExternalEffect`, `SecretsWrite` with `ALL` and `id()`.
  - `FloorInput`: `Tool` (name, args, repo_root) and `Task` (write_paths, tool_hints).
  - Pure action-only classifier `classify_floor(input)` taking no level, `PermissionMode`, `RiskPosture`, or trust score (FLOOR-01).
  - Precedence: `SecretsWrite` -> `PolicyGrant` -> `HistoryRewrite` -> `ExternalEffect`.
  - Canonical tool tables:
    - `HISTORY_REWRITE_TOOLS`: `vox_snapshot_restore`, `vox_undo`, `vox_workspace_merge`.
    - `POLICY_GRANT_TOOLS`: `vox_add_approval_allowlist_entry`, `vox_config_set`, `vox_trust_override`.
    - `EXTERNAL_EFFECT_TOOLS`: `vox_agent_gateway_call`, `vox_deploy`, `vox_mesh_dispatch`, `vox_openclaw_gateway_call`, `vox_plugin_install`, `vox_scientia_publication_publish`, `vox_scientia_publication_submit_local`, `vox_skill_add`, `vox_skill_install`.
    - `SHELL_COMMAND_TOOLS`: `vox_run_shell`, `vox_skill_run`.
    - `FILE_WRITE_TOOLS`: `vox_apply_structured_edit`, `vox_generate_code`.
  - Best-effort lexical escalators:
    - `command_rewrites_history`: tokenizes git and jj commands, detecting force-push, rewrite, rebase, and history modification.
    - `path_is_secrets` & `json_has_secret_key`: identifies `.env*`, key files, `.ssh`, AWS credentials, and secrets-shaped config keys.
    - `path_is_policy_file`: protects `Vox.toml`, `.vox/agents/*`, and permission mode contracts.
  - Pure mapping helper `floor_tier`: maps `Some(FloorCategory)` -> `ApprovalTier::Review`, `None` -> `ApprovalTier::AutoApprove`.

### B. Single Chokepoint Dispatch Hook (`vox-orchestrator-mcp::floor_gate`)
- Created `crates/vox-orchestrator-mcp/src/floor_gate.rs` and registered `mod floor_gate;` in `crates/vox-orchestrator-mcp/src/lib.rs`:
  - `GatePlan`: contains `floor`, `gated`, `mode_auto_approved`, `allowlist_eligible`.
  - `plan_gate`: enforces the canonical three guard lines:
    ```rust
    let gated = floor.is_some() || crate::permission_modes::is_gated_tool(name);
    let mode_auto_approved = floor.is_none() && crate::permission_modes::mode_auto_approves(mode, name);
    let allowlist_eligible = floor.is_none() && crate::permission_modes::allowlist_eligible(name);
    ```
  - `approval_summary`: value-free redaction for config keys and file paths (`summary_args`), never revealing sensitive values in persistent approval records.
- Integrated into `crates/vox-orchestrator-mcp/src/dispatch.rs`:
  - Dangerous tool gate updated to evaluate `plan_gate`.
  - Static dispatch probe skips floored tools via `vox_orchestrator::oversight::floor::name_is_floored(name)`, preventing 300 s timeouts.

### C. Test Updates & Documentation
- Updated `crates/vox-integration-tests/tests/skill_install_test.rs` and `skill_add_remove_test.rs` to handle floored tool calls by polling and resolving pending approvals.
- Added "Safety floor" lookup row in [`docs/src/architecture/where-things-live.md`](file:///Users/brbrainerd/dev/vox/docs/src/architecture/where-things-live.md).

---

## 3. Verification & Acceptance Proofs

1. **Safety Floor Unit Tests:**
   `cargo test -p vox-orchestrator --lib -- oversight::floor::safety_floor_tests`
   - 12 passed; 0 failed.
2. **Floor Gate & Dispatch Tests:**
   `cargo test -p vox-orchestrator-mcp --lib -- floor_gate:: registry_dispatch_tests permission_modes`
   - `floor_gate::tests`: 5 passed; 0 failed (including tracer test `tracer_config_set_parks_under_accept_all`).
   - `registry_dispatch_tests`: 13 passed; 0 failed.
3. **Existing Gate Compatibility:**
   `cargo test -p vox-orchestrator-mcp --test pending_approvals_tests`
   - 14 passed; 0 failed.
4. **Integration Tests:**
   `cargo test -p vox-integration-tests --test skill_deploy_test --test skill_add_remove_test`
   - 2 passed; 0 failed.
5. **Static Analysis & Linting:**
   - `cargo clippy --no-deps -p vox-orchestrator -p vox-orchestrator-mcp --all-targets -- -D warnings`: exited 0 with no warnings.
   - `cargo run -q -p vox-doc-pipeline -- --lint-only --paths architecture/where-things-live.md`: exited 0 with no hard errors.
   - `cargo run -q -p vox-arch-check`: exited 0 with 0 errors.
