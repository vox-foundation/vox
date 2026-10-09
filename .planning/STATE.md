---
gsd_state_version: "1.0"
milestone: v1.1
milestone_name: Research Trial Flywheel
current_phase: "06.2"
current_phase_name: Capability Index SSOT (INSERTED)
status: executing
stopped_at: Phase 06.2 context gathered
last_updated: "2026-10-09T06:13:05.330Z"
last_activity: 2026-10-08
last_activity_desc: Milestone v1.1 initialized (requirements, research, roadmap)
state_head: a3c4e8fda7890c77a62c851bd8c71090cc73cf1d
progress:
  total_phases: 11
  completed_phases: 1
  total_plans: 8
  completed_plans: 0
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-10-01)

**Core value:** The compiler, orchestrator, and runtime that everything else depends on must keep building and passing CI throughout.
**Current focus:** v1.1 — inserted Phase 06.2 (Capability Index SSOT), first of discoverability phases 06.2–06.6, then Phase 7

## Current Position

Phase: 06.2 (Capability Index SSOT (INSERTED)) — READY TO EXECUTE
Plan: —
Status: Ready to execute
Last activity: 2026-10-08 — Discoverability phases 06.2–06.6 (DISC-01..16) inserted into v1.1

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

- 2026-10-08: Discoverability inserted into v1.1 as phases 06.2–06.6 (user chose insertion over a separate milestone). Audience: contributors, .vox app authors, GUI end users. Done = coverage gate + one capability SSOT + time-to-find journeys + orphan count 0.

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

Last session: 2026-10-09T03:46:51.158Z
Stopped at: Phase 06.2 context gathered
Resume file: .planning/phases/06.2-capability-index-ssot-inserted/06.2-CONTEXT.md
