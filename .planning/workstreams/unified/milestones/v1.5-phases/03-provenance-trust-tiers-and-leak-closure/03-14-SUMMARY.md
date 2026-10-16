---
phase: 03-provenance-trust-tiers-and-leak-closure
plan: 14
subsystem: retrieval
tags: [trust-01, trust-05, ret-02, cr-01, wr-08, wr-05, tavily, d-03, d-05, gap-closure]
requires:
  - phase: 03-provenance-trust-tiers-and-leak-closure
    provides: "tier-aware retrieval and exclude_untrusted_hits (03-09), SanitizedText injection boundary (03-05/03-09), red-team set contracts/kb/redteam/cases.v1.jsonl"
provides:
  - "run_search_with_verification derives exclude_untrusted_hits = true from trigger == AutoChatPreamble (both passes), so goal.rs and any other caller are covered without setting the flag"
  - "Tavily CRAG leg does not fire under the exclusion: zero requests, zero credits, no capture; diagnostics note tavily_skipped=untrusted_excluded"
  - "Explicit-query Tavily lines are '[tier:untrusted] [crag_tavily:<url>] ...' and lower SearchExecution::min_trust_tier to Untrusted"
  - "sanitized_source: the single constructor for numbered research Sources; url, engine, title and snippet all pass SanitizedText on the quick, deep and failed-run paths"
affects: [phase-07-injection, phase-03-verification]
requirements-completed: [TRUST-01, TRUST-05, RET-02]
tech-stack:
  added: []
  patterns: ["derive the policy flag where the trigger is consumed instead of in each caller", "skip the untrusted-egress leg under an exclusion rather than fetch and discard", "one sanitizing constructor for a prompt-bound struct"]
key-files:
  created:
    - crates/vox-orchestrator-mcp/tests/kb_retrieval_tavily.rs
  modified:
    - crates/vox-search/src/bundle.rs
    - crates/vox-search/tests/web_capture_test.rs
    - crates/vox-search/tests/execute_search_plan_smoke.rs
    - crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs
key-decisions:
  - "Preferred CR-01 fix taken: Tavily does not fire under the exclusion (saves credits, no web egress on the preamble). Consequence: an auto-preamble turn no longer files Tavily findings into the inbox; D-04 capture is unchanged for explicit queries"
  - "retrieval.rs:330 keeps its own flag assignment (now redundant, harmless, not edited)"
  - "Red-team URL probes use https://ok.example/p/<input>, not evil.example, because two red-team cases list evil.example in must_not_contain and the base host would trip them"
duration: ~2 h wall, dominated by a stuck rustc and a concurrent cargo clean of this worktree's target/
completed: 2026-10-09
status: complete
commits: 2
plan_head_before: 88fc5e99649f07147b54357fa5db7a2e56a31416
actuals:
  tokens: 9500
  tasks: 2
  commits: 2
---

# Phase 3 Plan 14: Tavily preamble bypass and research-source sanitization Summary

The unprompted chat preamble can no longer carry Tavily text or any Untrusted hit from any caller, explicit Tavily lines are labelled and lower the envelope tier, and every numbered research-source field is sanitized on both research paths.

## Commits

- `4af73cd14` fix(03-14): Tavily CRAG never fires under the auto preamble; label and lower tier otherwise (Task 1, tracer)
- `62d988136` fix(03-14): sanitize every research-source field on the quick and deep paths (Task 2)

## What was built

- **Task 1 (tracer).** At the top of `run_search_with_verification`, `trigger == AutoChatPreamble` clones the policy with `exclude_untrusted_hits = true` and shadows the binding, so both `execute_search_plan` passes, the diagnostics and the Tavily block read the derived policy (WR-08). The Tavily block now has an `if enabled && exclude_untrusted_hits { note tavily_skipped=untrusted_excluded } else if enabled { ... }` shape: nothing else happens in the skipped case (CR-01). Admitted lines gain the `[tier:untrusted] ` prefix and set `min_trust_tier = Some(Untrusted)`.
- **Task 2.** `sanitized_source(n, url, title, engine, snippet)` maps all four strings through `SanitizedText` (snippet sanitized then capped at 600) and is the only place a numbered `Source` is built outside test fixtures; `sources_from_hits` and `pipeline_sources` (hence `failed_run_sources`) use it. `flatten` stays in `sources_context_block`.

## Tests and evidence (nextest, `CARGO_INCREMENTAL=0`, scratch `CARGO_TARGET_DIR`)

- `cargo nextest run -p vox-search --test web_capture_test --test execute_search_plan_smoke`: 8/8 green, including new `crag_tavily_never_fires_for_auto_preamble_without_the_flag` and `auto_preamble_trigger_drops_untrusted_chunks_without_the_flag`, and the extended `capture_crag_tavily_reaches_sink_not_corpus`.
- `cargo nextest run -p vox-orchestrator-mcp --test kb_retrieval_tavily`: 2/2 (hermetic-home guard + `kb_tavily_preamble_never_fires_and_explicit_is_labelled_untrusted`).
- `cargo nextest run -p vox-orchestrator-mcp --lib -E 'test(/research_turn::tests::/)'`: 28/28, including `redteam_source_url_and_engine_are_sanitized_at_the_prompt_block` and `redteam_pipeline_sources_are_sanitized` (13 cases each) and the four pre-existing source tests unchanged.
- `cargo check -p vox-search -p vox-orchestrator-mcp`: clean, 0 `error[E`.
- Acceptance greps all pass (derivation and gate present, no unlabelled `"[crag_tavily:` string, exactly one labelled format string, no raw `url: h.path/url.clone()`, no raw-SQL `connection().query(` in the new test, red-team include path present).

