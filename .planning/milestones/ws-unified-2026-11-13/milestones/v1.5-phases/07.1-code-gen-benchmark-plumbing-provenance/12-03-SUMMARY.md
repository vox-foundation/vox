---
phase: 12-code-gen-benchmark-plumbing-provenance
plan: 03
subsystem: mens-inference
tags: [candle, sampling, seed, top-p, pass-at-k, eval-local, reproducibility]
requires: []
provides:
  - "vox mens serve: seed/top_p on ChatCompletionRequest and GenerateRequest, forwarded through InferenceRequest into the plugin prompt JSON"
  - "metal and CUDA Candle plugins: generation_rng(seed), nucleus top_p, ranked_logits index tie-break"
  - "vox mens eval-local: --k flag, n-sample loop, per_problem_pass_at_k, sample_seed (BLAKE3 via vox-crypto)"
affects: [12-02, 12-05]
status: complete
actuals:
  tokens: 7800
  tasks: 3
  commits: 3
plan_head_before: 0213f129e5cdf18b1c9e702e75cb7e02a489bc8d
tech-stack:
  added: []
  patterns:
    - "seed present => StdRng::seed_from_u64; absent => from_entropy (no behavior change for existing callers)"
    - "tied logits ranked by (logit desc, token index asc): a total order, so a seeded draw never depends on sort_unstable internals"
    - "pass@1/pass@k are means of per-problem unbiased estimates from vox_eval::corpus_score::pass_at_k"
key-files:
  created: []
  modified:
    - crates/vox-ml-cli/src/commands/ai/serve/schema.rs
    - crates/vox-ml-cli/src/commands/ai/serve/handlers.rs
    - crates/vox-ml-cli/src/commands/ai/serve/worker.rs
    - crates/vox-plugin-mens-candle-metal/src/inference.rs
    - crates/vox-plugin-mens-candle-cuda/src/inference.rs
    - crates/vox-ml-cli/src/commands/mens/eval_local.rs
    - crates/vox-ml-cli/src/commands/mens/eval_collateral.rs
    - crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs
    - crates/vox-ml-cli/src/commands/mens/populi/action_populi_enum.rs
    - crates/vox-ml-cli/src/commands/mens/populi/gpu_tests_body.rs
key-decisions:
  - "CUDA plugin gets the identical seed/top_p/tie-break change as metal, so no D-12 deviation is recorded"
  - "sample_seed(7, \"001\", 0) is pinned to 780188561 for 12-02's defactored copy"
  - "An inference error stays in the sample list as pass:false so every item has exactly n samples"
requirements-completed: []
---

# Phase 12 Plan 03: Seeded MENS sampling and unbiased eval-local pass@k Summary

Seed and top_p now travel from a `vox mens serve` request to a seeded, nucleus-capable sampler in both Candle plugins, and `vox mens eval-local` draws all n samples with recorded per-sample seeds and reports the unbiased pass@k.

BENCH-02 is intentionally NOT marked complete here (12-01 and 12-02 also deliver it; the orchestrator marks it).

## Accomplishments

- **Tracer (Task 1, a5c8ebc96):** `seed: Option<u64>` / `top_p: Option<f32>` on `ChatCompletionRequest` and `GenerateRequest` (`serde(default)`), copied onto `InferenceRequest`, emitted as `"seed"` / `"top_p"` in `inference_payload`, parsed by the metal `PromptRequest`, and consumed by `generation_rng(seed)` replacing `thread_rng()`. Tracer feedback gate: both `<verify>` commands re-run end to end and pass, so expansion proceeded.
- **Samplers (Task 2, 18c1662f3):** `ranked_logits` ranks by logit desc then token index asc; nucleus keeps the shortest best-first prefix whose mass reaches `top_p` (at least one token) and renormalizes; non-finite or non-positive `top_p` is treated as 1.0. The CUDA plugin got the Task 1 plumbing plus the Task 2 sampler in one commit.
- **eval-local (Task 3, c3c75fc69):** `per_problem_pass_at_k` delegates to `pass_at_k(n, c, k)`; `sample_seed` is BLAKE3 over length-prefixed framing, masked to 31 bits; `run_eval_local` takes `k` after `samples` and rejects `k < 1` or `k > samples` before any model work. The single-sample block is replaced by an n-sample loop; report keys `pass_rate_at_1` / `pass_rate_at_k` are kept (now means of per-problem estimates), and `n`, `k`, per-item `n`/`c`, and per-sample `seed` are recorded. `passed_at_1` / `passed_at_k` report fields are now f64 sums of estimates (no other reader in the tree).
- Pinned constant for 12-02: `sample_seed(7, "001", 0) == 780188561`.

