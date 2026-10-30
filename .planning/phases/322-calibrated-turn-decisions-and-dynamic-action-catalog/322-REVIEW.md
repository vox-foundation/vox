---
phase: 322-calibrated-turn-decisions-and-dynamic-action-catalog
reviewed: 2026-10-30T00:00:00Z
depth: deep
files_reviewed: 22
files_reviewed_list:
  - crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/turn_pack.rs
  - crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/resolve.rs
  - crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/types.rs
  - crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/mod.rs
  - crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/turn_pack_tests.rs
  - crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/turn_pack_eval_tests.rs
  - crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/tests.rs
  - crates/vox-orchestrator-mcp/src/selective_metrics.rs
  - crates/vox-orchestrator-mcp/src/lib.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chatarc.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/mod.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chatarc_tests.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chatarc_eval_tests.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs
  - crates/vox-orchestrator-mcp/src/skill_search_index.rs
  - crates/vox-orchestrator-mcp/src/models_tools.rs
  - crates/vox-config/src/inference.rs
  - crates/vox-config/src/config_registry.rs
  - crates/vox-config/src/operator_registry.rs
  - contracts/config/env-vars.v1.yaml
  - crates/vox-cli/src/main.rs
findings:
  critical: 2
  warning: 9
  info: 5
  total: 16
status: issues_found
---

# Phase 322: Code Review Report

**Reviewed:** 2026-10-30
**Depth:** deep (committed objects at 7a2256588, phase commits only; cross-checked callers in browser_tools.rs, t3_wave3_tools.rs, infer.rs, mentions.rs, prompt_canonical.rs, tool_selection.rs, command_catalog.rs, vox-orchestrator select.rs)
**Files Reviewed:** 22
**Status:** issues_found

## Summary

These invariants hold:
- With `VOX_TURN_DECISION_PACK` off, `ModelSelectionRequest` is byte-identical. `pack = None` returns the caller's inputs, and `allow_free = false` matches `SelectionIntent::for_task`'s default.
- With ChatARC off (and also with it on, because `TAU_CHOICE = 1.01`), the system prompt and tool pins are identical.
- The pack never emits a model id and never touches `PreferFree` or `FREE_FLOOR`.
- `allow_free` is gated on `res.clutch.is_none()` after the source-policy block (resolve.rs:299-312).
- A ChatARC tool pin still passes every `select_tools_for_turn` filter.
- Excluded skills are filtered before ChatARC gathers candidates and again before injection.
- The `#[cfg(test)]` seams do not compile into production.

Three claims fail:
1. "The pack and ChatARC read only the user's own message." The input is `expanded_prompt`, which inlines up to 8000 chars of every `@mention`ed file and is wrapped in canonicalizer boilerplate.
2. "Calibrated allow-free is safe." A negated request such as "don't use a free model" turns into allow-free, and the deny rule can never win.
3. "The pack only affects the default chat turn." `allow_free` acts at every call site that sets `capability_prompt` with no clutch, including browser tools whose instruction an LLM can write.

Both flags are off by default, which contains the BLOCKERs. They are still incorrect behavior behind a flag that is documented as safe to enable.

## Critical Issues

### CR-01: The pack and ChatARC read `@mention` file contents, not just the user's message (prompt-injection surface)

**File:** `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs:539-540, 565-575, 935-939, 1010, 1150-1165`; `crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/resolve.rs:510-518`; `crates/vox-orchestrator-mcp/src/chat_tools/chatarc.rs:222-224`

**Issue:** Both ChatARC (`pick_for_turn(state, &expanded_prompt, ..)`) and the turn pack (`capability_prompt: Some(expanded_prompt)` → `turn_pack::evaluate`) score `expanded_prompt`, which is built in two steps:
- `resolve_mentions` (mentions.rs) replaces each `@file` with `"\n\n--- @{filename} ({rel}) ---\n{up to 8000 chars of file content}\n---\n"`.
- `canonicalize_prompt(.., order_invariant=true, ..)` then always prepends `Objectives (treat as a single set; order does not imply priority):\n\n1. ...`.

