---
gsd_state_version: "1.0"
milestone: v1.3
milestone_name: Self-Maintaining Public Docs
current_phase: 20
current_phase_name: Deploy Unblock & Public-Surface Honesty
current_plan: "20-01 (wave 1: 20-01..20-05 parallel)"
status: planning
stopped_at: Completed 20-01-PLAN.md
last_updated: "2026-10-09T09:18:54.985Z"
last_activity: 2026-10-08
last_activity_desc: Phase 20 planned (12 plans); Phase 20.1 (collapsible reader layout) inserted
state_head: e5fdf388cf33e98a1fbb0bdcb70c9a9fb26313b5
progress:
  total_phases: 6
  completed_phases: 0
  total_plans: 12
  completed_plans: 5
  percent: 0
---

# Project State

## Project Reference

See: `.planning/PROJECT.md`; workstream brief `CONTEXT.md`; roadmap `ROADMAP.md`; requirements `REQUIREMENTS.md` (29 v1.3 requirements, all mapped); research `research/SUMMARY.md`.

**Core value:** voxlang.org stays accurate to the code without constant manual upkeep — derivable reference is generated, everything else is anchored to code and checked deterministically in CI, and readers and agents can see what is verified.

## Current Position

Phase: 20 of 20–24 (Deploy Unblock & Public-Surface Honesty)
Plan: 5 of 12 (planned, 7 waves; checker passed after 2 revisions)
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
- [Phase 20]: 20-03: docs-freshness ROADMAP is the single authoritative docs design; overlapping specs deprecated with superseded notices
- [Phase 20]: 20-04: Tutorials are verified by scripts/docs/tutorial-verify.vox into a generated record (contracts/documentation/tutorial-verification.v1.json) keyed by git blob SHA; docs-astro/tests/unit/tutorial-record.test.mjs fails when a tutorial changes without regeneration (P20-D10, no verified_against frontmatter).
- [Phase 20]: 20-04: User-facing minimums are Node.js >= 22.13 and pnpm >= 11 (generated pnpm-workspace.yaml uses pnpm 11 allowBuilds; pnpm 11 requires Node 22.13). CI's Node 24 / pnpm 11 pin is cited, not used as the minimum.
- [Phase 20]: 20-04: tut-actor-basics documents shipped actor behavior only (on handlers, spawn_process mailbox, empty dispatch in vox build, no spawn/send/state/persistence) instead of the aspirational model.
- [Phase 20]: 20-05: vox-ssg relinked to crates/vox-cli/src/utils/ssg/mod.rs; retired-crate links unlinked as inline code, never redirected
- [Phase 20]: 20-01: wrangler pinned at user-approved 4.146.0 (exact, pnpm 11 lockfile); allowBuilds workerd: false (postinstall unneeded for pages deploy)
- [Phase 20]: 20-01: docs-deploy failures edit one docs-deploy-broken issue in place (no comments); only a green push run on main closes it

### Blockers / Gates

- DEPLOY-01: Cloudflare token rotated 2026-10-08; manual run 37869058499 green. Still needs a green push-triggered run before closing #462 (plan 20-11).
- 20-01 checkpoint: user approves the `wrangler` pin version (recommended 4.146.0).
- P20-D8 (archive unpublish → `/retired/`) and P20-D9 (`/repo/` prefix) are defaults; override before 20-12 / 20-09 run.
- MEASURE-06 (Phase 21): user approves page dispositions case by case and the research/roadmap sidebar policy.
- Phase 22: needs phase research (symbol-span hashing, attestation, FP measurement); user authorizes switching the gate to blocking after baselining.
- v1.4 prerequisites (user-only): `vox-doc-verify` crate edges, bot PAT secret, `DocClaimJudge` routing category + budget.
- Runs in parallel with `research-trial-flywheel` (7–11) and `autonomy-ux` (12–19); do not touch their files.

## Session Continuity

**Last session:** 2026-10-09T09:18:54.966Z

**Stopped At:** Completed 20-01-PLAN.md
**Resume File:** None

## Performance Metrics

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 20 P02 | 12min | 3 tasks | 10 files |
| Phase 20 P03 | 20min | 2 tasks | 5 files |
| Phase 20 P04 | 3h | 3 tasks | 9 files |
| Phase 20 P05 | 25min | 3 tasks | 13 files |
| Phase 20 P01 | 57min | 3 tasks | 5 files |
