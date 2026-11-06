# Phase 210 Plan 01 Summary: Dynamic Auto-Discovering GUI Surface, Sub-Tab, Dock & Popover CV Scanner

**Status:** Complete
**Requirements Verified:** `CV-AUTO-DISCOVER-01`, `CV-AUTO-DISCOVER-02`, `CV-AUTO-DISCOVER-03`, `CV-AUTO-DISCOVER-04`

## Key Changes

1. **Dynamic Surface & Dock Panel Introspection (`crates/vox-gui/ui/src/lib/cvSurfaceDiscovery.ts`)**:
   - Exported `discoverAllGuiSurfacesAndPanels()`, `ALWAYS_CORE_DOCK_PANEL_IDS`, `CORE_DOCK_PANEL_IDS`, `OPT_IN_DOCK_PANEL_IDS`, and `CHAT_DOCK_PANEL_IDS`.
   - Replaced hardcoded surface arrays in `crates/vox-gui/ui/e2e/review/states.ts` and `crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts` with dynamic runtime discovery from `SURFACE_REGISTRY` and `navigation.ts`.
   - Wired `ChatSurface.tsx` to import dock panel ID constants directly from `cvSurfaceDiscovery.ts`.
2. **In-Page Sub-Tab & Popover Discovery + Layout Detectors (`crates/vox-gui/ui/e2e/review/audits.ts`, `crates/vox-gui/ui/src/lib/cvFlywheel.ts`)**:
   - Added `composerToolbarRowCount`, `composerToolbarHeightPx`, `duplicateControlGroupCount`, `discoveredSubTabLabels`, and `discoveredPopoverTriggers`.
   - Enhanced `clippedInteractiveCount` to detect buttons overflowing right-rail/meter bounds or compact action buttons (`< 120px`) with truncated text.
3. **Verification**:
   - `pnpm --dir crates/vox-gui/ui exec vitest run src/lib/cvSurfaceDiscovery.test.ts src/guards/reviewStates.guard.test.ts src/lib/cvFlywheel.test.ts` — `3/3` suites (`10/10` tests) passed.
