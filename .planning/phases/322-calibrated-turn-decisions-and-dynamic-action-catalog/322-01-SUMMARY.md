---
phase: 322-calibrated-turn-decisions-and-dynamic-action-catalog
plan: 01
subsystem: vox-orchestrator-mcp model routing (model_route_policy), vox-config flags
tags: [DEC-01, DEC-03, turn-pack, model-routing, feature-flag, mutation-proof]
status: complete
requires: []
provides:
  - "turn_pack.rs: PACK_VERSION, TAU_APPLY, TAU_FREE, UNCALIBRATED, Question, Rule, RULES, Answer, TurnPack, evaluate, evaluate_with_rules, SelectionInputs, selection_inputs, turn_selection_request"
  - "McpChatModelResolution.task_unset / complexity_unset (default false)"
  - "vox_config::inference::turn_decision_pack_enabled / chatarc_enabled (default off)"
  - "build_selection_request(..., allow_free_in_performance_mode, ...) is pub(super)"
affects:
  - "322-02 (calibrates RULES confidences; resolve-level M8/M9)"
  - "322-03 (consumes chatarc_enabled)"
tech-stack:
  added: []
  patterns:
    - "Debug-string goldens captured from unmodified code before the change"
    - "explicit *_unset fields instead of sentinel-equals-Default"
    - "one named local per permission guard so each is mutation-provable"
key-files:
  created:
    - crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/turn_pack.rs
    - crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/turn_pack_tests.rs
    - .planning/phases/322-calibrated-turn-decisions-and-dynamic-action-catalog/evidence/322-01-goldens-before.txt
  modified:
    - crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/mod.rs
    - crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/resolve.rs
    - crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/types.rs
    - crates/vox-orchestrator-mcp/src/models_tools.rs
    - crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs
    - crates/vox-config/src/inference.rs
    - crates/vox-config/src/config_registry.rs
    - crates/vox-config/src/operator_registry.rs
    - contracts/config/env-vars.v1.yaml
decisions:
  - "Unset is explicit (task_unset / complexity_unset, default false): three live call sites set the Default values deliberately, so a sentinel would let the pack override real choices"
  - "The pack reads only res.capability_prompt; when it is None no pack is built (never the user_prompt fallback with history/file/web text)"
  - "RULES table is #[rustfmt::skip] two lines per rule with an explicit confidence field per rule, to stay under the 300 non-blank-line budget while keeping the per-rule confidence 322-02 rewrites"
metrics:
  duration: "~2h30m (wall clock, dominated by cargo rebuilds at load average 45-90)"
  completed: 2026-10-30
actuals:
  tokens: 8900
  tasks: 3
  commits: 4
plan_head_before: 88be7b8bc9f298e6dc9d5a7530ac0dc9ec165792
plan_head_after: 239e75ae72b333b33c7316d1ca0b7e8e4e8db8e7
---

# Phase 322 Plan 01: Turn Decision Pack (DEC-01) and Flag-Off Identity (DEC-03) Summary

A versioned, deterministic keyword pack (`turn-pack.v1`) runs at the existing `decide()` site in `resolve_mcp_chat_model_sync_inner`. It sits after the source-policy clutch block and fills only the routing inputs the caller left unset: `task`, `complexity`, `prefer_local`, and `allow_free_in_performance_mode` (the last only when no clutch is set). It sits behind the default-off flag `VOX_TURN_DECISION_PACK`. With the flag off, the request is byte-identical to goldens captured from the unmodified builder. Every rule still carries the uncalibrated 0.5 prior, so even with the flag on no real prompt patches anything until 322-02 calibrates the rules.

## Tasks

