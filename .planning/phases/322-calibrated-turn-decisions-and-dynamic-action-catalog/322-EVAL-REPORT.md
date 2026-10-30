# Phase 322 Eval Report: Turn Decision Pack Selective Prediction (DEC-04)

**Pack version:** `turn-pack.v2`. The first held-out run was on `turn-pack.v1`. Two questions failed the 0.10 risk gate in that run, so both were disabled (fail closed) and the version was bumped to v2. See "Fail-closed decision" below.

**Measured by:** `selective_eval_report` in `crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/turn_pack_eval_tests.rs`. It prints one `TURN_PACK_EVAL <json>` line and fails if any enabled question's risk at τ is above 0.10.

## Fixtures

| Split | Path | Turns | SHA-256 |
| ----- | ---- | ----- | ------- |
| eval (frozen, held out) | `crates/vox-orchestrator-mcp/tests/fixtures/turn_pack/eval.v1.jsonl` | 28 | `3322f031870eeb15d3f3d29195efe5be58a280610cef3b4915b82d48171a611f` |
| calib | `crates/vox-orchestrator-mcp/tests/fixtures/turn_pack/calib.v1.jsonl` | 30 | `debe0529f66ec76737a86fd4cee896af85e6f395f08b43fe0b72956f95ee5208` |

The eval split was committed alone in 85afc56db, and its hash was recorded in `evidence/322-02-eval-freeze.txt` in 8e39ce622. Both commits came before any rule confidence changed. `shasum -a 256 -c` on that file still passes at the end of the plan, and the report line's `eval_sha256` (computed in Rust from the `include_str!` bytes) matches it.

Each split has at least 4 dual-wording pairs and at least 2 option-order pairs, and every value of every question appears in it. The splits share no prompt. `fixtures_are_well_formed_and_balanced` enforces both properties in CI.

Labeling convention (gold describes what the turn needs, not what a keyword catches):
- **complexity:** 0 is trivial, 1 is moderate, 2 is hard or multi-step.
- **stakes:** 0 has no side effects, 1 is a reversible repo change, 2 is destructive, production, or irreversible.
- **allow_free_tier:** true only if all three hold: no privacy need, stakes below 2, and complexity below 2. When complexity is 1 and stakes is 1, it is false unless the user explicitly asks for a cheap or free model.
- **privacy_local:** true when the turn carries secrets, personal, health, financial, or confidential data, or asks to stay local.

## Per-question results (eval split, n = 28, turn-pack.v2)

| Question | n | answered | accuracy on answered | ECE | τ | coverage@τ | risk@τ | cov@err 0.05 | cov@err 0.10 | disabled |
| -------- | - | -------- | -------------------- | --- | - | ---------- | ------ | ------------ | ------------ | -------- |
| TaskShape | 28 | 13 | 0.846 | 0.178 | 1.01 | 0.000 | 0.000 | 0.000 | 0.000 | **yes** |
| Complexity | 28 | 1 | 1.000 | 0.600 | 0.80 | 0.000 | 0.000 | 0.036 | 0.036 | no |
| Stakes | 28 | 4 | 0.750 | 0.050 | 1.01 | 0.000 | 0.000 | 0.000 | 0.000 | **yes** |
| AllowFreeTier | 28 | 1 | 1.000 | 0.140 | 0.85 | 0.036 | 0.000 | 0.036 | 0.036 | no |
| PrivacyLocal | 28 | 3 | 1.000 | 0.200 | 0.80 | 0.107 | 0.000 | 0.107 | 0.107 | no |

The two disabled questions as measured at their v1 τ (the run that tripped the gate):

| Question | τ (v1) | coverage@τ | risk@τ | eval errors at τ |
| -------- | ------ | ---------- | ------ | ---------------- |
| TaskShape | 0.80 | 0.286 (8 of 28) | **0.250** | e20 and e21. These are the option-order pair "either fix the flaky test … or plan the next release". `task_shape.2.1` (plan, 0.86) outranks `task_shape.0.1` (fix, 0.67), but gold is `code_edit`. |
| Stakes | 0.80 | 0.143 (4 of 28) | **0.250** | e06. "secret" in a "live Stripe secret key" prompt fires `stakes.2.1`, but gold stakes is 1. |

