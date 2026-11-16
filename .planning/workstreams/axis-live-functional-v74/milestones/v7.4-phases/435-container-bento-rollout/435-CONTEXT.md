---
phase: 435
title: "@container Concept-Art Bento Compaction & Resizable Master-Detail Rollout — Context"
status: ready_for_execution
decided_at: "2026-11-14"
requirements:
  - AXIS-BENTO-01
  - AXIS-BENTO-02
  - AXIS-BENTO-03
  - AXIS-BENTO-04
---

# Phase 435 Context: `@container` Concept-Art Bento Compaction & Resizable Master-Detail Rollout

## Locked Decisions (Smart Discuss — Accepted `3/3`)

1. **Area 1 (`AXIS-BENTO-01` — `OratioStudioView` & `PopuliMeshBentoView`):**
   - Replace generic 2-card `CommandCardsView` `<pre>` CLI dumps on `oratio` and `populi` in `crates/vox-gui/ui/src/components/surfaces/decoratorRegistry.ts` with dedicated live Bento consoles:
     - **`OratioStudioView`** (`crates/vox-gui/ui/src/components/surfaces/Oratio/OratioStudioView.tsx`): Live push-to-talk `oratio_transcribe` (`invokeCmd('oratio_transcribe', { seconds })`), `DiegeticFxStage` `'voice-harmonics'` acoustic VU stage, VAD sensitivity & capture-window `<RomanRotaryKnob>`s persisted via `getGuiPreference`/`setGuiPreference` (`vox.oratio.vad_threshold.v1`, `vox.oratio.capture_seconds.v1`), runtime/backend probe telemetry (`execute_command` `['oratio', 'doctor']` & `['oratio', 'status']`), and scrollable transcript history (`<BentoScrollRegion>`).
     - **`PopuliMeshBentoView`** (`crates/vox-gui/ui/src/components/surfaces/Populi/PopuliMeshBentoView.tsx`): Live peer roster via `useMeshNodesFull` (`vox_mesh_nodes`), `DiegeticFxStage` `'sunlit-colonnade'` QUIC mesh stage, gossip cadence `<RomanRotaryKnob>` (`vox.populi.gossip_cadence_ms.v1`), peer health matrix + selected-node detail inspector, and overlay/registry probes (`execute_command` `['populi', 'status']` & `['populi', 'registry-snapshot']`).
2. **Area 2 (`AXIS-BENTO-02` & `AXIS-BENTO-04` — `@container/surface`, `<BentoGrid>` & `<MasterDetailWorkbench>`):**
   - Upgrade `SurfaceScrollHost.tsx` with `@container/surface min-w-0` and `overflow-x-hidden overflow-y-auto` (`scrollHostOverflowPx === 0`).
   - Ship `crates/vox-gui/ui/src/components/ui/BentoGrid.tsx` exporting `<BentoGrid>`, `<BentoCard>`, `<BentoScrollRegion>` (`tabIndex={0}`, `role="region"`, `aria-label`, `min-w-0 min-h-0`), and `<MasterDetailWorkbench>` (`ResizablePanelGroup` when container width `>= 960px` and `!embedded`, automatic `<Sheet side="right">` drawer fallback `< 960px` or inside `320px` Chat companion docks, `.v4` schema-validated layout persistence via `getGuiPreference` / `setGuiPreference`, and `requestAnimationFrame`-coalesced `ResizeObserver`).
   - Roll out `<MasterDetailWorkbench>` / `@container/surface` `<BentoGrid>` across all 6 Tier A surfaces (`catalog`, `harness-health`, `approvals`, `gsd`, `mercatus`, `vox-search` / `VoxGraphStatusPanel`), eliminating `>50%` right-hand whitespace gutters on `1920×1080` monitors.
3. **Area 3 (`AXIS-BENTO-03` & `AXIS-BENTO-04` — Tier B Operational/CRUD Compaction):**
   - Apply `@container/surface` `<BentoGrid>` + `<BentoScrollRegion>` compaction across Tier B surfaces (`policies`, `explorer`, `memory`, `repository`, `publications`, `scientia`, `models`, `coderabbit`, `runs`, `tasks`, `sub-agents`, `skills`, `activity`, `needs-you`, `settings`, `mesh`, `mens`, `research`), preserving single `<RomanSurfaceHeader>` (`friezeHeaderCount === 1`), full-bleed chart fill (`chartPanelFillRatio >= 0.84`), and `clippedInteractiveCount === 0` down to `320px` Chat docks.
