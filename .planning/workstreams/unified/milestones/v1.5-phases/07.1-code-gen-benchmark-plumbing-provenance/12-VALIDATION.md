---
phase: "12"
slug: "code-gen-benchmark-plumbing-provenance"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-10"
---

# Phase 12 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution. Source: `12-RESEARCH.md` §Validation Architecture.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `#[test]`/`#[tokio::test]` under cargo-nextest; `wiremock` for loopback HTTP; `serial_test` for env mutation |
| **Config file** | `.config/nextest.toml` (profiles `ci`, `ci-gate`) |
| **Quick run command** | `cargo nextest run -p <crate> -E 'test(<name>)'` (run in background, redirect output to the scratchpad) |
| **Full suite command** | hosted CI `ci-gate` (local equivalent `vox ci pre-push --complete`; executors don't run it) |
| **Estimated runtime** | ~60–180 seconds per crate filter on a warm cache |

---

## Sampling Rate

- **After every task commit:** `cargo check -p <touched crates>` plus that task's new tests only (background).
- **After every plan wave:** all of the wave's new tests, one nextest filter per crate.
- **Before `/gsd-verify-work`:** hosted CI green.
- **Max feedback latency:** 180 seconds.

---

## Per-Task Verification Map

Task IDs bound by the planner on 2026-10-11 (`12-NN-TM` = plan NN, task M). Commands are the exact ones in each plan's `<verify>` block; executors run them in the background.

| Requirement | Behavior | Task ID | Test Type | Automated Command | File Exists | Status |
|-------------|----------|---------|-----------|-------------------|-------------|--------|
| BENCH-01 | `mens/<run>` resolves to a `voxlocal` config; an unknown `mens/*` id errors and never reaches openrouter | 12-02-T1 | unit | `cargo nextest run -p vox-cli --lib -E 'test(eval_corpus::tests::resolve_)'` | ❌ W0 | ⬜ pending |
| BENCH-01 | generation hits the configured base URL with the model and sampling params | 12-02-T1 | wiremock | `cargo nextest run -p vox-cli --lib -E 'test(eval_corpus::tests::generate_candidate_)'` | ❌ W0 | ⬜ pending |
| BENCH-01 | a 409 model mismatch aborts the run; `RATE_LIMITED:` and `CONTEXT_LENGTH_EXCEEDED:` count as infra errors | 12-02-T2, 12-02-T3 | unit + wiremock | `cargo nextest run -p vox-cli --lib -E 'test(eval_corpus::tests::infra_) \| test(eval_corpus::tests::sample_loop_)'` | ❌ W0 | ⬜ pending |
| BENCH-02 | the wire body carries `seed`, and omits it when `None` | 12-01-T1, 12-01-T2 | wiremock | `cargo nextest run -p vox-llm-egress --test wire_mock -E 'test(seed)'` | ❌ W0 | ⬜ pending |
| BENCH-02 | `sample_seed` is deterministic; the generator is called exactly n times per fixture | 12-02-T2, 12-02-T3 | unit + wiremock | `cargo nextest run -p vox-cli --lib -E 'test(eval_corpus::tests::sample_)'` | ❌ W0 | ⬜ pending |
| BENCH-02 | `per_problem_pass_at_k` equals `pass_at_k`: (5,1,1)=0.2, (5,2,3)=0.9 | 12-03-T3 | unit | `cargo nextest run -p vox-ml-cli --lib -E 'test(eval_local::tests::unbiased_)'` | ❌ W0 | ⬜ pending |
| BENCH-02 | the seeded MENS sampler is reproducible (metal and CUDA) | 12-03-T1, 12-03-T2 | unit | `cargo nextest run -p vox-plugin-mens-candle-metal --lib -E 'test(seeded_)'` (and the same for `vox-plugin-mens-candle-cuda`) | ❌ W0 | ⬜ pending |
| BENCH-03 | the report carries a CI, reps, the analysis seed and MDD(n) | 12-04-T1, 12-04-T3 | unit | `cargo nextest run -p vox-eval --lib -E 'test(report::)'` | ❌ W0 | ⬜ pending |
| BENCH-03 | compare applies Holm, labels gaps below MDD as unresolved, errors on a hash mismatch, excludes incomplete runs | 12-05-T1, 12-05-T2 | unit | `cargo nextest run -p vox-eval --lib -E 'test(compare::)'` | ❌ W0 | ⬜ pending |
| BENCH-03 | pass@1 after k compiler-feedback turns (design §8) | 12-08-T1, 12-08-T2 | unit + wiremock | `cargo nextest run -p vox-cli --lib -E 'test(eval_corpus::tests::feedback_)'` | ❌ W0 | ⬜ pending |
| BENCH-04 | removing any single required field gives `run_complete=false` (mutation-proven) | 12-04-T2 | unit | `cargo nextest run -p vox-eval --lib -E 'test(provenance::)'` | ❌ W0 | ⬜ pending |
| BENCH-04 | scoreboard write-back is skipped for incomplete runs and `--from-dir` rows | 12-07-T3 | unit | `cargo nextest run -p vox-cli --lib -E 'test(eval_corpus::tests::write_back_)'` | ❌ W0 | ⬜ pending |
| BENCH-04 | `problem_set_hash` is byte-sensitive and stable | 12-07-T2 | unit | `cargo nextest run -p vox-cli --lib -E 'test(eval_corpus::tests::problem_set_hash_)'` | ❌ W0 | ⬜ pending |
| BENCH-05 | `AdapterCard` lineage round-trips; legacy JSON still parses | 12-06-T2 | unit | `cargo nextest run -p vox-populi --lib --features mens -E 'test(adapter_card::)'` | ✅ file / ❌ tests | ⬜ pending |
| BENCH-05 | eval refuses a missing or swapped card; flags syntax drift | 12-06-T2, 12-07-T3 | unit | `cargo nextest run -p vox-cli --lib -E 'test(eval_corpus::tests::card_)'` | ❌ W0 | ⬜ pending |
| BENCH-05 | a local training success writes `adapter_card.json` | 12-06-T1 | unit | `cargo nextest run -p vox-ml-cli --lib --features gpu -E 'test(local_card_tests::)'` | ✅ file / ❌ tests | ⬜ pending |
| BENCH-06 | the bundle signs and verifies offline; any tampered byte fails; NaN is rejected | 12-09-T1, 12-09-T2 | unit | `cargo nextest run -p vox-cli --lib -E 'test(eval_bundle::)'` | ❌ W0 (Phase 7) | ⬜ pending |
| BENCH-06 | a forced mid-write failure leaves no partial run | 12-09-T3 | integration | `cargo nextest run -p vox-cli --lib -E 'test(eval_bundle::tests::forced_mid_write_failure_leaves_no_partial_run)'` | ❌ blocked on Phase 7 | ⬜ pending |

Mutation proofs (each with a before/after presence check, recorded in the plan SUMMARY): (1) remove the provenance gate term → 12-04-T2; (2) route `mens/*` back to openrouter → 12-02-T1; (3) drop `seed` from `build_request` → 12-01-T1; (4) replace `pass_at_k` with any-sample → 12-03-T3; (5) skip signature verification → 12-09-T2; (6) remove the `run_complete` filter in compare → 12-05-T2. Additional guard proofs: no early stop (12-02-T3), adapter re-hash (12-06-T2), scoreboard write-back (12-07-T3).

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/vox-eval/src/{report,provenance,compare}.rs` with `#[cfg(test)]` modules, written test-first
- [ ] eval_corpus.rs test additions (resolver, wiremock generate, infra classification, sample_seed, problem_set_hash, write-back predicate, card gate)
- [ ] vox-llm-egress wire seed test
- [ ] eval_local aggregator tests; metal seeded-sampler test
- [ ] Mutation proofs (each with a before/after presence check): remove the provenance gate call; route `mens/*` back to openrouter; drop `seed` from `build_request`; replace `pass_at_k` with any-sample; skip signature verification; remove the `run_complete` filter in compare

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| End-to-end live run against a real local MENS checkpoint | BENCH-01/02 | No trained MENS run exists on this machine (`mens/runs` holds only logs) | After a training run: `vox mens serve` with `VOX_LOCAL_ENDPOINT` set, then `vox model eval-corpus --model mens/<run> --seed 7` twice; the sample sets must match |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 180s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