coverage@error takes its thresholds from every distinct item confidence, including ones below τ. That is why Complexity shows 0.036 at error 0.05 while its coverage@τ is 0: its only answer, e08 at 0.40, is correct.

## Control agreement (eval split)

Agreement is the fraction of pairs whose two turns get the same answer value. An abstention counts as an answer.

| Control | pairs | all questions | TaskShape | Complexity | Stakes | AllowFreeTier | PrivacyLocal |
| ------- | ----- | ------------- | --------- | ---------- | ------ | ------------- | ------------ |
| dual_wording | 4 | 0.25 | 0.50 | 1.00 | 1.00 | 0.75 | 0.75 |
| option_order | 2 | 1.00 | 1.00 | 1.00 | 1.00 | 1.00 | 1.00 |

The dual-wording misses are lexical. One wording names the cue in a form the rule table lacks: "cheapest" vs "cheap" (e12/e13), "Only use a local model" vs "Keep this on my machine" (e16/e17), and "Sketch a roadmap" vs "Lay out the phases" (e18/e19).

Option order is invariant by construction: the tie-break picks the highest confidence, then the lexicographically smallest rule id. `rule_order_does_not_change_answers` checks this on every fixture prompt under forward, reversed and rotated tables, and on a synthetic equal-confidence tie (M12).

## patched_turns

`patched_turns` counts eval prompts whose `turn_selection_request` for a default chat turn differs from the unpatched request. The default chat turn has task and complexity unset, no clutch, and Performance.

| Version | patched_turns |
| ------- | ------------- |
| v1 | 8 of 28 |
| v2 | 1 of 28 (e13, allow-free) |

Under v2, the only answers that can act are AllowFreeTier (at ≥ 0.85) and PrivacyLocal (at ≥ 0.80). The privacy answer raises `prefer_local`, and in this default resolution `prefer_local` is already true from the CodeGen placeholder.

## Calibration (calib split only)

Each confidence is the Laplace precision `(correct + 1) / (fired + 2)` on the calib split, rounded to 2 decimals. `rule_confidences_match_calib_split` recomputes these values in CI.

| Rule | fired | correct | confidence |
| ---- | ----- | ------- | ---------- |
| task_shape.0.1 | 4 | 3 | 0.67 |
| task_shape.1.1 | 4 | 4 | 0.83 |
| task_shape.2.1 | 5 | 5 | 0.86 |
| task_shape.3.1 | 2 | 2 | 0.75 |
| complexity.0.1 | 3 | 1 | 0.40 |
| complexity.2.1 | 0 | 0 | 0.50 |
| stakes.0.1 | 0 | 0 | 0.50 |
| stakes.2.1 | 3 | 3 | 0.80 |
| allow_free_tier.1.1 | 5 | 5 | 0.86 |
| allow_free_tier.0.1 | 0 | 0 | 0.50 |
| privacy_local.1.1 | 3 | 3 | 0.80 |

### Calib-only rule keyword edits

Each edit fixes a named calib miss. All of them were made before any eval metric was computed. No rule, label or eval line was changed after the eval ran.

| Calib id | Rule | Change |
| -------- | ---- | ------ |
| c05 | task_shape.1.1 | add "how does" |
| c17 | task_shape.1.1 | add "what does" |
| c06 | task_shape.1.1 | add "look up" |
| c07 | task_shape.1.1 | add "investigate" |
| c11 | task_shape.3.1 | add "hey" |
| c23 | allow_free_tier.1.1 | add "quick one" |
| c24 | allow_free_tier.1.1 | add "small change" |
| c15 | privacy_local.1.1 | add "don t send". The normalizer splits "don't" into "don t", so the existing "do not send" could never match the contracted form. |
| c14 | privacy_local.1.1 | add "keep it local" |

## Fail-closed decision (turn-pack.v1 → v2)

The v1 held-out run measured risk@τ = 0.25 for TaskShape and 0.25 for Stakes, against a gate of 0.10. As the plan requires, no rule, label or eval line was edited. The response was:

