---
phase: 12-code-gen-benchmark-plumbing-provenance
plan: 02
subsystem: testing
tags: [eval-corpus, model-registry, llm-facade, seeding, pass-at-k, mens]
requires:
  - phase: 12-01
    provides: LlmConfig.seed carried through llm_chat to the wire
  - phase: 12-03
    provides: pinned sample_seed(7, "001", 0) == 780188561
  - phase: 12-04
    provides: SampleRecord, FailureClass, VerifyFacts, classify_failure
provides:
  - "eval-corpus resolves --model through ModelRegistry (+ MensCatalog over <repo>/mens/runs) and llm_config_for_spec; no hardcoded OpenRouter config"
  - "classify_generation_error (Infra / FatalConfig / Model) matching the facade's real error strings"
  - "sample_seed, --seed, --top-p, validate_sampling"
  - "draw_samples + SampleSource/SamplePlan/DrawOutcome: exactly n samples per fixture, one SampleRecord per sample"
  - "VerifyOutcome.assertion_failed / RunOutcome.assertion_failed"
affects: [12-05, 12-07, 12-09]
status: complete
actuals:
  tokens: 9300
  tasks: 3
  commits: 3
plan_head_before: 662e08d99ea3d3e49354ee2930ed7d1617663bdb
tech-stack:
  added: []
  patterns:
    - "one resolve_target + llm_config_for_spec pair shared by bench and eval-corpus"
    - "sample loop as a pure-ish unit taking a verify closure, so tests need no vox binary"
key-files:
  created: []
  modified:
    - crates/vox-cli/src/commands/model/eval_corpus.rs
    - crates/vox-corpus/src/humaneval_runner/verify.rs
key-decisions:
  - "An unknown mens/* id is a hard error before any generation; no other mens/ special case exists"
  - "from-dir samples carry seed 0 (they are not sampled); live samples carry sample_seed(base, id, i)"
  - "classify_generation_error also matches the facade's 'LLM API returned error (5xx)' and 'HTTP request failed:' strings, not only the plan's 'provider status' forms"
requirements-completed: [BENCH-01, BENCH-02]
---

# Phase 12 Plan 02: eval-corpus through the registry and facade Summary

`vox model eval-corpus --model mens/<run>` now resolves through `ModelRegistry` (refreshed with `MensCatalog` over `<repo>/mens/runs`) and `llm_config_for_spec` to a voxlocal `LlmConfig`, generates via `llm_chat` with a per-sample BLAKE3-derived seed, always draws all n samples, and keeps infra errors out of scoring.

## Accomplishments

- **Tracer (Task 1, 561cf8245):** `with_mens_runs`, `resolve_eval_model` (reuses `bench::resolve_target` and `llm_config_for_spec(.., TaskCategory::CodeGen)`), `Generated`, and `generate_candidate(&LlmConfig, prompt)` returning the provider-reported model string. The unknown-`mens/*` guard is the only mens special case. Tracer gate: a wiremock test shows one POST to `/v1/chat/completions` with `model`, `seed`, `temperature`, `top_p`, `max_tokens` in the body. Tracer verify re-run end to end before expansion.
- **Seed, validation, classification (Task 2, d3afbe9ff):** `--seed` (default 0) and `--top-p` (default 1.0); `sample_seed` is a defactored copy of 12-03's function (pinned vector 780188561 asserted); `validate_sampling` rejects n=0, k=0, k>n, top_p outside (0,1], non-finite or negative temperature; `classify_generation_error` replaces `is_infra_error`; warning for voxlocal without `VOX_LOCAL_ENDPOINT` (11434 Ollama vs 11435 `vox mens serve`).
- **draw_samples (Task 3, 1bec70013):** the sample loop is extracted with its semantics intact (spend cap before each sample, infra `continue`d without touching attempts, model failures as all-false attempts, no early break). 409 "Loaded model is" aborts the run. Each sample yields a `SampleRecord` with seed, code, `provider_model` and a `classify_failure` class (`parse_ok` from an in-process lex+parse, `timed_out` from "timed out" in the detail). Fixtures are sorted by id in `select_fixtures`; records are sorted by `(fixture_id, sample_index)`. `VerifyOutcome`/`RunOutcome` carry `assertion_failed`, set from `assertion failed: ` in the run stage's stderr. `run()` still writes today's report (12-07 swaps in the typed one).

## Verification

