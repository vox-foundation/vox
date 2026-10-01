---
gsd_state_version: "1.0"
milestone: v1.1
milestone_name: Research Trial Flywheel
status: Awaiting roadmap approval
stopped_at: v1.1 requirements and roadmap drafted and reconciled with research; awaiting user approval and schema/Tier C decisions
last_updated: "2026-10-01T06:55:00.000Z"
last_activity: 2026-10-01
last_activity_desc: Milestone v1.1 initialized (requirements, research, roadmap)
progress:
  total_phases: 5
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
  percent: 0
current_phase: 7
current_phase_name: Trial Identity, Contracts & Observational Baseline
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

Progress: [░░░░░░░░░░] 0%

## Performance Metrics

**Velocity:**

- Total plans completed: 0 (v1.1)
- Average duration: -
- Total execution time: 0 hours

*Updated after each plan completion*

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- v1.1 governance rulings G1–G25 and R1–R9 confirmed by the user; see the Research Trial Flywheel plan.
- Research reconciliation added TRIAL-05, TELEM-02, METER-01, and SCORE-04 to Phase 7 because a baseline on the current tree would be untrustworthy (.planning/research/SUMMARY.md).

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

Last session: 2026-10-01
Stopped at: v1.1 roadmap drafted; awaiting approval
Resume file: None
