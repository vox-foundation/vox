---
gsd_state_version: "1.0"
milestone: v1.3
milestone_name: Self-Maintaining Public Docs
status: planning
last_updated: "2026-10-09T00:15:00.000Z"
last_activity: 2026-10-08
progress:
  total_phases: 5
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
  percent: 0
---

# Project State

## Project Reference

See: `.planning/PROJECT.md`; workstream brief `CONTEXT.md`; roadmap `ROADMAP.md`; requirements `REQUIREMENTS.md` (29 v1.3 requirements, all mapped); research `research/SUMMARY.md`.

**Core value:** voxlang.org stays accurate to the code without constant manual upkeep — derivable reference is generated, everything else is anchored to code and checked deterministically in CI, and readers and agents can see what is verified.

## Current Position

Phase: 20 of 20–24 (Deploy Unblock & Public-Surface Honesty)
Plan: —
Status: Awaiting roadmap approval
Last activity: 2026-10-08 — Roadmap created (5 phases, 20–24)

## Progress

**Phases Complete:** 0/5
**Current Plan:** N/A

## Accumulated Context

### Decisions

- D1–D10 recorded in `REQUIREMENTS.md` (separate workstream; all five outcomes across v1.3–v1.4; draft PRs only, never auto-merge; hybrid build, keep Starlight; case-by-case pruning; v1.3 = phases 20–24 with no new crate edges; Internals section; drift gate blocks new drift only; no hosted-bot trial, pivot rule instead; no LLM on PRs).
- Roadmap: requirement→phase mapping from `REQUIREMENTS.md` kept unchanged. Phase 23 runs in parallel with 22 after 21; Phase 24 depends on 22's ledger.

### Blockers / Gates

- **DEPLOY-01 (Phase 20): user must rotate the Cloudflare Pages API token** (`Pages:Edit` on `vox-docs`). Blocks every reader-visible outcome; other Phase 20 work can proceed meanwhile.
- MEASURE-06 (Phase 21): user approves page dispositions case by case and the research/roadmap sidebar policy.
- Phase 22: needs phase research (symbol-span hashing, attestation, FP measurement); user authorizes switching the gate to blocking after baselining.
- v1.4 prerequisites (user-only): `vox-doc-verify` crate edges, bot PAT secret, `DocClaimJudge` routing category + budget.
- Runs in parallel with `research-trial-flywheel` (7–11) and `autonomy-ux` (12–19); do not touch their files.

## Session Continuity

**Stopped At:** Roadmap drafted, awaiting approval
**Resume File:** None