- `TAU_DISABLED = 1.01` was added next to `PACK_VERSION`. `tau_for` returns it for TaskShape and Stakes. `selection_inputs` reads every threshold from that table, the report marks both questions `disabled`, and `PACK_VERSION` is now `turn-pack.v2`.
- **Stakes is a veto, so it fails closed differently.** Its only role is to veto free-tier eligibility. A threshold above 1.0 would make the veto never fire, which fails *open*. A disabled Stakes therefore vetoes on any stakes-2 firing, whatever the firing's confidence. An unreliable stakes detector can only block more free routing, never less. Its eval error (e06) was itself a false "high stakes" call, which is the conservative direction. `disabled_questions_fail_closed` pins this behavior.
- TaskShape disabled means the pack never patches `task`. Because the pack also recomputes `prefer_local` from task, `prefer_local` changes only through a confident privacy answer.

## Raw TURN_PACK_EVAL lines

v1 (gate tripped; `/tmp/322-02-t3-red.log`):

```text
TURN_PACK_EVAL {"pack_version":"turn-pack.v1","eval_sha256":"3322f031870eeb15d3f3d29195efe5be58a280610cef3b4915b82d48171a611f","n":28,"questions":{"TaskShape":{"n":28,"answered":13,"accuracy":0.8461538461538461,"ece":0.1776923078757066,"tau":0.800000011920929,"coverage_at_tau":0.2857142857142857,"risk_at_tau":0.25,"coverage_at_error_0_05":0.0,"coverage_at_error_0_10":0.0,"disabled":false},"Complexity":{"n":28,"answered":1,"accuracy":1.0,"ece":0.5999999940395355,"tau":0.800000011920929,"coverage_at_tau":0.0,"risk_at_tau":0.0,"coverage_at_error_0_05":0.03571428571428571,"coverage_at_error_0_10":0.03571428571428571,"disabled":false},"Stakes":{"n":28,"answered":4,"accuracy":0.75,"ece":0.050000011920928955,"tau":0.800000011920929,"coverage_at_tau":0.14285714285714285,"risk_at_tau":0.25,"coverage_at_error_0_05":0.0,"coverage_at_error_0_10":0.0,"disabled":false},"AllowFreeTier":{"n":28,"answered":1,"accuracy":1.0,"ece":0.13999998569488525,"tau":0.8500000238418579,"coverage_at_tau":0.03571428571428571,"risk_at_tau":0.0,"coverage_at_error_0_05":0.03571428571428571,"coverage_at_error_0_10":0.03571428571428571,"disabled":false},"PrivacyLocal":{"n":28,"answered":3,"accuracy":1.0,"ece":0.19999998807907104,"tau":0.800000011920929,"coverage_at_tau":0.10714285714285714,"risk_at_tau":0.0,"coverage_at_error_0_05":0.10714285714285714,"coverage_at_error_0_10":0.10714285714285714,"disabled":false}},"dual_wording_agreement":{"pairs":4,"all_questions":0.25,"per_question":{"TaskShape":0.5,"Complexity":1.0,"Stakes":1.0,"AllowFreeTier":0.75,"PrivacyLocal":0.75}},"option_order_agreement":{"pairs":2,"all_questions":1.0,"per_question":{"TaskShape":1.0,"Complexity":1.0,"Stakes":1.0,"AllowFreeTier":1.0,"PrivacyLocal":1.0}},"patched_turns":8}
```

v2 (published; `/tmp/322-02-t3.log`):

