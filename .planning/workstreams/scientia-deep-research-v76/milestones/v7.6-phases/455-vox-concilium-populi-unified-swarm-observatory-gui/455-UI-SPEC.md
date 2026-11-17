# Phase 455 UI Design Contract (`455-UI-SPEC.md`) — Vox Concilium & Populi Unified Swarm Observatory

**Status:** Operator-Approved (`"1 approved 2 add viagsd-phase"`, `2026-11-15`)
**Concept References:**
- `vox_concilium_concept_art_proposal.md`
- `vox_concilium_concept_prototype.html`
- `vox_concilium_ui_concept_1794792064030.jpg`
- `vox_populi_mesh_swarm_topology_ui_v2_1794792098519.jpg`

## 1. Visual Hierarchy & Palette
- **Surface Base:** Obsidian (`bg-[#0B0D12]` / `bg-surface-base`) with Roman warm-gold accents (`#F3E5AB` / `#D4AF37` borders `border-[#D4AF37]/25`), subtle glass cards (`BentoCard` / `RomanSurfaceHeader`), and crisp monospace telemetry typography.
- **Semantic Span & Node Tones:**
  - `Thinking / Reasoning`: Warm Gold (`#D4AF37`)
  - `Tool Call / Writing`: Emerald Teal (`#10B981`)
  - `Parked · 0 tok/s`: Cool Slate (`#64748B` dashed hibernation bar with `0 tok/s · Asleep on #seq-N` badge)
  - `Lock Contention / Trip`: Coral Rose (`#F43F5E` `[FileLock]` shield)
  - `@mention Doorbell Wakeup`: Golden SVG Bézier arc connecting `#seq` board post to woken agent span.
- **Strict Guards:**
  - `0` raw Unicode emojis or dingbats (`zeroEmojiAndDingbat.test.ts` clean — use `Icon.*` SVG components exclusively).
  - Radix `data-slot` attributes on all interactive primitives (`noRawNativeControlsGuard.test.ts` clean — use `VoxSelect`, custom `<button data-slot="button">`, etc.).
  - Every new `.tsx` / `.ts` file strictly `<400` LoC.
  - `0` `5000ms` blind `setInterval` polling loops in consolidated views (event-driven via `listenAgentEvents` / `listenOrchStatus` + explicit manual refresh button).

## 2. Mode 1: Vox Concilium (`ConciliumView.tsx`) — Consolidating `SubAgents` + `Flow` + `TrajectoryBraid`
1. **Global "Now Playing" Telemetry Strip (`NowPlayingStrip.tsx`):**
   - Live counters for `Active Workers`, `Parked (0 tok/s)`, `Head #seq`, `Active File Locks`, and instant live ticker of current agent actions.
2. **Perfetto-Style W3C `traceparent` Causal Swimlanes (`CausalSwimlanePanel.tsx`):**
   - One horizontal lane per swarm worker (`@coordinator`, `@researcher`, `@coder`, `@verifier`, etc.) with compute-tier badge (`CLOUD`, `MENS GPU`, `QUIC PEER`), millisecond span bars (`Thinking`, `Tool`, `Parked · 0 tok/s`), `[FileLock]` path shields, and SVG golden wakeup arcs for `@mention` doorbells.
3. **Channel-Filtered Shared `#seq` Sequence Board (`SharedSequenceBoard.tsx`):**
   - `#channel` filter pills (`#all`, `#main`, `#parser`, `#verify`, `#scientia`, `#ops`), monotonic `#seq-N` badges, `@mention` chips, diff/test receipt tags, and inline `@steer` quick-post composer.
4. **Live Agent Lens (`LiveAgentLens.tsx`):**
   - Four operator pilot controls for the selected worker: `Park (0 tok/s)`, `Steer @agent`, `Fork Warm Context (jj share)`, and `Swap Compute Tier` (`Cloud Bandit` / `Local MENS GPU` / `Populi QUIC Peer`).
5. **Compatibility Aliases:**
   - `SubAgentsView.tsx` renders `<ConciliumView initialFocus="board" ... />`, `AgentFlow.tsx` renders `<ConciliumView initialFocus="swimlanes" ... />`, and `AgentTrajectoryBraid.tsx` renders compact causal swimlane bars so all `SURFACE_REGISTRY` routes and `HarnessRiverPanel` embeds stay 100% compatible.

## 3. Mode 2: Vox Populi Placement Map (`PopuliTopologyView.tsx`) — Consolidating `MeshView` + `PopuliMeshBentoView`
1. **Hybrid Compute Tier Node Cards (`HybridComputeNodeCard.tsx`):**
   - Three-column placement topology: **Local Host MENS GPU** (VRAM gauge + warm KV-cache prefix indicator), **Populi QUIC Peer Mesh (`iroh`)** (RTT, gossip cadence knob, peer trust tier), and **Cloud Frontier Router** (Thompson bandit arm weights + fallback status), each docking live `@agent` pills running on that node.
2. **Rust-Enforced `FileAffinityMap` Lock Matrix (`FileAffinityLockMatrix.tsx`):**
   - Central directory/prefix lock matrix showing exclusive vs. shared leases, owning `@agent`, compute node, `SwarmWorkspaceMode` (`inherit | share | branch`), and contention alerts.
3. **Placement & Mesh Dispatch Workbench (`PopuliDispatchWorkbench.tsx`):**
   - Clean mesh `.vox` dispatch + worker compute-tier hot-swap & gossip cadence controls, driven by event push (`listenAgentEvents` / `listenOrchStatus`) with zero `5000ms` blind polling loops.
4. **Compatibility Aliases:**
   - Both `PopuliMeshBentoView` (`surfaceDecorators.populi`) and `MeshView` (`case 'mesh'`) render the unified `<PopuliTopologyView />` while preserving `export interface MeshNode` and existing prop signatures.
