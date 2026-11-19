---
phase: 455
phase_name: "vox-concilium-populi-unified-swarm-observatory-gui"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 4
  lessons: 3
  patterns: 6
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 455 Learnings: vox-concilium-populi-unified-swarm-observatory-gui

## Decisions

### Adapt SubAgentsView toast callback (pushToast) with typed cause
Adapt `SubAgentsView` toast callback (`pushToast`) with typed `cause: ToastCause` (`'backend-error' | 'backend-ok'`) to preserve strict TypeScript parity.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 455-01-SUMMARY.md

---

### Keep every Concilium UI module strictly under 400 LoC (LiveAgentLens.tsx
Keep every Concilium UI module strictly under `400` LoC (`LiveAgentLens.tsx`: `292`, `CausalSwimlanePanel.tsx`: `289`, `ConciliumView.tsx`: `269`, `conciliumLaneModel.ts`: `255`, `SharedSequenceBoard.tsx`: `220`, `NowPlayingStrip.tsx`: `183`, `conciliumDefaultSnapshot.ts`: `172`).

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 455-01-SUMMARY.md

---

### Split PopuliTopologyView into <400-LoC focused modules (HybridComputeNod...
Split `PopuliTopologyView` into `<400`-LoC focused modules (`HybridComputeNodeCard.tsx`: `347`, `usePopuliTopologyData.ts`: `334`, `PopuliDispatchWorkbench.tsx`: `293`, `PopuliQuicTopologyRing.tsx`: `182`, `FileAffinityLockMatrix.tsx`: `164`, `PopuliPeerMasterPanel.tsx`: `161`, `PopuliTopologyView.tsx`: `153`, `PeerAttestationInspector.tsx`: `143`, `populiHeaderGauges.ts`: `98`).

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 455-02-SUMMARY.md

---

### Preserve export interface MeshNode on MeshView.tsx and surfaceDecorators...
Preserve `export interface MeshNode` on `MeshView.tsx` and `surfaceDecorators.populi` on `PopuliMeshBentoView` so downstream Mesh callers and e2e contracts stay intact.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 455-02-SUMMARY.md

## Lessons

### Previous ConciliumView.tsx:222 used body
Previous `ConciliumView.tsx:222` used `body: String(frame.kind?.message ?? ...)` directly inside the `SequenceBoardEntry` literal, tripping `toastBodyGuard` (`LEAK_SINK_PATTERN`) and risking `__TAURI_INTERNALS__` text exposure.

**Context:** Surfaced in structured phase code review.
**Source:** 455-REVIEW.md

---

### Fixed by extracting applyAgentEventFrame in conciliumLaneModel.ts and ro...
Fixed by extracting `applyAgentEventFrame` in `conciliumLaneModel.ts` and routing `frame.kind.message` through `sanitizeErrorForToast`, while also propagating CONC-01 fields (`swarm_seq`, `compute_tier`, `workspace_mode`, `span_id`, `parked_since_seq`) and deduplicating `board_tail` by `seq`.

**Context:** Surfaced in structured phase code review.
**Source:** 455-REVIEW.md

---

### Previous handlePilotAction invoked setHasRemoteSnapshot(true) and setLas...
Previous `handlePilotAction` invoked `setHasRemoteSnapshot(true)` and `setLastReceipt(...)` inside the `setSnapshot((prev) => ...)` updater callback (violating React 19 StrictMode purity and overwriting the server's `res.seq`/`res.message`).

**Context:** Surfaced in structured phase code review.
**Source:** 455-REVIEW.md

## Patterns

### Parked workers surface 0 tok/s · Asleep on #seq-N badges with determinis...
Parked workers surface `0 tok/s · Asleep on #seq-N` badges with deterministic `#channel` steer inputs and `VoxSelect` compute-tier swap controls.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 455-01-SUMMARY.md

---

### ConciliumView focus modes (swimlanes | board) unify Flow and SubAgents o...
`ConciliumView` focus modes (`swimlanes` | `board`) unify Flow and SubAgents onto a single W3C traceparent swimlane + `#seq` bulletin surface

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 455-01-SUMMARY.md

---

### All interactive select controls use <VoxSelect />; zero raw <select>, ze...
All interactive select controls use `<VoxSelect />`; zero raw `<select>`, zero `5000ms` blind polling, zero banned emoji/dingbat characters, `sanitizeErrorForToast` toast-body compliance

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 455-01-SUMMARY.md

---

### Compute placement refreshes reactively via coalesced Tauri event subscri...
Compute placement refreshes reactively via coalesced Tauri event subscriptions rather than timer polling.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 455-02-SUMMARY.md

---

### Single <PopuliTopologyView /> shared across PopuliMeshBentoView and Mesh...
Single `<PopuliTopologyView />` shared across `PopuliMeshBentoView` and `MeshView` with docked worker pills, VRAM telemetry, and `FileAffinityMap` lock shields

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 455-02-SUMMARY.md

---

### Coalesced event-driven listenAgentEvents + listenOrchStatus refresh repl...
Coalesced event-driven `listenAgentEvents` + `listenOrchStatus` refresh replacing blind 5s `setInterval` polling loops

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 455-02-SUMMARY.md

## Surprises

