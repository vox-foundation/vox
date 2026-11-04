---
phase: 322-calibrated-turn-decisions-and-dynamic-action-catalog
verified: 2026-10-30T16:30:00Z
status: gaps_found
score: 2/4 roadmap success criteria verified (requirements: 3 satisfied, 2 partial, 1 not met)
covered_files:
  - ".planning/phases/322-calibrated-turn-decisions-and-dynamic-action-catalog/322-01-PLAN.md"
  - ".planning/phases/322-calibrated-turn-decisions-and-dynamic-action-catalog/322-01-SUMMARY.md"
  - ".planning/phases/322-calibrated-turn-decisions-and-dynamic-action-catalog/322-02-PLAN.md"
  - ".planning/phases/322-calibrated-turn-decisions-and-dynamic-action-catalog/322-02-SUMMARY.md"
  - ".planning/phases/322-calibrated-turn-decisions-and-dynamic-action-catalog/322-03-PLAN.md"
  - ".planning/phases/322-calibrated-turn-decisions-and-dynamic-action-catalog/322-03-SUMMARY.md"
  - "contracts/config/env-vars.v1.yaml"
  - "crates/vox-cli/src/main.rs"
  - "crates/vox-config/src/config_registry.rs"
  - "crates/vox-config/src/inference.rs"
  - "crates/vox-config/src/operator_registry.rs"
  - "crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs"
  - "crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs"
  - "crates/vox-orchestrator-mcp/src/chat_tools/chat/message_turn_trace_tests.rs"
  - "crates/vox-orchestrator-mcp/src/chat_tools/chatarc.rs"
  - "crates/vox-orchestrator-mcp/src/chat_tools/chatarc_eval_tests.rs"
  - "crates/vox-orchestrator-mcp/src/chat_tools/chatarc_tests.rs"
  - "crates/vox-orchestrator-mcp/src/chat_tools/mod.rs"
  - "crates/vox-orchestrator-mcp/src/lib.rs"
  - "crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/mod.rs"
  - "crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/resolve.rs"
  - "crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/tests.rs"
  - "crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/turn_pack.rs"
  - "crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/turn_pack_eval_tests.rs"
  - "crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/turn_pack_tests.rs"
  - "crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/types.rs"
  - "crates/vox-orchestrator-mcp/src/models_tools.rs"
  - "crates/vox-orchestrator-mcp/src/selective_metrics.rs"
  - "crates/vox-orchestrator-mcp/src/skill_search_index.rs"
  - "crates/vox-orchestrator-mcp/tests/fixtures/chatarc/calib.v1.jsonl"
  - "crates/vox-orchestrator-mcp/tests/fixtures/chatarc/catalog.v1.json"
  - "crates/vox-orchestrator-mcp/tests/fixtures/chatarc/eval.v1.jsonl"
  - "crates/vox-orchestrator-mcp/tests/fixtures/chatarc/system_prompt_golden.txt"
  - "crates/vox-orchestrator-mcp/tests/fixtures/turn_pack/calib.v1.jsonl"
  - "crates/vox-orchestrator-mcp/tests/fixtures/turn_pack/eval.v1.jsonl"
