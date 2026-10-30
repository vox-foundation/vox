# Phase 322 Eval Report: Turn Decision Pack Selective Prediction (DEC-04)

**Pack version:** `turn-pack.v3`. The first held-out run was on `turn-pack.v1`. Two questions failed the 0.10 risk gate in that run, so both were disabled (fail closed) and the version was bumped to v2. v3 adds the firing-based free-tier and Stakes vetoes from the code review; no rule, threshold or answer changed, and every metric below is identical under v3. See "Fail-closed decision" and "Review fixes" below.

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
- **Stakes is a veto, so it fails closed differently.** Its only role is to veto free-tier eligibility. A threshold above 1.0 would make the veto never fire, which fails *open*. A disabled Stakes therefore vetoes on any stakes-2 firing, whatever the firing's confidence. (v3: the veto is computed from the firings themselves, so it also holds when a stakes-0 rule wins the argmax and at any Stakes threshold; see "Review fixes".) An unreliable stakes detector can only block more free routing, never less. Its eval error (e06) was itself a false "high stakes" call, which is the conservative direction. `disabled_questions_fail_closed` pins this behavior.
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

## Review fixes (322-REVIEW.md, turn-pack.v3)

The code review found that the pack's safety story was weaker than this report claimed. The fixes changed what the pack reads and when it vetoes. They changed no rule keyword, confidence, threshold, label or eval line.

| Finding | Change | Test (RED → GREEN) |
| ------- | ------ | ------------------ |
| CR-01 | The pack scores `McpChatModelResolution.decision_prompt`: the user's own words before `@mention` expansion and canonicalization, or a research slash command's query. ChatARC gets the same text. Before the fix both scored `expanded_prompt`, which inlines mentioned files and the "Objectives …" header. | `pack_scores_decision_prompt_not_mention_expanded_text`, `turn_decision_text_is_the_users_own_words` |
| WR-01 | `decision_prompt` being `Some` is the pack's opt-in. Only `try_run_agent_turn` sets it. | `resolution_without_decision_prompt_is_never_patched` |
| CR-02 | `TurnPack.free_veto` is set by any deny-side AllowFreeTier rule or a negated free phrase. Examples are "don't use a free model" and "quick question, best model". | `negated_or_deny_side_free_cues_veto_allow_free` |
| WR-02 | `free_veto` is also set by credential words or anything `vox_redact::redact_owned` would elide (`sk-`, `ghp_`, `AKIA`, JWT, bearer and `api_key=` values, emails, private IPs). | `secret_shaped_text_vetoes_allow_free` |
| WR-04 | `TurnPack.stakes_veto` is set by any stakes-2 firing, independent of the argmax winner and of the Stakes τ. | `stakes_veto_holds_when_a_stakes_0_rule_outranks_it` |

**Metric change: none.** The vetoes are computed beside the answers, not into them. Every per-question number, both agreement tables and `patched_turns` (1 of 28, e13) are byte-identical to v2. Only `pack_version` changed. No eval turn with an allow-free answer carries a negation, a deny cue or a secret shape. `rule_confidences_match_calib_split` passes unchanged, so no confidence needed re-derivation. `shasum -a 256 -c evidence/322-02-eval-freeze.txt` still reports OK. The ChatARC eval line is unchanged (`chatarc.v2`, `disabled: true`). WR-09 only stops a disabled ChatARC from gathering candidates, and the eval calls `decide` directly.

The negation and credential lexicons were chosen from the review's examples before the eval was re-run, not tuned on it. e13 ("No need for anything fancy, a free model is fine") still allows free routing, because "no need" is not a listed negation.

v3 (`/tmp/322-fix-eval.log`):

