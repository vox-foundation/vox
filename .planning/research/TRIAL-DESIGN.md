# Research Trial Flywheel — Trial Design Research

Implementation research for the settled rulings. Evidence gathered on `feat/research-trial-flywheel`.

## 1. What exists vs. what is missing

**Reusable (with caveats)**
- `PreregistrationV1` (`crates/vox-research-events/src/preregistration.rs:70-90`): has one `MetricSpec` (`:76`), one
  `TestSpec` (`:77`), `StopRule{max_n,alpha,threshold}` (`:50-57`), `cost_cap_usd` (`:81`), `supersedes` (`:87`),
  `analysis_tree_commit` (`:89`). It cannot express arms, several metrics, looks, or token/call caps, so it stays a conversion input only.
- Signing (`crates/vox-orchestrator/src/preregistration/signing.rs:52-73`, `:76-106`): Ed25519 via `vox_crypto::facades`
  over `canonical_json`. Reuse the *pattern* for `CampaignPreregV1`.
- Trusty URI (`preregistration/trusty_uri.rs:21-42`): content hash with `id` blanked. Caveats: it hashes with `sha2`
  directly (`:13`), which violates the AGENTS.md crypto policy (route this through `vox-crypto`), and the BTreeMap reorder is top-level
  only (`:38-41`). Nested key order relies on serde derive field order. Use a recursive canonicaliser (sort keys at every
  depth and reject non-finite floats) for the new type.
- `PreregGate::check_campaign` (`preregistration/gate.rs:36-65`) and `research_gate::check_campaign_prereg`
  (`research_gate.rs:33-38`): there are **no production callers** (rg shows only tests). The gate also trusts whatever key is embedded in the
  prereg (`signing.rs:62-64`, `:87-97`), so any self-generated key passes. It needs an allowlisted signer set.
- `DeviationDetector` (`preregistration/deviation.rs:39`) compares metric name and test kind only. Extend it to arm set, looks, and spend.
- `BayesianStoppingRule::should_stop` (`preregistration/symbolic.rs:129-138`): a raw posterior threshold with no error
  control under repeated looks. Do not use it for confirmatory decisions.
- `wilson_score_interval` (`crates/vox-orchestrator/src/models/registry.rs:56-68`): z is hard-coded to 1.96 (`:60`). Add a
  `wilson_interval(successes, n, z)` variant so multiplicity-adjusted levels work.
- `pareto_frontier` and `dominates` (`models/pareto.rs:30-68`): already correct for incomparable `None` axes. It is reporting-only
  by ADR-046 (`:1-6`), so reuse it for the arm scoreboard and keep it out of the decision path.
- `research_eval_runs` / `research_eval_samples` tables (`crates/vox-db/src/schema/domains/scientia.rs:93-121`) and
  `GoldenQueryItem` plus the metric helpers (`crates/vox-cli-research/src/eval.rs:10-118`).
- `LlmResponse{prompt_tokens, completion_tokens, cost_usd}` (`crates/vox-actor-runtime/src/llm/types.rs:331-346`).
- Atomic check-and-reserve idiom (`crates/vox-orchestrator/src/budget/mod.rs:795-869`). The pattern is sound, but the ledger is not (see §4).

**Missing**
- Arms/conditions model, campaign/run/replicate/attempt identity, lifecycle tables, batch/look schedule.
- A sequential stats engine (alpha spending, Holm, permutation, bootstrap). No `statrs`, `rand_chacha`, or `libm` in the root `Cargo.toml`.
  `rand 0.8` (`Cargo.toml:284`) is present.
- Per-run spend accounting. `response_to_content_and_model` (`crates/vox-research-shim/src/research/orchestrator/stages.rs:536-538`)
  drops tokens and `cost_usd`. Synthesis returns `(String, String)` (`stages.rs:178-180`) and the judge returns `i32` (`stages.rs:74`).
- `run_eval` problems: `_concurrency` is unused (`eval.rs:123`). The loop is sequential (`:154-167`). Failures are dropped with
  only a log line (`:158-165`), which biases results toward survivors. The run row is written only at the end (`:206-210`), because of the FK at `scientia.rs:117`.
  The config is hard-coded `{"policy_version":2}` (`:191`). A missing gold answer is imputed as 0.5 recall (`:93`).
  `latency_p50_ms` is actually the mean (`:200`).

## 2. Data shapes and lifecycle

Put the work in a **new crate pair** rather than adding to `vox-research-shim`. That crate is already ~11.4k LoC against
`max_loc = 8_000` in `layers.toml:211`. Proposed pair: `vox-research-trial-core` (L2: pure types, stats, ledger math, no I/O)
and a runner inside `vox-cli-research` (L5) or a new L3 crate. Both need layer rows. Persist as Tier A rows in a new `scientia` domain fragment.