covered_digest: "v3:sha256:06caa6f223a87e1bb662c5553e592ebeaabeea05b008fde426b92c32b653f223"
behavior_unverified: 0
overrides_applied: 0
gaps:
  - truth: "Post-tool low confidence reuses apply_repair_escalation_to_config; stop/verify stay on evaluate_stop_hook / evaluate_post_tool_batch_hook (ROADMAP SC3, DEC-02)"
    status: failed
    reason: "None of the three anchor functions exists at HEAD or on any git ref. No post-tool decision, model_still_ok signal, or re-entry into repair escalation was built. The phase recorded this as blocked (RESEARCH F1) rather than delivering it."
    artifacts:
      - path: "crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs"
        issue: "No evaluate_post_tool_batch_hook / evaluate_stop_hook / apply_repair_escalation_to_config; no post-tool pack epoch. SelectionIntent::repair_loop() is called only by vox-cli repair.rs:211 and gui_ops/harness.rs:205, neither a chat post-tool hook."
    missing:
      - "The agent_loop/{batch_and_eval_gate.rs,lifecycle_hooks.rs} refactor (currently untracked in another agent's Cursor checkout) landing on a ref this branch can merge, or an operator decision to build the hooks here or descope DEC-02"
      - "A post-tool pack epoch whose low model_still_ok re-enters apply_repair_escalation_to_config, with tests"
  - truth: "Flag-off identity tests cover build_selection_request, resolve_mens_fast_apply_target, consult_oracle_model_advisory and apply_repair_escalation_to_config (DEC-03)"
    status: partial
    reason: "Only build_selection_request is covered (goldens G1..G5, captured from unmodified code in 517d55ebe and mutation-proven by M1). The other three functions do not exist on any ref, so they have no identity tests. Same root cause as the DEC-02 gap."
    artifacts:
      - path: "crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/turn_pack_tests.rs"
        issue: "Covers build_selection_request only (GOLDEN_G1..G5, lines 18-22)"
    missing:
      - "Flag-off identity goldens for resolve_mens_fast_apply_target, consult_oracle_model_advisory and apply_repair_escalation_to_config once those functions exist on this branch"
  - truth: "ChatARC calibrated Choice is consumed by an activation path (ROADMAP SC4, CAT-02)"
    status: partial
    reason: "The consumers are wired and mutation-proven (skill injection mod.rs:245, tool pin agent_loop.rs:743/855, command hint message.rs:942). But the shipped chatarc.v2 has TAU_CHOICE = 1.01 and the largest calibrated bin confidence is 0.83 (chatarc.rs CHOICE_BINS), so choose_with can never return Some. With VOX_CHATARC on, pick_for_turn gathers and scores candidates and then always returns None. No activation path ever receives a choice in production. This is a correct fail-closed response to a failed held-out gate (chatarc.v1 risk@tau 0.167 > 0.10), but the calibrated choosing the phase goal describes does not happen."
    artifacts:
      - path: "crates/vox-orchestrator-mcp/src/chat_tools/chatarc.rs"
        issue: "TAU_CHOICE = 1.01 with max bin confidence 0.83: Choice is permanently inert"
    missing:
      - "A ChatARC scorer that passes a fresh frozen held-out split at risk@tau <= 0.10 (the report proposes an absolute-score floor or a stopword-aware tokenizer), re-fit CHOICE_BINS on calib, and TAU_CHOICE below 1.0"
      - "Or an operator override accepting 'wired + fail-closed after gate' as meeting CAT-02 (see the override suggestion in the report body)"
human_verification:
  - test: "Decide the daemon command-hint gap: vox-orchestrator-d hosts vox_chat_message for `vox mcp` and the GUI but registers no CommandHintProvider, so ChatARC has no command candidates there."
    expected: "Operator picks: accept the gap, authorize a crate edge (USER-AUTHORIZED-ONLY), or approve a subprocess call to `vox commands --json`"
    why_human: "Closing it needs a crate-edge exception or a new exec trust boundary. Both need the operator's authorization."
  - test: "Decide DEC-02 / DEC-03 disposition: wait for the Cursor checkout's agent_loop refactor, build the hooks on this branch, or descope."
    expected: "A recorded decision; until then the phase cannot be marked complete against DEC-02"
    why_human: "The dependency is another agent's uncommitted work, which this phase must not copy."
---

# Phase 322: Calibrated turn decisions and dynamic action catalog Verification Report

**Phase Goal:** Fully plumb System One / RLCD-style calibrated decisions into the live model-route and agent-loop control points (no facade). ChatARC composes the catalogs the harness already discovers (skills, MCP, clap commands) and chooses among shortlists.
**Verified:** 2026-10-30 (HEAD 7a2256588, worktree for all phase crates equals HEAD)
**Status:** gaps_found
**Re-verification:** No (initial verification)

## Goal Achievement

