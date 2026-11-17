---
phase: 455-vox-concilium-populi-unified-swarm-observatory-gui
review_scope: plans 455-01 & 455-02 (CONC-05..08) — /gsd-code-review --fix
reviewed_at: 2026-11-16
reviewer: cr-fix-455-closeout
status: clean
files_reviewed: 24
findings:
  critical: 2
  warning: 5
  info: 5
  fixed: 12
  skipped: 0
---

# Phase 455 Code & Architecture Review (`--fix`)

## Scope Reviewed
- Shared DTOs & RPCs: [`crates/vox-gui/ui/src/types/concilium.ts`](crates/vox-gui/ui/src/types/concilium.ts) (`171` LoC), [`crates/vox-gui/ui/src/transport/conciliumRpc.ts`](crates/vox-gui/ui/src/transport/conciliumRpc.ts) (`44` LoC), [`crates/vox-gui/ui/src/transport.ts`](crates/vox-gui/ui/src/transport.ts)
- Mode 1 (`Concilium/`): [`ConciliumView.tsx`](crates/vox-gui/ui/src/components/surfaces/Concilium/ConciliumView.tsx) (`269` LoC), [`conciliumLaneModel.ts`](crates/vox-gui/ui/src/components/surfaces/Concilium/conciliumLaneModel.ts) (`255` LoC), [`conciliumDefaultSnapshot.ts`](crates/vox-gui/ui/src/components/surfaces/Concilium/conciliumDefaultSnapshot.ts) (`172` LoC), [`CausalSwimlanePanel.tsx`](crates/vox-gui/ui/src/components/surfaces/Concilium/CausalSwimlanePanel.tsx) (`289` LoC), [`LiveAgentLens.tsx`](crates/vox-gui/ui/src/components/surfaces/Concilium/LiveAgentLens.tsx) (`292` LoC), [`SharedSequenceBoard.tsx`](crates/vox-gui/ui/src/components/surfaces/Concilium/SharedSequenceBoard.tsx) (`220` LoC), [`NowPlayingStrip.tsx`](crates/vox-gui/ui/src/components/surfaces/Concilium/NowPlayingStrip.tsx) (`183` LoC), [`ConciliumView.test.tsx`](crates/vox-gui/ui/src/components/surfaces/Concilium/ConciliumView.test.tsx) (`154` LoC)
- Consolidated Mode 1 routes/embeds: [`AgentFlow.tsx`](crates/vox-gui/ui/src/components/surfaces/Flow/AgentFlow.tsx) (`45` LoC), [`SubAgentsView.tsx`](crates/vox-gui/ui/src/components/surfaces/SubAgents/SubAgentsView.tsx) (`109` LoC), [`AgentTrajectoryBraid.tsx`](crates/vox-gui/ui/src/components/surfaces/Harness/AgentTrajectoryBraid.tsx) (`183` LoC)
- Mode 2 (`Populi/` & `Mesh/`): [`PopuliTopologyView.tsx`](crates/vox-gui/ui/src/components/surfaces/Populi/PopuliTopologyView.tsx) (`153` LoC), [`usePopuliTopologyData.ts`](crates/vox-gui/ui/src/components/surfaces/Populi/usePopuliTopologyData.ts) (`334` LoC), [`populiHeaderGauges.ts`](crates/vox-gui/ui/src/components/surfaces/Populi/populiHeaderGauges.ts) (`98` LoC), [`PopuliQuicTopologyRing.tsx`](crates/vox-gui/ui/src/components/surfaces/Populi/PopuliQuicTopologyRing.tsx) (`182` LoC), [`PopuliPeerMasterPanel.tsx`](crates/vox-gui/ui/src/components/surfaces/Populi/PopuliPeerMasterPanel.tsx) (`161` LoC), [`PopuliDispatchWorkbench.tsx`](crates/vox-gui/ui/src/components/surfaces/Populi/PopuliDispatchWorkbench.tsx) (`293` LoC), [`PeerAttestationInspector.tsx`](crates/vox-gui/ui/src/components/surfaces/Populi/PeerAttestationInspector.tsx) (`143` LoC), [`HybridComputeNodeCard.tsx`](crates/vox-gui/ui/src/components/surfaces/Populi/HybridComputeNodeCard.tsx) (`347` LoC), [`FileAffinityLockMatrix.tsx`](crates/vox-gui/ui/src/components/surfaces/Populi/FileAffinityLockMatrix.tsx) (`164` LoC), [`PopuliMeshBentoView.tsx`](crates/vox-gui/ui/src/components/surfaces/Populi/PopuliMeshBentoView.tsx) (`18` LoC), [`MeshView.tsx`](crates/vox-gui/ui/src/components/surfaces/Mesh/MeshView.tsx) (`30` LoC)

