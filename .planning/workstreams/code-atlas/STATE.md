---
gsd_state_version: "1.0"
milestone: v1.4
milestone_name: Code Atlas
status: planning
last_updated: "2026-10-11T16:20:00.000Z"
last_activity: 2026-10-11
progress:
  total_phases: 7
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
  percent: 0
---

# Project State

## Project Reference

See: `.planning/PROJECT.md`; workstream brief `CONTEXT.md`; roadmap `ROADMAP.md`; requirements `REQUIREMENTS.md` (25 v1.4 requirements, all mapped); research `research/SUMMARY.md`; handoff `HANDOFF.md`.

**Core value:** The Graphify codebase graph — crates → modules → files → symbols, including `.vox` kinds — is a navigable visual language on any branch or worktree, docked beside Chat so the atlas updates as the conversation names and changes code.

## Current Position

Phase: 25 of 25–31 (Graph Data Floor)
Plan: —
Status: Discussing Phase 25 (2 of 4 areas decided)
Last activity: 2026-10-11 — handoff written; phase 25 discuss checkpoint saved

## Progress

**Phases Complete:** 0/7
**Current Plan:** N/A

## Accumulated Context

### Decisions

- D1–D7 recorded in `REQUIREMENTS.md` (workstream `code-atlas`; milestone v1.4; phases 25+ after docs-freshness 24; codebase graph not Agent Flow; all seven capabilities in-milestone; pulses and vs-`main` later in this milestone, never faked; xyflow 12 + dockview 6.6.1; no new crate / no GUI→reader edge / no reader→compiler edge; same-webview float only).
- Roadmap: data floor before canvas; canvas before dock; `code_touch` before pulses; vs-`main` as Phase 27 (later, still this milestone). VOX-01 extractor in 25; VOX-02 chrome in 30.
- Phase 25 (discussion, partial): existing edges → `calls`; ship `crate_dep` + query-time `contains`; dangling kept as `mentions`+`dangling`; empty query = crate level; explicit `level` param; focused crate = crate + 1-hop `crate_dep`; default corpus `repo-code-graph` + its identity. See `phases/25-graph-data-floor/25-DISCUSS-CHECKPOINT.json`.

### Blockers / Gates

- Phase 27: needs `/gsd-plan-phase --research-phase` (worktree identity vs commit-keyed corpus; removed-node coloring).
- Phase 29: needs `/gsd-plan-phase --research-phase` (`code_touch` producer list; fail closed).
- elkjs@0.12.0 only if crate-level visual QA after Phase 26 fails — not a scheduled add.
- Runs in parallel with `research-trial-flywheel` (7–11), `autonomy-ux` (12–19), and `docs-freshness` (20–24); do not touch their files.

### Todos

- Finish `/gsd-discuss-phase 25 --ws code-atlas` (stale-cache policy, Vox kind set), then `/gsd-plan-phase 25 --ws code-atlas`.

## Session Continuity

**Stopped At:** Phase 25 discussion — edge-kind honesty and empty-query LOD decided; stale-cache policy asked but unanswered; Vox kind set not started
**Resume File:** `.planning/workstreams/code-atlas/HANDOFF.md` (checkpoint: `phases/25-graph-data-floor/25-DISCUSS-CHECKPOINT.json`)
