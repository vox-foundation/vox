# Requirements: Vox v1.1 Research Trial Flywheel

**Defined:** 2026-10-01
**Core Value:** The compiler, orchestrator, and runtime that everything else depends on must keep building and passing CI throughout.
**Milestone value:** Each research run produces reproducible evidence, comparable quality/latency/cost metrics, searchable verified knowledge, and a safe proposal for the next experiment.

**Source:** Research Trial Flywheel plan (grill rulings G1–G25, review rulings R1–R9) and `.planning/research/`. v1.0 requirements are archived in `.planning/milestones/v1.0-REQUIREMENTS.md`.

## v1.1 Requirements

### Trial Contract

- [ ] **TRIAL-01**: Every campaign has one signed `campaign_id`; every arm attempt has a unique `run_id` plus `replicate_id`, `attempt_index`, and `retry_of_run_id`, propagated through the research pipeline, the eval runner, telemetry, and Tier A rows.
- [ ] **TRIAL-02**: A versioned, immutable expanded campaign preregistration carries hypotheses, arms, metrics, hard gates, validity rules, budgets, statistical tests, stopping rules, and decisions; signing plus immediate verification freezes it, tampering is rejected, and supersession creates a new linked campaign. `PreregistrationV1` is a conversion input only.
- [ ] **TRIAL-03**: Campaign lifecycle Draft → Signed → Running → Completed/Failed/Stopped is enforced; illegal transitions are rejected.
- [ ] **TRIAL-04**: An interrupted attempt is always terminalized and retried under a new linked `run_id`; same-`run_id` resume is not possible in v1.1.
- [ ] **TRIAL-05**: The campaign preregistration gate is called by the production trial runner (proven by a mutation test) and accepts only signers from an allowlisted key set; canonicalization sorts keys at every depth and rejects NaN/non-finite numbers; hashing and signing go through `vox-crypto` (no direct `sha2` import).

### Persistence & Telemetry

- [ ] **STORE-01**: Campaign/run/sample data lives in an active Tier A fragment (new domain per G22, or a scientia extension, as decided at human schema approval) defined by a canonical contract and approved before migration; it migrates at the next free DB baseline (95 as of 2026-10-01), with a guard that fails loudly if a shared DB already carries that version without these tables; quarantined `scientia_prereg` is untouched.
- [ ] **STORE-02**: Each campaign/run/sample batch persists atomically; a forced mid-write failure rolls back the whole unit.
- [ ] **STORE-03**: Tier C evidence-artifact ownership, retention, deletion/tombstones, hashing, encryption/access, and Tier A references are decided and implemented; until then live campaigns are observational only and never replay- or promotion-eligible.
- [ ] **TELEM-01**: Research-trial telemetry is structural only (IDs, hashes, versions, numeric metrics, bounded statuses, stage/tool identifiers, reason codes) and enforced by an allowlist that rejects unknown and content-bearing keys; the allowlist lands before any new producer.
- [ ] **TELEM-02**: One trace context spans every LLM/tool call in a run; LLM turn records carry the real session/run ID (no `anon-session`); research session IDs are unique per attempt and never resolved via `INSERT OR IGNORE` + `last_insert_rowid()`.
- [ ] **METER-01**: Tokens (in/out), provider-reported or estimated cost, latency, real TTFT (or explicit `NotApplicable` for non-streaming), and tool-call/step counts flow from `LlmResponse` through research stages into run results; run duration covers verification, citation audit, and persistence.
- [ ] **SCORE-04**: Evaluator correctness: a no-evidence non-abstaining answer is not counted as abstention; zero-citation answers do not score citation precision 1.0; groundedness is negation-aware and does not auto-pass very short answers; failed queries count as worst score rather than being dropped; no 0.5 imputation for missing gold; `latency_p50_ms` is a true median; harness output validates against its results schema.
- [ ] **TEST-01**: Default tests use deterministic research/executor fixtures and perform no model or external HTTP access; live Crossref/OpenAlex coverage runs only in an explicit ignored/live lane.
- [ ] **BASE-01**: An observational baseline with no policy changes records answer quality, groundedness, citation precision/diversity, retrieval hit rate, latency/TTFT, tokens, estimated cost, tool calls/steps, completion rate, reproducibility, and knowledge-reuse rate.