- `cargo nextest run -p vox-cli --lib -E 'test(eval_corpus::)'`: 19 passed (new: 3 resolve/generate_candidate, 4 infra/sample_ args+seed+classification, 4 sample_loop_). Per-task runs: Task 1 alone 5 passed (includes 2 pre-existing `resolve_vox_binary_*`), Task 2 alone 4 passed.
- `cargo nextest run -p vox-corpus --lib -E 'test(verify::tests::assertion_marker_)'`: 1 passed.
- `cargo check -p vox-cli -p vox-corpus --all-targets`: exit 0, no warnings.
- Acceptance greps: non-comment `LlmConfig::openrouter` in eval_corpus.rs = 0; `llm_config_for_spec(` = 1; `resolve_target(` = 1; `resolve_chat_config` = 0; `pub fn classify_generation_error` = 1; `fn is_infra_error` = 0; `vox:defactored-from vox-ml-cli` = 1; `RATE_LIMITED_PREFIX` = 4; `async fn draw_samples` = 1; `sample_seed(plan.base_seed` = 1.

### Mutation proof 2 (unknown mens/* guard)

- Before: `grep -c 'starts_with("mens/")' eval_corpus.rs` = 1.
- Mutated (guard pattern changed to `mens-DISABLED/`): count 0. `resolve_unknown_mens_id_is_refused` FAILED (`unwrap_err()` on an `Ok` value: the id resolved to an OpenRouter spec).
- Restored (byte-identical to a saved copy): count 1, test passes.

### No-early-stop mutation proof (D-03)

- Before: `grep -c 'NO early break' eval_corpus.rs` = 1.
- Mutated (`if outcome.tests_passed { break; }` inserted after the verify call): `sample_loop_draws_exactly_n_without_early_stop` FAILED (`left: 1, right: 3`, "all n drawn even though the first passed").
- Restored (byte-identical): count of the inserted `if outcome.tests_passed {` = 0, all 19 tests pass.

Both proofs ran on the final combined file state (see Deviations 3).

## Deviations from Plan

**1. [Rule 1 - Bug] Infra classification matches the facade's real strings.** The plan's marker list only had `provider status 5xx`, but `llm_chat` formats provider failures as `LLM API returned error (<code>): ...` and transport failures as `HTTP request failed: ...` (`vox-actor-runtime/src/llm/chat.rs` `map_egress_error`). Added `returned error (500|502|503|504)` and `http request failed:` markers; tests cover both forms. The 409 check uses the "Loaded model is" substring, which holds in either wrapping.

**2. [Structure] `pub assertion_failed: bool` count is 2, not 1.** The plan asked for the field on both `VerifyOutcome` and `RunOutcome` but its acceptance grep expects a count of 1. Both structs are public with public fields, so the field is `pub` on each; the literal grep reads 2.

**3. [Process] Tasks 2 and 3 were first built and tested together.** The host's `syspolicyd` stalled every freshly linked binary (build scripts and test binaries sat at 0% CPU for about 13 minutes, see `~/dev/memory/macos-syspolicyd-stalls-fresh-test-binaries.md`), so the first successful build compiled Tasks 1-3 as one file state. After the stall cleared, each task was re-verified in isolation (Task 1: 5 tests; Task 2: 4 tests) before its commit; Task 3 was committed from the file state that passed all 19 tests, and `git show HEAD:...eval_corpus.rs` is byte-identical to that tested state. The tests were written alongside the code in each task rather than observed red first (red was a compile error until the code existed); the two mutation proofs supply the failing-test evidence.

**4. [Scope] `config_digest` not extended.** It does not include `--seed` or `--top-p`. The plan did not ask for it; 12-07 owns row identity and provenance. Flagged for that plan.

## Flagged assumption (unresolved, from the plan)

Edge 5: a voxlocal endpoint that is up but serving Ollama rather than `vox mens serve` is not detected automatically. The per-sample `provider_model` string is recorded (empty for from-dir), which is the evidence 12-07 provenance will consume. Reviewer to confirm this is acceptable.

## Known Stubs

None.

## Skipped checks

- No `cargo clippy` run (lean verification; the pre-commit hooks ran fmt-fix, tdd-guard, command-sync, ci-status).
- The manual row (two live `vox model eval-corpus --seed 7` runs against a served MENS checkpoint) was not attempted: no trained run is served here.
- `vox-orchestrator-mcp` lib-test build failure noted in the dispatch was not touched or needed.

## Threat Flags

None. T-12-02-01/02 mitigated by the mens guard (mutation proof 2); T-12-02-03 by `classify_generation_error` and the infra-out-of-attempts test; T-12-02-04 by the 409 abort test and recorded `provider_model`; T-12-02-05 unchanged (accepted).

## Self-Check: PASSED

- Commits present: 561cf8245, d3afbe9ff, 1bec70013.
- Both `files_modified` paths exist and were modified by these commits.
