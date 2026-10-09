---
gsd_state_version: "1.0"
milestone: v1.3
milestone_name: Self-Maintaining Public Docs
current_phase: "20.1"
current_phase_name: Collapsible Reader Layout
current_plan: Not started
status: planning
stopped_at: Phase 20 complete, ready to plan Phase 20.1
last_updated: "2026-10-09T20:05:55.542Z"
last_activity: 2026-10-09
last_activity_desc: Phase 20 complete, transitioned to Phase 20.1
state_head: c3dda8dc6fb14f4539aefc29a31ea51eb446166f
progress:
  total_phases: 6
  completed_phases: 1
  total_plans: 12
  completed_plans: 12
  percent: 17
---

# Project State

## Project Reference

See: `.planning/PROJECT.md`; workstream brief `CONTEXT.md`; roadmap `ROADMAP.md`; requirements `REQUIREMENTS.md` (29 v1.3 requirements, all mapped); research `research/SUMMARY.md`.

**Core value:** voxlang.org stays accurate to the code without constant manual upkeep — derivable reference is generated, everything else is anchored to code and checked deterministically in CI, and readers and agents can see what is verified.

## Current Position

Phase: 20.1 of 20–24 (Collapsible Reader Layout)
Plan: 12 of 12 (planned, 7 waves; checker passed after 2 revisions)
Status: Ready to plan
Last activity: 2026-10-09 — Phase 20 complete, transitioned to Phase 20.1

## Progress

**Phases Complete:** 0/6
**Current Plan:** Not started

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
- [Phase 20]: 20-06: page-status.mjs is the single status rule; sidebar links use Starlight route ids via docSlug
- [Phase 20]: 20-12: docs content dir is a marker-guarded mirror of per-entry symlinks; the archive is excluded by not mirroring it (docsLoader has no exclude option)
- [Phase 20]: 20-07: starlight-llms-txt 0.10.0 patched via pnpm patch so llms-full and custom sets honour exclude; Internals ids come from frontmatter
- [Phase 20]: 20-08: Astro's content loader only logs remark errors, so dead links fail the build via docLinksGate() at astro:build:done
- [Phase 20]: 20-09: repo Markdown mounted at /repo/<route>/ via the content mirror; routeData sets every page's edit URL and date from its true repo file
- [Phase 20]: 20-11: DEPLOY-02 left Pending — user skipped the live two-forced-failure drill; only the liveness half (#636 open/close) is verified live
- [Phase 20]: 20-11: DEPLOY-01/03 complete on live evidence — push run 37960645178 green, smoke 31 passed incl. archive redirect, #462 closed with root-cause comment

### Blockers / Gates

- DEPLOY-01: Cloudflare token rotated 2026-10-08; manual run 37869058499 green. Still needs a green push-triggered run before closing #462 (plan 20-11).
- 20-01 checkpoint: user approves the `wrangler` pin version (recommended 4.146.0).
- P20-D8 (archive unpublish → `/retired/`) and P20-D9 (`/repo/` prefix) are defaults; override before 20-12 / 20-09 run.
- MEASURE-06 (Phase 21): user approves page dispositions case by case and the research/roadmap sidebar policy.
- Phase 22: needs phase research (symbol-span hashing, attestation, FP measurement); user authorizes switching the gate to blocking after baselining.
- v1.4 prerequisites (user-only): `vox-doc-verify` crate edges, bot PAT secret, `DocClaimJudge` routing category + budget.
- Runs in parallel with `research-trial-flywheel` (7–11) and `autonomy-ux` (12–19); do not touch their files.

## Session Continuity

**Last session:** 2026-10-09T17:28:55.832Z

**Stopped At:** Phase 20 complete, ready to plan Phase 20.1
**Resume File:** None

## Performance Metrics

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 20 P02 | 12min | 3 tasks | 10 files |
| Phase 20 P03 | 20min | 2 tasks | 5 files |
| Phase 20 P04 | 3h | 3 tasks | 9 files |
| Phase 20 P05 | 25min | 3 tasks | 13 files |
| Phase 20 P01 | 57min | 3 tasks | 5 files |
| Phase 20 P06 | 9 min | 3 tasks | 12 files |
| Phase 20 P12 | 10min | 2 tasks | 6 files |
| Phase 20 P07 | 10 min | 2 tasks | 7 files |
| Phase 20 P08 | 25min | 3 tasks | 7 files |
| Phase 20 P09 | 25min | 3 tasks | 20 files |
| Phase 20 P10 | ~90min | 3 tasks | 14 files |
| Phase 20 P11 | multi-session (live deploy + drills) | 3 tasks | 3 files |
