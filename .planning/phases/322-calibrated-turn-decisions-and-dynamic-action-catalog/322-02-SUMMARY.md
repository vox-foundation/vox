---
phase: 322-calibrated-turn-decisions-and-dynamic-action-catalog
plan: 02
subsystem: vox-orchestrator-mcp model routing (model_route_policy turn pack)
tags: [DEC-01, DEC-04, turn-pack, calibration, selective-prediction, mutation-proof]
status: complete
requires:
  - "322-01: turn_pack.rs (RULES, evaluate_with_rules, selection_inputs, turn_selection_request), VOX_TURN_DECISION_PACK"
provides:
  - "tests/fixtures/turn_pack/{eval,calib}.v1.jsonl: 28 frozen held-out + 30 calibration labeled turns"
  - "selective_metrics.rs (#[cfg(test)]): laplace_precision, ece, coverage_and_risk, coverage_at_error (reused by 322-03)"
  - "turn_pack.rs: calibrated RULES confidences, TAU_DISABLED, tau_for(Question), turn_selection_request_at(tau, ...), PACK_VERSION turn-pack.v2"
  - "turn_pack_eval_tests.rs: fixture balance, calibration recompute, order controls, selective_eval_report (TURN_PACK_EVAL line)"
  - "322-EVAL-REPORT.md (## ChatARC left for 322-03)"
affects:
  - "322-03 (reuses selective_metrics; fills the ChatARC section of 322-EVAL-REPORT.md)"
tech-stack:
  added: []
  patterns:
    - "eval split committed and SHA-256-pinned before calibration; shasum -c in every verify"
    - "per-question threshold table (tau_for) so a question is disabled in one place; mechanism tests run at the enabled table"
    - "a disabled veto question fails closed by vetoing on any firing, not by never firing"
key-files:
  created:
    - crates/vox-orchestrator-mcp/tests/fixtures/turn_pack/eval.v1.jsonl
    - crates/vox-orchestrator-mcp/tests/fixtures/turn_pack/calib.v1.jsonl
    - crates/vox-orchestrator-mcp/src/selective_metrics.rs
    - crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/turn_pack_eval_tests.rs
    - .planning/phases/322-calibrated-turn-decisions-and-dynamic-action-catalog/evidence/322-02-eval-freeze.txt
    - .planning/phases/322-calibrated-turn-decisions-and-dynamic-action-catalog/322-EVAL-REPORT.md
  modified:
    - crates/vox-orchestrator-mcp/src/lib.rs
    - crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/mod.rs
    - crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/turn_pack.rs
    - crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/turn_pack_tests.rs
    - crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/tests.rs
decisions:
  - "TaskShape and Stakes disabled (TAU_DISABLED = 1.01, turn-pack.v2): held-out risk at tau 0.25 each, above the 0.10 gate; no rule, label or eval line edited"
  - "Disabled Stakes fails closed as a veto: any stakes-2 firing blocks free eligibility, whatever its confidence (a threshold above 1.0 alone would make the veto never fire, i.e. fail open)"
  - "Thresholds read from tau_for(Question); turn_selection_request_at(tau, ...) lets the 322-01 mechanism tests keep every guard mutation-provable at the enabled thresholds"
  - "Resolve-level fixture spec gets Research's strength tag and an intelligence index so decide() can pick it; without them every resolve test passed vacuously (decide() returned None for all four)"
metrics:
  duration: "~2h55m wall clock (02:14-05:09), dominated by cargo rebuilds at load average 45-150"
  completed: 2026-10-30
actuals:
  tokens: 17500
  tasks: 3
  commits: 6
plan_head_before: ce87f116aeb1b222be100e2b2bc99c685abd4f96
plan_head_after: 27c64fe1c
---

# Phase 322 Plan 02: Calibrated Turn Pack and Held-Out Selective-Prediction Eval Summary

The 322-01 keyword pack now has measured confidences. Each rule's confidence is its Laplace precision on a 30-turn calibration split, and a CI test recomputes it. The pack was then measured on a 28-turn held-out split that was frozen and SHA-256-pinned before calibration. On that split, TaskShape and Stakes had risk at τ of 0.25, which fails the 0.10 gate. Both were disabled to fail closed, which made this `turn-pack.v2`. The remaining enabled questions (Complexity, AllowFreeTier, PrivacyLocal) have risk 0.0 at τ, but on very few answered items. On the live resolve path, a free model can compete under Performance only when the flag is on, no clutch is set, and the turn carries a `capability_prompt`. Mutations M8 to M10 prove each of those three conditions.

## Tasks

