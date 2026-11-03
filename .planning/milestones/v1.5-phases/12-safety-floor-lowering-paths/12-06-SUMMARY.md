---
phase: 12-safety-floor-lowering-paths
plan: 06
type: execute
completed_at: 2026-10-14T12:12:00Z
requirements: [FLOOR-05, FLOOR-01, FLOOR-02, FLOOR-03]
files_modified:
  - .planning/workstreams/autonomy-ux/phases/12-safety-floor-lowering-paths/12-VALIDATION.md
  - .planning/workstreams/unified/phases/12-safety-floor-lowering-paths/12-VALIDATION.md
---

# Phase 12 Plan 06 Summary: Safety-Floor Mutation Gate & Phase Verification (`FLOOR-05`)

**Execution Mode:** Autonomous (`/gsd-autonomous`)  
**Status:** Completed & Verified  

---

## 1. Pre-Mutation Presence & Working Tree Audit

Before running the mutation matrix across `vox-orchestrator`, `vox-orchestrator-mcp`, and `contracts/orchestration/permission-modes.v2.yaml`, all canonical guard lines were verified with `grep -cF` / `grep -cxF` and probe file absence checks (`test ! -e crates/vox-orchestrator-mcp/src/floor_scan_probe.rs && test ! -e crates/vox-cli/src/floor_scan_probe.rs`):

- `crates/vox-orchestrator-mcp/src/floor_gate.rs`:
  - `let gated = floor.is_some() || crate::permission_modes::is_gated_tool(name);` → count `1`
  - `let mode_auto_approved = floor.is_none() && crate::permission_modes::mode_auto_approves(mode, name);` → count `1`
  - `let allowlist_eligible = floor.is_none() && crate::permission_modes::allowlist_eligible(name);` → count `1`
  - `if !in_agent_origin() {` → count `1`
  - `let floored = info.summary.starts_with("[floor:") || vox_orchestrator::oversight::floor::name_is_floored(&info.tool);` → count `1`
  - `MUTATION-12-06` marker count → `0`
- `crates/vox-orchestrator-mcp/src/dispatch.rs`:
  - `if gate.gated {` → count `1`
  - `crate::floor_gate::refuse_agent_floor_resolution(state, approval_id, outcome)` → count `1`
  -*(Note: An earlier interrupted mutation run had left `dispatch.rs` reverted to pre-`12-01` gate wiring; `tracer_config_set_parks_under_accept_all`, `agent_loop_cannot_approve_floored_park`, and `gateway_cannot_approve_floored_park` immediately caught the missing hook and failed at `did not park approval within 15s` / `vox_skill_install should park`, providing a live end-to-end demonstration of `M4` and `M24` before restoration.)*
- `crates/vox-orchestrator/src/oversight/floor.rs`:
  - `"vox_config_set",`, `"vox_trust_override",`, `"vox_undo",`, `"vox_mesh_dispatch",` → exact-line count `1` each
  - `MUTATION-12-06` marker count → `0`
- `crates/vox-orchestrator/src/orchestrator/task_dispatch/submit/attention_fields.rs`:
  - `max_strictness(crate::oversight::floor::floor_tier(` → count `1`
  - `crate::mode::strictest_risk_for_task(task, overrides)` → count `1`
- `crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/success/socrates.rs`:
  - `configured_risk_for_task(task, overrides)` → count `1`
  - `self.configured || (self.hinted && !trust_relax_gates)` → count `1`
  - `(socrates_enforce || bypass_disallowed)` → count `1`
  - `global || configured.unwrap_or(false)` → count `1`
- `crates/vox-orchestrator-mcp/src/memory_tools/handlers_preferences.rs`:
  - `if is_oversight_preference_key(&params.key) {` → count `1`
- `crates/vox-orchestrator-mcp/src/autonomous_tool_dispatch.rs`, `agent_loop.rs`, `daemon_route.rs`, `daemon_extra.rs`, `http_gateway/rpc_tools.rs`:
  - All agent-origin scope wrappers and `"agent_origin": true` markers present with count `1`.

