# Roadmap: Vox

## Milestones

- ✅ **v1.0 Architectural Housekeeping** — Phases 1–6 (shipped 2026-10-01) — [archive](milestones/v1.0-ROADMAP.md)
- 🚧 **v1.1 Research Trial Flywheel** — Phases 7–11 (in progress)

## Overview

v1.1 turns Vox's existing research pipeline, telemetry, evaluation, model routing, and retrieval into a governed measure–compare–promote–replay loop. It builds bottom-up: one trial identity and atomic persistence first, then the race harness and scorecards that consume it, then the knowledge loop whose ablations need the harness, then promotion and replay that need all three, and finally adversarial hardening across the whole loop. Each phase is usable on its own: Phase 7 already yields an observational baseline.

## Phases

**Phase Numbering:**

- Integer phases (7, 8, 9): Planned milestone work
- Decimal phases (7.1, 7.2): Urgent insertions (marked with INSERTED)

<details>
<summary>✅ v1.0 Architectural Housekeeping (Phases 1–6) — shipped 2026-10-01</summary>

- [x] Phase 1: Dead Crate Cleanup — Remove & Confirm (2/2 plans)
- [x] Phase 2: Wire Up & Reclassify Dormant Crates (2/2 plans)
- [x] Phase 3: Extract Misplaced Crates to Plugin Architecture (6/6 plans)
- [x] Phase 4: GUI/Dashboard Architecture Consolidation (2/2 plans)
- [x] Phase 5: Multi-Agent Coordination & Trust Hardening (7/7 plans)
- [x] Phase 6: Model Routing Transparency & ML Dependency Health (4/4 plans)

</details>

### 🚧 v1.1 Research Trial Flywheel

- [ ] **Phase 7: Trial Identity, Contracts & Observational Baseline** - One signed campaign/run identity, atomic Tier A persistence, structural telemetry, hermetic fixtures, and a no-policy-change baseline
- [ ] **Phase 8: Race Harness & Multidimensional Scorecards** - Bounded parallel condition matrix with spend reservations, arm isolation, group-sequential stopping, hard-gate + Pareto scoring, and reports
- [ ] **Phase 9: Verified Knowledge Loop** - Tier C evidence decision, authoritative findings with provenance, acknowledged projections, frozen per-arm manifests, and memory ablations
- [ ] **Phase 10: Promotion Governance & Replay** - Offline replay, shadow challengers, two-campaign confirmation, dual-control scoped designation, and suspension
- [ ] **Phase 11: Adversarial Trials & Hardening** - Adversarial/stress trials, mutation-tested gates, evidence bundles, and full gate pass
- [ ] **Phase 12: Code-Gen Benchmark Plumbing & Provenance** - MENS routable in eval-corpus, seeded unbiased sampling, paired statistics, complete provenance record, training lineage, signed run bundles on the Phase 7 identity
- [ ] **Phase 13: Triggered Language Benchmark & Cross-Language Tiers** - Behavioral syntax fingerprint, post-merge/tag/promotion triggers (never PR), syntax tier, grown held-out split, parallel-language and fresh tiers, estimand-labelled reports

## Phase Details

### Phase 7: Trial Identity, Contracts & Observational Baseline