```text
TURN_PACK_EVAL {"pack_version":"turn-pack.v3","eval_sha256":"3322f031870eeb15d3f3d29195efe5be58a280610cef3b4915b82d48171a611f","n":28,"questions":{"TaskShape":{"n":28,"answered":13,"accuracy":0.8461538461538461,"ece":0.1776923078757066,"tau":1.0099999904632568,"coverage_at_tau":0.0,"risk_at_tau":0.0,"coverage_at_error_0_05":0.0,"coverage_at_error_0_10":0.0,"disabled":true},"Complexity":{"n":28,"answered":1,"accuracy":1.0,"ece":0.5999999940395355,"tau":0.800000011920929,"coverage_at_tau":0.0,"risk_at_tau":0.0,"coverage_at_error_0_05":0.03571428571428571,"coverage_at_error_0_10":0.03571428571428571,"disabled":false},"Stakes":{"n":28,"answered":4,"accuracy":0.75,"ece":0.050000011920928955,"tau":1.0099999904632568,"coverage_at_tau":0.0,"risk_at_tau":0.0,"coverage_at_error_0_05":0.0,"coverage_at_error_0_10":0.0,"disabled":true},"AllowFreeTier":{"n":28,"answered":1,"accuracy":1.0,"ece":0.13999998569488525,"tau":0.8500000238418579,"coverage_at_tau":0.03571428571428571,"risk_at_tau":0.0,"coverage_at_error_0_05":0.03571428571428571,"coverage_at_error_0_10":0.03571428571428571,"disabled":false},"PrivacyLocal":{"n":28,"answered":3,"accuracy":1.0,"ece":0.19999998807907104,"tau":0.800000011920929,"coverage_at_tau":0.10714285714285714,"risk_at_tau":0.0,"coverage_at_error_0_05":0.10714285714285714,"coverage_at_error_0_10":0.10714285714285714,"disabled":false}},"dual_wording_agreement":{"pairs":4,"all_questions":0.25,"per_question":{"TaskShape":0.5,"Complexity":1.0,"Stakes":1.0,"AllowFreeTier":0.75,"PrivacyLocal":0.75}},"option_order_agreement":{"pairs":2,"all_questions":1.0,"per_question":{"TaskShape":1.0,"Complexity":1.0,"Stakes":1.0,"AllowFreeTier":1.0,"PrivacyLocal":1.0}},"patched_turns":1}
```

## Live resolve path (L-01, L-03)

These tests use a registry that holds only a free model, under Performance. They are in `model_route_policy/tests.rs`.
- `pack_on_lets_free_model_compete_under_performance`: flag on, the calib prompt c17 is the `decision_prompt` (v3; it was the `capability_prompt` before the review fixes), and the result is `Ok("free-model")`.
- `pack_off_keeps_free_model_excluded_under_performance`: flag off, so the free model is excluded.
- `source_policy_clutch_blocks_pack_allow_free`: the source policy `Interactive = genius` sets a clutch, so the free model is excluded.
- `flag_on_without_capability_prompt_builds_no_pack`: there is no `capability_prompt`, so no pack is built and the free model is excluded.

Mutations M8, M9 and M10 prove that each of these three exclusions depends on its guard.

## Caveats

- **The same model family wrote the labels and the rules.** One agent wrote both splits and the 322-01 keyword table. The eval split was frozen before calibration, but the labeler knew the rule vocabulary. This is not an independent human-labeled set.
- **The sample is small.** n is 28 (eval) and 30 (calib). At low coverage a single error moves risk@τ by about 0.2 or more. Both gate failures were one or two items: e06, and the pair e20/e21. The questions that pass do so on 1 to 3 answered items (AllowFreeTier answered exactly 1 eval turn), so their 0.0 risk is weak evidence.
- **Lexical rules have a recall ceiling.** Coverage@τ is at most 0.11 for every enabled question, and the dual-wording misses are vocabulary gaps. Most turns get no pack answer and fall through to today's request.
- **Calib numbers are in-sample, eval numbers are held out.** The confidences are fit to calib, and the eval numbers above are the only out-of-sample evidence.
- **allow_free only fires when no clutch is set, and only on the default chat turn.** It is therefore inert wherever the GUI or daemon always sends a clutch, and a source-policy clutch also counts as set. Since the review fixes, the pack also runs only when `decision_prompt` is set, which only `try_run_agent_turn` does, so browser tools, t3 consensus, cognitive profiles and `call_llm_with_pref` never get a pack.
- **Hosted Jev and MENS chat distillation are excluded by L-11** (RESEARCH F9). The DEC-04 clause "Jev gated by inference_privacy_local_only_from_env" is out of scope.
- **The flag is off by default** (`VOX_TURN_DECISION_PACK`). With it off, the request is byte-identical to the 322-01 goldens, and those golden tests still pass inside the `turn_pack` filter run.