The goal is half delivered. The model-route half is real and live. The turn pack runs on the default chat turn (`message.rs:284-285` marks `task_unset`/`complexity_unset`), flag-off identity is pinned to goldens captured from unmodified code, and the clutch and allow-free guards are mutation-proven. The agent-loop half (post-tool decisions) was not built, because its anchors do not exist on any ref. ChatARC composes all three catalogs and is wired into all three consumers, but the shipped calibration can never choose.

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
| - | ----- | ------ | -------- |
| 1 | The turn-start pack feeds `build_selection_request` without overriding clutch-derived axes, or `allow_free` when a clutch is set. Flag off matches today's request byte for byte on fixtures. | VERIFIED | `resolve.rs:510-533`: pack is `None` unless `turn_decision_pack_enabled()` and `capability_prompt` is set, then `turn_pack::turn_selection_request`. `turn_pack.rs:210-216`: `may_set_allow_free = res.clutch.is_none()`, axes still built by `build_selection_request` from `res.clutch`/`res.risk`. Tests passed in this run: `flag_off_request_matches_golden_g1..g5` (Debug strings, `turn_pack_tests.rs:18-22`), `allow_free_never_set_when_clutch_set`, `source_policy_clutch_blocks_pack_allow_free`, `pack_off_keeps_free_model_excluded_under_performance`, `flag_on_without_capability_prompt_builds_no_pack`. Goldens in `evidence/322-01-goldens-before.txt` were committed in 517d55ebe, before any source edit. |
| 2 | `allow_free_in_performance_mode` is documented and tested as scorer eligibility only. The `free_floor` slug order stays in `bootstrap_inference` / `FREE_FLOOR`. | VERIFIED | `turn_pack.rs` doc comment at the `allow_free` computation: "Scorer eligibility only ... never reads or reorders the free-floor list". Non-comment matches of `FREE_FLOOR|FALLBACK_MODELS|PreferFree|SelectionPolicy|size_select|mesh_select|SpokeRouter` in turn_pack.rs: 0. Tests passed: `free_floor_order_is_pinned`, `allow_free_pack_changes_only_scorer_eligibility`, `pack_on_lets_free_model_compete_under_performance`. |
| 3 | Post-tool low confidence reuses `apply_repair_escalation_to_config`; stop/verify stay on `evaluate_stop_hook` / `evaluate_post_tool_batch_hook`. | FAILED | `git grep` at HEAD over `crates/` finds none of the three symbols. `git log --all -S <symbol>` for each one hits only docs commits (a6fadad43, 286cf0b18, a1d4245c7, 88be7b8bc, ce87f116a), and `-- '*.rs'` returns nothing. No `post_tool`, `stop_hook` or `model_still_ok` exists in `vox-orchestrator-mcp/src`. Nothing was built. |
| 4 | The ChatARC shortlist Choice is plumbed into an existing activation path and stays correct when a new skill, MCP tool or clap command appears. Selective-prediction metrics are published. Hosted Jev sits behind local-only privacy. | PARTIAL (gap) | The plumbing is real: `message.rs:935-957` (pick, then the skill arg and command hint), `message.rs:1162` (tool pin), `mod.rs:245` (`skill_to_inject`, explicit pin wins), `agent_loop.rs:743,855` (`turn_pin_names`). Discovery tests passed: `new_skill/tool/command_appears_on_next_discovery_pass`. Metrics were published (`322-EVAL-REPORT.md`, `chatarc_eval_report`). However, `TAU_CHOICE = 1.01` against a max bin confidence of 0.83 means the Choice never fires. Hosted Jev was not built (excluded by L-11/F9), so the privacy clause is vacuous, not demonstrated. |

**Score:** 2/4 roadmap truths verified (0 present-but-behavior-unverified).

### Requirements Coverage

