---
phase: 322-calibrated-turn-decisions-and-dynamic-action-catalog
fixed_at: 2026-10-30T10:30:00Z
review_path: .planning/phases/322-calibrated-turn-decisions-and-dynamic-action-catalog/322-REVIEW.md
iteration: 1
findings_in_scope: 7
fixed: 7
skipped: 0
deferred: 9
status: all_fixed
---

# Phase 322: Code Review Fix Report

**Fixed at:** 2026-10-30
**Source review:** .planning/phases/322-calibrated-turn-decisions-and-dynamic-action-catalog/322-REVIEW.md
**Iteration:** 1
**Scope:** the operator selected 7 of the 16 findings: CR-01, CR-02, WR-01, WR-02, WR-04, WR-05 and WR-09. The other 9 are deferred and listed below.

**Summary:**
- Findings in scope: 7
- Fixed: 7 (CR-01 and WR-01 share one commit, because one field fixes both)
- Skipped: 0
- Deferred (out of scope by operator choice): 9

**Where verification ran:** in the main checkout `/Users/brbrainerd/dev/vox-mens-compare` on branch `gsd/mens-compare-316`, not in a worktree. The operator's hard rules required this. Every cargo call ran one at a time under `nice -n 19 timeout`.

## Status of every review finding

| Finding | Severity | Status | Commit |
| ------- | -------- | ------ | ------ |
| CR-01 | Critical | fixed | 80de04797 |
| CR-02 | Critical | fixed | d145eac0b |
| WR-01 | Warning | fixed | 80de04797 |
| WR-02 | Warning | fixed | f5ce89a9b |
| WR-03 | Warning | deferred | none |
| WR-04 | Warning | fixed | 90d6d6119 |
| WR-05 | Warning | fixed (`cargo check -p vox-cli` only) | dee54b4a5 |
| WR-06 | Warning | deferred | none |
| WR-07 | Warning | deferred, partly addressed by CR-01 | none |
| WR-08 | Warning | deferred | none |
| WR-09 | Warning | fixed | 8ccf1bbec |
| IN-01 | Info | deferred | none |
| IN-02 | Info | deferred | none |
| IN-03 | Info | deferred | none |
| IN-04 | Info | deferred | none |
| IN-05 | Info | deferred | none |

## Fixed Issues

### CR-01 and WR-01: the pack and ChatARC read @mention file text; allow_free acted at every resolver call site

**Files modified:** `model_route_policy/types.rs`, `model_route_policy/resolve.rs`, `model_route_policy/tests.rs`, `models_tools.rs`, `chat_tools/chat/message.rs`, `chat_tools/chat/message_turn_trace_tests.rs`, `chat_tools/chatarc.rs` (doc comment only)
**Commit:** 80de04797
**Applied fix:**
- `McpChatModelResolution` has a new field, `decision_prompt: Option<String>`, which defaults to `None`.
- The turn pack in `resolve.rs` now reads only `decision_prompt`. It never reads `capability_prompt`, whose Task 8f contract is the post-expansion text, and it never reads the `user_prompt` fallback.
- `chat_message` derives `decision_text` with `turn_decision_text(raw_prompt, &intent)`. That is `params.prompt` before `@mention` expansion and canonicalization, or a research slash command's query. The same text goes to `chatarc::pick_for_turn` and to a new `try_run_agent_turn` parameter.
- `try_run_agent_turn` is the only place that sets `decision_prompt`.

**Design for WR-01:** a `Some` `decision_prompt` is itself the pack's opt-in. No caller other than the default chat turn sets it: not browser tools, not t3 consensus, not the cognitive-profile path, not `call_llm_with_pref`, not `models_tools`. All of those get no pack and today's request.

This was the smallest correct design. A separate `allow_free_unset` flag would have duplicated the gate, because the review requires a raw-text field for CR-01 anyway. The comments that claimed "scored on the user's own message" are corrected in message.rs, chatarc.rs and resolve.rs.

**Tests (RED → GREEN):**
- `pack_scores_decision_prompt_not_mention_expanded_text`: the capability prompt is mention-expanded text whose file body says "cheap … any free model", and the decision prompt is "Summarize @notes.md". The free model stays excluded. RED gave `Ok("free-model")`.
- `resolution_without_decision_prompt_is_never_patched`: `capability_prompt` is set (c17) but there is no marker, so the free model stays excluded. RED gave `Ok("free-model")`.
- `turn_decision_text_is_the_users_own_words`: a real `resolve_mentions` and `canonicalize_prompt` run shows that the expanded text carries "free model" and the "Objectives" header, while the decision text is the raw prompt. For `/research …` the decision text is the query. RED failed on a `todo!()` scaffold.

**Mutations:**
- M-CR01: `match res.decision_prompt.as_deref()` → `match res.capability_prompt.as_deref()`. `pack_scores_decision_prompt_not_mention_expanded_text` FAILED.
- M-WR01: `match res.decision_prompt.as_deref()` → `.or(res.capability_prompt.as_deref())`. `resolution_without_decision_prompt_is_never_patched` FAILED.