## ChatARC

**Version:** `chatarc.v2`. The first held-out run was `chatarc.v1` (τ 0.80). Its risk at τ was 0.167, above the 0.10 gate, so v2 fails closed: `TAU_CHOICE = 1.01` and ChatARC never acts. See "Fail-closed decision (chatarc.v1 → v2)" below.

**Measured by:** `chatarc_eval_report` in `crates/vox-orchestrator-mcp/src/chat_tools/chatarc_eval_tests.rs`. It prints one `CHATARC_EVAL <json>` line and fails if risk at `TAU_CHOICE` is above 0.10 while ChatARC is enabled (`TAU_CHOICE ≤ 1.0`).

### What is measured

ChatARC builds one BM25 shortlist (`SkillSearchIndex::from_docs`) over all candidates. The shortlist holds the top `K_MAX - 1 = 9` actions, then `none`. Choice takes the top action. Its confidence is the calibrated precision of the bin its margin `(s1 - s2) / s1` falls in. The action is returned only when that confidence is at least `TAU_CHOICE`. The eval builds candidates from a hermetic catalog with the production converters (`skill_candidates`, `tool_candidates`, `command_candidates`). It runs `shortlist`, `choose_with` and `decide` exactly as `pick_for_turn` does.

- **answered:** eval queries with a non-empty action shortlist. They feed ECE and coverage@error with `(bin confidence, top-1 == gold)`. A gold-`none` query is never correct for an action.
- **coverage / risk at τ:** the share of all n queries where ChatARC acts, and the error rate among those.
- **recall@K:** the share of non-none golds that appear among the shortlisted actions.
- **none_abstention:** the share of gold-`none` queries whose final output is no action.
- **end_to_end_accuracy:** the share of queries whose final output (an action or none) equals gold.
- **dual_wording_agreement:** the share of paired wordings whose two final outputs are equal.

### Fixtures

| File | Path | Size | SHA-256 |
| ---- | ---- | ---- | ------- |
| catalog | `crates/vox-orchestrator-mcp/tests/fixtures/chatarc/catalog.v1.json` | 10 skills, 12 tools, 10 commands | `8bed53e527f041eae9d8ab509680e11d4c5ebbf40d5fb6eff2e56e448426e8f7` |
| eval (frozen, held out) | `crates/vox-orchestrator-mcp/tests/fixtures/chatarc/eval.v1.jsonl` | 29 queries | `22d8569a1b69e7dc2906e194256854452d85afb6b2c9290375935e74d7a37644` |
| calib | `crates/vox-orchestrator-mcp/tests/fixtures/chatarc/calib.v1.jsonl` | 34 queries | `c1940ac9db008ee9c12702c26b6e633b99ba919bc7b629313df7faa28a1d1c70` |

The catalog and the eval split were committed together in d2126f658, before anything else in this plan. The eval hash was recorded in `evidence/322-03-eval-freeze.txt` in the next commit, before the calib split was written and before any bin existed. `shasum -a 256 -c` on that file passes, and the report's `eval_sha256` (computed in Rust from the `include_str!` bytes) matches it. The tools are real Vox MCP tool names with their registry descriptions. The skills and commands are Vox-shaped, and their descriptions were written for the fixture. The catalog includes near-duplicates so that the margin matters: `vox_run_tests` / `vox_test_all`, `vox_kb_search` / `vox_memory_search`, `vox_spawn_agent` / `vox_submit_task`, `vox graph status` / `vox graph refresh`, `vox doctor` / `vox secrets doctor`, and the skill `vox-diagram-authoring` / the command `vox diagram check`.

| Split | none | skill | tool | command | dual_wording pairs |
| ----- | ---- | ----- | ---- | ------- | ------------------ |
| eval | 7 | 7 | 8 | 7 | 4 |
| calib | 7 | 9 | 10 | 8 | 4 |