**Goal**: Every research run is attributable to a signed, immutable campaign and persisted atomically, with structural-only telemetry and hermetic tests, so a baseline can be captured without changing any policy.
**Depends on**: Nothing (first v1.1 phase)
**Requirements**: TRIAL-01, TRIAL-02, TRIAL-03, TRIAL-04, TRIAL-05, STORE-01, STORE-02, TELEM-01, TELEM-02, METER-01, SCORE-04, TEST-01, BASE-01
**Gate**: Human approval of the canonical campaign/run Tier A schema before any migration (STORE-01).
**Success Criteria** (what must be TRUE):

  1. Signing a campaign freezes it; any tampered field fails verification; supersession yields a new linked `campaign_id`.
  2. `campaign_id`/`run_id`/`replicate_id`/`attempt_index`/`retry_of_run_id` appear consistently in pipeline, eval, telemetry, and DB rows for the same attempt.
  3. A forced mid-write failure leaves no partial campaign/run/sample batch in the database.
  4. The telemetry allowlist rejects unknown and content-bearing keys, and default tests pass with network access disabled.
  5. The production runner refuses an unsigned campaign or one signed by a non-allowlisted key (mutation-tested).
  6. Every LLM call in one run shares one trace context, and two attempts of the same query get distinct session IDs.
  7. Run results carry tokens, cost, latency, TTFT, and tool-call counts; a hallucinated no-evidence answer is not scored as an abstention.
  8. A baseline campaign records every BASE-01 metric with no policy change.

**Plans:** 1/12 plans executed in 7 waves

Plans:

- [ ] 07-01-PLAN.md — vox_crypto::canonical generic canonicalizer + orchestrator Trusty-URI rewire (byte-identical); user writes the 3 crate-edge exceptions (Wave 1)
- [ ] 07-02-PLAN.md — Clavis SecretIds for the trial signing key (vault-only) and signer allowlist; contracts regenerated (Wave 1)
- [ ] 07-03-PLAN.md — research pipeline trial seam: run label, result-cache bypass, judge failure Missing, content bridge off in trials, verdict-cache hits (Wave 1)
- [x] 07-04-PLAN.md — STORE-01 Tier A contract + checkpoint:decision (grain, model_benchmark_results option) (Wave 1)
- [ ] 07-05-PLAN.md — vox-research-trial-core: CampaignPreregV1, allowlist-first typestate gate, lifecycle, RunIdentity (Wave 2)
- [ ] 07-06-PLAN.md — vox-db `trials` domain at baseline 96 + loud stamped-without-tables guard (Wave 2)
- [ ] 07-07-PLAN.md — vox-db trials API: exclusive transaction, atomic batch persist with failpoint, lifecycle CAS, retries (Wave 3)
- [ ] 07-08-PLAN.md — SCORE-04 residuals (pure score_result, R5/R7/R8) + hermetic default lane (TEST-01) (Wave 3)
- [ ] 07-09-PLAN.md — production trial runner: typestate gate caller, fixture/pipeline executors, Clavis loaders (Wave 4)
- [ ] 07-10-PLAN.md — runner metering, structural telemetry producers + canary, interrupted-attempt retries (Wave 5)
- [ ] 07-11-PLAN.md — BASE-01 report from Tier A on vox-eval statistics + results.v2 schema (Wave 6)
- [ ] 07-12-PLAN.md — `vox research eval` baseline/sign/campaign flags + live baseline checkpoint:human-verify (Wave 7)

**Research**: `.planning/research/TRIAL-DESIGN.md`, `OBSERVABILITY.md`, `QUALITY-EVAL.md`

### Phase 8: Race Harness & Multidimensional Scorecards

**Goal**: One `vox research eval` invocation races preregistered conditions in parallel within a hard budget and produces comparable, statistically honest scorecards.
**Depends on**: Phase 7
**Requirements**: RACE-01, RACE-02, RACE-03, RACE-04, SCORE-01, SCORE-02, SCORE-03, REPORT-01
**Success Criteria** (what must be TRUE):

  1. Concurrent arms never exceed the campaign's reserved token/call/cost budget, including under reservation races, because admission goes through a durable ledger rather than the in-memory `BudgetManager`.
  2. Two arms with different configs never share result-cache, verdict-cache, or learned-search-policy state.
  3. Stopping boundaries are evaluated only at completed batches; spend, safety, and wall-time stops halt immediately and fail closed.
  4. Scorecards show hard gates and a Pareto frontier with versioned metrics; a hallucinated no-evidence answer fails a hard gate; the legacy `quality_score` cannot drive eligibility.
  5. The campaign report compares each arm against baseline and champion in JSON and a concise CLI summary.