**Not mutation-covered:** which variable `chat_message` passes at its two call sites. No test drives `chat_message` with a mentioned file and observes the pack. The helper test pins what the text is, and the resolve test pins what the pack reads.

### CR-02: negated or conflicting free-tier requests became allow-free

**Files modified:** `model_route_policy/turn_pack.rs`, `model_route_policy/turn_pack_tests.rs`
**Commit:** d145eac0b
**Applied fix:**
- `TurnPack.free_veto` is set from firings alone, in either of two cases: any deny-side AllowFreeTier rule fires (it loses the argmax), or a phrase in the `FREE_NEGATIONS` lexicon fires ("don t use", "do not use", "not a free", "no free", "avoid", and others).
- `selection_inputs` requires `!pack.free_veto`.
- Rule answers and confidences are unchanged.
- `PACK_VERSION` is now `turn-pack.v3`.

**Test:** `negated_or_deny_side_free_cues_veto_allow_free`. The controls "Use a free model for this." and "Quick question: what is a slice?" still allow free routing. Three prompts are vetoed: "Please don't use a free model for this.", "Do not use the free tier." and "Quick question, but use the best model.". RED failed on the first vetoed prompt.

**Mutation M-CR02:** the veto line → `false && (…)`. The test FAILED on "Please don't use a free model for this.".

### WR-02: credential-bearing prompts could become free-tier eligible

**Files modified:** `model_route_policy/turn_pack.rs`, `model_route_policy/turn_pack_tests.rs`
**Commit:** f5ce89a9b
**Applied fix:** `free_veto` is also set in two cases:
- A `SECRET_CUES` word fires: api key, password, token, private key, secret, credential(s), ssn, sk, bearer, and others.
- `vox_redact::redact_owned(prompt) != prompt`. This is the existing high-precision redactor: `sk-`, `ghp_`, `AKIA`, JWT, bearer and `api_key=` values, emails, private IPs and home paths. `vox-redact` was already a dependency, so no new crate edge was added.

**Test:** `secret_shaped_text_vetoes_allow_free` covers an API key with `sk-proj-…`, a password, a "token" question, and a bare `AKIAIOSFODNN7EXAMPLE` that only the redactor catches. RED failed on the API-key prompt.

**Mutation M-WR02:** the secret line → `|= false && (…)`. The test FAILED on the API-key prompt.

**Not done:** the review's other WR-02 point, that the labeling convention requires complexity < 2 for allow-free while runtime never checks complexity. It is listed under deferred items.

### WR-04: the Stakes veto depended on argmax order and on the τ sentinel

**Files modified:** `model_route_policy/turn_pack.rs`, `model_route_policy/turn_pack_tests.rs`
**Commit:** 90d6d6119
**Applied fix:**
- `TurnPack.stakes_veto` is set whenever any stakes-2 rule fires.
- `selection_inputs` vetoes on it directly. The old `high_stakes` expression, which read the argmax winner and `stakes_tau > 1.0`, is deleted.
- Two existing tests now set `stakes_veto` instead of a stakes-2 answer: `allow_free_blocked_by_high_stakes_or_privacy` and `disabled_questions_fail_closed`.

**Test:** `stakes_veto_holds_when_a_stakes_0_rule_outranks_it`. In this test a stakes-0 rule at 0.90 wins the argmax over a stakes-2 rule at 0.60, and the veto still fires. It fires under the production τ table and also with Stakes enabled at 0.85. RED failed with `!allows(tau, risky)`.

**Mutation M-WR04:** `fired(Question::Stakes, 2)` → `pack.stakes.is_some_and(|a| a.value == 2)` (argmax again). The test FAILED.

### WR-05: the vox-cli command-hint provider blocked a tokio worker

**Files modified:** `crates/vox-cli/src/main.rs`
**Commit:** dee54b4a5
**Applied fix:**
- The first provider call starts one named background build on a 32 MiB thread, guarded by `std::sync::Once`, and returns no hints.
- Later calls return `HINTS.get().cloned().unwrap_or_default()` and never block.
- A spawn failure logs `tracing::warn!`, and so does a panicking build (caught with `catch_unwind`).
- The build stays on first use rather than at registration, so a plain `vox` invocation still builds nothing. The review noted that registration was cheap and was not the problem.

**Verification:** `cargo check -p vox-cli` passed with 0 warnings (`/tmp/322-fix-cli.log`), as the operator required. There is no test and no mutation proof.

**Not done:** the review's suggestion to skip `apply_capability_ids` in the hint walk.

### WR-09: VOX_CHATARC=1 added cost and had no effect