---

## 2. Mutation-Proof Table (`M1`–`M34`)

| ID | File | Mutation Applied | Presence (`Before→Mut→Restored`) | Mapped Test | Observed Failure Line | Restored Pass Result |
|---|---|---|---|---|---|---|
| M1 | `floor_gate.rs` | Drop `floor.is_some() \|\| ` in `let gated = ...` | `1 → 0 → 1` | `--test pending_approvals_floor_tests -- floor_policy_grant_tools_park_under_accept_all` | `pending_approvals_floor_tests.rs:94: vox_config_set did not park under accept_all` | `1 passed; 0 failed` |
| M2 | `floor_gate.rs` | Drop `floor.is_none() && ` in `let mode_auto_approved = ...` | `1 → 0 → 1` | `--test pending_approvals_floor_tests -- floor_force_push_parks_under_accept_all` | `pending_approvals_floor_tests.rs:48: vox_run_shell force-push did not park under accept_all` | `1 passed; 0 failed` |
| M3 | `floor_gate.rs` | Drop `floor.is_none() && ` in `let allowlist_eligible = ...` | `1 → 0 → 1` | `--test pending_approvals_floor_tests -- floor_ignores_allowlist` | `pending_approvals_floor_tests.rs:198: allowlisted floored tool did not park` | `1 passed; 0 failed` |
| M4 | `dispatch.rs` | Replace `if gate.gated {` with `if crate::permission_modes::is_gated_tool(name_canonical) {` | `1 → 0 → 1` | `--test pending_approvals_floor_tests -- floor_policy_grant_tools_park_under_accept_all` (and `floor_gate::tests::tracer_config_set_parks_under_accept_all`) | `floor_gate.rs:251:31: did not park approval within 15s` / `pending_approvals_floor_tests.rs:94` | `1 passed; 0 failed` |
| M5 | `oversight/floor.rs` | Delete `"vox_config_set",` from `POLICY_GRANT_TOOLS` | `1 → 0 → 1` | `--test pending_approvals_floor_tests -- floor_policy_grant_tools_park_under_accept_all` | `pending_approvals_floor_tests.rs:94: vox_config_set did not park under accept_all` | `1 passed; 0 failed` |
| M6 | `oversight/floor.rs` | Delete `"vox_trust_override",` from `POLICY_GRANT_TOOLS` | `1 → 0 → 1` | `--test pending_approvals_floor_tests -- floor_policy_grant_tools_park_under_accept_all` | `pending_approvals_floor_tests.rs:102: vox_trust_override did not park under accept_all` | `1 passed; 0 failed` |
| M7 | `oversight/floor.rs` | Delete `"vox_undo",` from `HISTORY_REWRITE_TOOLS` | `1 → 0 → 1` | `--test pending_approvals_floor_tests -- floor_history_rewrite_tools_park` | `pending_approvals_floor_tests.rs:71: vox_undo did not park under accept_all` | `1 passed; 0 failed` |
| M8 | `oversight/floor.rs` | Delete `"vox_mesh_dispatch",` from `EXTERNAL_EFFECT_TOOLS` | `1 → 0 → 1` | `--test pending_approvals_floor_tests -- floor_external_effects_park_under_accept_all` | `pending_approvals_floor_tests.rs:146: vox_mesh_dispatch did not park under accept_all` | `1 passed; 0 failed` |
| M9 | `oversight/floor.rs` | Insert `return false; // MUTATION-12-06` at start of `fn path_is_secrets` | `0 → 1 → 0` (marker) | `--test pending_approvals_floor_tests -- floor_secrets_path_write_parks` | `pending_approvals_floor_tests.rs:169: .env write did not park` | `1 passed; 0 failed` |
| M10 | `oversight/floor.rs` | Insert `return false; // MUTATION-12-06` at start of `fn json_has_secret_key` | `0 → 1 → 0` (marker) | `--test pending_approvals_floor_tests -- floor_secrets_path_write_parks` | `pending_approvals_floor_tests.rs:182: assertion failed: summary.starts_with("[floor:secrets_write]")` | `1 passed; 0 failed` |
| M11 | `oversight/floor.rs` | Insert `return false; // MUTATION-12-06` at start of `fn command_rewrites_history` | `0 → 1 → 0` (marker) | `--test pending_approvals_floor_tests -- floor_force_push_parks_under_accept_all` | `pending_approvals_floor_tests.rs:48: git push --force did not park` | `1 passed; 0 failed` |
| M12 | `attention_fields.rs` | Replace `effective_tier.max_strictness(crate::oversight::floor::floor_tier(...))` with `effective_tier` | `1 → 0 → 1` | `--lib -- attention_fields::tests::floor_secrets_manifest_write_lifts_task_to_review` | `attention_fields.rs:285: assertion left == right failed: left: Confirm, right: Review` | `1 passed; 0 failed` |
| M13 | `socrates.rs` | Replace `configured_risk_for_task(task, overrides)` with `effective_risk_for_task(task, overrides)` | `1 → 0 → 1` | `--lib -- check_gate_hint_cannot_lower_category_policy` | `socrates.rs:604: expected GateOutcome::Requeued when hint attempts to lower category policy` | `1 passed; 0 failed` |
| M14 | `attention_fields.rs` | Replace `crate::mode::strictest_risk_for_task(task, overrides)` with `crate::mode::effective_risk_for_task(task, overrides)` | `1 → 0 → 1` | `--lib -- attention_fields::tests::risk_hint_cannot_lower_category_tier` | `attention_fields.rs:248: assertion left == right failed: left: AutoApprove, right: Review` | `1 passed; 0 failed` |
| M15 | `socrates.rs` | Replace `self.configured \|\| (self.hinted && !trust_relax_gates)` with `(self.configured \|\| self.hinted) && !trust_relax_gates` | `1 → 0 → 1` | `--lib -- trust_relax_cannot_waive_configured_grounding` | `socrates.rs:648: trust_relax_gates must not waive configured grounding enforcement` | `1 passed; 0 failed` |
| M16 | `socrates.rs` | Replace `(socrates_enforce \|\| bypass_disallowed)` with `(socrates_enforce \|\| (bypass_disallowed && !trust_relax_gates))` | `1 → 0 → 1` | `--lib -- trust_relax_cannot_waive_configured_bypass_disallowed` | `socrates.rs:692: trust_relax_gates must not waive high-risk bypass_disallowed` | `1 passed; 0 failed` |
| M17 | `handlers_preferences.rs` | Replace `if is_oversight_preference_key(&params.key) {` with `if false {` | `1 → 0 → 1` | `--lib -- memory_tools::handlers_preferences::tests::preference_set_refuses_oversight_keys` | `handlers_preferences.rs:146: expected OVERSIGHT_PREF_REFUSED for key socrates_gate_enforced` | `1 passed; 0 failed` |
| M18 | `permission-modes.v2.yaml` | Set `vox_deploy` `always_requires_approval: false` in YAML only | `1 → 0 → 1` | `--test pending_approvals_tests -- risk_classes_yaml_matches_rust_table` | `pending_approvals_tests.rs:412: always_requires_approval mismatch for vox_deploy` | `1 passed; 0 failed` |
| M19 | `permission_modes.rs` | Set `vox_deploy` `always_requires_approval: false` in Rust only | `1 → 0 → 1` | `--test pending_approvals_tests -- risk_classes_yaml_matches_rust_table` | `pending_approvals_tests.rs:412: always_requires_approval mismatch for vox_deploy` | `1 passed; 0 failed` |
| M20 | `permission-modes.v2.yaml` | Delete `vox_mesh_dispatch` under `floor_categories.external_effect.tools` | `1 → 0 → 1` | `--test pending_approvals_tests -- floor_categories_yaml_matches_rust_floor` | `pending_approvals_tests.rs:458: floor_categories.external_effect tools mismatch` | `1 passed; 0 failed` |
| M21 | `socrates.rs` | Replace `global \|\| configured.unwrap_or(false)` with `configured.unwrap_or(global)` | `1 → 0 → 1` | `--lib -- check_gate_lax_category_cannot_lower_global_enforce` | `socrates.rs:626: lax category policy must not disable global socrates_gate_enforce` | `1 passed; 0 failed` |
| M22 | `oversight/floor.rs` | Insert `return false; // MUTATION-12-06` at start of `fn path_is_policy_file` | `0 → 1 → 0` (marker) | `--test pending_approvals_floor_tests -- floor_policy_file_write_parks` | `pending_approvals_floor_tests.rs:121: Vox.toml write did not park` | `1 passed; 0 failed` |
| M23 | `floor_gate.rs` | Insert `return args.to_string().chars().take(200).collect(); // MUTATION-12-06` at start of `fn summary_args` | `0 → 1 → 0` (marker) | `--test pending_approvals_floor_tests -- floor_secrets_path_write_parks` (and `floor_gate::tests::approval_summary_redacts_secret_values`) | `floor_gate.rs:207: assertion failed: !summary.contains("FLOOR_TEST_SECRET_VALUE")` | `1 passed; 0 failed` |
| M24 | `dispatch.rs` | Replace `crate::floor_gate::refuse_agent_floor_resolution(state, approval_id, outcome)` with `None::<String>` | `1 → 0 → 1` | (1) `--test autonomous_tool_dispatch_tests -- autonomous_dispatch_cannot_approve_floored_park` and (2) `--lib -- agent_loop::tests::agent_loop_cannot_approve_floored_park` | (1) `autonomous_tool_dispatch_tests.rs:142: expected FLOOR_SELF_APPROVAL_REFUSED` and (2) `agent_loop.rs:2468: expected FLOOR_SELF_APPROVAL_REFUSED in tool response` | Both `1 passed; 0 failed` |
| M25 | `floor_gate.rs` | Replace `if !in_agent_origin() {` with `if false {` | `1 → 0 → 1` | `--test daemon_extra_tests -- daemon_gui_path_can_approve_floored_park` | `daemon_extra_tests.rs:318: GUI resolve failed with FLOOR_SELF_APPROVAL_REFUSED` | `1 passed; 0 failed` |
| M26 | `autonomous_tool_dispatch.rs` | Unwrap `crate::floor_gate::with_agent_origin(...)` around `handle_tool_call_with_mode` | `1 → 0 → 1` | `--test autonomous_tool_dispatch_tests -- autonomous_dispatch_cannot_approve_floored_park` | `autonomous_tool_dispatch_tests.rs:142: expected FLOOR_SELF_APPROVAL_REFUSED` | `1 passed; 0 failed` |
| M27 | `agent_loop.rs` | Unwrap `crate::floor_gate::with_agent_origin(...)` around `handle_tool_call_with_receipt` | `1 → 0 → 1` | `--lib -- agent_loop::tests::agent_loop_cannot_approve_floored_park` | `agent_loop.rs:2468: expected FLOOR_SELF_APPROVAL_REFUSED` | `1 passed; 0 failed` |
| M28 | `floor_gate.rs` | Drop ` \|\| vox_orchestrator::oversight::floor::name_is_floored(&info.tool)` | `1 → 0 → 1` | `--lib -- floor_gate::tests::refuse_agent_floor_resolution_matrix` | `floor_gate.rs:334: recovered-on-restart floored tool must still be refused` | `1 passed; 0 failed` |
| M29 | `floor_gate.rs` | Drop `info.summary.starts_with("[floor:") \|\| ` | `1 → 0 → 1` | `--lib -- floor_gate::tests::refuse_agent_floor_resolution_matrix` | `floor_gate.rs:349: arg-floored vox_skill_run must be refused via [floor: prefix` | `1 passed; 0 failed` |
| M30 | `daemon_route.rs` | Replace `"agent_origin": true` with `"agent_origin": false` | `1 → 0 → 1` | `--test stdio_daemon_routing_tests -- stdio_forwarded_resolve_cannot_approve_floored_park` | `stdio_daemon_routing_tests.rs:88: expected FLOOR_SELF_APPROVAL_REFUSED` | `1 passed; 0 failed` |
| M31 | `daemon_extra.rs` | Replace `req.params.get("agent_origin")...` RHS with `false` | `1 → 0 → 1` | `--test stdio_daemon_routing_tests -- stdio_forwarded_resolve_cannot_approve_floored_park` | `stdio_daemon_routing_tests.rs:88: expected FLOOR_SELF_APPROVAL_REFUSED` | `1 passed; 0 failed` |
| M32 | `http_gateway/rpc_tools.rs` | Unwrap `crate::floor_gate::with_agent_origin(handle_tool_call(state, name, args))` | `1 → 0 → 1` | `--lib -- http_gateway::floor_origin_tests::gateway_cannot_approve_floored_park` | `http_gateway/floor_origin_tests.rs:144: expected FLOOR_SELF_APPROVAL_REFUSED on /v1/tools/call` | `1 passed; 0 failed` |
| M33 | `crates/vox-orchestrator-mcp/src/floor_scan_probe.rs` | Create untracked probe calling `crate::dispatch::handle_tool_call(state, name, args); // MUTATION-12-06` | `0 → 1 → 0` (file count) | `--test floor_entry_points_tests -- entry_point_callers_are_classified` | `floor_entry_points_tests.rs:92: unclassified handle_tool_call caller in floor_scan_probe.rs` | `1 passed; 0 failed` |
| M34 | `crates/vox-cli/src/floor_scan_probe.rs` | Create untracked probe referencing `orch_daemon_method::TOOL_CALL; // MUTATION-12-06` | `0 → 1 → 0` (file count) | `--test floor_entry_points_tests -- entry_point_callers_are_classified` | `floor_entry_points_tests.rs:138: unclassified TOOL_CALL client in floor_scan_probe.rs` | `1 passed; 0 failed` |

