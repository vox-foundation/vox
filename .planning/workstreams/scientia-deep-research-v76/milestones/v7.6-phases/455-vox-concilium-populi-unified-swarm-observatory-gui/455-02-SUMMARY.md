---
phase: 455-vox-concilium-populi-unified-swarm-observatory-gui
plan: 02
subsystem: axis-gui-populi
tags: [react, vox-populi, file-affinity, compute-fabric, vitest]
requires:
  - phase: 454-causal-swarm-event-stamping-persistent-sequence-board
    provides: "`/api/v2/concilium/placement` and `concilium_placement` Tauri handler"
  - phase: 455-vox-concilium-populi-unified-swarm-observatory-gui
    provides: Shared TypeScript Concilium/Populi DTOs in `types/concilium.ts`
provides:
  - "`<PopuliTopologyView />` (`153` LoC) Mode 2 Hybrid Compute Fabric combining `HybridComputeNodeCard`, `FileAffinityLockMatrix`, `PopuliDispatchWorkbench`, `PeerAttestationInspector`, `PopuliPeerMasterPanel`, `PopuliQuicTopologyRing`, `populiHeaderGauges`, and `usePopuliTopologyData`"
  - "Unified delegation from `PopuliMeshBentoView` (`18` LoC) and `MeshView` (`30` LoC) while preserving `export interface MeshNode`"
  - "Event-driven `listenAgentEvents` + `listenOrchStatus` coalesced placement updates with zero `5000ms` polling loops"
affects: [vox-gui-ui]
tech-stack:
  added: []
  patterns:
    - "Single `<PopuliTopologyView />` shared across `PopuliMeshBentoView` and `MeshView` with docked worker pills, VRAM telemetry, and `FileAffinityMap` lock shields"
    - "Coalesced event-driven `listenAgentEvents` + `listenOrchStatus` refresh replacing blind 5s `setInterval` polling loops"
key-files:
  created:
    - crates/vox-gui/ui/src/components/surfaces/Populi/FileAffinityLockMatrix.tsx
    - crates/vox-gui/ui/src/components/surfaces/Populi/HybridComputeNodeCard.tsx
    - crates/vox-gui/ui/src/components/surfaces/Populi/PeerAttestationInspector.tsx
    - crates/vox-gui/ui/src/components/surfaces/Populi/PopuliDispatchWorkbench.tsx
    - crates/vox-gui/ui/src/components/surfaces/Populi/PopuliPeerMasterPanel.tsx
    - crates/vox-gui/ui/src/components/surfaces/Populi/PopuliQuicTopologyRing.tsx
    - crates/vox-gui/ui/src/components/surfaces/Populi/populiHeaderGauges.ts
    - crates/vox-gui/ui/src/components/surfaces/Populi/usePopuliTopologyData.ts
    - crates/vox-gui/ui/src/components/surfaces/Populi/PopuliTopologyView.tsx
    - crates/vox-gui/ui/src/components/surfaces/Populi/PopuliTopologyView.test.tsx
  modified:
    - crates/vox-gui/ui/src/components/surfaces/Populi/PopuliMeshBentoView.tsx
    - crates/vox-gui/ui/src/components/surfaces/Populi/PopuliMeshBentoView.test.tsx
    - crates/vox-gui/ui/src/components/surfaces/Mesh/MeshView.tsx
    - crates/vox-gui/ui/src/components/surfaces/Mesh/MeshView.test.tsx
key-decisions:
  - "Split `PopuliTopologyView` into `<400`-LoC focused modules (`HybridComputeNodeCard.tsx`: `347`, `usePopuliTopologyData.ts`: `334`, `PopuliDispatchWorkbench.tsx`: `293`, `PopuliQuicTopologyRing.tsx`: `182`, `FileAffinityLockMatrix.tsx`: `164`, `PopuliPeerMasterPanel.tsx`: `161`, `PopuliTopologyView.tsx`: `153`, `PeerAttestationInspector.tsx`: `143`, `populiHeaderGauges.ts`: `98`)."
  - "Preserve `export interface MeshNode` on `MeshView.tsx` and `surfaceDecorators.populi` on `PopuliMeshBentoView` so downstream Mesh callers and e2e contracts stay intact."
patterns-established:
  - "Compute placement refreshes reactively via coalesced Tauri event subscriptions rather than timer polling."
requirements-completed: [CONC-07, CONC-08]
duration: 40min
completed: 2026-11-16
---

# Phase 455 Plan 02: Vox Populi Hybrid Compute Fabric & Mesh Consolidation Summary

**Phase 455 Plan 02 delivered `<PopuliTopologyView />` with docked swarm workers, `FileAffinityMap` lock matrices, compute hot-swap dispatch controls (`CONC-07`), and consolidated `PopuliMeshBentoView` + `MeshView` with event-driven placement updates and zero `5000ms` polling (`CONC-08`).**

## Accomplishments

### `CONC-07`: Hybrid Compute Fabric & `FileAffinityMap` Matrix
- Implemented [`HybridComputeNodeCard.tsx`](crates/vox-gui/ui/src/components/surfaces/Populi/HybridComputeNodeCard.tsx) (`347` LoC), [`FileAffinityLockMatrix.tsx`](crates/vox-gui/ui/src/components/surfaces/Populi/FileAffinityLockMatrix.tsx) (`164` LoC), [`PopuliDispatchWorkbench.tsx`](crates/vox-gui/ui/src/components/surfaces/Populi/PopuliDispatchWorkbench.tsx) (`293` LoC), [`PeerAttestationInspector.tsx`](crates/vox-gui/ui/src/components/surfaces/Populi/PeerAttestationInspector.tsx) (`143` LoC), [`PopuliQuicTopologyRing.tsx`](crates/vox-gui/ui/src/components/surfaces/Populi/PopuliQuicTopologyRing.tsx) (`182` LoC), [`PopuliPeerMasterPanel.tsx`](crates/vox-gui/ui/src/components/surfaces/Populi/PopuliPeerMasterPanel.tsx) (`161` LoC), [`usePopuliTopologyData.ts`](crates/vox-gui/ui/src/components/surfaces/Populi/usePopuliTopologyData.ts) (`334` LoC), and [`PopuliTopologyView.tsx`](crates/vox-gui/ui/src/components/surfaces/Populi/PopuliTopologyView.tsx) (`153` LoC).

### `CONC-08`: Populi/Mesh Consolidation & Zero Polling
- Routed [`PopuliMeshBentoView.tsx`](crates/vox-gui/ui/src/components/surfaces/Populi/PopuliMeshBentoView.tsx) (`18` LoC) and [`MeshView.tsx`](crates/vox-gui/ui/src/components/surfaces/Mesh/MeshView.tsx) (`30` LoC) directly into `<PopuliTopologyView />` while keeping `export interface MeshNode` intact.
- Verified reactive `listenAgentEvents` placement updates with zero `5000ms` `setInterval` registrations in [`PopuliTopologyView.test.tsx`](crates/vox-gui/ui/src/components/surfaces/Populi/PopuliTopologyView.test.tsx) and full 16-file (`61/61` tests) Vitest guard coverage.

---
*Phase: 455-vox-concilium-populi-unified-swarm-observatory-gui*
*Completed: 2026-11-16*
