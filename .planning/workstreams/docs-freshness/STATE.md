---
gsd_state_version: "1.0"
milestone: v1.4
milestone_name: OpenWiki-Style Self-Organizing & Self-Healing Public Docs
status: planning
last_updated: "2026-10-14T02:08:51.129Z"
last_activity: 2026-10-13
progress:
  total_phases: 5
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
  percent: 0
---

# Project State

## Project Reference

See: `.planning/PROJECT.md`; workstream brief `CONTEXT.md`; roadmap `ROADMAP.md`; requirements `REQUIREMENTS.md` (19 v1.4 requirements across Phases 32–36, all mapped; 41 v1.3 requirements shipped); research `research/SUMMARY.md` + `docs/src/architecture/docs-bot-evaluation-and-pivot-rule-2026.md`.

**Core value:** `voxlang.org` stays accurate to the code and self-organizes without manual upkeep or SaaS subscriptions — derivable reference, crate API symbols, and subsystem wiki hubs with Mermaid diagrams are generated from the code graph, claims are anchored and verified deterministically + nightly via `vox_actor_runtime::llm`, and readers/agents see calibrated trust signals.

## Current Position

Phase: 32 of 32–36 (Trust Signal Calibration & Ledger Synchronization)
Plan: —
Status: Ready to plan Phase 32
Last activity: 2026-10-13 — Milestone v1.4 roadmap & 19 requirements defined (Phases 32–36)

## Progress

**v1.4 Phases Complete:** 0/5 (Phases 32–36 planned)
**Prior Milestone (v1.3):** 6/6 phases complete (Phases 20, 20.1, 21, 22, 23, 24 shipped 2026-10-13)

## Accumulated Context

### Decisions

- D1–D12 (v1.3) recorded in `REQUIREMENTS.md`.
- D13 (2026-10-13): v1.4 in `docs-freshness` includes all 5 pillars (`SIGNAL`, `WIKI`, `DEBT`, `VERIFY`, `BOT`) to import OpenWiki-style self-organization and continuous self-healing in-house.
- D14 (2026-10-13): Zero new crate edges — implement LLM claim verification inside `vox-doc-pipeline` / `vox-cli` via `vox_actor_runtime::llm`, authorize `TaskCategory::DocClaimJudge` in `model-routing.v1.yaml`, and reuse `SSOT_AUTOREGEN_TOKEN` for bot draft PRs.
- D15 (2026-10-13): Global phase numbering continues at **Phases 32–36** (after `code-atlas` Phases 25–31).

### Blockers / Gates

- Runs in parallel with `research-trial-flywheel` (Phases 7–11), `autonomy-ux` (Phases 12–19), and `code-atlas` (Phases 25–31); do not touch their files.

## Session Continuity

**Completed:** Milestone v1.4 initialized (`REQUIREMENTS.md`, `ROADMAP.md`, `STATE.md`, `MILESTONES.md`)
**Next Step:** `/gsd-discuss-phase 32 --ws docs-freshness` or `/gsd-plan-phase 32 --ws docs-freshness`
