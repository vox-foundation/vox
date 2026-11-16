---
phase: "12"
slug: "safety-floor-lowering-paths"
status: complete
nyquist_compliant: true
wave_0_complete: true
created: "2026-10-11"
---

# Phase 12 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution. Source: `12-RESEARCH.md` § Validation Architecture.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `#[test]` / `#[tokio::test]` + `serial_test` |
| **Config file** | none (existing harness: `crates/vox-orchestrator-mcp/tests/pending_approvals_floor_tests.rs`, `IsolatedDbEnv`) |
| **Quick run command** | `cargo test -p vox-orchestrator --lib -- oversight::floor::safety_floor_tests` |
| **Full suite command** | `cargo test -p vox-orchestrator-mcp --test pending_approvals_floor_tests --test pending_approvals_tests --test autonomous_tool_dispatch_tests --test daemon_extra_tests --test stdio_daemon_routing_tests --test floor_entry_points_tests && cargo test -p vox-orchestrator --lib -- oversight::floor::safety_floor_tests resolve_grounding_socrates_enforce_tests attention_fields::tests mode::strictest_risk_tests` |
| **Estimated runtime** | ~60 seconds per crate (after first build) |

---

## Sampling Rate

- **After every task commit:** `cargo test -p <touched crate> -- <filter>`
- **After every plan wave:** full suite command + `cargo clippy -p vox-orchestrator -p vox-orchestrator-mcp -- -D warnings`
- **Before `/gsd-verify-work`:** full suite + `cargo run -q -p vox-arch-check` + `vox ci crate-edges` + `vox ci pre-push`
- **Max feedback latency:** 120 seconds

---

## Per-Task Verification Map

Task IDs are assigned by the planner; rows map requirement → behavior → command.

| Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| FLOOR-01 | T-12-01 | Force-push argv via skill/shell parks under `accept_all` | call-site | `cargo test -p vox-orchestrator-mcp --test pending_approvals_floor_tests -- floor_force_push_parks_under_accept_all` | ✅ | ✅ green |
| FLOOR-01 | T-12-01 | `vox_snapshot_restore` / `vox_undo` / `vox_workspace_merge` park under `accept_all` | call-site | `cargo test -p vox-orchestrator-mcp --test pending_approvals_floor_tests -- floor_history_rewrite_tools_park` | ✅ | ✅ green |
| FLOOR-01 | T-12-02 | `vox_config_set`, `vox_trust_override`, allowlist grant park under `accept_all` | call-site | `cargo test -p vox-orchestrator-mcp --test pending_approvals_floor_tests -- floor_policy_grant_tools_park_under_accept_all` | ✅ | ✅ green |
| FLOOR-01 | T-12-02 | Policy file (`Vox.toml`, `.vox/agents/*`, `contracts/orchestration/*`) write parks under `accept_all` | call-site | `cargo test -p vox-orchestrator-mcp --test pending_approvals_floor_tests -- floor_policy_file_write_parks` | ✅ | ✅ green |
| FLOOR-01 | T-12-02 | Floored call ignores allowlist entry and max trust | call-site | `cargo test -p vox-orchestrator-mcp --test pending_approvals_floor_tests -- floor_ignores_allowlist` | ✅ | ✅ green |
| FLOOR-01 | T-12-01 | Autonomous dispatcher origin parks floored call | call-site | `cargo test -p vox-orchestrator-mcp --test autonomous_tool_dispatch_tests -- autonomous_dispatch_floor_parks` | ✅ | ✅ green |
| FLOOR-01 | T-12-04b | Approval summary for config/policy writes shows keys/paths only, never values | unit | `cargo test -p vox-orchestrator-mcp --lib -- floor_gate::tests::approval_summary_config_writes_show_keys_only` | ✅ | ✅ green |
| FLOOR-01 | — | Classifier table (name / argv / path); signature takes no level or trust | unit | `cargo test -p vox-orchestrator --lib -- oversight::floor::safety_floor_tests` | ✅ | ✅ green |
| FLOOR-01 | — | Every floored/gated tool name is registered or reserved | unit | `cargo test -p vox-orchestrator-mcp --lib -- floor_gate::tests::every_gated_tool_is_registered_or_reserved` | ✅ | ✅ green |
| FLOOR-01 | T-12-08 | Autonomous dispatcher (`McpToolDispatcher`) cannot approve/modify its own floored park | call-site | `cargo test -p vox-orchestrator-mcp --test autonomous_tool_dispatch_tests -- autonomous_dispatch_cannot_approve_floored_park` | ✅ | ✅ green |
| FLOOR-01 | T-12-08 | Chat agent loop (`run_agent_turn`) cannot approve/modify its own floored park | call-site | `cargo test -p vox-orchestrator-mcp --lib -- agent_loop::tests::agent_loop_cannot_approve_floored_park` | ✅ | ✅ green |
| FLOOR-01 | T-12-08 | GUI daemon path (`orch.tool_call` without `agent_origin`) can still approve a floored park | call-site | `cargo test -p vox-orchestrator-mcp --test daemon_extra_tests -- daemon_gui_path_can_approve_floored_park` | ✅ | ✅ green |
| FLOOR-01 | T-12-08 | Stdio MCP forwarder (`call_tool_via_daemon`) marks `agent_origin: true` and cannot approve a floored park | call-site | `cargo test -p vox-orchestrator-mcp --test stdio_daemon_routing_tests -- stdio_forwarded_resolve_cannot_approve_floored_park` | ✅ | ✅ green |
| FLOOR-01 | T-12-08 | HTTP gateway (`/v1/tools/call`, `/v1/mobile`) runs inside `with_agent_origin` and cannot approve a floored park | call-site | `cargo test -p vox-orchestrator-mcp --lib -- http_gateway::floor_origin_tests::gateway_cannot_approve_floored_park` | ✅ | ✅ green |
| FLOOR-01 | T-12-08 | Every non-test `handle_tool_call*` caller and `TOOL_CALL` client in the workspace is classified in `floor_entry_points_tests` | source-scan | `cargo test -p vox-orchestrator-mcp --test floor_entry_points_tests -- entry_point_callers_are_classified` | ✅ | ✅ green |
| FLOOR-02 | T-12-03 | `permission-modes.v2.yaml` ↔ Rust parity incl. `vox_deploy` always-ask and `floor_categories` parity | parity | `cargo test -p vox-orchestrator-mcp --test pending_approvals_tests -- risk_classes_yaml_matches_rust_table` | ✅ | ✅ green |
| FLOOR-02 | T-12-03 | `permission-modes.v2.yaml` `floor_categories` matches `oversight::floor` tables | parity | `cargo test -p vox-orchestrator-mcp --test pending_approvals_tests -- floor_categories_yaml_matches_rust_floor` | ✅ | ✅ green |
| FLOOR-02 | T-12-03 | External publish, plugin/skill install, mesh dispatch park under `accept_all` | call-site | `cargo test -p vox-orchestrator-mcp --test pending_approvals_floor_tests -- floor_external_effects_park_under_accept_all` | ✅ | ✅ green |
| FLOOR-02 | T-12-04 | Secrets-shaped key / `.env` / credential path write parks | call-site | `cargo test -p vox-orchestrator-mcp --test pending_approvals_floor_tests -- floor_secrets_path_write_parks` | ✅ | ✅ green |
| FLOOR-02 | T-12-04 | Background task with manifest write to `.env` gets floor tier (Review) without keyword | unit | `cargo test -p vox-orchestrator --lib -- attention_fields::tests::floor_secrets_manifest_write_lifts_task_to_review` | ✅ | ✅ green |
| FLOOR-03 | T-12-05 | `risk:"high"` hint cannot disable configured grounding/Socrates (`check_socrates_gate`) | unit (inverted) | `cargo test -p vox-orchestrator --lib -- resolve_grounding_socrates_enforce_tests::check_gate_hint_cannot_lower_category_policy` | ✅ | ✅ green |
| FLOOR-03 | T-12-05 | Lax `task_category` / `trigger_source` policy cannot lower global grounding/Socrates enforcement | unit | `cargo test -p vox-orchestrator --lib -- resolve_grounding_socrates_enforce_tests::check_gate_lax_category_cannot_lower_global_enforce` | ✅ | ✅ green |
| FLOOR-03 | T-12-05 | `risk:"high"` hint cannot lower category tier (`populate_task_attention_fields`) | unit (inverted) | `cargo test -p vox-orchestrator --lib -- attention_fields::tests::risk_hint_cannot_lower_category_tier` | ✅ | ✅ green |
| FLOOR-03 | T-12-06 | Trust relax cannot waive configured enforcement or bypass disallow | integration | `cargo test -p vox-orchestrator --lib -- resolve_grounding_socrates_enforce_tests::trust_relax_cannot_waive_configured_` | ✅ | ✅ green |
| FLOOR-03 | T-12-07 | `vox_preference_set` refuses policy keys; config unchanged; non-policy keys still write | call-site | `cargo test -p vox-orchestrator-mcp --lib -- memory_tools::handlers_preferences::tests::preference_set_refuses_oversight_keys` | ✅ | ✅ green |
| FLOOR-05 | all | Each guard (`M1`–`M34`): removal (`grep -c` 0) ⇒ mapped test fails; restore (`grep -c` 1) ⇒ passes | mutation protocol | `cargo test -p vox-orchestrator --lib -- oversight::floor::safety_floor_tests` (and M1–M34 table in `12-06-SUMMARY.md`) | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `crates/vox-orchestrator/src/oversight/floor.rs` — classifier table tests
- [x] `crates/vox-orchestrator-mcp/tests/pending_approvals_floor_tests.rs` — call-site floor tests
- [x] Inverted versions of the two lowering-asserting tests (`explicit_risk_posture_cannot_lower_category_policy`, `risk_hint_cannot_lower_category_tier`)
- [x] `every_gated_tool_is_registered_or_reserved` test
- [x] Newly floored tools added to `SKIP_DISPATCH_PROBE` via `oversight::floor::name_is_floored(name)`

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 120s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** complete