```text
CampaignPreregV1 {            // immutable once Signed; canonical JSON → content id
  schema_version, campaign_id /*= trusty id*/, kind: Exploratory|HoldoutConfirmation,
  confirms: Option<{nomination_id, parent_campaign_id}>, converted_from: Option<prereg_v1_id>,
  substrate {repo_commit, eval_set_digest, query_ids[]},   // holdout set disjoint (digest-checked)
  arms[] {arm_id, label, is_control, condition: ResearchCondition /*full expanded ResearchConfig + query knobs*/},
  replicates_per_cell, randomization {seed: u64, rng: "chacha8@<ver>", block: per_query},
  metrics[] {name, direction, role: Primary|Secondary|Guardrail{margin}, unit, scorer_version},
  analysis {test: PairedSignFlip|Wilcoxon, ci: Wilson|PairedBootstrap{b}, alpha, spending: OBF|Pocock,
            looks[] {look_index, info_fraction}, multiplicity: HolmWithinLook+FixedSequenceMetrics,
            futility: NonBinding{min_effect}},
  spend_caps {max_tokens, max_calls, max_cost_micros, per_attempt_worst_case{tokens,calls,cost_micros}},
  retry_policy {max_attempts, retry_on: [Transport, Timeout]}, signer_key_id, signed_at, signature }
TrialRun { run_id, campaign_id, arm_id, query_id, replicate_id, attempt_index, retry_of_run_id: Option,
           look_index, state, reservation_id, started/finished_ms, resolved_models[], outcome: Option<Outcome> }
Outcome  { scores{metric→f64|bool}, latency_ms, tokens_in/out, calls, cost_micros, served_from_cache, error_class }
```

Lifecycles. Campaign: `Draft → Signed → Running → {Completed | Stopped{reason: EfficacyBoundary|Futility|SpendCap|Operator} | Failed}`. Only `Signed` may go to `Running`.
- Run: `Reserved → Running → {Succeeded | Failed{class} | Interrupted}`. `Interrupted` is terminal, and on restart the runner creates a
  new run (`attempt_index+1`, `retry_of_run_id=old`). No run row is ever updated after it reaches a terminal state.