### Race Harness

- [ ] **RACE-01**: `vox research eval` runs a campaign matrix with bounded concurrency (zero/invalid concurrency rejected), seeds and config snapshots, stage-model attribution, and warm/cold-cache modes; it remains the single CLI surface (run/report/replay modes).
- [ ] **RACE-02**: Arms are isolated from ambient state: campaign/arm/config/model/threshold/context hashes participate in cache identity, and search-policy feedback/domain penalties are disabled or frozen per signed arm.
- [ ] **RACE-03**: Live calls are admitted only through atomic worst-case token/call/cost reservations recorded in a durable vox-db ledger (single conditional update in the run-row transaction; the in-memory fail-open `BudgetManager` is not reused), with worst cases multiplied by cascade fan-out, against frozen campaign pricing; providers without a defensible pre-dispatch liability bound are ineligible; concurrent reservations cannot overspend.
- [ ] **RACE-04**: Preregistered group-sequential batches evaluate stopping boundaries only after admitted batches finish; safety, spend, and wall-time stops are immediate and fail-closed; cancellation and partial failure leave consistent state.

### Scoring & Reports

- [ ] **SCORE-01**: Metrics and evaluator are versioned with defined direction, normalization, missing-data/abstention behavior, cold-start/division-by-zero guards, and weighted views that must sum to 1.00; every scoring-policy change carries a fixed-corpus snapshot.
- [ ] **SCORE-02**: Arms are scored with hard gates plus a Pareto frontier, never a single opaque number; the legacy aggregate `quality_score` is display-only; hallucinated no-evidence answers fail hard gates.
- [ ] **SCORE-03**: Confirmatory metric families apply multiplicity/sequential adjustment; exploratory outcomes may nominate later trials but cannot make the current arm promotion-eligible.
- [ ] **REPORT-01**: Each campaign produces a machine-readable JSON report and a concise CLI summary comparing arms against baseline and the current champion.

### Knowledge Loop

- [ ] **KNOW-01**: Verified findings persist authoritatively in Tier A with provenance, citations, content hashes, campaign/config IDs, confidence, supersession, and expiry/revalidation state; failed arms and negative findings remain searchable.
- [ ] **KNOW-02**: Memory (`MemoryManager::persist_fact`) and search-index writes are acknowledged projections with status, retry, and reconciliation, never proof of the authoritative write; a reconciler compares projected and authoritative content hashes, and campaign fact keys cannot collide at the memory cache cap.
- [ ] **KNOW-03**: Approved authored research is mirrored into `search_documents`/chunks and verified via `vox db retrieval-status` (extended to count documents, chunks, and projection states) and research queries; `docs/src/archive/` is never ingested.
- [ ] **KNOW-04**: Each arm uses a content-addressed knowledge manifest frozen at signing and retrieval scoped to that manifest's allowlist (knowledge-graph FTS lane, `MEMORY.md`, KB entries, and post-signing web cache disabled inside arms); no post-signing finding can enter that campaign except via a preregistered treatment snapshot.
- [ ] **KNOW-05**: Knowledge-reuse impact is measured with four ablations: empty, current, stale/conflicting, and curated verified memory.

### Promotion & Replay

- [ ] **REPLAY-01**: Replay deterministically re-evaluates a campaign offline from governed, content-addressed captured evidence with provider access disabled; a fresh live fetch is a new replication campaign.
- [ ] **PROMO-01**: After each campaign the system generates candidate policy changes from failure clusters and Pareto results and runs them as shadow challengers.
- [ ] **PROMO-02**: Promotion eligibility requires preregistered metrics, no hard-gate regression, concordant adjusted evidence from a nomination campaign and a separately signed holdout confirmation campaign, provenance completeness, and replay success.
- [ ] **PROMO-03**: Champion designation requires two distinct, cryptographically authenticated, policy-authorized humans independent of campaign signing, authorship, execution, and evidence curation; designation never mutates active runtime policy.
- [ ] **PROMO-04**: Champions are scoped by a signed, versioned workload-domain key; exact eligible scope wins over broader champions; changing precedence requires a new signed governance-policy version.
- [ ] **PROMO-05**: Proposal states AwaitingConfirmation → AwaitingApproval → Designated → Suspended/Revoked/EvidenceExpired are enforced; one conflict-free safety principal may suspend immediately; revocation, reinstatement, extension, or scope change requires the dual-control quorum; evidence expiry suspends eligibility without rewriting history.