**Plans**: TBD

### Phase 9: Verified Knowledge Loop

**Goal**: Verified findings accumulate authoritatively and searchably, and trials measure whether that knowledge helps later runs without contaminating evaluations.
**Depends on**: Phase 8
**Requirements**: STORE-03, KNOW-01, KNOW-02, KNOW-03, KNOW-04, KNOW-05
**Gate**: Human decision on Tier C evidence-artifact ownership and retention (STORE-03) before replay- or promotion-eligible live campaigns.
**Success Criteria** (what must be TRUE):

  1. A verified finding is retrievable through research queries with full provenance; negative findings are retrievable too.
  2. A failed memory or search projection is visible with status and is reconciled on retry.
  3. A finding written after signing never appears in that campaign's arms.
  4. All four memory ablations run and report knowledge-reuse impact.

**Plans**: TBD

### Phase 10: Promotion Governance & Replay

**Goal**: The system proposes and validates its next experiment but cannot promote a worse or unaudited policy, and any campaign can be replayed offline.
**Depends on**: Phase 9
**Requirements**: REPLAY-01, PROMO-01, PROMO-02, PROMO-03, PROMO-04, PROMO-05
**Success Criteria** (what must be TRUE):

  1. Replay with provider access disabled reproduces a campaign's scores from captured evidence.
  2. A challenger lacking holdout confirmation, replay success, or provenance completeness is never eligible.
  3. Designation fails without two independent authorized approvers, rejects role conflicts, and never mutates runtime policy.
  4. The safety principal can suspend a champion alone; revocation or reinstatement requires the quorum; evidence expiry suspends eligibility.

**Plans**: TBD

### Phase 11: Adversarial Trials & Hardening

**Goal**: The whole loop survives adversarial conditions, every guard is proven by mutation, and each campaign ships an auditable evidence bundle.
**Depends on**: Phase 10
**Requirements**: AUDIT-01, AUDIT-02, AUDIT-03, AUDIT-04
**Success Criteria** (what must be TRUE):

  1. Adversarial trials (hallucination, conflicting memory, provider failure, budget exhaustion, cancellation) all end in the expected fail-closed state.
  2. Removing any security/reliability guard makes its test fail.
  3. Every campaign emits an evidence bundle with a working replay command.
  4. `vox ci data-storage-guard`, drift checks, docs lint, and `vox ci pre-push --complete` pass.

**Plans**: TBD

### Phase 12: Code-Gen Benchmark Plumbing & Provenance

**Goal**: Any model, MENS included, can be scored writing Vox with reproducible sampling, honest intervals, and a provenance record that traces every result to its language version, benchmark content, model weights, and training data.
**Depends on**: Phase 7 (campaign/run identity, signing, atomic Tier A persistence)
**Requirements**: BENCH-01, BENCH-02, BENCH-03, BENCH-04, BENCH-05, BENCH-06, BENCH-13
**Design**: `docs/src/architecture/vox-mens-continuous-benchmark-provenance-design-2026-10-10.md` §§2, 5, 8, 9
**Success Criteria** (what must be TRUE):

  1. `vox model eval-corpus --model mens/<run>` produces scored samples through the `vox_actor_runtime::llm` facade (no hardcoded OpenRouter config).
  2. Two runs with the same seed and config produce identical sample sets for deterministic providers; `eval_local` pass@k equals the unbiased estimator.
  3. Every report carries per-arm bootstrap CIs, exact McNemar + Holm for pairwise claims, and the resolution floor; no CI-overlap significance remains.
  4. A run missing any required provenance field is persisted with `run_complete = false` and excluded from comparisons (mutation-tested).
  5. Training writes an `AdapterCard` with the `train.jsonl` sha256 and training syntax fingerprint; eval refuses to label a `mens/*` row without one.
  6. Each run emits a vox-crypto-signed bundle; a tampered bundle fails verification.