## Verification

- `cargo nextest run -p vox-ml-cli --lib --features gpu -E 'test(the_payload_carries_seed_and_top_p) | test(chat_completion_request_accepts_seed_and_top_p)'`: 2 passed.
- `cargo nextest run -p vox-plugin-mens-candle-metal --lib -E 'test(top_p_) | test(equal_logits_) | test(seeded_)'`: 5 passed.
- `cargo nextest run -p vox-plugin-mens-candle-cuda --lib -E 'test(top_p_) | test(equal_logits_) | test(seeded_)'`: 5 passed (CUDA crate builds against CPU candle on this Mac; the `cuda` feature itself was not built).
- `cargo nextest run -p vox-ml-cli --lib --features gpu -E 'test(eval_local::tests::unbiased_) | test(eval_local::tests::sample_seed_) | test(eval_local::tests::rejects_k_)'`: 5 passed.
- `cargo check -p vox-ml-cli --features gpu --all-targets`: exit 0.
- Acceptance greps: `pub seed: Option<u64>` in schema.rs = 2; `"seed": req.seed` in worker.rs = 1; `generation_rng(seed)` = 1 in each plugin; `fn per_problem_pass_at_k` = 1; non-comment `one generation suffices` = 0; `k: usize` in action_populi_enum.rs = 2.

### Mutation proof 4 (before/after presence check)

- Before: `grep -c 'pass_at_k(n, c, k)' eval_local.rs` = 1.
- Mutated: body of `per_problem_pass_at_k` replaced with `if c > 0 { 1.0 } else { 0.0 }`: count 0. Run of `unbiased_` tests: 3 run, 1 passed, 2 FAILED (`unbiased_pass_at_1_is_not_any_sample`, `unbiased_pass_at_k_matches_estimator`; the third, n-c<k, legitimately equals 1.0 under both rules).
- Restored from a saved byte-identical copy: count 1, all 5 target tests pass.

## Deviations from Plan

**1. [Rule 3 - Blocking] eval-local tests need `--features gpu`.** The plan's Task 3 `<verify>` runs `cargo nextest run -p vox-ml-cli --lib -E 'test(eval_local::...)'` without the feature, but `mod eval_local` is `#[cfg(feature = "gpu")]` in `mens/mod.rs`, so that command runs 0 tests. All Task 3 test runs and the mutation proof used `--features gpu`. No code change.

**2. [Structure] Tracer commit split reconstructed.** Task 1 and Task 2 both edit the metal `inference.rs`. Task 2's metal edits were written before the Task 1 commit, so the Task 1 file state was reconstructed (Task 2 hunks removed), committed, and Task 2 re-applied from a saved copy. Both commits are self-consistent; Task 1's metal build carried an unused `top_p` parameter warning, by design per the plan.

**3. [Rule 3 - Blocking] Existing test constructions updated.** Three `ChatCompletionRequest { .. }` literals in `handlers.rs` tests and two `.generate(..)` calls in the metal live tests needed the new fields/arguments (`seed: None, top_p: None`, `None, None`).

## Known Stubs

None.

## Notes

- `git rev-list --count` over `plan_head_before..HEAD` is 5, not 3: two commits by concurrent agents (977ef988f, 139bdd20e) landed on the shared branch between this plan's commits. This plan's own commits are a5c8ebc96, 18c1662f3, c3c75fc69.
- Skipped: the manual row in the plan's verification (two `vox model eval-corpus --seed 7` runs against a live `vox mens serve`) needs a trained MENS run and was not attempted. No CUDA-feature build was attempted (no CUDA on this host). `cargo clippy` was not run (lean verification); the pre-commit hook ran only its own checks.
- Default `--temperature` is 0.0 (greedy), so with the default `--samples 1 --k 1` behavior is unchanged except for the added `seed` key in the prompt JSON.

## Threat Flags

None. Mitigations T-12-03-01 to -03 applied as planned (seed reaches `seed_from_u64` and is tested; hostile `top_p` cannot empty the candidate set; mutation proof 4).

## Self-Check: PASSED

- Commits a5c8ebc96, 18c1662f3, c3c75fc69 exist in `git log`.
- All 10 `files_modified` paths exist and were modified by these commits.