| Task | Name | Commit | Files |
| ---- | ---- | ------ | ----- |
| 1a | Capture goldens from unmodified code | 517d55ebe | evidence/322-01-goldens-before.txt |
| 1 | Tracer: flag-off identity plus task patch | 448e16ffa | turn_pack.rs, turn_pack_tests.rs, mod.rs, resolve.rs, types.rs, models_tools.rs, vox-config inference.rs |
| 2 | Complexity, prefer_local, allow_free, confidence gate, caller marking | eec5acc03 | turn_pack.rs, turn_pack_tests.rs, chat/message.rs |
| 3 | Env SSOT registration, free-floor pin, mutation proofs | 239e75ae7 | config_registry.rs, operator_registry.rs, env-vars.v1.yaml, turn_pack_tests.rs |

## Goldens (L-02, DEC-03)

`.planning/phases/322-calibrated-turn-decisions-and-dynamic-action-catalog/evidence/322-01-goldens-before.txt` holds 5 lines (`GOLDEN G1..G5`). They were captured by a temporary `capture_selection_request_goldens` test that was added to the unmodified `resolve.rs` and run (`/tmp/322-01-capture.log`, 1 passed). The file was then restored with `git checkout`. The precondition `git diff --quiet -- crates/vox-orchestrator-mcp crates/vox-config` exited 0 before capture and again after the restore. The goldens were committed (517d55ebe) before any source edit was committed. `turn_pack_tests.rs` embeds the same five strings verbatim.

## RED to GREEN evidence

| Step | Command filter | Log | Result |
| ---- | -------------- | --- | ------ |
| T1 RED (vox-config) | `flags_parse_truthy` | /tmp/322-01-t1-red-cfg.log | E0425 `turn_decision_pack_enabled_from` / `chatarc_enabled_from` not found, exit 101 |
| T1 GREEN (vox-config) | `flags_parse_truthy config_registry operator_registry research_prefer_free_tier` | /tmp/322-01-t1-cfg.log | 13 passed, incl. `turn_decision_pack_flags_parse_truthy`, `chatarc_flags_parse_truthy`, `research_prefer_free_tier_parses_truthy` |
| T1 RED (mcp) | `turn_pack` | /tmp/322-01-t1-red.log | E0432 unresolved import `super::turn_pack`, exit 101 |
| T1 GREEN (mcp) | `turn_pack` | /tmp/322-01-t1.log | 8 passed: `flag_off_request_matches_golden_g1..g5`, `flag_on_confident_task_patch_changes_request`, `task_set_by_caller_is_never_patched`, `pack_call_site_follows_source_policy_clutch_resolution` |
| T1 regression | `model_route_policy` | /tmp/322-01-t1-reg.log | 57 passed |
| T2 RED | `turn_pack` | /tmp/322-01-t2-red.log | 17 passed, 4 FAILED: `complexity_unset_is_patched_from_confident_score`, `prefer_local_recomputed_from_patched_task`, `allow_free_set_only_when_clutch_unset`, `at_tau_patches` |
| T2 RED | `resolution_for_tier` | /tmp/322-01-t2-tier-red.log | 1 passed, 1 FAILED: `resolution_for_tier_cloud_marks_complexity_set` |
| T2 GREEN | `turn_pack` | /tmp/322-01-t2.log | 21 passed |
| T2 GREEN | `resolution_for_tier` | /tmp/322-01-t2-tier.log | 2 passed |
| T2 regression | `model_route_policy chat::message` | /tmp/322-01-t2-reg.log | 95 passed, no warnings |
| T3 | vox-config `config_registry` | /tmp/322-01-t3-cfg.log | 7 passed (incl. `every_operator_env_is_in_config_keys`, `keys_are_unique`) |
| T3 | `turn_pack` | /tmp/322-01-t3.log | 23 passed (adds `free_floor_order_is_pinned`, `allow_free_pack_changes_only_scorer_eligibility`) |

Six T2 tests passed in RED as well. Three of them, the caller-set guards and the clutch/stakes/privacy negatives, assert that a field is *not* patched, and the tracer already patched nothing beyond `task`. A fourth, `privacy_local_raises_prefer_local_when_task_unset`, passed vacuously because the tracer kept the derived `true`. All of them are made meaningful by GREEN and by M3-M5 below.

## Mutation proofs (L-15)