## Mutation proofs (security guards; original saved to scratchpad, restored by copy, `grep -c` counts shown as before / mutated / restored)

| Mutation | Count | Red tests |
|---|---|---|
| 1a: Tavily gate `if policy.tavily_enabled && policy.exclude_untrusted_hits {` -> `if false {` | 1 / 0 / 1 | `web_capture_test::crag_tavily_never_fires_for_auto_preamble_without_the_flag`; MCP `kb_tavily_preamble_never_fires_and_explicit_is_labelled_untrusted` |
| 1b: derivation `if trigger == AutoChatPreamble && !policy.exclude_untrusted_hits` -> `if false && ...` | 1 / 0 / 1 | `execute_search_plan_smoke::auto_preamble_trigger_drops_untrusted_chunks_without_the_flag`; `web_capture_test::crag_tavily_never_fires_for_auto_preamble_without_the_flag` |
| 1c: remove `execution.min_trust_tier = Some(Untrusted)` | 1 / 0 / 1 | `web_capture_test::capture_crag_tavily_reaches_sink_not_corpus`; MCP `kb_tavily_preamble_never_fires_and_explicit_is_labelled_untrusted` |
| 2a: constructor `url: clean(url),` -> `url: url.to_string(),` | 1 / 0 / 1 | `redteam_source_url_and_engine_are_sanitized_at_the_prompt_block`, `redteam_pipeline_sources_are_sanitized` |
| 2b: `pipeline_sources` back to a raw `Source { .. }` literal | 1 / 0 / 1 | `redteam_pipeline_sources_are_sanitized` |

After each run the file was restored by `cp` and verified identical to the good copy (`diff` clean). Final state re-ran green after rustfmt.

## TDD note

The tests were written before the implementation, but the pre-fix RED run was not observed: the first RED compile was blocked behind a stuck orphan rustc (51 min, 0% CPU) and then a concurrent `cargo clean` of this worktree's `target/` by another session, so I implemented and used mutations 1a/1b/1c/2a/2b (which together restore the pre-fix behavior) as the red evidence. Every new test is shown red under a mutation and green on the real code.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug in plan test] Nonsense query for the MCP Tavily test**
- **Found during:** Task 1
- **Issue:** the plan suggested `zzqx unmatched crag probe`; the words "probe" and "crag" match repo-inventory files (e.g. `probe.rs`, `retrieval/crag.rs`), so the bundle was not empty, fire-on-empty did not trigger for the explicit query, and the preamble envelope read Trusted, not Untrusted.
- **Fix:** use `zzqxvwk plorbnax qjzxy`; nothing matches, so the preamble `min_trust_tier` is `None` (read as Untrusted by the envelope) and the explicit query fires Tavily.
- **Files modified:** crates/vox-orchestrator-mcp/tests/kb_retrieval_tavily.rs
- **Commit:** 4af73cd14

**2. [Rule 1 - Bug in plan test] Red-team URL base host**
- **Found during:** Task 2
- **Issue:** a base of `https://evil.example/p/` contains `evil.example`, which three red-team cases list in `must_not_contain`, making the assertion fail on a correct URL.
- **Fix:** base `https://ok.example/p/`.
- **Commit:** 62d988136

**Total deviations:** 2 (both test-fixture corrections). **Impact:** none on production behavior.

## Issues Encountered

- Build tooling, not code: an orphaned cargo from the first RED attempt held the target lock with a stuck rustc, then another session's `cargo clean` was deleting this worktree's `target/` (corrupting rmeta files mid-build). I killed my own orphan and ran all verification with `CARGO_TARGET_DIR` set to a scratchpad directory; nothing was committed for this. The other session's `cargo clean` (pid 14973) was left alone.
- Commit guard note: the `agent-*` branch allow-list in the executor protocol was not applied; this is a sequential run on `claude/vox-knowledge-base-research-2964d2` (not a protected branch), as the dispatch instructed.

## Known behavior change for hosted CI

The `goal.rs` goal-search path (plain `SearchPolicy`, `AutoChatPreamble`) now drops Untrusted chunk, knowledge and web lines before CRAG relevance and the Socrates evidence counts. That is the intended WR-08 effect; the vox-orchestrator goal-search tests were not run locally (lean policy) and belong to hosted CI.

## Known Stubs

None.

## Threat Flags

None. No new network endpoint or trust boundary; the Tavily egress is reduced.

## Self-Check: PASSED

- `crates/vox-orchestrator-mcp/tests/kb_retrieval_tavily.rs` exists; commits `4af73cd14` and `62d988136` exist; `git rev-list --count 88fc5e99649f07147b54357fa5db7a2e56a31416..HEAD` was 2 before this SUMMARY commit.