| Requirement | Source Plan | Verdict | Evidence |
| ----------- | ----------- | ------- | -------- |
| DEC-01 | 322-01, 322-02 | SATISFIED | The pack runs before the builder (`resolve.rs:510-533`), fills only caller-unset fields (`turn_pack.rs:210-216`, guards M2-M5 mutation-proven), sets allow_free only when the clutch is unset, and emits no model ids (`TurnPack` carries only rule ids). Live caller: `message.rs:284-285`. Caveat: as calibrated in v2, `task` never patches (TaskShape disabled) and `complexity` never patches (rule confidences 0.40/0.50 < TAU_APPLY 0.80). Only AllowFreeTier (0.86 >= 0.85) and PrivacyLocal (0.80 >= 0.80) can act, with 1 of 28 eval turns patched. That is within "patches only", but the shipped pack's live effect is narrow. |
| DEC-02 | none (blocked) | NOT MET | Anchors absent from every ref (see truth 3). No post-tool decision, no repair re-entry. |
| DEC-03 | 322-01 | PARTIAL | `build_selection_request` flag-off identity: goldens G1..G5 pass and M1 is mutation-proven. `resolve_mens_fast_apply_target`, `consult_oracle_model_advisory` and `apply_repair_escalation_to_config` do not exist, so they have no identity tests. |
| DEC-04 | 322-02 | SATISFIED | `selective_eval_report` publishes accuracy, ECE, coverage@tau, risk@tau and coverage@error 0.05/0.10 per question on the frozen n=28 split, and fails if an enabled question's risk is above 0.10 (`turn_pack_eval_tests.rs:310-312`). TaskShape and Stakes were disabled after failing the gate (risk 0.25 each), and disabled Stakes fails closed as a veto (`disabled_questions_fail_closed`). `selective_metrics` is `#[cfg(test)]` (`lib.rs:132-133`), so there is no unused facade. No MENS distill. No SIZE-02/MESH-01 references. Hosted Jev was not built, so its privacy gate is vacuously honored. Caveat: answered counts are tiny (1-13 per question). |
| CAT-01 | 322-03 | SATISFIED (warning) | `pick_for_turn` (chatarc.rs) composes `state.orchestrator.skill_registry.list` minus exclusions, the `select_tools_for_turn` output over `TOOL_REGISTRY` (M21 proves it is the filtered set), and `registered_command_hints()` fed by vox-cli's `command_catalog::build_catalog` (`vox-cli/src/main.rs:57-84`). Candidates are rebuilt every turn and there is no third registry. Warnings: (a) "deferred catalog hints" do not exist on this branch (RESEARCH F6); (b) `vox-orchestrator-d`, the host for `vox mcp` and GUI chat, registers no provider, so command candidates are absent on the main chat path. |
| CAT-02 | 322-03 | PARTIAL | The top-K is bounded (K_MAX 10, `none` always last, `shortlist_is_at_most_k_and_ends_with_none`, M14/M15). It fails open (`decide_if_enabled_off_is_none_and_gathers_nothing`, M16). The install gate is untouched and the reliability prior is not read ("may"). The consumers are wired, but the calibrated gate never fires (TAU_CHOICE 1.01 > max bin 0.83), so in production nothing is ever "consumed". |

No orphaned requirements: REQUIREMENTS.md maps exactly DEC-01..04 and CAT-01..02 to this phase. DEC-02 is claimed by no plan; it was recorded as blocked in 322-01-SUMMARY.

### Override suggestion (CAT-02 / SC4)

The CAT-02 shortfall looks intentional and principled: the phase shipped a correct fail-closed response to a failed held-out gate. If the operator accepts "consumer wired and mutation-proven; calibrated gate disabled after a failed held-out eval" as meeting CAT-02, add to this file's frontmatter:

```yaml
overrides:
  - must_have: "ChatARC calibrated Choice is consumed by an activation path (ROADMAP SC4, CAT-02)"
    reason: "Consumers wired and mutation-proven (M14-M22); chatarc.v1 failed the frozen held-out risk gate (0.167 > 0.10) so v2 fails closed with TAU_CHOICE 1.01; a better scorer is follow-up work"
    accepted_by: "<operator>"
    accepted_at: "<ISO timestamp>"
```

DEC-02 and DEC-03 should not be overridden this way. They are undelivered, not deliberately disabled.

### Required Artifacts

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `llm_bridge/model_route_policy/turn_pack.rs` | Versioned pack, calibrated rules, tau table | VERIFIED | `turn-pack.v2`, 11 rules with calib confidences, `tau_for`, `selection_inputs`, `turn_selection_request` called from resolve.rs |
| `llm_bridge/model_route_policy/turn_pack_tests.rs` / `turn_pack_eval_tests.rs` | Goldens, guards, eval | VERIFIED | All named tests pass in this run |
| `selective_metrics.rs` | Metrics helpers | VERIFIED | `#[cfg(test)]`, used by both eval reports |
| `chat_tools/chatarc.rs` | Shortlist, Choice, consumers, provider seam | VERIFIED as code; WIRED; Choice inert | See CAT-02 |
| `vox-cli/src/main.rs` provider | Command catalog into ChatARC | WIRED (vox-cli process only) | Not registered in vox-orchestrator-d |
| `vox-config` flags `VOX_TURN_DECISION_PACK`, `VOX_CHATARC` | Default-off flags, SSOT rows | VERIFIED | `inference.rs:450-456`; `turn_decision_pack_flags_parse_truthy`, `chatarc_flags_parse_truthy`, `every_operator_env_is_in_config_keys` pass |
| Post-tool hook extensions | DEC-02 | MISSING | Anchors absent on every ref |

