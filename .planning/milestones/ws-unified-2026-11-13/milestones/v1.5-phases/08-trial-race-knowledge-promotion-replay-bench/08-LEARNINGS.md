---
phase: 8
phase_name: "trial-race-knowledge-promotion-replay-bench"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 1
  lessons: 2
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 8 Learnings: trial-race-knowledge-promotion-replay-bench

## Decisions

### Phase 08 (RACE-01..04, SCORE-01..03, REPORT-01)
**Phase 08 (`RACE-01..04`, `SCORE-01..03`, `REPORT-01`)**: `CampaignLifecycle` state machine, per-arm isolation key (`ArmIsolationKey`), atomic `SpendReservationLedger` (`reserve_worst_case`), anytime-valid sequential stopping rules, and multi-objective Pareto non-dominated frontier + scalarized scorecards with frozen pre-registered metric weights.

**Rationale:** Primary phase objective established in plan.
**Source:** 08-01-PLAN.md

## Lessons

### Verified by trial_flywheel::tests::phase08_race_harness_reservations_sto...
Verified by `trial_flywheel::tests::phase08_race_harness_reservations_stopping_and_pareto_scorecards`.

**Context:** Verified in phase acceptance checks.
**Source:** 08-VERIFICATION.md

---

### Verified by trial_flywheel::tests::phase09_frozen_knowledge_manifest_and...
Verified by `trial_flywheel::tests::phase09_frozen_knowledge_manifest_and_four_way_memory_ablations`.

**Context:** Verified in phase acceptance checks.
**Source:** 08-VERIFICATION.md

## Patterns

## Surprises

