---
gsd_state_version: "1.0"
milestone: v1.7
milestone_name: VoxDiagram
current_phase: 83
current_phase_name: Core Crate - Parser, IR & Diagnostics
current_plan: 7
status: executing
stopped_at: Completed 83-06-PLAN.md
last_updated: "2026-10-16T20:06:19.891Z"
last_activity: 2026-10-16
last_activity_desc: Phase 83 execution started
state_head: 57fa90407eabc0c6e75b497a1b98f9b353c4d268
progress:
  total_phases: 9
  completed_phases: 0
  total_plans: 7
  completed_plans: 6
---

# Project State: VoxDiagram (`voxdiagram`)

## Project Reference

See: `.planning/PROJECT.md` (shared, owned by the `unified` workstream, not edited here) and `.planning/workstreams/voxdiagram/PROJECT-MILESTONE.md`.

**Core value:** For this milestone, agents and humans author diagrams that cannot render-crash, and every live docs diagram renders deterministically.
**Current focus:** Phase 83 — Core Crate - Parser, IR & Diagnostics

## Current Position

Phase: 83 (Core Crate - Parser, IR & Diagnostics) — EXECUTING
Plan: 7 of 7
Status: Ready to execute
Last activity: 2026-10-16 — Phase 83 execution started

Progress: [░░░░░░░░░░] 0% (0/9 phases)

## Progress

**Phases Complete:** 0
**Current Plan:** 7

## Key Decisions (milestone-level)

- **Numbering:** Phases 83-91; phases 77-82 belong to the parallel v1.6 milestone in workstream `unified`.
- **Crate edges (pre-authorized by the user 2026-10-16, chat):** exactly `vox-doc-pipeline -> vox-diagram` (lands in Phase 87) and `vox-orchestrator-mcp -> vox-diagram` (lands in Phase 88). The CLI uses a `vox-doc-pipeline` re-export. Any other edge needs fresh approval; never regenerate a baseline to admit an edge.
- **No standalone authorization phase:** font policy folded into Phase 85, Mermaid CDN loader coordination into Phase 89, `profile-parity` into Phase 88, non-inferiority rule into Phase 91.
- **Corpus baseline:** 41 mermaid fences / 26 files under `docs/src` (excluding `archive/`): 37 graph, 2 sequence, 1 state, 1 gantt. Gantt stays Mermaid on the allowlist.
- **Parallelism:** after Phase 83, Phases 84, 85, 87 can run in parallel; 88 and 89 in parallel; 91 alongside 89 and 90.

## Accumulated Context

### Pending Todos

None yet.

### Blockers/Concerns

- An uncommitted jsdelivr `mermaid@11` loader exists in another session's checkout (`docs-astro/src/components/PageSidebar.astro`); coordinate in Phase 89, do not assume removal.
- `dagre` 0.1.1 is single-maintainer with no commits since May 2026; keep the one-file adapter and `dugong` fallback (Phase 85 go/no-go).
- Phase 88 touches the operations catalog and generated artifacts that the `unified` milestone also edits; regenerate through `ssot-autoregen`.
- Any allowlist entry beyond the Gantt (Phase 90) requires user sign-off.

## Session Continuity

**Last session:** 2026-10-16T20:06:19.877Z

**Stopped At:** Completed 83-06-PLAN.md
**Resume File:** None

## Performance Metrics

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 83 P01 | 70min | 2 tasks | 21 files |
| Phase 83 P02 | 60min | 3 tasks | 14 files |
| Phase 83 P03 | 20min | 2 tasks | 5 files |
| Phase 83 P04 | 75min | 3 tasks | 30 files |
| Phase 83 P05 | 45m | 3 tasks | 9 files |
| Phase 83 P06 | 80m | 3 tasks | 41 files |

## Decisions

- [Phase 83]: 83-01: all phase deps declared in the tracer plan so Cargo.toml/Cargo.lock have one owner; later plans must not touch them
- [Phase 83]: 83-01: resolver keeps first declaration on duplicate ids with no diagnostic until plan 03
- [Phase 83]: 83-02: lexer emits its own diagnostics and best-effort tokens (Str terminated:false, BadIdent) that the parser accepts, so one mistake yields one diagnostic
- [Phase 83]: 83-02: direction: is a statement only directly inside graph { }; in a container it is an unexpected-token
- [Phase 83]: 83-04: Collector tracks has_error apart from stored diagnostics so dropped Errors still invalidate the report
- [Phase 83]: 83-04: items past the node/edge cap stay in the AST (IR withheld by the Error) so edges to them do not cascade into undeclared-node
- [Phase 83]: 83-04: skip_to_stmt_end is brace-aware and stops at a line-leading node/container; unquoted labels are recovered as the label
- [Phase 83]: 83-05: generator ids unique by DFS counter suffix; edges are indices into declared nodes; noise is a shrinkable die stream
- [Phase 83]: 83-06: apply_fixes applies each diagnostic's fixes as one atomic group, rejecting out-of-range, off-boundary or overlapping groups; equal-offset inserts keep diagnostic order and are never deduplicated
- [Phase 83]: 83-06: undeclared-node declare-fix only for a single-segment ref or the last segment of a dotted ref; scopes without both braces get no fix until closed (repair converges in a second round)