Post-task check:
- `grep -c 'MUTATION-12-06' crates/vox-orchestrator/src/oversight/floor.rs crates/vox-orchestrator-mcp/src/floor_gate.rs` → `0` for both files.
- `test ! -e crates/vox-orchestrator-mcp/src/floor_scan_probe.rs && test ! -e crates/vox-cli/src/floor_scan_probe.rs` → `0` (neither probe exists).

---

## 3. Validation Coverage Table (`12-VALIDATION.md` Parity)

Every row in `12-VALIDATION.md` maps to an automated test binary and single-filter command (zero manual-only rows):

| Validation Row | Requirement | Threat Ref | Test Function | Automated Command | Result |
|---|---|---|---|---|---|
| 1 | FLOOR-01 | T-12-01 | `floor_force_push_parks_under_accept_all` | `cargo test -p vox-orchestrator-mcp --test pending_approvals_floor_tests -- floor_force_push_parks_under_accept_all` | ✅ pass |
| 2 | FLOOR-01 | T-12-01 | `floor_history_rewrite_tools_park` | `cargo test -p vox-orchestrator-mcp --test pending_approvals_floor_tests -- floor_history_rewrite_tools_park` | ✅ pass |
| 3 | FLOOR-01 | T-12-02 | `floor_policy_grant_tools_park_under_accept_all` | `cargo test -p vox-orchestrator-mcp --test pending_approvals_floor_tests -- floor_policy_grant_tools_park_under_accept_all` | ✅ pass |
| 4 | FLOOR-01 | T-12-02 | `floor_policy_file_write_parks` | `cargo test -p vox-orchestrator-mcp --test pending_approvals_floor_tests -- floor_policy_file_write_parks` | ✅ pass |
| 5 | FLOOR-01 | T-12-02 | `floor_ignores_allowlist` | `cargo test -p vox-orchestrator-mcp --test pending_approvals_floor_tests -- floor_ignores_allowlist` | ✅ pass |
| 6 | FLOOR-01 | T-12-01 | `autonomous_dispatch_floor_parks` | `cargo test -p vox-orchestrator-mcp --test autonomous_tool_dispatch_tests -- autonomous_dispatch_floor_parks` | ✅ pass |
| 7 | FLOOR-01 | T-12-04b | `approval_summary_config_writes_show_keys_only` | `cargo test -p vox-orchestrator-mcp --lib -- floor_gate::tests::approval_summary_config_writes_show_keys_only` | ✅ pass |
| 8 | FLOOR-01 | — | `safety_floor_tests::*` (11 unit tests) | `cargo test -p vox-orchestrator --lib -- oversight::floor::safety_floor_tests` | ✅ pass |
| 9 | FLOOR-01 | — | `every_gated_tool_is_registered_or_reserved` | `cargo test -p vox-orchestrator-mcp --lib -- floor_gate::tests::every_gated_tool_is_registered_or_reserved` | ✅ pass |
| 10 | FLOOR-01 | T-12-08 | `autonomous_dispatch_cannot_approve_floored_park` | `cargo test -p vox-orchestrator-mcp --test autonomous_tool_dispatch_tests -- autonomous_dispatch_cannot_approve_floored_park` | ✅ pass |
| 11 | FLOOR-01 | T-12-08 | `agent_loop_cannot_approve_floored_park` | `cargo test -p vox-orchestrator-mcp --lib -- agent_loop::tests::agent_loop_cannot_approve_floored_park` | ✅ pass |
| 12 | FLOOR-01 | T-12-08 | `daemon_gui_path_can_approve_floored_park` | `cargo test -p vox-orchestrator-mcp --test daemon_extra_tests -- daemon_gui_path_can_approve_floored_park` | ✅ pass |
| 13 | FLOOR-01 | T-12-08 | `stdio_forwarded_resolve_cannot_approve_floored_park` | `cargo test -p vox-orchestrator-mcp --test stdio_daemon_routing_tests -- stdio_forwarded_resolve_cannot_approve_floored_park` | ✅ pass |
| 14 | FLOOR-01 | T-12-08 | `gateway_cannot_approve_floored_park` | `cargo test -p vox-orchestrator-mcp --lib -- http_gateway::floor_origin_tests::gateway_cannot_approve_floored_park` | ✅ pass |
| 15 | FLOOR-01 | T-12-08 | `entry_point_callers_are_classified` | `cargo test -p vox-orchestrator-mcp --test floor_entry_points_tests -- entry_point_callers_are_classified` | ✅ pass |
| 16 | FLOOR-02 | T-12-03 | `risk_classes_yaml_matches_rust_table` | `cargo test -p vox-orchestrator-mcp --test pending_approvals_tests -- risk_classes_yaml_matches_rust_table` | ✅ pass |
| 17 | FLOOR-02 | T-12-03 | `floor_categories_yaml_matches_rust_floor` | `cargo test -p vox-orchestrator-mcp --test pending_approvals_tests -- floor_categories_yaml_matches_rust_floor` | ✅ pass |
| 18 | FLOOR-02 | T-12-03 | `floor_external_effects_park_under_accept_all` | `cargo test -p vox-orchestrator-mcp --test pending_approvals_floor_tests -- floor_external_effects_park_under_accept_all` | ✅ pass |
| 19 | FLOOR-02 | T-12-04 | `floor_secrets_path_write_parks` | `cargo test -p vox-orchestrator-mcp --test pending_approvals_floor_tests -- floor_secrets_path_write_parks` | ✅ pass |
| 20 | FLOOR-02 | T-12-04 | `floor_secrets_manifest_write_lifts_task_to_review` | `cargo test -p vox-orchestrator --lib -- attention_fields::tests::floor_secrets_manifest_write_lifts_task_to_review` | ✅ pass |
| 21 | FLOOR-03 | T-12-05 | `check_gate_hint_cannot_lower_category_policy` | `cargo test -p vox-orchestrator --lib -- resolve_grounding_socrates_enforce_tests::check_gate_hint_cannot_lower_category_policy` | ✅ pass |
| 22 | FLOOR-03 | T-12-05 | `check_gate_lax_category_cannot_lower_global_enforce` | `cargo test -p vox-orchestrator --lib -- resolve_grounding_socrates_enforce_tests::check_gate_lax_category_cannot_lower_global_enforce` | ✅ pass |
| 23 | FLOOR-03 | T-12-05 | `risk_hint_cannot_lower_category_tier` | `cargo test -p vox-orchestrator --lib -- attention_fields::tests::risk_hint_cannot_lower_category_tier` | ✅ pass |
| 24 | FLOOR-03 | T-12-06 | `trust_relax_cannot_waive_configured_*` (3 tests) | `cargo test -p vox-orchestrator --lib -- resolve_grounding_socrates_enforce_tests::trust_relax_cannot_waive_configured_` | ✅ pass |
| 25 | FLOOR-03 | T-12-07 | `preference_set_refuses_oversight_keys` | `cargo test -p vox-orchestrator-mcp --lib -- memory_tools::handlers_preferences::tests::preference_set_refuses_oversight_keys` | ✅ pass |
| 26 | FLOOR-05 | all | `M1`–`M34` mutation matrix + classifier suite | `cargo test -p vox-orchestrator --lib -- oversight::floor::safety_floor_tests` | ✅ pass |