`chatarc_fixtures_are_well_formed` enforces these properties in CI: every gold id exists in the catalog under its kind, the splits share no query, each split has at least 20 queries, at least 3 per gold kind, and at least 3 dual-wording pairs with equal gold.

Labeling convention: gold is the single catalog action that best serves the user's message. It is `none` when no catalog action fits, which covers small talk, general knowledge, and a request that no catalog action can do (e21, "book me a flight").

### Calibration (calib split only)

| Bin (margin ≥) | fired | correct | Laplace precision |
| -------------- | ----- | ------- | ----------------- |
| 0.00 | 8 | 3 | 0.40 |
| 0.25 | 8 | 5 | 0.60 |
| 0.50 | 16 | 14 | 0.83 |

Two calib queries got an empty shortlist. `CHOICE_BINS = [(0.0, 0.40), (0.25, 0.60), (0.5, 0.83)]`, and `choice_bins_match_calib_split` recomputes it in CI (M22). No catalog entry, label or bin boundary was changed after calibration.

### Published metrics (eval split, n = 29)

| Version | τ | answered | coverage@τ | risk@τ | ECE | cov@err 0.05 | cov@err 0.10 | recall@K | none_abstention | end_to_end_accuracy | dual_wording (4 pairs) | disabled |
| ------- | - | -------- | ---------- | ------ | --- | ------------ | ------------ | -------- | --------------- | ------------------- | ---------------------- | -------- |
| chatarc.v1 | 0.80 | 25 | 0.414 (12 of 29) | **0.167** | 0.010 | 0.000 | 0.000 | 0.909 | 0.714 | 0.517 | 0.25 | no |
| chatarc.v2 | 1.01 | 25 | 0.000 | 0.000 | 0.010 | 0.000 | 0.000 | 0.909 | 1.000 | 0.241 | 1.00 | **yes** |

ECE, coverage@error and recall@K do not depend on τ, so they are the same in both rows. The v2 agreement of 1.00 holds trivially, because every output is `none`.

**v1 errors at τ.** Both errors are false activations on gold-`none` queries, each in the 0.83 bin:
- e17 "thanks, that's all for now" picked `vox_test_all`, at margin 0.82. The tool's indexed text starts "vox test all", and only that one document contains the token "all".
- e21 "book me a flight to Denver next Tuesday" picked `vox_submit_task`, at margin 0.77. The match is only on function words: its description contains "a" once and "to" twice.

The 10 correct actions at τ were e04, e07, e08, e10, e14, e15, e16, e22, e24 and e26. Below τ, two other actions were wrong: e09, where `vox_submit_task` outranked `secret-management` at 0.60, and e12, where `vox_build_crate` outranked `vox graph query` at 0.40. e01 and e23 got an empty shortlist, which is a lexical miss ("committed" vs "uncommitted", "files" vs "file"). These are the two recall@K misses.

coverage@error is 0 at both targets. Every threshold that admits any action also admits e17 and e21, which sit in the top bin, so the risk is never below 0.10.

### Fail-closed decision (chatarc.v1 → v2)

The plan requires that bins, catalog and labels never be changed from eval results. So the response to the tripped gate was:
- `TAU_CHOICE = 1.01`. No calibrated confidence reaches it, so `choose_with` never returns an action, and every consumer gets exactly today's behaviour (L-08).
- `CHATARC_VERSION = "chatarc.v2"`, and the report JSON marks `disabled: true`.
- The bins stay calibrated (0.40 / 0.60 / 0.83). The mechanism (shortlist, Choice, skill injection, tool pin, command hint, provider seam, flag gate) is still covered by its own tests at explicit thresholds, and every guard in it is mutation-proven (M14 to M22).

The cause is structural, not a single bad label. A raw BM25 margin cannot tell an off-catalog request from an on-catalog one: "all" and "to" give high margins whenever only one document contains them. Raising the bar needs an out-of-catalog signal, such as an absolute-score floor or a stopword-aware tokenizer. That change would need its own calib-only fit and a fresh frozen eval split. It is recorded here as follow-up work, not attempted.

### Raw CHATARC_EVAL lines

v1 (gate tripped; `/tmp/322-03-t3-v1.log`):

