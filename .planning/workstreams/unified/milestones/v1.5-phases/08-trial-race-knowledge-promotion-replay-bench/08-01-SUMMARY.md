---
phase: 08-trial-race-knowledge-promotion-replay-bench
plan: 01
status: complete
completed_at: 2026-10-15
---

# Summary: Phases 08–11 & 42 Plan 01 — Research Trial Flywheel & Triggered Code-Gen Benchmark Governance

## Changes Made
1. **`crates/vox-eval/src/trial_flywheel.rs` & `crates/vox-eval/src/lib.rs`**:
   - **Phase 08 (`RACE-01..04`, `SCORE-01..03`, `REPORT-01`)**: Implemented `CampaignLifecycle` state machine (`Draft -> Frozen -> Running -> Evaluating -> Completed | Halted`), `ArmIsolationKey::for_arm` guaranteeing zero shared state across arms, `SpendReservationLedger` with worst-case cost reservation (`reserve_worst_case`) preventing concurrent overspend, `evaluate_stopping_rule` for anytime-valid confidence boundaries, `validate_metric_weights`, and `evaluate_pareto_scorecards` computing both the non-dominated Pareto frontier and scalarized utility under frozen pre-registered weights.
   - **Phase 09 (`KNOW-01..05`)**: Implemented `FrozenKnowledgeManifest` enforcing read-only trial runs once signed (unless `allow_knowledge_write_treatment` is explicitly enabled) and rejecting `docs/src/archive/` paths (`ArchiveIngestionBlocked`), `reconcile_projection_hashes` across all 4 required projections (`MarkdownSsot`, `VectorStore`, `SearchIndex`, `RelationalRegistry`), and `evaluate_memory_ablations` across all 4 mandatory regimes (`Empty`, `Current`, `StaleConflicting`, `CuratedVerified`).
   - **Phase 10 (`REPLAY-01`, `PROMO-01..05`)**: Implemented `OfflineReplayBundle` + `verify_offline_replay`, `evaluate_promotion_eligibility` requiring both an exploratory win and an independent holdout confirmation win with zero hard-gate regressions, `authorize_champion_designation` enforcing dual-control conflict-free quorum (blocking campaign authors and single-principal approvals), `suspend_champion_by_safety_principal` allowing unilateral safety rollback, and `resolve_scoped_champion` preventing silent cross-scope champion transplantation.
   - **Phase 11 (`HARD-01..04`, `AUDIT-01..03`)**: Implemented `run_adversarial_trial_suite()` proving all 6 adversarial scenarios (prompt-injected task payload, poisoned retrieval chunk, uncalibrated LLM-only judge, concurrent spend race, archive-path leak, and author self-approval quorum) fail closed.
   - **Phase 42 (`BENCH-07..13`)**: Implemented `should_dispatch_benchmark_campaign` and `assemble_triggered_benchmark_report` enforcing estimands `E1..E6`, conflict-of-interest disclosure, and automatic suppression of `E5` (external parity claims) when external anchor hashes or prompt templates drift.

## Verification
- `timeout 900s cargo test -p vox-eval --lib trial_flywheel::tests` — **4 passed; 0 failed**.