## Findings & Fixes Applied (`--fix`)

1. **[CR-01 / Critical — Fixed] Guard failure (`toastBodyGuard.test.ts`) in `ConciliumView.tsx`:**
   - Previous `ConciliumView.tsx:222` used `body: String(frame.kind?.message ?? ...)` directly inside the `SequenceBoardEntry` literal, tripping `toastBodyGuard` (`LEAK_SINK_PATTERN`) and risking `__TAURI_INTERNALS__` text exposure.
   - Fixed by extracting `applyAgentEventFrame` in `conciliumLaneModel.ts` and routing `frame.kind.message` through `sanitizeErrorForToast`, while also propagating CONC-01 fields (`swarm_seq`, `compute_tier`, `workspace_mode`, `span_id`, `parked_since_seq`) and deduplicating `board_tail` by `seq`.

2. **[CR-02 / Critical — Fixed] Impure `setState` calls inside `setSnapshot` updater (`ConciliumView.tsx`):**
   - Previous `handlePilotAction` invoked `setHasRemoteSnapshot(true)` and `setLastReceipt(...)` inside the `setSnapshot((prev) => ...)` updater callback (violating React 19 StrictMode purity and overwriting the server's `res.seq`/`res.message`).
   - Moved `setHasRemoteSnapshot(true)` and `setLastReceipt(...)` outside `setSnapshot`, added `mountedRef` + `refreshSeqRef` unmount/race guards, and preserved backend receipt text when `conciliumPilotAction` succeeds (and surfaced `'error'` toast when `res.ok === false`).

3. **[WR-01 / Warning — Fixed] Repeated pilot actions overwrote prior state on `legacyAgents`/`subAgentNodes` (`ConciliumView.tsx`):**
   - Previous `baseLanes = buildLanesFromLegacy(...) ?? prev.lanes` re-seeded from unmodified props on every pilot action, discarding earlier `park_agent` / `fork_agent` / `steer_agent` mutations.
   - Fixed via `hasRemoteSnapshotRef` so `applyPilotActionToSnapshot` seeds from legacy props only before the first mutation/remote snapshot and evolves `prev.lanes` thereafter; added regression test in `ConciliumView.test.tsx`.

4. **[WR-02 / Warning — Fixed] Module `<400`-LoC headroom & decomposition (`ConciliumView.tsx`, `PopuliTopologyView.tsx`, `PopuliDispatchWorkbench.tsx`):**
   - Extracted `conciliumDefaultSnapshot.ts` (`172`) & `conciliumLaneModel.ts` (`255`) from `ConciliumView.tsx` (`394 -> 269` LoC).
   - Extracted `PopuliQuicTopologyRing.tsx` (`182`), `PopuliPeerMasterPanel.tsx` (`161`), `populiHeaderGauges.ts` (`98`), and `usePopuliTopologyData.ts` (`334`) from `PopuliTopologyView.tsx` (`385 -> 153` LoC).
   - Extracted `PeerAttestationInspector.tsx` (`143`) from `PopuliDispatchWorkbench.tsx` (`385 -> 293` LoC). Every file is strictly `<= 347` LoC.

5. **[WR-03 / Warning — Fixed] Unstable `pushToast` callback causing redundant overlay probe re-runs & missing event coalescing (`PopuliTopologyView` / `usePopuliTopologyData.ts`):**
   - Stabilized `pushToast` via `pushToastRef`, guarded `handleMigrateWorkerHere` with `mountedRef.current`, and added an in-flight/pending coalescing loop on `listenAgentEvents` + `listenOrchStatus` refreshes (also refreshing `nodes`).

6. **[WR-04 / Warning — Fixed] Fabricated fallback VRAM & concurrent migration click race (`HybridComputeNodeCard.tsx`):**
   - Removed guessed 50% `vram_used_mb` synthesis (`Math.round(n.vram_gb * 512)` -> `null`) when only `MeshNode.vram_gb` is known, and changed `canMigrate` to check `!migratingTargetNodeId` so concurrent migration clicks are blocked while any migration is in flight.

7. **[WR-05 / Warning — Fixed] Out-of-range slice index & legend/path count mismatch (`AgentTrajectoryBraid.tsx`):**
   - Aligned legend rows and SVG paths to `trajectories.slice(0, 6)`, clamped `row.slices` index lookups to `[0, SLICE_ROLES.length - 1]` so `'undefined'` cannot render, and eliminated duplicate `buildSmoothBraidPoints` computation; added regression test in `AgentTrajectoryBraid.test.tsx`.

8. **[IN-01..05 / Info — Fixed] Accessibility, input hygiene, lifecycle & key uniqueness:**
   - Added `aria-pressed={isLive}` + descriptive `aria-label` and distinct paused styling on `NowPlayingStrip.tsx` Live/Paused toggle; added `aria-pressed` on `HybridComputeNodeCard.tsx` node/worker buttons and `PopuliPeerMasterPanel.tsx` peer rows.
   - Reset `steerDirective` when changing inspected agent and clamped `progress` percentage `[0..100]` + added Enter key submission in `LiveAgentLens.tsx`.
   - Guarded empty lanes and non-finite `windowDurationMs` in `CausalSwimlanePanel.tsx`; disabled empty steer submission and hardened mention/entry React keys in `SharedSequenceBoard.tsx`.
   - Added `mountedRef` unmount guard and deduplicated `<VoxSelect>` node options in `PopuliDispatchWorkbench.tsx`; hardened row keys in `FileAffinityLockMatrix.tsx`; memoized `SubAgentsView.tsx` callbacks with `useCallback`.

## Post-Fix Verification (Exact Observed Runs)
- **Typecheck:** `timeout 300s pnpm --dir crates/vox-gui/ui exec tsc --noEmit` → exit `0`
- **Vitest (16 files — 7 surfaces + 9 guards):** `Test Files 16 passed (16)`, `Tests 61 passed (61)` (`ConciliumView.test.tsx` `5`, `SubAgentsView.test.tsx` `3`, `AgentFlow.test.tsx` `6`, `AgentTrajectoryBraid.test.tsx` `2`, `PopuliTopologyView.test.tsx` `3`, `PopuliMeshBentoView.test.tsx` `3`, `MeshView.test.tsx` `6`, `zeroEmojiAndDingbat.test.ts` `2`, `noRawNativeControlsGuard.test.ts` `2`, `ipcBoundaries.test.ts` `3`, `transportIpcGuard.test.ts` `2`, `commandBackendsMigration.test.ts` `11`, `surfaceRegistryEscape.test.ts` `4`, `toastBodyGuard.test.ts` `1`, `noPaperFallbackGuard.test.ts` `4`, `reviewStates.guard.test.ts` `4`)
- **Bundle & Dead-File Checks:** `pnpm --dir crates/vox-gui/ui run knip:files` → exit `0` (`Knip found no issues`); `pnpm --dir crates/vox-gui/ui run bundle` → exit `0` (`built in 1.25s`)
- **Module Size & Polling Invariants:** `max` file size `347` LoC (`< 400` LoC), `0` raw `<select>` / `<input type="range">`, `0` banned emojis/dingbats, `0` `5000ms` blind polling loops, `0` direct `@tauri-apps/api/{core,event}` leaks.

## Verdict
**CLEAN** — Phase 455 satisfies all `CONC-05..08` UI, accessibility, reactivity, typing, guard-suite, and `<400`-LoC modularity constraints.