```text
TURN_PACK_EVAL {"pack_version":"turn-pack.v2","eval_sha256":"3322f031870eeb15d3f3d29195efe5be58a280610cef3b4915b82d48171a611f","n":28,"questions":{"TaskShape":{"n":28,"answered":13,"accuracy":0.8461538461538461,"ece":0.1776923078757066,"tau":1.0099999904632568,"coverage_at_tau":0.0,"risk_at_tau":0.0,"coverage_at_error_0_05":0.0,"coverage_at_error_0_10":0.0,"disabled":true},"Complexity":{"n":28,"answered":1,"accuracy":1.0,"ece":0.5999999940395355,"tau":0.800000011920929,"coverage_at_tau":0.0,"risk_at_tau":0.0,"coverage_at_error_0_05":0.03571428571428571,"coverage_at_error_0_10":0.03571428571428571,"disabled":false},"Stakes":{"n":28,"answered":4,"accuracy":0.75,"ece":0.050000011920928955,"tau":1.0099999904632568,"coverage_at_tau":0.0,"risk_at_tau":0.0,"coverage_at_error_0_05":0.0,"coverage_at_error_0_10":0.0,"disabled":true},"AllowFreeTier":{"n":28,"answered":1,"accuracy":1.0,"ece":0.13999998569488525,"tau":0.8500000238418579,"coverage_at_tau":0.03571428571428571,"risk_at_tau":0.0,"coverage_at_error_0_05":0.03571428571428571,"coverage_at_error_0_10":0.03571428571428571,"disabled":false},"PrivacyLocal":{"n":28,"answered":3,"accuracy":1.0,"ece":0.19999998807907104,"tau":0.800000011920929,"coverage_at_tau":0.10714285714285714,"risk_at_tau":0.0,"coverage_at_error_0_05":0.10714285714285714,"coverage_at_error_0_10":0.10714285714285714,"disabled":false}},"dual_wording_agreement":{"pairs":4,"all_questions":0.25,"per_question":{"TaskShape":0.5,"Complexity":1.0,"Stakes":1.0,"AllowFreeTier":0.75,"PrivacyLocal":0.75}},"option_order_agreement":{"pairs":2,"all_questions":1.0,"per_question":{"TaskShape":1.0,"Complexity":1.0,"Stakes":1.0,"AllowFreeTier":1.0,"PrivacyLocal":1.0}},"patched_turns":1}
```

## Live resolve path (L-01, L-03)

These tests use a registry that holds only a free model, under Performance. They are in `model_route_policy/tests.rs`.
- `pack_on_lets_free_model_compete_under_performance`: flag on, the calib prompt c17 is the `capability_prompt`, and the result is `Ok("free-model")`.
- `pack_off_keeps_free_model_excluded_under_performance`: flag off, so the free model is excluded.
- `source_policy_clutch_blocks_pack_allow_free`: the source policy `Interactive = genius` sets a clutch, so the free model is excluded.
- `flag_on_without_capability_prompt_builds_no_pack`: there is no `capability_prompt`, so no pack is built and the free model is excluded.

Mutations M8, M9 and M10 prove that each of these three exclusions depends on its guard.

## Caveats

- **The same model family wrote the labels and the rules.** One agent wrote both splits and the 322-01 keyword table. The eval split was frozen before calibration, but the labeler knew the rule vocabulary. This is not an independent human-labeled set.
- **The sample is small.** n is 28 (eval) and 30 (calib). At low coverage a single error moves risk@τ by about 0.2 or more. Both gate failures were one or two items: e06, and the pair e20/e21. The questions that pass do so on 1 to 3 answered items (AllowFreeTier answered exactly 1 eval turn), so their 0.0 risk is weak evidence.
- **Lexical rules have a recall ceiling.** Coverage@τ is at most 0.11 for every enabled question, and the dual-wording misses are vocabulary gaps. Most turns get no pack answer and fall through to today's request.
- **Calib numbers are in-sample, eval numbers are held out.** The confidences are fit to calib, and the eval numbers above are the only out-of-sample evidence.
- **allow_free only fires when no clutch is set.** It is therefore inert wherever the GUI or daemon always sends a clutch, and a source-policy clutch also counts as set. In practice the allow-free answer only affects callers that leave `clutch` as `None`.
- **Hosted Jev and MENS chat distillation are excluded by L-11** (RESEARCH F9). The DEC-04 clause "Jev gated by inference_privacy_local_only_from_env" is out of scope.
- **The flag is off by default** (`VOX_TURN_DECISION_PACK`). With it off, the request is byte-identical to the 322-01 goldens, and those golden tests still pass inside the `turn_pack` filter run.

## ChatARC

Filled by 322-03.