```text
CHATARC_EVAL {"version":"chatarc.v1","eval_sha256":"22d8569a1b69e7dc2906e194256854452d85afb6b2c9290375935e74d7a37644","n":29,"tau":0.800000011920929,"bins":[[0.0,0.4000000059604645],[0.25,0.6000000238418579],[0.5,0.8299999833106995]],"answered":25,"coverage_at_tau":0.41379310344827586,"risk_at_tau":0.16666666666666666,"ece":0.009600014686584495,"coverage_at_error_0_05":0.0,"coverage_at_error_0_10":0.0,"recall_at_k":0.9090909090909091,"none_abstention":0.7142857142857143,"end_to_end_accuracy":0.5172413793103449,"dual_wording_agreement":{"pairs":4,"agreement":0.25},"disabled":false}
```

v2 (published; `/tmp/322-03-t3.log`; per-item list included):

```text
CHATARC_EVAL {"version":"chatarc.v2","eval_sha256":"22d8569a1b69e7dc2906e194256854452d85afb6b2c9290375935e74d7a37644","n":29,"tau":1.0099999904632568,"bins":[[0.0,0.4000000059604645],[0.25,0.6000000238418579],[0.5,0.8299999833106995]],"answered":25,"coverage_at_tau":0.0,"risk_at_tau":0.0,"ece":0.009600014686584495,"coverage_at_error_0_05":0.0,"coverage_at_error_0_10":0.0,"recall_at_k":0.9090909090909091,"none_abstention":1.0,"end_to_end_accuracy":0.2413793103448276,"dual_wording_agreement":{"pairs":4,"agreement":1.0},"disabled":true,"items":[{"id":"e02","top":"vox_run_tests","margin":0.058816198259592056,"confidence":0.4000000059604645,"correct":true},{"id":"e03","top":"vox_spawn_agent","margin":0.10031203180551529,"confidence":0.4000000059604645,"correct":true},{"id":"e04","top":"vox_kb_search","margin":0.5206109881401062,"confidence":0.8299999833106995,"correct":true},{"id":"e05","top":"vox_git_log","margin":0.3537076413631439,"confidence":0.6000000238418579,"correct":true},{"id":"e06","top":"vox_check_workspace","margin":0.14815370738506317,"confidence":0.4000000059604645,"correct":true},{"id":"e07","top":"systematic-debugging","margin":0.5512728691101074,"confidence":0.8299999833106995,"correct":true},{"id":"e08","top":"writing-plans","margin":0.8637480735778809,"confidence":0.8299999833106995,"correct":true},{"id":"e09","top":"vox_submit_task","margin":0.26995769143104553,"confidence":0.6000000238418579,"correct":false},{"id":"e10","top":"release-notes","margin":0.8364678621292114,"confidence":0.8299999833106995,"correct":true},{"id":"e11","top":"dependency-discipline","margin":0.3129673898220062,"confidence":0.6000000238418579,"correct":true},{"id":"e12","top":"vox_build_crate","margin":0.15419916808605194,"confidence":0.4000000059604645,"correct":false},{"id":"e13","top":"vox ci ssot-drift","margin":0.32036468386650085,"confidence":0.6000000238418579,"correct":true},{"id":"e14","top":"vox diagram check","margin":0.6928173303604126,"confidence":0.8299999833106995,"correct":true},{"id":"e15","top":"vox doctor","margin":0.592011034488678,"confidence":0.8299999833106995,"correct":true},{"id":"e16","top":"vox graph refresh","margin":0.531480073928833,"confidence":0.8299999833106995,"correct":true},{"id":"e17","top":"vox_test_all","margin":0.8159666061401367,"confidence":0.8299999833106995,"correct":false},{"id":"e18","top":"vox graph status","margin":0.40248963236808777,"confidence":0.6000000238418579,"correct":false},{"id":"e19","top":"test-driven-development","margin":0.0,"confidence":0.4000000059604645,"correct":false},{"id":"e20","top":"release-notes","margin":0.0620855949819088,"confidence":0.4000000059604645,"correct":false},{"id":"e21","top":"vox_submit_task","margin":0.7742431163787842,"confidence":0.8299999833106995,"correct":false},{"id":"e22","top":"vox_git_diff","margin":0.6592679023742676,"confidence":0.8299999833106995,"correct":true},{"id":"e24","top":"test-driven-development","margin":0.5151358842849731,"confidence":0.8299999833106995,"correct":true},{"id":"e25","top":"vox_test_all","margin":0.1094086691737175,"confidence":0.4000000059604645,"correct":false},{"id":"e26","top":"vox ci pre-push","margin":0.7789803743362427,"confidence":0.8299999833106995,"correct":true},{"id":"e27","top":"vox_submit_task","margin":0.19218911230564117,"confidence":0.4000000059604645,"correct":false}]}
```