The code comments are therefore wrong. chatarc.rs:222 says "never the assembled prompt, which carries history, files and retrieved text", message.rs:933 says "scored on the user's own message", and resolve.rs:510 says "never ... file and web text". The research path in the same function (message.rs:541-552) explicitly avoids this text, because a mention "inlines up to 8000 chars of file content, whose words must not trip research cues".

The consequence is that any workspace file the user mentions steers routing. That includes cloned third-party repos, vendored docs, and generated or downloaded files.
- If the file contains `cheap`, `no rush`, `any model`, `free tier` or `quick question` (for example a code comment like `// cheap to clone`), `allow_free_in_performance_mode` turns on. The user's code and history then become eligible for free-tier providers, which typically have weaker data-retention terms.
- If the file contains `private` or `offline`, the privacy answer fires.
- Once ChatARC's τ is lowered, file text also decides which skill body is injected into the system prompt and which tool is pinned.

**Fix:** Score the user's own words, not the expanded or canonicalized text. Capture the raw message once and pass it to both consumers:
```rust
// message.rs, next to raw_prompt_for_research
let decision_text: &str = if intent.explicit && intent.mode != ResearchMode::None {
    &intent.query
} else {
    &params.prompt // pre-@mention, pre-canonicalization
};
let chatarc_pick = chatarc::pick_for_turn(state, decision_text, &params.skill_exclusions);
// and add a separate `decision_prompt: Option<String>` on McpChatModelResolution
// for the pack, rather than reusing `capability_prompt` (whose Task 8f contract is post-expansion).
```
Add a regression test: a mentioned file containing "free model" must not set `allow_free_in_performance_mode`.

### CR-02: Negated or conflicting free-tier requests turn into allow-free; the deny rule can never win

**File:** `crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/turn_pack.rs:85-88, 142-158, 248-251`

**Issue:** `evaluate_with_rules` matches plain substrings with no negation handling, and keeps the single highest-confidence answer for each question. `allow_free_tier.1.1` (allow, 0.86) always beats `allow_free_tier.0.1` (deny, 0.50). Concrete flips:
- "Please **don't use a free model** for this" normalizes to `" please don t use a free model for this "`, which contains `" free model "`. The answer is 1 at 0.86 ≥ TAU_FREE, so `allow_free = true`.
- "Do not use the free tier": same result.
- "Quick question, but use the **best model**": `quick question` (0.86) beats `best model` (0.50), so `allow_free = true`.

The deny rule also has no effect on any input. When it fires alone, answer 0 is ignored (only `== Some(1)` acts). When it fires with the allow rule, it loses. So a user's explicit instruction against free models is turned into the opposite routing decision.

**Fix:** Treat the deny side as a veto, as is already done for Stakes, and block negated allow cues:
```rust
let deny_free = rules_fired(prompt, Question::AllowFreeTier).any(|r| r.answer == 0);
let negated = ["don t use", "do not use", "not the free", "no free"].iter().any(|n| text.contains(n));
let allow_free = may_set_allow_free && sure(..) == Some(1) && !deny_free && !negated && !high_stakes && !private;
```
Add eval and mechanism fixtures for "don't use a free model" and "quick question, best model". Both must give `allow_free == false`.

## Warnings

### WR-01: `allow_free` is not limited to the default chat turn; it acts at every `capability_prompt` call site with no clutch

**File:** `turn_pack.rs:217`; `resolve.rs:513-518`; `browser_tools.rs:974, 1028, 1182`; `t3_wave3_tools.rs:211-222`; `message.rs:1002-1012`; `infer.rs:978-988`

