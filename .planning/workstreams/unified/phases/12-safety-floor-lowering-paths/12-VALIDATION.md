---
phase: "12"
slug: "safety-floor-lowering-paths"
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-11"
---

# Phase 12 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution. Source: `12-RESEARCH.md` § Validation Architecture.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `#[test]` / `#[tokio::test]` + `serial_test` |
| **Config file** | none (existing harness: `crates/vox-orchestrator-mcp/tests/pending_approvals_tests.rs`, `IsolatedDbEnv`) |
| **Quick run command** | `cargo test -p vox-orchestrator safety_floor` |
| **Full suite command** | `cargo test -p vox-orchestrator-mcp --test pending_approvals_tests && cargo test -p vox-orchestrator -- safety_floor socrates attention_fields` |
| **Estimated runtime** | ~60 seconds per crate (after first build) |

---

## Sampling Rate

- **After every task commit:** `cargo test -p <touched crate> <filter>`
- **After every plan wave:** full suite command + `cargo clippy -p vox-orchestrator -p vox-orchestrator-mcp -- -D warnings`
- **Before `/gsd-verify-work`:** full suite + `cargo run -q -p vox-arch-check` + `vox ci crate-edges` + `vox ci pre-push`
- **Max feedback latency:** 120 seconds

---

## Per-Task Verification Map

Task IDs are assigned by the planner; rows map requirement → behavior → command.

| Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| FLOOR-01 | T-12-01 | Force-push argv via skill/shell parks under `accept_all` | call-site | `cargo test -p vox-orchestrator-mcp --test pending_approvals_tests floor_force_push_parks_under_accept_all` | ❌ W0 | ⬜ pending |
| FLOOR-01 | T-12-01 | `vox_snapshot_restore` / `vox_undo` / `vox_workspace_merge` park under `accept_all` | call-site | `… pending_approvals_tests floor_history_rewrite_tools_park` | ❌ W0 | ⬜ pending |
| FLOOR-01 | T-12-02 | `vox_config_set`, `vox_trust_override`, allowlist grant park under `accept_all` | call-site | `… pending_approvals_tests floor_policy_grant_tools_park_under_accept_all` | ❌ W0 | ⬜ pending |
| FLOOR-01 | T-12-02 | Floored call ignores allowlist entry and max trust | call-site | `… pending_approvals_tests floor_ignores_allowlist` | ❌ W0 | ⬜ pending |
| FLOOR-01 | T-12-01 | Autonomous dispatcher origin parks floored call | call-site | `cargo test -p vox-orchestrator-mcp autonomous_dispatch_floor_parks` | ❌ W0 | ⬜ pending |
| FLOOR-01 | — | Classifier table (name / argv / path); signature takes no level or trust | unit | `cargo test -p vox-orchestrator safety_floor` | ❌ W0 | ⬜ pending |
| FLOOR-01 | — | Every floored/gated tool name is registered or reserved | unit | `cargo test -p vox-orchestrator-mcp every_gated_tool_is_registered_or_reserved` | ❌ W0 | ⬜ pending |
| FLOOR-02 | T-12-03 | `permission-modes.v2.yaml` ↔ Rust parity incl. `vox_deploy` always-ask; drift fails | parity | `… pending_approvals_tests risk_classes_yaml_matches_rust_table` | ✅ (update) | ⬜ pending |
| FLOOR-02 | T-12-03 | External publish, plugin/skill install, mesh dispatch park under `accept_all` | call-site | `… pending_approvals_tests floor_external_effects_park_under_accept_all` | ❌ W0 | ⬜ pending |
| FLOOR-02 | T-12-04 | Secrets-shaped key / `.env` / credential path write parks | call-site | `… pending_approvals_tests floor_secrets_path_write_parks` | ❌ W0 | ⬜ pending |
| FLOOR-02 | T-12-04 | Background task with manifest write to `.env` gets floor tier (Review) without keyword | unit | `cargo test -p vox-orchestrator attention_fields floor_` | ❌ W0 | ⬜ pending |
| FLOOR-03 | T-12-05 | `risk:"high"` hint cannot disable configured grounding/Socrates | unit (inverted) | `cargo test -p vox-orchestrator explicit_risk_posture_cannot_lower` | ⚠ invert existing | ⬜ pending |
| FLOOR-03 | T-12-05 | `risk:"high"` hint cannot lower category tier | unit (inverted) | `cargo test -p vox-orchestrator attention_fields risk_hint_cannot_lower_category` | ⚠ invert existing | ⬜ pending |
| FLOOR-03 | T-12-06 | Trust relax cannot waive configured enforcement | integration | `cargo test -p vox-orchestrator trust_relax_cannot_waive_configured` | ❌ W0 | ⬜ pending |
| FLOOR-03 | T-12-07 | `vox_preference_set` refuses policy keys; config unchanged; non-policy keys still write | call-site | `cargo test -p vox-orchestrator-mcp preference_set_refuses_oversight_keys` | ❌ W0 | ⬜ pending |
| FLOOR-05 | all | Each guard: removal (grep -c 0) ⇒ mapped test fails; restore (grep -c 1) ⇒ passes | mutation protocol | per-guard, recorded in SUMMARY | — | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/vox-orchestrator/src/oversight/floor.rs` (or `safety_floor` tests module) — failing classifier table tests
- [ ] `crates/vox-orchestrator-mcp/tests/pending_approvals_tests.rs` — failing call-site tests above
- [ ] Inverted versions of the two lowering-asserting tests
- [ ] `every_gated_tool_is_registered_or_reserved` test
- [ ] Newly floored tools added to `SKIP_DISPATCH_PROBE`

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Mutation proof per guard | FLOOR-05 | Requires deliberate source edits | grep -c guard → 1; neutralize; grep -c → 0; run mapped test → must fail; restore; grep -c → 1; rerun → pass. Record failing assertion in SUMMARY. |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
