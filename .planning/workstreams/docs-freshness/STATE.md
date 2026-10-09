---
gsd_state_version: "1.0"
milestone: v1.3
milestone_name: Self-Maintaining Public Docs
current_phase: 20
current_phase_name: Deploy Unblock & Public-Surface Honesty
current_plan: "20-01 (wave 1: 20-01..20-05 parallel)"
status: planning
stopped_at: Completed 20-02-PLAN.md
last_updated: "2026-10-09T07:42:46.291Z"
last_activity: 2026-10-08
last_activity_desc: Phase 20 planned (12 plans); Phase 20.1 (collapsible reader layout) inserted
state_head: 66c038f62d046e29d04467d952ec269190b4a945
progress:
  total_phases: 6
  completed_phases: 0
  total_plans: 12
  completed_plans: 1
  percent: 0
---

# Project State

## Project Reference

See: `.planning/PROJECT.md`; workstream brief `CONTEXT.md`; roadmap `ROADMAP.md`; requirements `REQUIREMENTS.md` (29 v1.3 requirements, all mapped); research `research/SUMMARY.md`.

**Core value:** voxlang.org stays accurate to the code without constant manual upkeep — derivable reference is generated, everything else is anchored to code and checked deterministically in CI, and readers and agents can see what is verified.

## Current Position

Phase: 20 of 20–24 (Deploy Unblock & Public-Surface Honesty)
Plan: 1 of 12 (planned, 7 waves; checker passed after 2 revisions)
Status: Ready to execute — `/gsd-execute-phase 20`
Last activity: 2026-10-08 — Phase 20 planned (12 plans); Phase 20.1 (collapsible reader layout) inserted

## Progress

**Phases Complete:** 0/6
**Current Plan:** 20-01 (wave 1: 20-01..20-05 parallel)

## Accumulated Context

### Decisions

- D1–D10 recorded in `REQUIREMENTS.md` (separate workstream; all five outcomes across v1.3–v1.4; draft PRs only, never auto-merge; hybrid build, keep Starlight; case-by-case pruning; v1.3 = phases 20–24 with no new crate edges; Internals section; drift gate blocks new drift only; no hosted-bot trial, pivot rule instead; no LLM on PRs).
- Roadmap: requirement→phase mapping from `REQUIREMENTS.md` kept unchanged. Phase 23 runs in parallel with 22 after 21; Phase 24 depends on 22's ledger.
- [Phase 20]: 20-02: docs page dates come from git-dates.mjs via routeData middleware; Starlight lastUpdated off (content dir is a gitignored symlink)
- [Phase 20]: 20-02: date-map key is docs/src/ + entry.filePath minus src/content/docs/ (shared by routeData and feed.xml)
- [Phase 20]: 20-02: build-output specs are pure-fs Playwright tests over dist/ using tests/lib/dist.ts

### Blockers / Gates

- DEPLOY-01: Cloudflare token rotated 2026-10-08; manual run 37869058499 green. Still needs a green push-triggered run before closing #462 (plan 20-11).
- 20-01 checkpoint: user approves the `wrangler` pin version (recommended 4.146.0).
- P20-D8 (archive unpublish → `/retired/`) and P20-D9 (`/repo/` prefix) are defaults; override before 20-12 / 20-09 run.
- MEASURE-06 (Phase 21): user approves page dispositions case by case and the research/roadmap sidebar policy.
- Phase 22: needs phase research (symbol-span hashing, attestation, FP measurement); user authorizes switching the gate to blocking after baselining.
- v1.4 prerequisites (user-only): `vox-doc-verify` crate edges, bot PAT secret, `DocClaimJudge` routing category + budget.
- Runs in parallel with `research-trial-flywheel` (7–11) and `autonomy-ux` (12–19); do not touch their files.

## Session Continuity

**Last session:** 2026-10-09T07:42:46.279Z

**Stopped At:** Completed 20-02-PLAN.md
**Resume File:** None

## Performance Metrics

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 20 P02 | 12min | 3 tasks | 10 files |
