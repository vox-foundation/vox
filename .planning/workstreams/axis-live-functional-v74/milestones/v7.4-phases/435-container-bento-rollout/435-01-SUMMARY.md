---
phase: 435
plan: 01
title: "@container Concept-Art Bento Compaction & Resizable Master-Detail Rollout"
status: complete
completed_at: "2026-11-14"
requirements_completed:
  - AXIS-BENTO-01
  - AXIS-BENTO-02
  - AXIS-BENTO-03
  - AXIS-BENTO-04
---

# Summary 435-01: `@container` Concept-Art Bento Compaction & Resizable Master-Detail Rollout

## Delivered Capabilities

### 1. `@container/surface` & Bento / Master-Detail Primitives (`Step 0` — `AXIS-BENTO-02`, `AXIS-BENTO-04`)
- **`crates/vox-gui/ui/src/components/layout/SurfaceScrollHost.tsx`**: Added `@container/surface min-w-0 overflow-x-hidden overflow-y-auto` onto the scroll viewport while preserving `role="region"`, `aria-label="Surface content"`, and `tabIndex={0}` (`scrollHostOverflowPx === 0`).
- **`crates/vox-gui/ui/src/components/ui/Select.tsx`**: Marked `SelectTrigger` with `data-allow-ellipsis="true"` so narrow combobox triggers inside `320px` Chat companion docks never trip false-positive `clippedInteractiveCount`.
- **`crates/vox-gui/ui/src/components/ui/BentoGrid.tsx` & `BentoGrid.test.tsx`**: Shipped `<BentoGrid>` (`balanced`, `dense`, `triad`, `hero` `@container/surface` presets), `<BentoCard>`, `<BentoScrollRegion>` (`tabIndex={0}`, `role="region"`, `ariaLabel` for Axe `scrollable-region-focusable`), and `<MasterDetailWorkbench>` (`ResizablePanelGroup` when container width `>= 960px` & `!embedded`, automatic `<Sheet side="right">` drawer fallback `< 960px` or embedded docks, `rAF`-coalesced `ResizeObserver`, and `.v4` schema-validated layout persistence `parseWorkbenchLayoutV4` / `serializeWorkbenchLayoutV4` via `voxTransport.getGuiPreference` / `setGuiPreference`).

### 2. Purpose-Built `OratioStudioView` & `PopuliMeshBentoView` (`Slice 435-A` — `AXIS-BENTO-01`)
- **`OratioStudioView.tsx` + `OratioStudioView.test.tsx`**: Replaced the generic 2-card `CommandCardsView` `<pre>` CLI dump on `oratio` with a live Bento speech-to-code studio featuring Push-to-Talk (`invokeCmd('oratio_transcribe', { seconds })`), `DiegeticFxStage` `'voice-harmonics'` + 16-band harmonic VU meter, two `<RomanRotaryKnob>` controls (`vox.oratio.vad_threshold.v1`, `vox.oratio.capture_seconds.v1`) with rollback + `sanitizeErrorForToast`, structured `['oratio', 'doctor']` / `['oratio', 'status']` diagnostic badges, and a focusable `<BentoScrollRegion>` transcript ledger (`vox.oratio.history.v1`).
- **`PopuliMeshBentoView.tsx` + `PopuliMeshBentoView.test.tsx`**: Replaced the generic `CommandCardsView` `<pre>` CLI dump on `populi` with a live QUIC mesh console featuring `useMeshNodesFull`, `DiegeticFxStage` `'sunlit-colonnade'`, a full-bleed (`w-full`, `data-cv-chart="true"`) interactive SVG QUIC topology ring/star, Gossip Cadence `<RomanRotaryKnob>` (`vox.populi.gossip_cadence_ms.v1`), `Run Overlay Probe` trigger, and `<MasterDetailWorkbench storageKey="vox.layout.populi.v4">` peer attestation inspector.
- **`decoratorRegistry.ts` & `tauriMock.ts`**: Registered `oratio: OratioStudioView` and `populi: PopuliMeshBentoView`, and added deterministic `oratio_transcribe` mock coverage in `tauriMock.ts`.

### 3. Tier A Master-Detail Bento & Resizable Splitters (`Slice 435-B` — `AXIS-BENTO-02`)
- Upgraded all 6 Tier A surfaces (`Catalog.tsx` + `CommandCatalogForm.tsx`, `HarnessHealthView.tsx`, `ApprovalsView.tsx` + `ApprovalsInspectorPane.tsx`, `GsdStudioView.tsx`, `Mercatus.tsx` + `MercatusInspectorPane.tsx`, `VoxGraphStatusPanel.tsx`) to use `<BentoGrid>`, `<BentoScrollRegion>`, and `<MasterDetailWorkbench>` (`vox.layout.{catalog,harness-health,approvals,gsd,mercatus,vox-graph}.v4`), eliminating `>50%` right-hand whitespace gutters on `1920×1080` monitors while adapting cleanly to `<960px` viewports and `320px` companion docks.

### 4. Tier B Responsive Bento & Scroll-Region Compaction (`Slices 435-C & 435-D` — `AXIS-BENTO-03`, `AXIS-BENTO-04`)
- Compacted all 18 Tier B operational, governance, compute, and knowledge surfaces (`PoliciesView`, `ExplorerView`, `MemoryView`, `RepositoryView`, `PublicationsView`, `ScientiaDashboard`, `CodeRabbitView`, `DiscoverySurface`, `SkillsPluginsView`, `ModelsView`, `RunsView`, `TasksView`, `SubAgentsView`, `NeedsYouSurface`, `SettingsView`, `MeshView`, `MensTrainingView`, `ResearchView`) onto `@container/surface` `<BentoGrid>` + `<BentoScrollRegion>` (`tabIndex={0}`, `role="region"`, `aria-label`), removing narrow `max-w-*` single-column bottlenecks and viewport-only `lg:`/`xl:` traps so every view fills wide desktop monitors and stacks cleanly inside `320px` Chat companion docks.