### Audit & Hardening

- [ ] **AUDIT-01**: Adversarial trials cover hallucinated answers, stale/conflicting memory, provider failure, budget exhaustion, reservation races, and cancellation.
- [ ] **AUDIT-02**: Security/reliability gates (signing, reservations, quorum, frozen manifest, telemetry allowlist) are mutation-tested; each guard's test fails when the guard is removed.
- [ ] **AUDIT-03**: Every campaign emits an evidence bundle: config snapshot, trace/span linkage, samples, scores, confidence, promotion decision, and replay command.
- [ ] **AUDIT-04**: Docs and gates are current: `vox ci data-storage-guard`, telemetry/contract drift checks, docs lint, and `vox ci pre-push --complete` pass.

### Code-Gen Benchmark (design: `docs/src/architecture/vox-mens-continuous-benchmark-provenance-design-2026-10-10.md`)

- [x] **BENCH-01**: `vox model eval-corpus` resolves the model under test through the registry and `vox_actor_runtime::llm` (including local `mens/*` checkpoints) instead of hardcoding `LlmConfig::openrouter`.
- [x] **BENCH-02**: Sampling is seeded and recorded (temperature, top_p, n, k, seed, max_tokens); all n samples are always drawn; `vox mens eval-local` pass@k uses the unbiased estimator.
- [ ] **BENCH-03**: Reports use the existing `vox-eval` statistics: paired cluster bootstrap over problems for pass@k, exact McNemar + Holm for preregistered pairwise claims, and `min_detectable_difference` printed beside every comparison; CI overlap is never a significance test.
- [ ] **BENCH-04**: Every run record carries the design doc §5 provenance fields (syntax fingerprint, Cargo version, vox git sha, binary sha256, problem-set/prompt/context hashes, condition, model identity incl. weights/snapshot, sampling, runner, preregistration id); any missing field forces `run_complete = false` and exclusion from comparisons.
- [x] **BENCH-05**: The MENS training lane writes an `AdapterCard` with base revision, adapter sha256, `train.jsonl` sha256, corpus hash, and training syntax fingerprint; eval refuses to label a `mens/*` row without one and flags training/eval syntax mismatch.
- [ ] **BENCH-06**: Each benchmark run is a Phase 7 campaign/run persisted atomically and emits a `vox-crypto`-signed bundle (record, per-sample outcomes, failure classes, generated code) that verifies offline.
- [ ] **BENCH-13**: eval-corpus can report pass@1 after k compiler-feedback repair turns (k ≤ 3, default 0, design doc §8). Repaired outcomes are recorded alongside the original first attempt and never replace it.
- [ ] **BENCH-07**: A behavioral syntax fingerprint (BLAKE3 via `vox-crypto` over canonical parse outcomes/AST and diagnostic codes of a fixed probe corpus, excluding crate version, spans, and message text) lives in `vox-grammar-export`, mutation-tested in both directions.
- [ ] **BENCH-08**: Triggers: release tag, post-merge fingerprint change on `main` (tiny detector dispatches the heavy workflow), MENS promotion, and manual dispatch; never `pull_request`/`merge_group`; reference solutions are validated first and the campaign fails closed with `reference_migration_required`; a fixed anchor subset detects provider model drift.
- [ ] **BENCH-09**: A `contracts/eval/vox-syntax/` tier keyed to the compiler feature matrix, with negative fixtures, reports its feature coverage percentage.
- [ ] **BENCH-10**: The MENS-scored held-out split reaches ≥150 problems, every fixture's tests meet a preregistered mutant-kill threshold, and held-out material stays `training_eligible: false` behind the leakage gate.
- [ ] **BENCH-11**: A parallel-language tier (Vox, Python, TypeScript, Rust) runs in a pinned, network-less verifier with matched context budgets; a rolling fresh tier gated by `added_at`/`--cutoff` is reported separately from the contaminated tier; vendored sources are recorded with license provenance.
- [ ] **BENCH-12**: Reports label estimands E1–E6, keep conditions C0–C3 as separate columns, include a COI statement, suppress E5 on anchor drift, and validate against a versioned schema; the unbacked "92%" claim in `language-benchmark-2026.md` is retracted.