**Plans**: 8/9 plans executed

Plans:

- [x] 12-01-PLAN.md — Seed through the LLM facade and wire (Wave 1)
- [x] 12-02-PLAN.md — eval-corpus registry routing (incl. mens/*), seeded n-sample loop, infra classification (Wave 2)
- [x] 12-03-PLAN.md — MENS serve + Candle sampler seed/top_p (metal and CUDA); eval-local unbiased pass@k (Wave 1)
- [x] 12-04-PLAN.md — vox-eval typed report, §5 provenance record, single completeness gate, CI/MDD stats (Wave 1)
- [x] 12-05-PLAN.md — `vox model eval-compare`: McNemar + Holm, paired bootstrap, resolution floor (Wave 2)
- [x] 12-06-PLAN.md — Training lineage AdapterCard writer and eval-side card check (Wave 1)
- [x] 12-07-PLAN.md — eval-corpus provenance collectors, typed report, mens label refusal, scoreboard guard (Wave 3)
- [x] 12-08-PLAN.md — pass@1 after k compiler-feedback turns (design §8) (Wave 4)
- [ ] 12-09-PLAN.md — Phase 7 run identity, signed offline-verifiable bundle, atomic persist (Wave 5, blocked on Phase 7)

### Phase 13: Triggered Language Benchmark & Cross-Language Tiers

**Goal**: The benchmark runs itself on every syntax change, release tag, and MENS promotion — never on PRs — and produces estimand-labelled results that can support the claims the design doc permits, including Vox-vs-other-language comparisons on fresh problems.
**Depends on**: Phase 12
**Requirements**: BENCH-07, BENCH-08, BENCH-09, BENCH-10, BENCH-11, BENCH-12
**Gate**: Human decisions on the results store, MENS inference host, and held-out authoring budget (design doc §12) before the first triggered campaign.
**Success Criteria** (what must be TRUE):

  1. A behavior-preserving parser refactor leaves the syntax fingerprint unchanged and a one-production grammar change flips it (mutation tests both ways); the crate version does not participate.
  2. A push to `main` that changes the fingerprint dispatches exactly one benchmark campaign; a push that does not, dispatches none; no PR or merge-queue event ever runs it.
  3. When any reference solution fails under new syntax, the campaign records `reference_migration_required` and calls no model.
  4. The syntax tier covers a reported percentage of the feature matrix with negative fixtures; the MENS held-out split has ≥150 problems; every fixture meets the preregistered mutant-kill threshold.
  5. Parallel Python/TypeScript/Rust problems run in a pinned, network-less verifier; fresh and contaminated tiers are reported separately.
  6. Reports label estimands E1–E6, keep conditions as separate columns, include the COI statement, and suppress E5 when the anchor subset shows model drift; the unbacked claim in `language-benchmark-2026.md` is retracted.

**Plans**: TBD

## Progress

**Execution Order:** Phases execute in numeric order: 7 → 8 → 9 → 10 → 11. Phases 12–13 depend only on Phase 7 and may run in parallel with 8–11.

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1–6 | v1.0 | 23/23 | Complete | 2026-10-01 |
| 7. Trial Identity, Contracts & Observational Baseline | v1.1 | 1/12 | In Progress|  |
| 8. Race Harness & Multidimensional Scorecards | v1.1 | 0/TBD | Not started | - |
| 9. Verified Knowledge Loop | v1.1 | 0/TBD | Not started | - |
| 10. Promotion Governance & Replay | v1.1 | 0/TBD | Not started | - |
| 11. Adversarial Trials & Hardening | v1.1 | 0/TBD | Not started | - |
| 12. Code-Gen Benchmark Plumbing & Provenance | v1.1 | 8/9 | In Progress|  |
| 13. Triggered Language Benchmark & Cross-Language Tiers | v1.1 | 0/TBD | Not started | - |