| Task | Name | Commit | Files |
| ---- | ---- | ------ | ----- |
| 1 | Freeze eval split, record SHA-256, write calib split | 85afc56db, 8e39ce622, a60f2fc50 | eval.v1.jsonl, evidence/322-02-eval-freeze.txt, calib.v1.jsonl |
| 2 | Selective metrics, order controls, calib-split calibration | dcbea30f9 | selective_metrics.rs, lib.rs, mod.rs, turn_pack.rs, turn_pack_tests.rs, turn_pack_eval_tests.rs |
| 3 | Eval report, resolve-level allow_free proof, fail-closed disable, mutation proofs | 27c64fe1c (code), plus the docs commit for this summary and the report | turn_pack.rs, turn_pack_tests.rs, turn_pack_eval_tests.rs, tests.rs, 322-EVAL-REPORT.md |

The eval freeze (`3322f031870eeb15d3f3d29195efe5be58a280610cef3b4915b82d48171a611f`) was committed in 85afc56db (eval alone) and 8e39ce622 (hash), before turn_pack.rs was opened in this session. `shasum -a 256 -c evidence/322-02-eval-freeze.txt` passes at the end of the plan.

## RED to GREEN evidence

| Step | Filter | Log | Result |
| ---- | ------ | --- | ------ |
| T2 RED (metrics) | `selective_metrics` | /tmp/322-02-t2-red-m.log | 4 FAILED (`todo!()`), exit 101. The first attempt was a scaffold compile error (E0106 lifetime in `pairs`), which was fixed and rerun. |
| T2 RED (eval tests) | `turn_pack_eval` | /tmp/322-02-t2-red.log | 3 passed, `rule_confidences_match_calib_split` FAILED |
| T2 RED (calibration) | `-- selective_metrics turn_pack_eval` | /tmp/322-02-t2-red2.log | metrics 4 ok; `rule_confidences_match_calib_split` FAILED with the drift table (8 rules off the 0.5 prior) |
| T2 GREEN | `selective_metrics` / `turn_pack` | /tmp/322-02-t2-m.log, /tmp/322-02-t2.log | 4 passed / 26 passed, no warnings |
| T3 precondition | `pack_off_keeps_free_model_excluded` | /tmp/322-02-t3-pre.log | passed, but vacuously (see Deviation 2), so it was re-established below |
| T3 RED | `turn_pack`, resolve tests | /tmp/322-02-t3-red.log, /tmp/322-02-t3-red-resolve.log | `selective_eval_report` FAILED (TaskShape and Stakes risk@τ 0.25); `disabled_questions_fail_closed` FAILED; `pack_on_lets_free_model_compete_under_performance` FAILED (Err: no model) |
| T3 fixture fix | resolve tests | /tmp/322-02-t3-red-resolve2.log | 4 passed, so `pack_off` now passes for the right reason (the FreeInPerformanceMode exclusion) |
| T3 GREEN | `turn_pack -- --nocapture` | /tmp/322-02-t3.log | 28 passed, `TURN_PACK_EVAL` v2 line printed |
| T3 GREEN | `free_model` | /tmp/322-02-t3-r.log | 5 passed |
| T3 regression | `model_route_policy` | /tmp/322-02-t3-all.log | 81 passed |

The flag-off identity still holds: the five 322-01 golden tests pass inside every `turn_pack` run.

## Calib-only rule edits

Each edit fixes a calib miss and was made before any eval metric was computed. Confidences were re-derived after the edits.

| Calib id | Rule | Change |
| -------- | ---- | ------ |
| c05 | task_shape.1.1 | add "how does" |
| c17 | task_shape.1.1 | add "what does" |
| c06 | task_shape.1.1 | add "look up" |
| c07 | task_shape.1.1 | add "investigate" |
| c11 | task_shape.3.1 | add "hey" |
| c23 | allow_free_tier.1.1 | add "quick one" |
| c24 | allow_free_tier.1.1 | add "small change" |
| c15 | privacy_local.1.1 | add "don t send". The normalizer turns "don't" into "don t", so "do not send" never matched. |
| c14 | privacy_local.1.1 | add "keep it local" |

These are the resulting confidences (Laplace, calib). task_shape.0.1 0.67, task_shape.1.1 0.83, task_shape.2.1 0.86, task_shape.3.1 0.75, complexity.0.1 0.40, complexity.2.1 0.50, stakes.0.1 0.50, stakes.2.1 0.80, allow_free_tier.1.1 0.86, allow_free_tier.0.1 0.50, privacy_local.1.1 0.80. `UNCALIBRATED` is deleted, along with the 322-01 test `uncalibrated_rules_never_patch`.

The tie-break was already order-invariant: highest confidence, then smallest rule id. No change was needed.

## Published metrics (eval split, n = 28, turn-pack.v2)

