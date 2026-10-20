---
phase: 12-safety-floor-lowering-paths
plan: 04
type: execute
status: completed
date: 2026-10-14
requirements: [FLOOR-01, FLOOR-02]
files_modified:
  - crates/vox-orchestrator/src/orchestrator/task_dispatch/submit/attention_fields.rs
  - crates/vox-orchestrator-mcp/tests/pending_approvals_floor_tests.rs
  - crates/vox-orchestrator-mcp/tests/autonomous_tool_dispatch_tests.rs
  - crates/vox-orchestrator-mcp/src/floor_gate.rs
---

# Phase 12 Plan 04 Summary: Call-Site Floor Tests, Background Lift, Autonomous Dispatch & Invariants

## 1. Plan Overview
Plan 12-04 proved the safety floor at every origin and extended it to background tasks:
- **Background Task Floor Lift (FLOOR-01, FLOOR-02):** Enqueued tasks whose write manifest hits secrets/policy or whose tool hints name floored actions are lifted to `ApprovalTier::Review` via `effective_tier.max_strictness(crate::oversight::floor::floor_tier(floor))`.
- **Call-Site Floor Proofs:** Created dedicated integration test suite `crates/vox-orchestrator-mcp/tests/pending_approvals_floor_tests.rs` covering all four categories (`PolicyGrant`, `HistoryRewrite`, `ExternalEffect`, `SecretsWrite`), force-push through `vox_run_shell`, allowlist bypass attempts, and forced 1.0 trust.
- **Autonomous Dispatch & Reserved Invariant:** Added `autonomous_dispatch_floor_parks` to prove autonomous bridge `McpToolDispatcher` parks floored calls, and established `RESERVED_GATED_NAMES` with bidirectional registry verification so gated names cannot silently become untracked phantoms.

---

## 2. Key Changes & Verifications

### Task 1: Tracer — Background Task Writing `.env` Lifted to Review
- **Implementation:**
  - In `crates/vox-orchestrator/src/orchestrator/task_dispatch/submit/attention_fields.rs`:
    - Updated `task_description_suggests_external` to strip `[[...]]` description hint blocks (`strip_description_hints`) before checking prose keywords.
    - Added write paths extraction from `file_manifest` and invoked `crate::oversight::floor::classify_floor(&crate::oversight::floor::FloorInput::task(&write_paths, &task.tool_hints))`.
    - Lifted tier with `let effective_tier = effective_tier.max_strictness(crate::oversight::floor::floor_tier(floor));`.
    - Added one-line doc note explaining that background floored actions are lifted to Review, with Phase 13 composing oversight level on top.
- **TDD Failing Assertion Before Lift:**
  - Line `crates/vox-orchestrator/src/orchestrator/task_dispatch/submit/attention_fields.rs:419:9` in `floor_secrets_manifest_write_lifts_task_to_review` panicked: `assertion left == right failed (left: Some(AutoApprove), right: Some(Review))` while baseline passed.
- **Passing Tests:**
  - `floor_secrets_manifest_write_lifts_task_to_review`
  - `floor_tool_hint_lifts_task_to_review`
  - `floor_unfloored_task_tier_unchanged`
  - All 15 tests in `attention_fields::tests` passed.

### Task 2: Call-Site Floor Tests for Every Category in `pending_approvals_floor_tests.rs`
- **Implementation:**
  - Created hermetic test binary `crates/vox-orchestrator-mcp/tests/pending_approvals_floor_tests.rs`.
  - Included `mod common;` and `IsolatedDbEnv` for env isolation.
  - Implemented `assert_parks` helper that dispatches through `handle_tool_call_with_mode`, polls pending approvals, resolves `Rejected`, and asserts error envelope.
  - Added 7 integration tests:
    1. `floor_policy_grant_tools_park_under_accept_all`
    2. `floor_force_push_parks_under_accept_all`
    3. `floor_history_rewrite_tools_park`
    4. `floor_external_effects_park_under_accept_all` (iterates all `tools_for(FloorCategory::ExternalEffect)`, >= 9 tools)
    5. `floor_secrets_path_write_parks`
    6. `floor_policy_file_write_parks`
    7. `floor_ignores_allowlist` (precondition verifies unfloored phantom call returns prompt error; assertion verifies force-push parks despite allowlist and 1.0 trust)
- **Verification:** All 7 integration tests passed in 3.29s.

### Task 3: Autonomous Dispatch & Registry-or-Reserved Invariant
- **Implementation:**
  - Added `autonomous_dispatch_floor_parks` in `crates/vox-orchestrator-mcp/tests/autonomous_tool_dispatch_tests.rs`.
  - Added `RESERVED_GATED_NAMES` in `crates/vox-orchestrator-mcp/src/floor_gate.rs`:
    `["vox_write_file", "vox_multi_replace", "vox_multi_replace_file", "vox_delete_file", "vox_run_shell", "vox_deploy"]`.
  - Added unit tests in `floor_gate.rs`:
    - `every_gated_tool_is_registered_or_reserved` (bidirectional: every gated tool is in `TOOL_REGISTRY` or `RESERVED_GATED_NAMES`, and no reserved tool is currently registered).
    - `floor_write_tools_match_scope_guard` (every `scope_guard::WRITE_TOOLS` is in `FILE_WRITE_TOOLS`).
- **Verification:**
  - `autonomous_dispatch_floor_parks` passed in 3.05s.
  - 7 unit tests in `floor_gate::tests` passed in 3.32s.
  - 6 unit tests in `scope_guard::write_gate_tests` passed in 3.09s.
  - `cargo clippy --no-deps -p vox-orchestrator -p vox-orchestrator-mcp --all-targets -- -D warnings` passed cleanly (exit code 0).

---

## 3. Threat Model & Residual Notes
- **T-12-01, T-12-02, T-12-03, T-12-04, T-12-13, T-12-18:** Fully mitigated and verified via call-site and invariant test suites.
- **T-12-23 (Residual Accepted):** Self-attestation on task-level Review is deferred to Phase 15 / BG-02b when the human approval inbox consumer is hooked up; individual tool actions remain strictly floored and gated regardless of self-attestation.