Enforce in SQL: UNIQUE(`campaign_id, arm_id, query_id, replicate_id, attempt_index`), a trigger or app check blocking transitions out of terminal states, and the run row inserted before the call (fixes eval.rs's end-only persistence).

## 3. Statistical approach (small n, multi-arm, multi-metric, group-sequential)

- **Unit = query, paired across arms.** Every arm runs the same query set. Average replicates within a (query, arm)
  cell first so replicates are not counted as independent samples, then analyse per-query differences against the control.
- **Primary test: paired sign-flip randomization test** on the differences. Use exact enumeration when 2^n ≤ 2^16, otherwise a seeded
  Monte Carlo with ≥10k flips and the (b+1)/(B+1) correction. It makes no normality assumption and is valid for bounded, skewed scores.
  Wilcoxon signed-rank is an alternative. For latency, analyse log-latency and report the Hodges–Lehmann estimate.
- **Looks:** use Lan–DeMets alpha spending at the preregistered information fractions t_k = n_k/N_max with the O'Brien–Fleming-type function
  α(t)=2−2Φ(z_{1−α/2}/√t), or Pocock-type α·ln(1+(e−1)t). For v1 without multivariate-normal integration, test look k at its
  **incremental spend** Δα_k = α(t_k)−α(t_{k−1}). The union bound makes this valid but conservative. Exact boundaries
  (Armitage–McPherson–Rowe recursion) can come later.
- **Arms:** at each look, run Holm–Bonferroni across the k arm-vs-control primary hypotheses at level Δα_k. An arm that crosses the boundary is
  nominated and stops. Use a **non-binding** futility rule (drop an arm when the upper CI bound is below the preregistered min effect). Because it is non-binding, alpha is unchanged.
- **Metrics:** a fixed-sequence (hierarchical gatekeeping) order. Primary quality first; secondary and guardrail metrics are tested only for arms
  that passed. Guardrails such as cost, latency, and failure rate are **non-inferiority** tests with preregistered margins. Do not build a composite
  score. Show Pareto (`pareto.rs`) for reporting only.
- **CIs:** Wilson for binary rates (failure, abstention, citation-supported), at z for α/k. Use a paired percentile bootstrap (seeded, B≥2000) for
  continuous differences. Bootstrap under-covers at n<15, so also report the permutation-inverted interval and flag small-n.
- **Failures:** use intention-to-treat. An arm's final failed attempt scores the metric floor and counts toward the failure-rate guardrail.
  Never drop it, as eval.rs:158-165 does today.
- **Confirmation:** the nominated arm and the control run in a separately signed `HoldoutConfirmation` campaign on a disjoint query
  digest, as a single fixed-n test at full α. Promotion needs both campaigns.
- **Implementation without heavy deps (~300 LoC):** Φ via an erfc rational approximation (W. J. Cody or A&S 7.1.26), Φ⁻¹ via
  Acklam/Wichura AS241, Holm, sign-flip enumeration and Monte Carlo, bootstrap, Wilson(z), HL estimator. For the RNG, avoid `StdRng`,
  because its algorithm is not stable across rand versions. Use `rand_chacha` (already in `Cargo.lock` via rand) or a 20-line PCG, and record the algorithm id in the prereg. Golden-test it against R/SciPy values that are hard-coded into the tests.

## 4. Spend reservation under concurrency

Do not reuse `BudgetManager` for this. It is fail-open when no budget entry exists (`budget/mod.rs:797-799`, `:761-762`).
It covers tokens only and lives in process memory. Its claim that the reservation is "superseded, not double-counted" (`:791-794`) is
false: `record_usage` adds real tokens on top of the reservation (`:370-373`). `record_cost` uses Relaxed atomics after the fact (`:395-397`).

Design:
- A `SpendLedger` keyed by `campaign_id`, durable in vox-db, with a pure `trial-core` state machine:
  `reserve(worst_case) → Reservation{id}` and `settle(id, actual) / release(id)`. Admit with a single statement:
  `UPDATE campaign_spend SET reserved = reserved + :w WHERE campaign_id=:c AND state='Running' AND spent + reserved + :w <= cap`
  and require `changes()==1`, all inside the same transaction that inserts the `Reserved` run row. This holds across processes.
- Per-attempt worst case = Σ over stages(`max_tokens`·(1+prompt bound)) × **cascade fan-out** (`stages.rs:511-519`, where a
  cascade may try several candidates) × waves × max subqueries × verifier claim cap. Compute it from the signed condition, never
  from a runtime estimate. Cost uses the max per-token price across every candidate in the cascade. If a price is unknown, refuse admission.
- Settlement: actual ≤ reserved is asserted. If actual > reserved, mark the run `Failed{BudgetViolation}` and stop the campaign (a fail-closed invariant).
  Hold all three axes: tokens, calls, and cost in micros (i64, never f64).
- Enforcement needs a metering seam. Thread `LlmResponse` usage out of `chat_with_cascade` (stages.rs:536) into a per-run
  accumulator, and enforce a per-run call counter *inside* the call wrapper. That way a runaway loop hits the reservation
  ceiling before the HTTP call is made, not after.
- Crash recovery: on startup, reservations whose run is not terminal are settled at **worst case** (conservative). The run goes to
  `Interrupted`, and a linked retry is created only if the remaining cap admits it.

## 5. Cache and ambient-state isolation risks in the pipeline

`run_research_with_context_and_session` (`pipeline.rs:51-1118`) reads and writes a lot of shared state:
1. **Result cache short-circuit** (`pipeline.rs:65-73`). The key (`pipeline_cache.rs:61-73`) omits `domain_mode`, `waves`,
   `site_scope`, and every `ResearchConfig` knob (models, verifier thresholds, context budget). An arm that differs only in
   waves or model gets **another arm's answer**. The pipeline also writes back during trials (`pipeline.rs:1110-1115`), so replicates stop being independent.
2. **Claim-verdict cache**: 14-day TTL, confidence ≥0.80 (`pipeline.rs:416-425`). Verdicts leak across arms and verifier thresholds.
3. **Session id collision**: `session_key = fnv(query|scope)` (`pipeline.rs:106-110`). `create_research_session` does
   `INSERT OR IGNORE` and then `last_insert_rowid()` (`crates/vox-db/src/research_pipeline.rs:33-42`). On a duplicate, the rowid is stale
   and may belong to an unrelated row, so arms and replicates write stage updates and artifacts to the wrong session. Pass `precreated_session_id` per run.
4. **Learned search policy**: rolling feedback, domain penalties, and blacklists come from the DB (`pipeline.rs:1185-1208`). Arm order and
   concurrent writes change retrieval mid-campaign. Pin `config.search_policy_feedback` (`:1190`) and snapshot the penalties into the prereg.
5. **Env/registry ambient state**: `ProviderRegistry::from_env_with_config` (`:83`), a fresh `ModelRegistry::new()` plus model resolution
   (`:84-87`), and the synthesized `SearchRuntimeContext` from `current_dir`/`temp_dir` with `MEMORY.md` (`:226-230`). eval.rs uses
   `cwd/memory.md` (`eval.rs:146-151`). The "memory on/off" arm needs an explicit per-run memory path or `None`.
6. **Cascade fallback** (`stages.rs:536-545`): the answering model can differ from the arm's model. Record `resolved_models`
   and treat a mismatch as a protocol deviation, or disable cascade for trial arms.
7. **Silent imputation**: judge failure becomes `fallback_quality_score` (`pipeline.rs:889-892`). Treat it as a missing primary
   metric (failure), not as a score.
8. **Process env in tests**: `std::env::set_var("OPENROUTER_BASE_URL", …)` (`wave.rs:712`) is global and races under parallel nextest.
   Inject an endpoint instead.

Fix: add a `TrialIsolation { cache: Bypass, verdict_cache: Bypass, persist: false, policy_snapshot, memory_path, session_id }` to the
call, with default-deny in trial mode. Cache keys should hash the full canonical condition.

## 6. Discriminating red tests (hermetic: fake LLM and fake search, no HTTP)

1. A tampered expanded prereg fails verification. A key outside the allowlist is refused (today's gate approves it). Gate wiring: removing the runner's `check_campaign` call must make a test fail, checked by mutation.
2. Canonicalisation: two preregs that differ only in nested key order produce the same id. A NaN in the prereg is rejected.
3. Two arms that differ only in `waves` (or model) on the same query produce distinct fake-LLM call logs, and `served_from_cache=false` for both.
4. Session isolation: two runs of the same query get distinct session ids, and stage writes land on their own session (catches research_pipeline.rs:33-42).
5. Reservation race: N tasks try to reserve against a cap that fits N−1. Exactly N−1 are admitted, across two `VoxDb` handles.
6. A fake provider that over-reports usage beyond the reservation leads to `BudgetViolation` and the campaign is `Stopped`. Unknown price means refused.
7. Call ceiling: a fake looping cascade stops at the reserved call count *before* the (k+1)th fake HTTP call.
8. Interrupt mid-run, then restart: the old run is `Interrupted` and immutable, a new run has `attempt_index=1` and `retry_of_run_id` set, and spend is settled at worst case.
9. A failed attempt counts as the floor score in the analysis. The arm mean is lower than a drop-failures mean.
10. Sign-flip exact p for a known vector matches the hard-coded SciPy value. Wilson(z) and Φ/Φ⁻¹ match reference values to 1e-9.
11. Alpha spending: Σ Δα_k = α. Monte Carlo under the null (fake equal arms, 2k campaigns, seeded) gives FWER ≤ α + MC error. With peeking but no spending, it would exceed α.
12. Holm with fixed-sequence gating: a secondary is never tested when the primary fails.
13. Holdout confirmation is refused if the query digest overlaps the exploration set or the signature is missing. A nomination alone cannot promote.
14. Determinism: same seed gives the same arm interleaving and bootstrap CI, bit-exact.
15. Judge failure records a missing primary metric, not `fallback_quality_score`.

## 7. Pitfalls

- Analysing replicates as independent samples. Analysing only successful attempts. Peeking outside preregistered looks (the runner must reject an analysis request for an unscheduled look).
- Using a composite quality score, or `pareto.rs` quality (which is reliability, `pareto.rs:73-76`), as the decision metric.
- Holding money in f64. Estimating the worst case from a runtime average. Forgetting cascade fan-out, waves, or the verifier cap.
  `StdRng` reproducibility drift. Wall-clock or ordering effects: interleave arms per query block using the seeded permutation.
- Web content drift between exploration and holdout. Record the retrieval snapshot and treat time as a covariate. Holdout must run soon after nomination.
- LLM-judge scorer drift. Pin `scorer_version` and the judge model in the prereg, and have the judge share no cache with the arms.
- Treating `PreregistrationV1` as runnable: its single metric and missing arms make it a conversion input only. Changing the
  schema mid-campaign: the signed condition must be the full expanded config, not a reference to mutable defaults.

## Summary
- Signing, trusty-id, gate, Wilson, and Pareto exist but are unwired or narrow. The gate has no production caller and accepts any key.
- The research pipeline leaks state across arms: a coarse result cache, a verdict cache, session-id collisions, and learned policy, so trial mode needs default-deny isolation.
- Spend: the existing `BudgetManager` is fail-open, tokens-only, and double-counts. Build a durable conditional-UPDATE ledger that covers all three axes and multiplies worst cases by cascade fan-out.
- Stats: a paired sign-flip test, Lan–DeMets incremental spending, Holm across arms, fixed-sequence across metrics, and Wilson/bootstrap CIs, in roughly 300 LoC with no heavy deps.
- Persist run rows before calls, keep terminal states immutable, link retries, and gate everything with 15 hermetic red tests.