| Question | answered | accuracy | ECE | τ | coverage@τ | risk@τ | cov@err 0.05 / 0.10 | disabled |
| -------- | -------- | -------- | --- | - | ---------- | ------ | ------------------- | -------- |
| TaskShape | 13 | 0.846 | 0.178 | 1.01 | 0.000 (v1: 0.286) | 0.000 (v1: **0.250**) | 0.000 / 0.000 | yes |
| Complexity | 1 | 1.000 | 0.600 | 0.80 | 0.000 | 0.000 | 0.036 / 0.036 | no |
| Stakes | 4 | 0.750 | 0.050 | 1.01 | 0.000 (v1: 0.143) | 0.000 (v1: **0.250**) | 0.000 / 0.000 | yes |
| AllowFreeTier | 1 | 1.000 | 0.140 | 0.85 | 0.036 | 0.000 | 0.036 / 0.036 | no |
| PrivacyLocal | 3 | 1.000 | 0.200 | 0.80 | 0.107 | 0.000 | 0.107 / 0.107 | no |

Dual-wording agreement across all questions is 0.25 over 4 pairs. Option-order agreement is 1.00 over 2 pairs. `patched_turns` went from 8 (v1) to 1 (v2). The full tables and the raw lines are in 322-EVAL-REPORT.md.

## Disabled questions (fail closed)

- **TaskShape**: risk@τ 0.25. The errors are e20 and e21, the option-order pair "fix the flaky test … or plan the next release". There "plan" (0.86) outranks "fix" (0.67), but gold is code_edit.
- **Stakes**: risk@τ 0.25. The error is e06: "secret" in a live Stripe secret key prompt fires stakes 2, but gold is 1.
- How the disable works: `TAU_DISABLED = 1.01` sits next to `PACK_VERSION`, and `tau_for` returns it for both questions. `selection_inputs` reads every threshold through the `tau` table. `PACK_VERSION` is now `turn-pack.v2`. The report JSON marks both `disabled: true`, and the rerun passes the gate.
- **Stakes' veto, a deliberate interpretation.** Stakes' only consumer is the allow-free veto. Applying TAU_DISABLED to the veto literally would mean it never fires, which fails *open*. Instead, a disabled Stakes vetoes on any stakes-2 firing, whatever its confidence. `disabled_questions_fail_closed` pins this behavior, and the report records the decision.

## Flag-off precondition outcome

The first `pack_off_keeps_free_model_excluded_under_performance` run passed (/tmp/322-02-t3-pre.log), but it passed for the wrong reason. A debug probe found that decide() returned `None` for the test free model in every configuration, including flag-on and Economy. The cause was in the fixture: the model's confidence was not routing-eligible (no intelligence index, so not `Confirmed`), and it lacked Research's strength tag. All four resolve tests were therefore vacuous. After the fixture fix, flag-off under Performance still does not return the free model, and this time the reason is the scorer's `FreeInPerformanceMode` exclusion. No fallback admits free models: `allow_cheapest_fallback` is false, and the result is an `Err`. resolve.rs was not changed.

## Mutation proofs (L-15)

Procedure for each row:
1. grep for the target, which prints 1.
2. Apply the mutation (scripted exact-string replace).
3. grep again, which prints 0.
4. Run the named test and confirm it exits 101 with `FAILED`.
5. Restore with an exact reverse write.
6. Confirm `git diff --quiet -- <file>` is clean and the grep prints 1.
7. Rerun the test and confirm `test result: ok. 1 passed`.

The driver is scratchpad `mutate.py`, and its output is in /tmp/322-02-mutations.txt. Every mutation ran against committed HEAD 27c64fe1c.

| # | Mutation | grep before/after/restored | Failing test | Mutated log | Restored |
| - | -------- | -------------------------- | ------------ | ----------- | -------- |
| M8 | resolve.rs `Some(text) if vox_config::inference::turn_decision_pack_enabled() => {` → `Some(text) if true => {` | 1 / 0 / 1 | `pack_off_keeps_free_model_excluded_under_performance` (got `Ok("free-model")`) | /tmp/322-02-M8.log: exit 101, 1 FAILED | ok, 1 passed |
| M9 | turn_pack.rs `let may_set_allow_free = res.clutch.is_none();` → `= true;` | 1 / 0 / 1 | `source_policy_clutch_blocks_pack_allow_free` (got `Ok("free-model")`) | /tmp/322-02-M9.log: exit 101, 1 FAILED | ok, 1 passed |
| M10 | resolve.rs `match res.capability_prompt.as_deref() {` → `match res.capability_prompt.as_deref().or(Some(user_prompt)) {` | 1 / 0 / 1 | `flag_on_without_capability_prompt_builds_no_pack` (got `Ok("free-model")`) | /tmp/322-02-M10.log: exit 101, 1 FAILED | ok, 1 passed |
| M11 | turn_pack.rs task_shape.0.1 `confidence: 0.67` → `0.77` | 1 / 0 / 1 | `rule_confidences_match_calib_split` (`task_shape.0.1 … 0.67 0.77 <- drift`) | /tmp/322-02-M11.log: exit 101, 1 FAILED | ok, 1 passed |
| M12 | turn_pack.rs tie-break `\|\| (rule.confidence == cur.confidence && rule.id < cur.rule_id)` → `\|\| false` (keep the first firing rule in table order) | 1 / 0 / 1 | `rule_order_does_not_change_answers`. It failed on the synthetic tie (`t.b` vs `t.a`); the fixtures have no equal-confidence ties. | /tmp/322-02-M12.log: exit 101, 1 FAILED | ok, 1 passed |
| M6b | turn_pack.rs `a.confidence >= tau` → `a.confidence >= 0.0` (calibrated rules) | 1 / 0 / 1 | `below_tau_never_patches` | /tmp/322-02-M6b.log: exit 101, 1 FAILED | ok, 1 passed |