### Mutation proofs (L-15)

All nine were PROVEN against committed HEAD 224b342da (driver output: `/tmp/322-03-mutations.txt`). Each guard was mutated, its named test failed with an assertion (not a compile error), and the test passed again after an exact reverse write that `git diff --quiet` confirmed. The full table is in 322-03-SUMMARY.md.

| # | Guard | Test that failed |
| - | ----- | ---------------- |
| M14 | shortlist bound `K_MAX - 1` | `shortlist_is_at_most_k_and_ends_with_none` (16 options, not 10) |
| M15 | trailing `none` option | same test (9 options, no `none`) |
| M16 | flag gate `if !enabled { return None; }` | `decide_if_enabled_off_is_none_and_gathers_nothing` (gather ran) |
| M17 | τ gate `confidence >= tau` | `choice_below_tau_is_none` (picked at 0.79) |
| M18 | explicit pin precedence | `explicit_pin_wins_over_chatarc_skill` (`("y", "chatarc")`) |
| M19 | provider-unset fail open | `provider_unset_yields_no_command_candidates` |
| M20 | pins unchanged without a pick | `turn_pin_names_unchanged_without_chatarc_tool` |
| M21 | tool candidates through `select_tools_for_turn` | `tool_candidates_respect_select_tools_for_turn_filters` (lane and `vox_chat_` leaked) |
| M22 | calibration drift | `choice_bins_match_calib_split` |

### Caveats

- **Hermetic catalog versus the real registry.** The catalog has 32 items. In production a chat turn would score several hundred candidates: the roughly 190 tools a plain turn admits, every installed skill, and every compiled-in leaf command. The bins were fit on the 32-item catalog. BM25 idf and the margins both shift with the size of the collection, so the calibration may not transfer. `real_registry_shortlist_is_bounded` checks only that the shortlist over the real `TOOL_REGISTRY` stays at most 10 options and ends with `none`. Nothing measures accuracy on the real registry.
- **The labels are agent-written.** One agent wrote the catalog descriptions, both splits and the scorer integration. The eval split was frozen before calibration. Even so, it is not an independent, human-labeled set.
- **The sample is small.** n is 29 (eval) and 34 (calib). One error in the top bin moves risk@τ by about 0.08, and the gate failed on two items.
- **Lexical recall has a ceiling.** recall@K is 0.91, and both misses are vocabulary gaps. The dual-wording agreement under v1 was 0.25: one wording of each action pair matched lexically and the other did not.
- **There is a daemon provider gap (operator decision, not planned around).** The command-hint seam lives in `chatarc.rs` (`register_command_hint_provider`), and only `vox-cli`'s `main()` registers a provider. Production chat is served by `vox-orchestrator-d`: `vox mcp` forwards `vox_chat_message` to it, and the GUI calls it too. The daemon does not depend on vox-cli, so in daemon-hosted chat the command candidates fail open to none, while skills and tools still work. Commands work only in in-process hosts, such as `vox harness eval` live runs. Closing the gap needs either a new crate edge (USER-AUTHORIZED-ONLY) or a subprocess call to `vox commands --json` (a new exec trust boundary).
- **The reliability prior was not read (L-09 "may").** Nothing writes skill reliability rows yet, so the prior would be a constant.
- **The flag is off by default** (`VOX_CHATARC`). With the flag off, nothing is gathered (M16), and the no-pin system prompt equals the golden captured from the unmodified builder (`evidence/322-03-system-prompt-golden.txt`). With the flag on, v2 still never acts.
