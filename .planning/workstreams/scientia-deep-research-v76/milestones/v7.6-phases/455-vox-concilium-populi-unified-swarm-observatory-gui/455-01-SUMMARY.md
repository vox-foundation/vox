---
phase: 455-vox-concilium-populi-unified-swarm-observatory-gui
plan: 01
subsystem: axis-gui-concilium
tags: [react, vox-concilium, w3c-swimlanes, seq-board, pilot-controls, vitest]
requires:
  - phase: 454-causal-swarm-event-stamping-persistent-sequence-board
    provides: "`/api/v2/concilium/*` gateway routes and `mission_control` Tauri commands"
provides:
  - "`<ConciliumView />` Mode 1 Causal Swarm Observatory (`NowPlayingStrip`, `CausalSwimlanePanel`, `SharedSequenceBoard`, `LiveAgentLens`, `conciliumLaneModel`, `conciliumDefaultSnapshot`)"
  - "Shared TypeScript Concilium/Populi DTOs (`types/concilium.ts`) and daemon RPC client (`transport/conciliumRpc.ts`)"
  - "Consolidation of `AgentFlow`, `SubAgentsView`, and `AgentTrajectoryBraid` onto canonical Concilium components"
affects: [vox-gui-ui]
tech-stack:
  added: []
  patterns:
    - "`ConciliumView` focus modes (`swimlanes` | `board`) unify Flow and SubAgents onto a single W3C traceparent swimlane + `#seq` bulletin surface"
    - "All interactive select controls use `<VoxSelect />`; zero raw `<select>`, zero `5000ms` blind polling, zero banned emoji/dingbat characters, `sanitizeErrorForToast` toast-body compliance"
key-files:
  created:
    - crates/vox-gui/ui/src/types/concilium.ts
    - crates/vox-gui/ui/src/transport/conciliumRpc.ts
    - crates/vox-gui/ui/src/components/surfaces/Concilium/conciliumDefaultSnapshot.ts
    - crates/vox-gui/ui/src/components/surfaces/Concilium/conciliumLaneModel.ts
    - crates/vox-gui/ui/src/components/surfaces/Concilium/NowPlayingStrip.tsx
    - crates/vox-gui/ui/src/components/surfaces/Concilium/CausalSwimlanePanel.tsx
    - crates/vox-gui/ui/src/components/surfaces/Concilium/SharedSequenceBoard.tsx
    - crates/vox-gui/ui/src/components/surfaces/Concilium/LiveAgentLens.tsx
    - crates/vox-gui/ui/src/components/surfaces/Concilium/ConciliumView.tsx
    - crates/vox-gui/ui/src/components/surfaces/Concilium/ConciliumView.test.tsx
  modified:
    - crates/vox-gui/ui/src/transport.ts
    - crates/vox-gui/ui/src/components/surfaces/Flow/AgentFlow.tsx
    - crates/vox-gui/ui/src/components/surfaces/Flow/AgentFlow.test.tsx
    - crates/vox-gui/ui/src/components/surfaces/SubAgents/SubAgentsView.tsx
    - crates/vox-gui/ui/src/components/surfaces/SubAgents/SubAgentsView.test.tsx
    - crates/vox-gui/ui/src/components/surfaces/Harness/AgentTrajectoryBraid.tsx
    - crates/vox-gui/ui/src/components/surfaces/Harness/AgentTrajectoryBraid.test.tsx
key-decisions:
  - "Adapt `SubAgentsView` toast callback (`pushToast`) with typed `cause: ToastCause` (`'backend-error' | 'backend-ok'`) to preserve strict TypeScript parity."
  - "Keep every Concilium UI module strictly under `400` LoC (`LiveAgentLens.tsx`: `292`, `CausalSwimlanePanel.tsx`: `289`, `ConciliumView.tsx`: `269`, `conciliumLaneModel.ts`: `255`, `SharedSequenceBoard.tsx`: `220`, `NowPlayingStrip.tsx`: `183`, `conciliumDefaultSnapshot.ts`: `172`)."
patterns-established:
  - "Parked workers surface `0 tok/s · Asleep on #seq-N` badges with deterministic `#channel` steer inputs and `VoxSelect` compute-tier swap controls."
requirements-completed: [CONC-05, CONC-06]
duration: 45min
completed: 2026-11-16
---

# Phase 455 Plan 01: Vox Concilium Causal Swarm Observatory Summary

**Phase 455 Plan 01 shipped `<ConciliumView />` and consolidated `AgentFlow`, `SubAgentsView`, and `AgentTrajectoryBraid` onto unified W3C traceparent swimlanes, `#seq` bulletin routing, and live pilot controls (`CONC-05`, `CONC-06`).**

## Accomplishments

### `CONC-05`: Mode 1 Concilium Observatory (`<ConciliumView />`)
- Added shared frontend DTOs ([`crates/vox-gui/ui/src/types/concilium.ts`](crates/vox-gui/ui/src/types/concilium.ts), `171` LoC) and transport wrappers ([`crates/vox-gui/ui/src/transport/conciliumRpc.ts`](crates/vox-gui/ui/src/transport/conciliumRpc.ts), `44` LoC) re-exported from [`crates/vox-gui/ui/src/transport.ts`](crates/vox-gui/ui/src/transport.ts).
- Implemented `<NowPlayingStrip />` (`183` LoC), `<CausalSwimlanePanel />` (`289` LoC, W3C `traceparent` spans + wakeup arcs + compute-tier badges + `[FileLock]` shields), `<SharedSequenceBoard />` (`220` LoC, `#channel` filters + watchdog/receipt badges + steer bar), and `<LiveAgentLens />` (`292` LoC, `<VoxSelect />` tier swap + pilot controls), backed by pure [`conciliumLaneModel.ts`](crates/vox-gui/ui/src/components/surfaces/Concilium/conciliumLaneModel.ts) (`255` LoC) + [`conciliumDefaultSnapshot.ts`](crates/vox-gui/ui/src/components/surfaces/Concilium/conciliumDefaultSnapshot.ts) (`172` LoC) inside [`ConciliumView.tsx`](crates/vox-gui/ui/src/components/surfaces/Concilium/ConciliumView.tsx) (`269` LoC).

### `CONC-06`: Surface Consolidation (`AgentFlow`, `SubAgentsView`, `AgentTrajectoryBraid`)
- Routed [`AgentFlow.tsx`](crates/vox-gui/ui/src/components/surfaces/Flow/AgentFlow.tsx) (`45` LoC) to `<ConciliumView initialFocus="swimlanes" />` and [`SubAgentsView.tsx`](crates/vox-gui/ui/src/components/surfaces/SubAgents/SubAgentsView.tsx) (`109` LoC) to `<ConciliumView initialFocus="board" />` with typed `ToastCause` adaptation and memoized callbacks.
- Upgraded [`AgentTrajectoryBraid.tsx`](crates/vox-gui/ui/src/components/surfaces/Harness/AgentTrajectoryBraid.tsx) (`183` LoC) to render 8-slice braided trajectories across up to 6 aligned lanes with `0 tok/s` indicators and clamped role lookup.
- Verified with `tsc --noEmit` (`exit 0`) and Vitest (`ConciliumView.test.tsx`, `SubAgentsView.test.tsx`, `AgentFlow.test.tsx`, `AgentTrajectoryBraid.test.tsx`, plus emoji/native-control/IPC/toast guards passing).

---
*Phase: 455-vox-concilium-populi-unified-swarm-observatory-gui*
*Completed: 2026-11-16*