All six are PROVEN.

## Deviations from Plan

### Auto-fixed issues

1. **[Rule 1 - Bug] The resolve-level tests were vacuous.** Found during Task 3. `registry_free_only()` reused the `free-model` spec unchanged. decide() returns `None` for that spec: its routing confidence is not `Confirmed`, and it has no Research strength tag (the gated ranking drops it, and the open ranking only wins when the model has a quality prior). As a result, flag-on also returned `Err`, and the three negative tests passed regardless of their guards. The fix is test-only. In `registry_free_only`, the spec gets `task_category_strength(Research)` pushed onto `strengths` and `capabilities.intelligence_index = Some(50.0)`. Production code was not touched. M8 to M10 now prove the negatives. Commit 27c64fe1c.
2. **[Rule 2 - Correctness] Fail closed for a veto.** Found during Task 3. The literal mechanism (TAU_DISABLED as the Stakes threshold) would have turned the allow-free veto off. The disabled Stakes vetoes on any firing instead. See "Disabled questions" above. Commit 27c64fe1c.
3. **[Rule 3 - Blocking] A threshold table (`tau_for`) and `turn_selection_request_at(tau, …)`.** Disabling TaskShape at its call site would have broken four 322-01 mechanism tests and made M2 (`task_set_by_caller_is_never_patched`) undetectable. `turn_pack_tests.rs` now shadows `turn_selection_request` with a wrapper that runs at `mechanism_tau` (the 322-01 thresholds, all enabled). Every 322-01 test runs unchanged, and production uses `tau_for`. resolve.rs is unchanged. Commit 27c64fe1c.
4. **`Rule` derives `Clone`**, so the order-control test can build reversed and rotated tables. Commit dcbea30f9.

### Process deviations

- The plan said "mutate with Edit" and "`git checkout -- <file>`". Mutations were applied and restored by a scripted exact-string replace, as the operator instructed and as 322-01 did. A post-checkout hook here starts a background prune. Each restore was confirmed with `git diff --quiet` and the grep count.
- `selective_eval_report` uses `vox_crypto::interop_sha256` (the workspace SSOT facade, documented for external formats with a fixed algorithm, which `shasum -a 256` is) plus `hex`. Both were already dependencies, so no new crate edge was added.
- `TURN_PACK_EVAL` also reports `tau` and `n` per question. Agreement is reported per question plus "all questions".
- The measured commit count includes b4d886c22 `feat(320): fail-fast ordering for eval-collateral-damage`. Another agent committed it on this branch at 03:08, between Task 2 and Task 3. It is not part of this plan, and none of its files were staged. The plan's own commits are 85afc56db, 8e39ce622, a60f2fc50, dcbea30f9 and 27c64fe1c, plus the docs commit for this summary.

## Blocked / out of scope

- Hosted Jev, with its privacy gate, and MENS chat distillation are excluded by L-11 (RESEARCH F9). They are recorded in the report caveats.
- DEC-02 (post-tool decisions) is still blocked, as recorded in 322-01 (RESEARCH F1).

## Known Stubs

None. The rules that never fire on calib keep 0.5 (Laplace of 0/0) by design and cannot patch.

## Threat Flags

None beyond the plan's threat model.
- T-322-05: the eval was frozen and hashed first, and both failing questions were disabled rather than refit.
- T-322-06: `rule_confidences_match_calib_split` (M11) covers it.
- T-322-07: every env-touching test holds `INFERENCE_PROFILE_TEST_LOCK` and sets the pack flag explicitly through `EnvKeyGuard`.
- T-322-08: M8, M9 and M10 cover it.

## Self-Check: PASSED

- FOUND: both fixtures, selective_metrics.rs, turn_pack_eval_tests.rs, evidence/322-02-eval-freeze.txt, 322-EVAL-REPORT.md.
- FOUND commits 85afc56db, 8e39ce622, a60f2fc50, dcbea30f9 and 27c64fe1c, all ancestors of HEAD.
- `shasum -a 256 -c evidence/322-02-eval-freeze.txt` reports OK.
