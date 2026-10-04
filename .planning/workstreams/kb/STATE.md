---
gsd_state_version: "1.0"
milestone: v1.1
milestone_name: Vox Knowledge Base — corpus to context
current_phase: 1
current_phase_name: Retrieval Eval Baseline
status: executing
stopped_at: Completed 01-01-PLAN.md
last_updated: "2026-10-04T08:12:23.011Z"
last_activity: 2026-10-04
last_activity_desc: Phase 1 execution started
state_head: 400b138336bb3f952de6a9e78ad203563aaea675
progress:
  total_phases: 12
  completed_phases: 0
  total_plans: 34
  completed_plans: 1
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (shared project context) and .planning/workstreams/kb/REQUIREMENTS.md (this workstream's scope)

**Core value:** Agents using any model get the right curated knowledge at the right time — measurably, visibly, and without letting unreviewed or model-generated text masquerade as trusted fact.
**Current focus:** Phase 1 — Retrieval Eval Baseline

## Current Position

Phase: 1 (Retrieval Eval Baseline) — EXECUTING
Plan: 2 of 12
Status: Ready to execute
Last activity: 2026-10-04 — Phase 1 execution started

Progress: [░░░░░░░░░░] 0%

## Performance Metrics

**Velocity:**

- Total plans completed: 0
- Average duration: - min
- Total execution time: 0 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| - | - | - | - |

*Updated after each plan completion*
**Per-Plan Metrics:**

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 1 P01 | 60min | 3 tasks | 15 files |

## Accumulated Context

### Decisions

- [Roadmap]: 12 phases; GUI split into Library (6), Inspector/composer (8), Inbox/Settings/Health (12), each after its backend contract.
- [Roadmap]: GROW-06 (side doors → inbox, MENS `KbSignals` → `kb_findings`) lands in Phase 3 with leak closure, not Phase 9.
- [Roadmap]: RET-04 (tier/staleness/scope/status on results, down-weighting) lands in Phase 5 with ranking; EVAL-06 (nightly generation evals) in Phase 7 with citations.
- [Roadmap]: Phase 2 re-keys chunk-level qrels once when the heading chunker replaces line chunks; doc-level metrics must not regress.
- [Research]: No new crate edges (`vox-search → vox-crypto`, `vox-cli → vox-forge` are user-authorization proposals only).
- [Phase 1]: 01-01: heading_path = title > H2 > H3 joined by ' › '; H1 never a segment; defined once in vox_search::kb_ids for Phase 2 reuse
- [Phase 1]: 01-01: run_eval takes the retriever's doc->chunk-id map explicitly; chunk qrels resolve to deepest ancestor-or-equal chunk

### Pending Todos

- Phase 1 spikes: VoxDb/Turso FTS5 runtime probe (feeds IDX-07); candle `modernbert` + tokenizers 0.21 load check (feeds Phase 5).
- Count legacy `kb_entries`/`knowledge_bases` rows on real user DBs before Phase 3 quarantine.

### Blockers/Concerns

- Copyright posture for promoted web knowledge needs a human/legal decision before Phase 9 ships promotion (interim: excerpts + URL + hash).
- `vox-cli-review` lifecycle (CodeRabbit lineage): if retired, Phase 9 needs the `vox-cli → vox-forge` edge proposal decided.
- Existing direct `blake3` use in `vox-search` evades the `crypto_ban` detector — do not copy the pattern; report the detector gap.

## Session Continuity

**Last session:** 2026-10-04T08:12:22.995Z

**Stopped At:** Completed 01-01-PLAN.md
**Resume File:** None
**Next:** `/gsd-plan-phase 1` (no research-phase needed)