## v2 Requirements

Deferred. Tracked but not in the current roadmap.

- **TRIAL-RESUME-01**: Same-`run_id` checkpoint resume, once durable idempotent checkpoints and side-effect accounting are specified and tested.
- **PROMO-APPLY-01**: Governed application of a designated champion to active runtime policy.
- **REPORT-UI-01**: GUI/dashboard presentation of campaigns and scorecards.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Automatic runtime policy mutation | v1.1 promotion is a human-authorized designation only (G16). |
| A second research CLI or parallel trial stack | `vox research eval` is the single surface; reuse existing pipeline, telemetry, eval, routing, retrieval (R3). |
| Reviving `scientia_prereg` | Quarantined; a new Tier A domain replaces it (G22). |
| Content in default telemetry | Queries, answers, URLs, titles, snippets live only in governed evidence storage (G24). |
| GUI/dashboard work | Deferred to REPORT-UI-01. |
| Physical-GPU runtime validation | Not required by this milestone. |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| TRIAL-01 | Phase 7 | Pending |
| TRIAL-02 | Phase 7 | Pending |
| TRIAL-03 | Phase 7 | Pending |
| TRIAL-04 | Phase 7 | Pending |
| STORE-01 | Phase 7 | Pending |
| STORE-02 | Phase 7 | Pending |
| TRIAL-05 | Phase 7 | Pending |
| TELEM-01 | Phase 7 | Pending |
| TELEM-02 | Phase 7 | Pending |
| METER-01 | Phase 7 | Pending |
| SCORE-04 | Phase 7 | Pending |
| TEST-01 | Phase 7 | Pending |
| BASE-01 | Phase 7 | Pending |
| RACE-01 | Phase 8 | Pending |
| RACE-02 | Phase 8 | Pending |
| RACE-03 | Phase 8 | Pending |
| RACE-04 | Phase 8 | Pending |
| SCORE-01 | Phase 8 | Pending |
| SCORE-02 | Phase 8 | Pending |
| SCORE-03 | Phase 8 | Pending |
| REPORT-01 | Phase 8 | Pending |
| STORE-03 | Phase 9 | Pending |
| KNOW-01 | Phase 9 | Pending |
| KNOW-02 | Phase 9 | Pending |
| KNOW-03 | Phase 9 | Pending |
| KNOW-04 | Phase 9 | Pending |
| KNOW-05 | Phase 9 | Pending |
| REPLAY-01 | Phase 10 | Pending |
| PROMO-01 | Phase 10 | Pending |
| PROMO-02 | Phase 10 | Pending |
| PROMO-03 | Phase 10 | Pending |
| PROMO-04 | Phase 10 | Pending |
| PROMO-05 | Phase 10 | Pending |
| AUDIT-01 | Phase 11 | Pending |
| AUDIT-02 | Phase 11 | Pending |
| AUDIT-03 | Phase 11 | Pending |
| AUDIT-04 | Phase 11 | Pending |
| BENCH-01 | Phase 12 | Complete |
| BENCH-02 | Phase 12 | Complete |
| BENCH-03 | Phase 12 | Pending (12-04 partial) |
| BENCH-04 | Phase 12 | Pending (12-04 partial) |
| BENCH-05 | Phase 12 | Complete |
| BENCH-06 | Phase 12 | Pending |
| BENCH-13 | Phase 12 | Pending |
| BENCH-07 | Phase 13 | Pending |
| BENCH-08 | Phase 13 | Pending |
| BENCH-09 | Phase 13 | Pending |
| BENCH-10 | Phase 13 | Pending |
| BENCH-11 | Phase 13 | Pending |
| BENCH-12 | Phase 13 | Pending |

**Coverage:**

- v1.1 requirements: 50 total
- Mapped to phases: 50
- Unmapped: 0 ✓

---
*Requirements defined: 2026-10-01; reconciled with .planning/research/ the same day*
