# Phase 12 Context — Code-Gen Benchmark Plumbing & Provenance

**Source of decisions:** `docs/src/architecture/vox-mens-continuous-benchmark-provenance-design-2026-10-10.md`. On 2026-10-10 the user chose this doc as the locked context in place of discuss-phase. Also binding: `docs/src/architecture/vox-efficacy-benchmark-adversarial-audit-2026-09-01.md` (C1–C7, S3) and `vox-efficacy-benchmark-execution-handoff-2026-09-04.md`.

## Locked decisions

- **D-01 One identity scheme.** Benchmark runs are Phase 7 campaigns and runs (`campaign_id`, `run_id`, signed preregistration, atomic Tier A persistence). There is no second identity scheme. `model_benchmark_results` (scientia, uncommitted on this branch) becomes a projection or view over these tables, not a parallel writer.
- **D-02 Routing.** `eval_corpus.rs::generate_candidate` must resolve the model through the registry and `vox_actor_runtime::llm`, including local `mens/*` checkpoints. Remove the hardcoded `LlmConfig::openrouter`. There is no MENS special-case path: same corpus, verifier and scoring as every other model.
- **D-03 Sampling.** n is fixed, there is never an early stop, and the seed is recorded and passed. `vox mens eval-local` pass@k must use `vox_eval::pass_at_k` (the unbiased estimator), not "any sample passed".
- **D-04 Statistics.** Reuse `vox-eval` `corpus_stats` (`bootstrap_ci`, `paired_compare`/exact McNemar, `holm_reject`, `min_detectable_difference`). Write no new stats code. CI overlap is never a significance test.
- **D-05 Provenance fields** are the design doc §5 list. A missing required field forces `run_complete = false` and exclusion from comparison. Nothing is backfilled after the fact.
- **D-06 Training lineage.** The training lane writes `AdapterCard` (`crates/vox-populi/src/mens/tensor/adapter_card.rs`) with the `train.jsonl` sha256 (a hash, not a path), the corpus hash, the base revision, the adapter sha256 and the training syntax fingerprint. Until Phase 13 lands the behavioral fingerprint, the training syntax fingerprint uses the existing `compute_ebnf_hash()`, and its algorithm is recorded in a field (`syntax_fingerprint_kind`).
- **D-07 Signing.** Signing and hashing go through `vox-crypto`; there are no direct `sha2`/`blake3`/`ed25519` imports outside it. Reuse the preregistration signing in `crates/vox-orchestrator/src/preregistration/signing.rs` or the Phase 7 equivalent.
- **D-08 Out of scope for Phase 12:** the behavioral syntax fingerprint, workflow triggers, new corpus tiers and cross-language verifiers (all Phase 13).

## Decisions after research (user, 2026-10-10)

- **D-09 Baseline 95 collision.** Fold the branch's uncommitted `model_benchmark_results` into Phase 7's Tier A baseline as a projection over campaign/run. Phase 12 adds no standalone migration and must not claim a DB baseline version of its own.
- **D-10 Signing key.** Bundles are signed by the Phase 7 allowlisted campaign signer (TRIAL-05). No new Clavis `SecretId`.
- **D-11 Content hashes.** `adapter_sha256`, `train_data_sha256`, `problem_set_hash` and `vox_binary_sha256` use SHA-256 via the `vox_crypto` `interop_sha256` facade, so they can be checked externally. Signatures and internal identity stay ed25519 and BLAKE3. This refines D-06.
- **D-12 MENS reproducibility is in scope.** Plumb `seed` and `top_p` through `LlmConfig` → the `vox mens serve` request → the local (metal) sampler, replacing the unseeded `thread_rng`. The CUDA copy gets the same change or a recorded deviation.
- **Defaults (no user input needed):** the comparison surface is a `vox model eval-compare` subcommand; the infrastructure-error ceiling for `run_complete` is 5% of samples, preregistered per campaign; the `train.jsonl` path is resolved by the planner from the research.

## Dependency note

Phase 12 depends on Phase 7, which is not started. Plans must state exactly which Phase 7 artifacts they consume (types, tables, signing API). Work that has no Phase 7 dependency (D-02, D-03, D-04, D-06) can be sequenced first.

## Open (not needed for Phase 12)

- Results server: the default is the shared vox-db via `VOX_DB_URL`.
- MENS inference host: the recommendation is local-host submission first.