**Files modified:** `chat_tools/chatarc.rs`, `chat_tools/chatarc_tests.rs`, `crates/vox-config/src/config_registry.rs`, `crates/vox-config/src/operator_registry.rs`, `contracts/config/env-vars.v1.yaml`
**Commit:** 8ccf1bbec
**Applied fix:**
- `decide_if_enabled` returns `None` before gathering whenever `tau > 1.0`. That means no skill list, no `select_tools_for_turn`, no command catalog and no BM25 build.
- The `VOX_CHATARC` config hint, the operator description and the env-var contract now say the choice is currently disabled pending recalibration (chatarc.v2) and never acts.

**Test:** `decide_if_enabled_disabled_tau_gathers_nothing`, where an enabled ChatARC at τ 1.01 has a panicking gather. RED: "gathered candidates while Choice is disabled".

**Mutation M-WR09:** `if tau > 1.0 {` → `if false && tau > 1.0 {`. The test FAILED with the gather panic.

**Not regenerated:** `docs/src/reference/env-vars.generated.md` and `config-keys.generated.md` were not hand-regenerated. CI `ssot-autoregen` owns them, per AGENTS.md.

## Mutation procedure

The driver was `scratchpad/mutate.py` plus `mutations.sh`, and its output is in `/tmp/322-fix-mutations.txt`. It ran against committed HEAD dee54b4a5. For each mutation:
1. grep the guard (count 1).
2. Apply the mutation by exact write, and grep again (count 0).
3. Run the named test. It must FAIL with an assertion or panic, not a compile error.
4. Restore by exact reverse write, and grep again (count 1).
5. Check that `git diff --quiet -- <file>` is clean. `git checkout` was never used.

Each restored test was re-run inside the next mutation's build, and it passed there. The last one (WR-09) was re-run in the final run. All six mutations were PROVEN.

## Eval re-run

- `selective_eval_report` and `rule_confidences_match_calib_split` both pass. TURN_PACK_EVAL is `turn-pack.v3`, and every per-question metric, both agreement tables and `patched_turns` = 1 are identical to v2. The vetoes do not change any rule answer, so no confidence needed re-derivation.
- `shasum -a 256 -c` reports OK for both `evidence/322-02-eval-freeze.txt` and `evidence/322-03-eval-freeze.txt`.
- No rule, label or eval line was edited.
- The details and the raw v3 line are in 322-EVAL-REPORT.md under "Review fixes".

## Deferred Issues (not selected by the operator)

### WR-03: PrivacyLocal cannot keep a turn local

**File:** `resolve.rs:387-388, 408, 495-508, 527-533`; `turn_pack.rs`
**Reason:** deferred, not in the selected set.
**Original issue:** the VoxLocal shortcut and the capability pin run before the pack, on the caller's CodeGen placeholder. A confident privacy answer therefore changes no locality, and `prefer_local` is only a scorer hint. Since v3 the privacy answer still vetoes free routing, so in practice it is a free-tier veto only.

### WR-06: a ChatARC skill pick is weaker than an explicit pin and easy to squat

**File:** `message.rs` (`active_skill_id`); `chatarc.rs:162-197`; `chat_tools/mod.rs:245-248`
**Reason:** deferred. It is latent while `TAU_CHOICE = 1.01`, and since WR-09 ChatARC does not even gather candidates.
**Original issue:** a ChatARC pick leaves `active_skill_id = None`. There is no absolute BM25 floor. The id-or-name lookup can inject a different skill.

### WR-07: calibration and serving inputs differ

**File:** `message.rs`; `chatarc_eval_tests.rs`; `turn_pack_eval_tests.rs`
**Reason:** deferred, though CR-01 removes the main skew. Production now feeds the pack and ChatARC the raw user message, which is the same distribution as the raw fixture strings. The review's suggested test, that the string passed equals `params.prompt` for a non-command turn, is only partly covered: `turn_decision_text_is_the_users_own_words` pins the helper, but no test pins the call-site wiring.

### WR-08: the production command-hint seam is never tested

**File:** `chatarc.rs:268-299`
**Reason:** deferred, not in the selected set.

### IN-01: rules that can never affect production output

**Reason:** deferred. Since CR-02, `allow_free_tier.0.1` is consumed as a veto. `complexity.*` and `stakes.0.1` remain eval-only.

### IN-02: `ChatArcPick.confidence` is never read outside tests

**Reason:** deferred.

### IN-03: command-hint sanitization covers only `\r` and `\n`

**Reason:** deferred.

### IN-04: process-env flag set in thread-parallel unit tests

**Reason:** deferred. The new resolve tests follow the existing pattern: they take `INFERENCE_PROFILE_TEST_LOCK` and set `VOX_TURN_DECISION_PACK` through `EnvKeyGuard`.

### IN-05: command candidates exist only when vox-cli hosts the server

**Reason:** deferred.

### WR-02 remainder: the allow-free convention's complexity < 2 is not enforced at runtime

**Reason:** deferred, outside the fix the operator specified. Enforcing it needs a confident complexity answer, and Complexity answers only one eval turn.

---

_Fixed: 2026-10-30_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