Procedure for each row: (1) grep for the guard line printed 1. (2) The mutation was applied. (3) The grep printed 0. (4) The named test was run, exited non-zero and printed `FAILED`. (5) The original text was restored. (6) The grep printed 1 and `git diff --quiet -- <file>` was clean. (7) The test was re-run and printed `test result: ok`. The driver was `/tmp/322-01-mutations.txt`, and each step is logged in `/tmp/322-01-M<n>.log` and `/tmp/322-01-M<n>-restored.log`.

| # | Mutation | grep before/after/restored | Failing test | Mutated log | Restored |
| - | -------- | -------------------------- | ------------ | ----------- | -------- |
| M1 | turn_pack.rs `allow_free: false` → `allow_free: true` | 1 / 0 / 1 | `flag_off_request_matches_golden_g1` | /tmp/322-01-M1.log: exit 101, 1 FAILED | ok, 1 passed |
| M2 | `let may_set_task = res.task_unset;` → `= true;` | 1 / 0 / 1 | `task_set_by_caller_is_never_patched` | /tmp/322-01-M2.log: exit 101, 1 FAILED | ok, 1 passed |
| M3 | `let may_set_complexity = res.complexity_unset;` → `= true;` | 1 / 0 / 1 | `complexity_set_by_caller_is_never_patched` | /tmp/322-01-M3.log: exit 101, 1 FAILED | ok, 1 passed |
| M4 | `let may_set_prefer_local = res.task_unset;` → `= true;` | 1 / 0 / 1 | `prefer_local_not_patched_when_caller_set_task` | /tmp/322-01-M4.log: exit 101, 1 FAILED | ok, 1 passed |
| M5 | `let may_set_allow_free = res.clutch.is_none();` → `= true;` | 1 / 0 / 1 | `allow_free_never_set_when_clutch_set` | /tmp/322-01-M5.log: exit 101, 1 FAILED | ok, 1 passed |
| M6 | `a.confidence >= tau` → `a.confidence >= 0.0` | 1 / 0 / 1 | `below_tau_never_patches` | /tmp/322-01-M6.log: exit 101, 1 FAILED | ok, 1 passed |
| M7 | chat/message.rs cloud arm: delete `complexity_unset: false,` | 1 / 0 / 1 | `resolution_for_tier_cloud_marks_complexity_set` | /tmp/322-01-M7.log: exit 101, 1 FAILED | ok, 1 passed |

All seven were PROVEN. M8 and M9 (the resolve-level flag gate and the policy-clutch ordering under a registry-backed test) belong to 322-02 Task 3, as the plan specifies.

## Blocked (L-04), recorded and not built

- **DEC-02** (post-tool decisions through `evaluate_post_tool_batch_hook`, `evaluate_stop_hook`, `apply_repair_escalation_to_config`): none of these functions exists on `gsd/mens-compare-316` or in any git ref (RESEARCH F1). They exist only as untracked files in another agent's Cursor checkout, which this plan must not copy. `SelectionIntent::repair_loop()` is used here only by `vox-cli` repair and `gui_ops/harness.rs`, and neither is a post-tool chat hook. `draft_pr` is not a pack directive.
- **DEC-03 remainder** (`resolve_mens_fast_apply_target`, `consult_oracle_model_advisory`, `apply_repair_escalation_to_config` identity): blocked for the same reason. Only the `build_selection_request` identity is covered (Task 1 goldens, M1).
- **Unblock condition:** the Cursor checkout's `agent_loop/{batch_and_eval_gate.rs,lifecycle_hooks.rs}` refactor lands on a ref this branch can merge. A follow-up then adds post-tool pack epochs and identity goldens for those functions.

## Pending and pre-existing items

