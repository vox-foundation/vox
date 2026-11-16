# Phase 12 Plan 02 Summary: Closing In-Band Lowering Paths for Task Oversight (FLOOR-03)

**Execution Mode:** Autonomous (`/gsd-autonomous`)  
**Status:** Completed & Verified  

---

## 1. Summary of Changes

Closed the two in-band lowering paths where tasks or agents could self-lower their safety posture:
1. **Strictest-Wins Precedence in `mode.rs`:**
   - Implemented `RiskPosture::strictness_rank` (High=0, Moderate=1, Low=2) and `RiskPosture::stricter`.
   - Added `configured_risk_for_task` (category policy > source policy, ignoring task hint).
   - Added `strictest_risk_for_task` (combines hint and configured policy with `stricter`).
   - Re-implemented `effective_risk_for_task` on top of `configured_risk_for_task` and documented that oversight consumers MUST use `strictest_risk_for_task` or `configured_risk_for_task`.
2. **Grounding & Socrates Resolution in `socrates.rs`:**
   - Introduced `Enforce { configured: bool, hinted: bool }` with `required(trust_relax_gates) -> bool { self.configured || (self.hinted && !trust_relax_gates) }`.
   - Updated `resolve_grounding_socrates_enforce` to return `(Enforce, Enforce)` where `configured` is computed via `configured_flag(global, configured) = global || configured.unwrap_or(false)`.
   - An agent-assigned `task_category` or `trigger_source` can never lower enforcement below global flags.
   - Narrows `trust_relax_gates` so it waives ONLY enforcement that the task's own hint added; it never waives configured/global enforcement, nor the high-risk bypass block.
   - Updated `check_socrates_gate` to compute `grounding_enforce = grounding.required(trust_relax_gates)` and `socrates_enforce = socrates.required(trust_relax_gates)`, removing `&& !trust_relax_gates` from both requeue arms.
3. **Approval Tier Strictest Precedence in `attention_fields.rs`:**
   - Updated `effective_approval_tier` to take `crate::mode::strictest_risk_for_task(task, overrides)` instead of `effective_risk_for_task`.
   - An explicit `task.risk_posture` hint can only escalate the approval tier (e.g. to Review), never lower it below the configured category/source policy or the trust-classified tier.
4. **Inverted and New Tests:**
   - Inverted and renamed `explicit_risk_posture_still_wins_over_category_policy` to `explicit_risk_posture_cannot_lower_category_policy`.
   - Inverted and renamed `explicit_risk_posture_wins_over_category_policy` to `risk_hint_cannot_lower_category_tier` (driven through `populate_task_attention_fields`).
   - Added `strictest_risk_tests` matrix in `mode.rs`.
   - Added `explicit_risk_posture_cannot_lower_global_enforce`, `explicit_risk_posture_can_raise_enforcement`, and `lax_category_policy_cannot_lower_global_enforce` in `socrates.rs`.
   - Added real call-site tests driving `check_socrates_gate`: `check_gate_hint_cannot_lower_category_policy`, `check_gate_lax_category_cannot_lower_global_enforce`, `trust_relax_cannot_waive_configured_grounding`, `trust_relax_cannot_waive_configured_socrates`, `trust_relax_cannot_waive_configured_bypass_disallowed`, and `trust_relax_still_waives_hint_only_enforcement`.
   - Added `risk_hint_can_raise_tier` in `attention_fields.rs`.

---

## 2. Acceptance Criteria Verification

- `grep -cF 'self.configured || (self.hinted && !trust_relax_gates)' crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/success/socrates.rs` == 1 (verified)
- `grep -cF 'configured_risk_for_task(task, overrides)' crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/success/socrates.rs` == 1 (verified)
- `grep -c 'fn explicit_risk_posture_cannot_lower_category_policy' crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/success/socrates.rs` == 1 (verified)
- `grep -cF 'global || configured.unwrap_or(false)' crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/success/socrates.rs` == 1 (verified)
- `grep -cE 'unwrap_or\(global|map_or\(global' crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/success/socrates.rs` == 0 (verified)
- `grep -c 'fn check_gate_hint_cannot_lower_category_policy\|fn check_gate_lax_category_cannot_lower_global_enforce\|fn lax_category_policy_cannot_lower_global_enforce' crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/success/socrates.rs` >= 3 (found 3, verified)
- `grep -c 'pub fn strictest_risk_for_task' crates/vox-orchestrator/src/mode.rs` == 1 (verified)
- `grep -c 'pub fn configured_risk_for_task' crates/vox-orchestrator/src/mode.rs` == 1 (verified)
- `grep -c '!trust_relax_gates' crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/success/socrates.rs` == 1 (verified)
- `grep -cF '(socrates_enforce || bypass_disallowed)' crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/success/socrates.rs` == 1 (verified)
- `grep -c 'fn trust_relax_cannot_waive_configured_' crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/success/socrates.rs` >= 3 (found 3, verified)
- `grep -cF 'crate::mode::strictest_risk_for_task(task, overrides)' crates/vox-orchestrator/src/orchestrator/task_dispatch/submit/attention_fields.rs` == 1 (verified)
- `grep -c 'fn risk_hint_cannot_lower_category_tier' crates/vox-orchestrator/src/orchestrator/task_dispatch/submit/attention_fields.rs` == 1 (verified)
- `grep -c 'populate_task_attention_fields(&' crates/vox-orchestrator/src/orchestrator/task_dispatch/submit/attention_fields.rs` >= 2 (found 3, verified)
- `cargo clippy --no-deps -p vox-orchestrator --all-targets -- -D warnings` exits 0 (verified)
- Full verification test suite: `cargo test -p vox-orchestrator --lib -- mode:: socrates attention_fields::tests` (64 passed, 0 failed, verified)