### Key Link Verification

| From | To | Via | Status |
| ---- | -- | --- | ------ |
| `message.rs` try_run_agent_turn | turn pack | `task_unset: true, complexity_unset: true` + `capability_prompt` (message.rs:279-285) | WIRED |
| `resolve.rs` decide site | `turn_pack::turn_selection_request` → `build_selection_request` | resolve.rs:510-533 | WIRED (flag-gated, M8) |
| `message.rs` chat turn | `chatarc::pick_for_turn` | message.rs:935 | WIRED (flag-gated; Choice inert) |
| ChatARC pick | skill injection | `build_system_prompt_with_skill(.., chatarc_skill)` → `skill_to_inject` (mod.rs:245) | WIRED |
| ChatARC pick | tool offering | `tool_pin_for` → `try_run_agent_turn` → `turn_pin_names` (agent_loop.rs:855) | WIRED, still filtered (`chatarc_tool_pin_is_still_filtered`) |
| ChatARC pick | command hinting | `append_command_hint` (message.rs:942) | WIRED |
| vox-cli `main()` | ChatARC command source | `register_command_hint_provider(chatarc_command_hints)` (main.rs:84) | WIRED in vox-cli only |
| vox-orchestrator-d | ChatARC command source | none | NOT WIRED (accepted by plan as operator decision) |
| Post-tool hook | `apply_repair_escalation_to_config` | n/a | NOT WIRED (functions absent) |

### Data-Flow Trace (Level 4)

| Artifact | Data | Source | Produces real data | Status |
| -------- | ---- | ------ | ------------------ | ------ |
| turn pack → SelectionIntent | `allow_free`, `prefer_local` | `evaluate(capability_prompt)` on the user's own message | Yes, for AllowFreeTier and PrivacyLocal; task/complexity never patch under v2 | FLOWING (narrow) |
| ChatARC pick → consumers | `ChatArcPick` | live skill registry + filtered TOOL_REGISTRY + provider | Candidates and scores yes; pick always `None` (TAU_CHOICE 1.01) | STATIC (constant None in production) |

### Behavioral Spot-Checks (re-run by the verifier)

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| Turn pack, routing, ChatARC, metrics, skill index | `nice -n 19 timeout 3000 cargo test -p vox-orchestrator-mcp --lib -- turn_pack model_route_policy chat_tools selective_metrics skill_search_index` (log /tmp/322-verify.log) | `test result: ok. 269 passed; 0 failed; 0 ignored; 719 filtered out` (92.70 s), exit 0 | PASS |
| vox-config flags and registries | `nice -n 19 timeout 1800 cargo test -p vox-config --lib` (log /tmp/322-verify-config.log) | `test result: ok. 143 passed; 0 failed`, exit 0 | PASS |
| Turn-pack eval freeze | `shasum -a 256 -c evidence/322-02-eval-freeze.txt` | `turn_pack/eval.v1.jsonl: OK` | PASS |
| ChatARC eval freeze | `shasum -a 256 -c evidence/322-03-eval-freeze.txt` | `chatarc/eval.v1.jsonl: OK` | PASS |
| Eval splits untouched after freeze | `git log 85afc56db^..HEAD -- turn_pack/eval.v1.jsonl`; `git log d2126f658^..HEAD -- chatarc/eval.v1.jsonl catalog.v1.json` | One commit each (the freeze commit) | PASS |
| System-prompt golden copy | SHA-256 of evidence golden vs `tests/fixtures/chatarc/system_prompt_golden.txt` at HEAD | both `8acd0e37...f1` | PASS |
| No Cargo manifest change | `git log 517d55ebe^..HEAD -- '*Cargo.toml' Cargo.lock` | empty (including the interleaved 320 commits) | PASS |

### Probe Execution