- `docs/src/reference/env-vars.generated.md` was not hand-edited. Regenerating it for the two new `env-vars.v1.yaml` rows is pending (CI `ssot-autoregen` owns it).
- Editing `crates/vox-config/src/inference.rs` makes `vox ci operator-env-guard` see three pre-existing literal env reads that are not in `OPERATOR_TUNING_ENVS`: `VOX_LOCAL_ENDPOINT`, `VOX_RESEARCH_PREFER_FREE_TIER`, and `VOX_HOME` (verified: 0 rows each in operator_registry.rs). This plan did not introduce them, and per the plan they were not added to any baseline.
- `crates/vox-orchestrator-mcp/src/llm_bridge/infer.rs` has its own `resolution_for_tier` with a cloud arm that raises complexity. It is untouched. Its bases come from callers that keep both unset flags `false` via `Default`, so it cannot hand a raised complexity to the pack.

## Negative checks

- Non-comment lines in turn_pack.rs that match `FREE_FLOOR|FALLBACK_MODELS|PreferFree|SelectionPolicy|size_select|mesh_select|SpokeRouter`: 0.
- `resolve.rs` diff since the goldens commit is 28 insertions and 8 deletions: `pub(super)` on the builder and `VOX_LOCAL_PREFERRED_TASKS`, one new parameter and its assignment, the call-site swap with the pack gate and debug log, and four test call updates.
- turn_pack.rs has 239 non-blank lines (budget 300).

## Deviations from Plan

### Auto-fixed / process deviations

1. **[Rule 3 - Blocking] vox-config edits were batched ahead of their task.** `vox-config` is a dependency of nearly every crate, and each edit forced a 20-40 minute downstream rebuild at load average 45-90. So the Task 3 registry rows (config_registry.rs, operator_registry.rs, env-vars.v1.yaml) were written together with the Task 1 `inference.rs` change, before the first mcp build. They were still committed separately, in the Task 3 commit (239e75ae7). The Task 1 mcp RED was rerun after this, so the RED log comes from a build containing the final vox-config.
2. **[Rule 3 - Blocking] Mutation restores used an exact reverse write instead of `git checkout -- <file>`.** This checkout's lefthook `post-checkout` hook launches `vox ci artifact-prune --include-worktrees --apply` in the background, and that prunes build artifacts. Every restoration was confirmed byte-identical to HEAD with `git diff --quiet -- <file>` and the grep count. The goldens step did use `git checkout`, as the plan specified.
3. **Mutations were applied by a scripted exact-string replace** (scratchpad `mutate.py`) instead of the Edit tool. The sequence was the same: one mutation at a time, with grep, test and restore between them.
4. **Extra test `ties_break_by_rule_id_not_table_order`** (not in the plan's behavior list). It pins the documented tie-break that 322-02's option-order control relies on.
5. **RULES is `#[rustfmt::skip]`** with two lines per rule, to meet the 300 non-blank-line budget (rustfmt's vertical layout gave 322 lines). Each rule keeps its own `confidence: UNCALIBRATED` field, so 322-02 can rewrite them.

### Commit hygiene

Code commits ran the pre-commit hooks (fmt-fix, tdd-guard, ci-status). None of them builds, so `--no-verify` was used only for the planning/evidence commit, as the plan instructs. Unrelated dirty and untracked files (320 evidence and others) were never staged.

## Known Stubs

None. `UNCALIBRATED` (0.5) is an intentional prior that sits below both thresholds. 322-02 replaces it with calib-split precision and deletes `uncalibrated_rules_never_patch`.

## Threat Flags

None beyond the plan's threat model. T-322-01 is mitigated by M2-M5. T-322-02 is mitigated because the pack reads `res.capability_prompt` only. T-322-03 is mitigated by `allow_free_pack_changes_only_scorer_eligibility`.

## Self-Check: PASSED

- FOUND: turn_pack.rs, turn_pack_tests.rs, evidence/322-01-goldens-before.txt (5 lines)
- FOUND commits: 517d55ebe, 448e16ffa, eec5acc03, 239e75ae7 (all ancestors of HEAD)
- Registry: `VOX_TURN_DECISION_PACK` and `VOX_CHATARC` present in config_registry.rs, operator_registry.rs, and env-vars.v1.yaml