**Issue:** `may_set_allow_free = res.clutch.is_none()` is the only gate. Task, complexity and prefer_local are gated on the opt-in `task_unset` / `complexity_unset` markers. `allow_free` has no opt-in marker, so with the flag on, the pack widens free eligibility for:
- `browser_extract`, `browser_extract_json` and `browser_act`. Their `instruction` can be written by the agent-loop LLM after it has read web pages.
- t3 consensus legs.
- The cognitive-profile chat path, which never passes the composer's clutch.
- The `call_llm_with_pref` fallback, which never sets a clutch.

That contradicts the module doc ("fills routing inputs the caller left unset") and the SUMMARY claim that only the default chat turn opts in.

**Fix:** Add an explicit opt-in to `McpChatModelResolution`, for example `allow_free_unset: bool` (default `false`, set only in `try_run_agent_turn`), and gate on `res.allow_free_unset && res.clutch.is_none()`.

### WR-02: The privacy and secret vetoes are too narrow for a gate that sends content to free providers

**File:** `turn_pack.rs:83-84, 89-90, 248-251`

**Issue:** The labeling convention in 322-EVAL-REPORT.md says `privacy_local` is true "when the turn carries secrets, personal, health, financial, or confidential data". The runtime lexicon for `privacy_local.1.1` has no `password`, `api key`, `token`, `private key` or `ssn`, and `stakes.2.1` has only `credentials` and `secret`. So "here is my API key sk-…, quick question" sets `allow_free = true`. Under Performance this moves a credential-bearing prompt from paid-only routing to free-tier eligibility, which is strictly worse than flag-off.

The convention also requires `complexity < 2` for allow-free, but runtime `allow_free` never checks complexity.

**Fix:** Veto `allow_free` whenever the existing secret scanner or redactor (or at least a credential-shape check) flags the decision text. Also add the missing privacy and credential keywords to the veto set, not just to calibrated rules.

### WR-03: PrivacyLocal cannot keep a turn local; the VoxLocal shortcut and capability pin run before the pack on the caller's task

**File:** `resolve.rs:387-388, 408, 495-508, 527-533`; `turn_pack.rs:233-238`

**Issue:** On the only caller that sets `task_unset` (the default chat turn), the task placeholder is `CodeGen` (types.rs:71). So `derived_prefer_local` is already `true`, and a confident privacy answer ("keep it local", "do not send", "confidential") changes nothing about locality. `prefer_local` is also only a scorer hint, so `decide()` can still return a cloud model. The VoxLocal shortcut (495-508) and `secrets_capability_pin_model_id(.., task, ..)` (408) run before the pack and read the pre-pack task.

If TaskShape is re-enabled later, the pack's task and these earlier branches will disagree.

**Fix:** Either document PrivacyLocal as a free-tier veto only, or make a confident privacy answer a hard local filter: compute the pack before line 408 and route `private` through the VoxLocal or local-only path. Use one post-pack `task` for every branch.

### WR-04: The Stakes fail-closed veto depends on argmax order and on the τ sentinel

**File:** `turn_pack.rs:239-244, 142-158`

**Issue:** `high_stakes = pack.stakes.is_some_and(|a| a.value == 2 && (stakes_tau > 1.0 || confident(&a, stakes_tau)))` sees only the winning Stakes answer. The comment's promise of a veto "on any stakes-2 firing, whatever its confidence" holds only because `stakes.2.1` (0.80) currently outranks `stakes.0.1` (0.50). Two plausible changes break it:
- If a recalibration puts `stakes.0.1` at or above `stakes.2.1`, "explain how to deploy to production" vetoes nothing.
- Re-enabling Stakes at τ = 0.85 makes 0.80 stakes-2 firings stop vetoing. Enabling the question weakens the safety veto.

**Fix:** Compute the veto from rule firings directly, independent of argmax and τ:
```rust
let high_stakes = RULES.iter().any(|r| r.question == Question::Stakes && r.answer == 2 && fires(r, prompt));
```
This needs the normalized text or the fired set in `TurnPack`. Pin the behavior with a co-firing test where stakes-0 has higher confidence than stakes-2.

