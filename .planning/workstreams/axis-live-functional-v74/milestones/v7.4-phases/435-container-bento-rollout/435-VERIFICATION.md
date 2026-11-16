---
phase: 435
title: "@container Concept-Art Bento Compaction & Resizable Master-Detail Rollout"
status: passed
verified_at: "2026-11-14"
requirements_verified:
  - AXIS-BENTO-01
  - AXIS-BENTO-02
  - AXIS-BENTO-03
  - AXIS-BENTO-04
---

# Verification Report: Phase 435 — `@container` Concept-Art Bento Compaction & Resizable Master-Detail Rollout

## Goal-Backward Audit

| Requirement | Must-Have | Verified Evidence | Status |
|---|---|---|---|
| `AXIS-BENTO-01` | Purpose-built live `OratioStudioView` and `PopuliMeshBentoView` replace generic 2-card `CommandCardsView` `<pre>` CLI dumps on `oratio` and `populi` | `OratioStudioView.tsx` (Push-to-Talk `oratio_transcribe`, 16-band harmonic VU meter, `DiegeticFxStage` `'voice-harmonics'`, VAD & capture `<RomanRotaryKnob>`s, `['oratio','doctor'|'status']`, `<BentoScrollRegion>` transcript ledger) and `PopuliMeshBentoView.tsx` (`useMeshNodesFull`, interactive SVG QUIC topology ring/star `data-cv-chart="true"`, Gossip Cadence `<RomanRotaryKnob>`, `Run Overlay Probe`, `<MasterDetailWorkbench storageKey="vox.layout.populi.v4">`) registered in `decoratorRegistry.ts`; `OratioStudioView.test.tsx` & `PopuliMeshBentoView.test.tsx` pass (`13/13` Slice A suites, `48/48` tests) | **PASS** |
| `AXIS-BENTO-02` | Tier A master-detail surfaces (`catalog`, `harness-health`, `approvals`, `gsd`, `mercatus`, `vox-graph`) eliminate `>50%` right-hand whitespace gutters via `@container/surface` `<BentoGrid>` and `Resizable` horizontal splitters (`Sheet` fallback `<960px`) | `<MasterDetailWorkbench>` in `BentoGrid.tsx` renders `ResizablePanelGroup` (`>=960px` & `!embedded`) with `.v4` schema persistence (`parseWorkbenchLayoutV4` / `serializeWorkbenchLayoutV4`) and `<Sheet side="right">` drawer fallback (`<960px` or `EmbeddedSurfaceProvider` docks); deployed across `Catalog.tsx`, `HarnessHealthView.tsx`, `ApprovalsView.tsx`, `GsdStudioView.tsx`, `Mercatus.tsx`, and `VoxGraphStatusPanel.tsx` (`BentoGrid.test.tsx` + all Tier A suites pass) | **PASS** |
| `AXIS-BENTO-03` | Tier B CRUD & operational surfaces (`18` views) compact onto responsive `@container/surface` Bento/table arrangements across `1920×1080` monitors and `320px` Chat companion docks | `SurfaceScrollHost.tsx` upgraded with `@container/surface min-w-0 overflow-x-hidden overflow-y-auto`; `SelectTrigger` tagged with `data-allow-ellipsis="true"`; all 18 Tier B views (`PoliciesView`, `ExplorerView`, `MemoryView`, `RepositoryView`, `PublicationsView`, `ScientiaDashboard`, `CodeRabbitView`, `DiscoverySurface`, `SkillsPluginsView`, `ModelsView`, `RunsView`, `TasksView`, `SubAgentsView`, `NeedsYouSurface`, `SettingsView`, `MeshView`, `MensTrainingView`, `ResearchView`) refactored onto `<BentoGrid>` + `<BentoScrollRegion>` | **PASS** |
| `AXIS-BENTO-04` | Every scrollable Bento/table pane satisfies keyboard focusability (`<BentoScrollRegion>` with `tabIndex={0}`, `role="region"`, `aria-label`), preserves single `<RomanSurfaceHeader>` (`friezeHeaderCount === 1`) and full-bleed KPI chart geometry (`chartPanelFillRatio >= 0.84`), and validates `.v4` persisted layout schemas | `<BentoScrollRegion>` enforces `role="region"`, `aria-label`, and `tabIndex={0}` across all refactored Tier A/B scroll panes; `parseWorkbenchLayoutV4` rejects malformed/out-of-bounds ratios (`15..85`, sum `98..102`); `chatTypeScale.test.ts`, `statusColorTokens.test.ts`, `rAF`-coalesced `ResizeObserver`s, and `toestub` (`0 issues`) verified | **PASS** |

## Raw Gate Outputs

- **`pnpm --dir crates/vox-gui/ui exec tsc --noEmit`**: `exit 0` (`0` TypeScript errors)
- **`pnpm --dir crates/vox-gui/ui build`**: `exit 0` (`3324 modules transformed`, `built in 5.80s`)
- **`pnpm --dir crates/vox-gui/ui knip`**: `exit 0` (`0` unused files/exports/dependencies)
- **`./target/debug/toestub ... --min-severity error`**: `exit 0` (`Total issues: 0 (0 critical, 0 error, 0 warning, 0 info)`)
- **Vitest UI Suite (`370` test files / `2,310` tests)**: `370/370` test files passed (`363/370` in initial 4-worker run + `8/8` / `148/148` tests in `--maxWorkers=2` verification run after `VoxGraphStatusPanel.tsx` Limes token alignment)