Step 7c: not applicable. No probes are declared by the phase.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| chatarc.rs, turn_pack.rs | none | TBD/FIXME/XXX/TODO | none | 0 matches |
| `chat_tools/chatarc.rs` | `TAU_CHOICE` | Feature constant that makes the whole Choice path unreachable in production | Warning | Documented fail-closed, not a stub. Code that runs every turn with the flag on (gathers and scores) yet can never act. |
| `resolve.rs` | 494-505 vs 510 | The VoxLocal pre-route uses the caller's task before the pack runs | Info | Moot while TaskShape is disabled. If TaskShape is re-enabled, a patched task will not reach the VoxLocal shortcut. |

### Info

- The research doc `docs/src/architecture/jev-system-one-harness-integration-research-2026.md` is corrupted at a1d4245c7 (RESEARCH F10, lean-ctx omission markers). This was not verified further here and is out of scope for this phase's code.
- The 320 commits interleaved in the range (b4d886c22, 04df7d397, 92799a531) touch no 322 file and no Cargo manifest.

### Human Verification Required

1. **Daemon command-hint gap.** Test: run chat through `vox mcp` or the GUI with `VOX_CHATARC=1`. Expected: no command candidates (by design today). Decision needed: accept, authorize a crate edge, or approve a subprocess bridge. Why human: both fixes are USER-AUTHORIZED-ONLY boundaries.
2. **DEC-02/DEC-03 disposition.** Wait for the Cursor `agent_loop` refactor to land, build the hooks here, or descope. Why human: the dependency is another agent's uncommitted work.

### Gaps Summary

There are two root causes.

**A. Missing upstream agent-loop hooks (DEC-02 NOT MET, DEC-03 PARTIAL, SC3 FAILED).** `apply_repair_escalation_to_config`, `evaluate_stop_hook`, `evaluate_post_tool_batch_hook`, `resolve_mens_fast_apply_target` and `consult_oracle_model_advisory` exist in no git object on this repository (`-S` hits are docs only). So the "agent-loop control points" half of the goal is undelivered. The phase correctly refused to fork parallel copies. That makes this a blocked dependency, not a sloppy miss, but it is still a gap. No later roadmap phase covers it, so nothing is deferred.

**B. ChatARC Choice is permanently inert (CAT-02 PARTIAL, SC4 PARTIAL).** The composition, consumers and fail-open paths are real and mutation-proven. The calibrated Choice failed its frozen held-out gate, and the shipped `TAU_CHOICE = 1.01` exceeds every reachable confidence (max bin 0.83). So "chooses among shortlists" never happens. This is the honest outcome of the gate. If the operator accepts it, use the override above. Otherwise a new scorer with a fresh frozen split is needed.

What is solid: DEC-01, DEC-04 and SC1/SC2 have strong evidence. That evidence is byte-identity goldens captured before the change, mutation proofs M1-M12, eval splits frozen and hash-verified before calibration, failing questions disabled rather than refit, 269 + 143 tests green in this run, and no Cargo manifest change.

---

_Verified: 2026-10-30_
_Verifier: Claude (gsd-verifier)_

## Post-review addendum (2026-10-30, orchestrator)

After this verification, `322-REVIEW.md` (2 BLOCKER, 9 WARNING, 5 INFO) was triaged and CR-01, CR-02, WR-01, WR-02, WR-04, WR-05 and WR-09 were fixed test-first with mutation proofs (commits 80de04797, d145eac0b, f5ce89a9b, 90d6d6119, 8ccf1bbec, dee54b4a5, b6b45361c; status of every finding in `322-REVIEW-FIX.md`). The pack now reads only `decision_prompt` (the user's own words, set only by the default chat turn — that field is the opt-in), negated/deny-side free cues and secret-shaped text veto `allow_free`, and the Stakes veto fires on any stakes-2 match (`turn-pack.v3`). The frozen eval metrics are unchanged; both freeze hashes still verify. Final targeted run: 276 passed, 0 failed.

Per-requirement verdicts above are unchanged: DEC-01 SATISFIED, DEC-02 BLOCKED (anchors absent from every ref), DEC-03 PARTIAL, DEC-04 SATISFIED, CAT-01 SATISFIED (daemon has no command provider), CAT-02 PARTIAL (wired and mutation-proven, but the Choice gate is closed at τ 1.01 after failing its held-out risk gate).