### WR-05: The vox-cli command-hint provider blocks a tokio worker (and does disk I/O) on the first ChatARC turn

**File:** `crates/vox-cli/src/main.rs:57-79, 84`; `chatarc.rs:224-245`; `command_catalog.rs` (`build_catalog` → `resolve_repo_root_for_ci` + `apply_capability_ids` loads the capability registry from disk)

**Issue:** Registration itself is lazy and cheap: line 84 stores a fn pointer, so a plain `vox` invocation pays nothing. The first `pick_for_turn` with `VOX_CHATARC=1` is different. It runs synchronously inside `async fn chat_message`, and inside `OnceLock::get_or_init` it spawns a 32 MiB thread, builds the whole clap tree, resolves the repo root, parses the capability registry, and then blocks on `join()`. That stalls a runtime worker, and every concurrent first turn queues behind the OnceLock. A spawn failure or a panic is cached as `Vec::new()` for the life of the process, with no log line.

**Fix:** Warm the cache off the request path, for example `std::thread::Builder::new().stack_size(..).spawn(|| HINTS.get_or_init(..))` at startup or on first registration. Have the provider return `HINTS.get().cloned().unwrap_or_default()` so it never blocks. Log a `tracing::warn!` on spawn or join failure. Use a catalog walk that skips `apply_capability_ids`, since hints do not need capability ids.

### WR-06: A ChatARC skill pick is weaker than an explicit pin and easy to squat (latent while τ = 1.01)

**File:** `message.rs:1161` (`active_skill_id = params.skill.clone()`); `chatarc.rs:162-197`; `chat_tools/mod.rs:245-248`

**Issue:** There are three problems:
- An explicit pin also sets `active_skill_id`, so `check_skill_tool_permission` restricts tools to the skill's allowlist. A ChatARC pick injects the skill body but leaves `active_skill_id = None`, so the skill runs with the full tool surface instead of its declared restriction.
- `margin(s1, 0) == 1.0` puts any single lexical hit in the top bin (0.83) with no floor on absolute BM25 score. A third-party skill whose description is stuffed with common query words can therefore select itself and have its body injected without user consent. Before this phase, a description only reached the tier-1 catalog line.
- The id-or-name lookup at mod.rs:248 can inject a different skill whose `name` equals the picked `id`.

**Fix:**
- Pass `params.skill.clone().or(chatarc skill id)` as `active_skill_id`.
- Require a minimum `s1` and a minimum query-term overlap before the top bin applies.
- Look up ChatARC picks by `m.id == id` only.
- Consider limiting automatic injection to first-party or vendored skills (`assets/skills/SOURCES.toml` provenance).

### WR-07: Calibration and serving inputs differ (train/serve skew), so the held-out metrics do not describe production

**File:** `message.rs:565-575, 935-939`; `chatarc_eval_tests.rs` (raw `query` strings); `turn_pack_eval_tests.rs` (raw `prompt` strings)

**Issue:** Both evals feed raw fixture strings. Production feeds `Objectives (treat as a single set; order does not imply priority):\n\n1. <prompt>`, plus any mention blocks (CR-01). In BM25 the constant tokens `objectives`, `treat`, `single`, `set`, `order`, `imply` and `priority` score real tools on every turn, for example the `*_set_*` tools. That adds a nonzero `s2` and shrinks margins, so CHOICE_BINS and the reported risk@τ were measured on a different distribution. If ChatARC τ is later lowered on the strength of these reports, the gate will be miscalibrated.

**Fix:** After CR-01 is fixed, the inputs are raw again. Then add a test asserting that the string passed to `pick_for_turn` and the pack equals `params.prompt` for a non-command turn. Otherwise, run the eval through `canonicalize_prompt` so it measures production inputs.

