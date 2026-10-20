# Phase 83: Knowledge Base, Deep Research, Scientia, Code Atlas & Mesh Live Surface Hardening - Context

**Gathered:** 2026-10-16
**Status:** Ready for execution & verification

<domain>
## Phase Boundary

Deliver Milestone v1.6 Phase 83 (`DATA-01..05`):
- **DATA-01**: `MemoryView.tsx` (`Library` search, facet filters, `@kb` Context Inspector drawer, `kb_findings` Inbox review, and KB Health tab) executes live queries, ingestion, and finding promotion/dismissal against `VoxDb` baseline 98.
- **DATA-02**: `ResearchView.tsx` and `ScientiaSurface.tsx` run live multi-wave sessions, render epistemic DAGs, promote finding candidates, and preview IMRaD Markdown / LaTeX / Replay Receipts from `VoxDb`.
- **DATA-03**: `CodeAtlasCanvas.tsx` and `VoxGraphStatusPanel.tsx` render the live workspace `vox_search_atlas` graph (`400` nodes / `800` edges LOD), diff overlay, and click-to-`@` insertion into `Loquela.tsx`.
- **DATA-04**: `MeshView.tsx` reports live `Populi` iroh QUIC node status, A2A mailbox counts, and queue telemetry with clear single-node/offline guidance.
- **DATA-05**: `Dashboard.tsx`, `RunsView.tsx`, `TasksView.tsx`, `ActivitySurface.tsx`, and `GamifyView.tsx` stream live daemon events with bounded ring-buffer memory.
</domain>

<decisions>
## Implementation Decisions

### Knowledge, Research & Scientia Surfaces (`DATA-01..02`)
- `MemoryView.tsx` routes search, facet filtering, `@kb` inspection, finding inbox review (`promote`/`dismiss`), and KB health snapshots through `vox_memory_*` and `vox_kb_*` backend commands.
- `ResearchView.tsx` and `ScientiaSurface.tsx` invoke `vox_research_*` and `vox_scientia_*` commands to manage multi-wave deep research sessions, epistemic claims/findings, and IMRaD / LaTeX / Replay Receipt previews.

### Code Atlas, Mesh & Bounded-Memory Event Streaming (`DATA-03..05`)
- `CodeAtlasCanvas.tsx` and `VoxGraphStatusPanel.tsx` enforce LOD caps (`400` nodes / `800` edges), diff overlays, and `@` symbol/file mention insertion into `Loquela.tsx`.
- `MeshView.tsx` queries `vox_mesh_*` / `Populi` node health, A2A mailbox metrics, and single-node/offline guidance.
- `Dashboard.tsx`, `RunsView.tsx`, `TasksView.tsx`, `ActivitySurface.tsx`, and `GamifyView.tsx` consume daemon event streams using bounded ring buffers to prevent unbounded DOM/heap growth during long-running sessions.
</decisions>
