---
gsd_state_version: "1.0"
milestone: v1.1
milestone_name: Research Trial Flywheel
current_phase: 7
current_phase_name: Trial Identity, Contracts & Observational Baseline
status: Awaiting roadmap approval
stopped_at: Completed 12-08-PLAN.md
last_updated: "2026-10-12T15:55:18.335Z"
last_activity: 2026-10-01
last_activity_desc: Milestone v1.1 initialized (requirements, research, roadmap)
state_head: 6cc8eb5e2f27e4a0e351ca0a8d8ddd2c3ea1b645
progress:
  total_phases: 7
  completed_phases: 1
  total_plans: 21
  completed_plans: 9
  percent: 14
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-10-01)

**Core value:** The compiler, orchestrator, and runtime that everything else depends on must keep building and passing CI throughout.
**Current focus:** v1.1 Research Trial Flywheel — Phase 7 (Trial Identity, Contracts & Observational Baseline)

## Current Position

Phase: 7 of 11 (Trial Identity, Contracts & Observational Baseline)
Plan: —
Status: Awaiting roadmap approval; then ready to plan
Last activity: 2026-10-01 — Milestone v1.1 initialized

Progress: [█░░░░░░░░░] 14%

## Performance Metrics

**Velocity:**

- Total plans completed: 0 (v1.1)
- Average duration: -
- Total execution time: 0 hours

*Updated after each plan completion*
**Per-Plan Metrics:**

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 12 P04 | 35min | 3 tasks | 3 files |
| Phase 12 P03 | ~45min | 3 tasks | 10 files |
| Phase 12 P01 | 25min | 2 tasks | 16 files |
| Phase 12 P02 | ~2h (13 min host exec stall) | 3 tasks | 2 files |
| Phase 12 P05 | ~100min | 3 tasks | 10 files |
| Phase 12 P06 | ~50min | 2 tasks | 4 files |
| Phase 12 P07 | ~1h | 3 tasks | 2 files |
| Phase 12 P08 | ~1h | 2 tasks | 4 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- v1.1 governance rulings G1–G25 and R1–R9 confirmed by the user; see the Research Trial Flywheel plan.
- Research reconciliation added TRIAL-05, TELEM-02, METER-01, and SCORE-04 to Phase 7 because a baseline on the current tree would be untrustworthy (.planning/research/SUMMARY.md).
- [Phase 7]: 12-04: completeness gate is one private fn (gate_reasons) shared by assemble and recheck_complete; required provenance fields have no serde default
- [Phase 7]: 12-03: CUDA Candle plugin gets the same seed/top_p/tie-break change as metal (no D-12 deviation); sample_seed(7,"001",0) pinned to 780188561
- [Phase 7]: 12-01: LlmConfig.seed needs no serde attribute; absent key deserializes to None
- [Phase 7]: 12-02: unknown mens/* id is a hard error; no silent cloud fallback for local checkpoints
- [Phase 7]: 12-05: eval-compare floor check uses exact-integer first-attempt difference (one division) so a difference equal to the floor is resolved, not one ulp under
- [Phase 7]: 12-08: repair turns reuse the single llm_chat path; repaired outcomes live in passed_by_turn and never replace the original AttemptOutcome (mutation-proved)
- [Phase 7]: 07-04: STORE-01 approved by user 2026-10-12: item-batch grain, model_benchmark_results option A

### Pending Todos

None yet.

### Blockers/Concerns

- Phase 7 migration is gated on human approval of the Tier A schema and placement (new domain vs scientia extension; baseline 95).
- Replay/promotion-eligible campaigns are gated on the Tier C evidence-storage decision (STORE-03).
- A dedicated trial-engine crate and the `vox-cli-research` → `vox-eval` edge need user-authorized crate-edge exceptions.

## Deferred Items

Items acknowledged and deferred at milestone close, most recent first:

| Category | Item | Status | Deferred At | Milestone |
|----------|------|--------|-------------|-----------|
| Scope | Same-run_id resume, champion auto-apply, GUI scorecards | v2 | 2026-10-01 | v1.1 |
| Scope | Hosted Mens/BaaS (ADR-009), Web IR standalone track (ADR-012) | v2 | 2026-09-22 | v1.0 |
| Scope | SWC parser migration (ADR-035), transport crypto provider collapse, God-object decomposition | Out of scope | 2026-09-22 | v1.0 |

## Session Continuity

Last session: 2026-10-11T16:08:18.429Z
Stopped at: Completed 12-08-PLAN.md
Resume file: None