### WR-08: The production command-hint seam is never tested

**File:** `chatarc.rs:268-299`

**Issue:** `#[cfg(test)]` replaces the whole `static RwLock` path with a thread-local. `registered_provider_is_read_each_pass` and `provider_unset_yields_no_command_candidates` therefore exercise only the test branch. The production `write()`/`read()` code and its poison-recovery handling have no coverage, and mutating it would not fail any test. This conflicts with AGENTS.md's "verify guards by mutation".

**Fix:** Keep one production code path. Make the thread-local an override that is checked first (`TEST_OVERRIDE.with(..).or_else(|| static.read()..)`). Add one test that holds a serial lock and asserts the static path round-trips.

### WR-09: `VOX_CHATARC=1` adds cost and has no effect (a facade flag)

**File:** `chatarc.rs:29, 208-245`; `config_registry.rs` (`VOX_CHATARC` hint); `message.rs:935-958`

**Issue:** With `TAU_CHOICE = 1.01`, `choose_with` can never return `Some`. Enabling the flag still lists skills, runs `select_tools_for_turn` over the full registry, builds the command catalog (WR-05), and builds a BM25 index on every turn. That includes deep-research and explicit-search turns, where the pick is never used. The operator-facing hint says the flag "enables the ChatARC action shortlist in chat turns", which the code does not do.

**Fix:** Short-circuit in `decide_if_enabled` when `tau > 1.0` (no gather). Update the config and env-var descriptions to say the flag is inert until a calibrated τ ships. Alternatively, mark the key `Status::Experimental` (or equivalent) until then.

## Info

### IN-01: Rules that can never affect production output

**File:** `turn_pack.rs:77-82, 87-88`
**Issue:** These rules have no effect on any production output:
- `complexity.0.1` (0.40) and `complexity.2.1` (0.50) never reach TAU_APPLY (0.80), so Complexity never patches.
- `stakes.0.1` (answer 0) is never consumed.
- `allow_free_tier.0.1` is never consumed (see CR-02).

In production the pack's only live effects are the allow-free answer and the privacy veto.
**Fix:** Mark these rules as eval-only, or remove them from `RULES` until they can act. Note this in PACK_VERSION history.

### IN-02: `ChatArcPick.confidence` is never read outside tests

**File:** `chatarc.rs:152`
**Issue:** No production code reads the field. It is only kept alive by the derived `PartialEq`.
**Fix:** Log it in a `tracing::debug!` at the pick site, or drop it.

### IN-03: Command-hint sanitization covers only `\r` and `\n`

**File:** `chatarc.rs:303-313`
**Issue:** U+2028, U+2029, U+0085 and other control characters pass through. The source is first-party clap `about` text, so the risk is low. The function is `pub(crate)` and generic, though.
**Fix:** Use `.map(|c| if c.is_control() || matches!(c, '\u{2028}' | '\u{2029}') { ' ' } else { c })`.

### IN-04: Process-env flag set in thread-parallel unit tests

**File:** `model_route_policy/tests.rs` (`resolve_free_only`, `EnvKeyGuard::set("VOX_TURN_DECISION_PACK", ..)`)
**Issue:** Under `cargo test` (threads, not processes), resolve tests that do not take `INFERENCE_PROFILE_TEST_LOCK` can observe the flag as on. nextest's process-per-test hides this.
**Fix:** Pass the flag value into `resolve_mcp_chat_model_sync_inner` through a pure seam, or have every resolve test take the lock.

### IN-05: Command candidates exist only when vox-cli hosts the server

**File:** `crates/vox-cli/src/main.rs:84`
**Issue:** The daemon and any other host register no provider, so ChatARC there has no command candidates. The commit message says this, but it is not documented in `chatarc.rs`'s module doc.
**Fix:** Add one line to the module doc, or register the provider in the daemon too.

---

_Reviewed: 2026-10-30_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