---

## 4. Recorded Deferrals and Accepted Residuals

1. **`T-12-25` (Raw daemon-socket / token-holding process spoofing human origin):** A process with direct access to the local `vox-orchestrator-d` TCP socket or bearer token (including local shell routes) can send a raw `orch.tool_call` without `agent_origin: true` or call `orch.resolve_approval` directly. Vox's own in-process agent loops (`McpToolDispatcher`, `run_agent_turn`), the stdio `vox mcp` forwarder (`call_tool_via_daemon`), and the HTTP gateway (`/v1/tools/call`, `/v1/mobile`) are all marked agent-origin by construction (`12-07` / `12-08`). Cryptographic / out-of-band human origin authentication is deferred to **`FLOOR-04b` / Phase 14 Success Criterion 5**.
2. **`T-12-23` (Floor-lifted background `Review` self-attestation):** A background task lifted to `ApprovalTier::Review` by `floor_tier` (`12-04`) is protected at submit time, but preventing an autonomous worker from self-attesting a `Review`-tier completion is owned by **`BG-02b` / Phase 15 Success Criterion 5**.
3. **GUI comments still citing the `v1` contract path (`contracts/orchestration/permission-modes.v1.yaml`):** Cosmetic comment cleanup in GUI TypeScript files is deferred to **Phase 18**.
4. **Stricter agent-loop check against the offered tool set:** Recommended defense-in-depth (filtering `vox_resolve_approval` out of the chat agent tool catalog in addition to the `with_agent_origin` refusal), optional and not required for Phase 12.

---

## 5. Phase Gate Output Summary

- `vox-orchestrator` unit tests (`resolve_grounding_socrates_enforce_tests`, `attention_fields::tests`, `oversight::floor::safety_floor_tests`): `40 passed; 0 failed; finished in 0.01s`.
- `vox-orchestrator-mcp` unit & floor origin tests (`floor_gate::`, `memory_tools::handlers_preferences::tests`, `agent_loop::tests::agent_loop_cannot_approve_floored_park`, `http_gateway::floor_origin_tests::gateway_cannot_approve_floored_park`, `mermaid_repair::tests`): all passed.
- `12-VALIDATION.md` checks:
  - `grep -c 'pending_approvals_floor_tests' 12-VALIDATION.md` = `7` (≥ 6)
  - `grep -c 'pending_approvals_tests floor_' 12-VALIDATION.md` = `0`
  - `grep -c '^nyquist_compliant: true' 12-VALIDATION.md` = `1`
