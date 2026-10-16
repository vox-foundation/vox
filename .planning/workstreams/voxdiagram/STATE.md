---
gsd_state_version: "1.0"
milestone: v1.7
milestone_name: VoxDiagram
current_phase: 83
current_phase_name: Core Crate - Parser, IR & Diagnostics
current_plan: N/A
status: planning
stopped_at: Roadmap created (Phases 83-91); ready to plan Phase 83
last_updated: "2026-10-16T18:53:51.533Z"
last_activity: 2026-10-16
last_activity_desc: Roadmap created for milestone v1.7 (9 phases, 25/25 requirements mapped)
state_head: 112eb173e719b8c2c6210020bba8008a2c64e9ae
progress:
  total_phases: 9
  completed_phases: 0
  total_plans: 7
  completed_plans: 0
---

# Project State: VoxDiagram (`voxdiagram`)

## Project Reference

See: `.planning/PROJECT.md` (shared, owned by the `unified` workstream, not edited here) and `.planning/workstreams/voxdiagram/PROJECT-MILESTONE.md`.

**Core value:** For this milestone, agents and humans author diagrams that cannot render-crash, and every live docs diagram renders deterministically.
**Current focus:** Phase 83 — Core Crate: Parser, IR & Diagnostics.

## Current Position

Phase: 83 (Core Crate - Parser, IR & Diagnostics) — READY TO EXECUTE
Plan: — (not yet planned)
Status: Ready to plan Phase 83
Last activity: 2026-10-16 — Roadmap created for milestone v1.7 (9 phases, 25/25 requirements mapped)

Progress: [░░░░░░░░░░] 0% (0/9 phases)

## Progress

**Phases Complete:** 0
**Current Plan:** N/A

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

**Stopped At:** Roadmap created; awaiting orchestrator commit and user approval.
**Resume File:** `.planning/workstreams/voxdiagram/ROADMAP.md`
