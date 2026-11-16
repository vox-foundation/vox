---
phase: 435
title: "Phase 435 Code Review — @container Concept-Art Bento Compaction & Resizable Master-Detail Rollout"
status: approved
reviewed_at: "2026-11-14"
---

# Code Review: Phase 435 (`AXIS-BENTO-01..04`)

## Dimensions Evaluated

1. **Container Responsiveness & Whitespace Elimination (`PASS`)**
   - `SurfaceScrollHost.tsx` establishes `@container/surface`, and `<BentoGrid>` / `<MasterDetailWorkbench>` respond to actual container width rather than `window.innerWidth` viewport breakpoints, eliminating `>50%` right-hand empty gutters on `1920×1080` while stacking cleanly to 1 column + `<Sheet>` inspector inside `320px` `DockWorkspaceShell` companion docks.
2. **Accessibility & Keyboard Focusability (`PASS`)**
   - Every scrollable Bento/table/tree/inspector region wraps content in `<BentoScrollRegion>` (`role="region"`, `tabIndex={0}`, mandatory `ariaLabel`), satisfying Axe `scrollable-region-focusable`.
3. **Architecture, File-Size Budgets & Performance Hygiene (`PASS`)**
   - `ApprovalsInspectorPane.tsx` and `MercatusInspectorPane.tsx` were cleanly extracted so every newly created or sub-500-line modified `.tsx` file remains well under `toestub`'s 500-non-blank-line limit (`0` findings).
   - Both `BentoGrid.tsx` and `ExplorerView.tsx` coalesce `new ResizeObserver(` callbacks via `requestAnimationFrame` (`vox/ui/layout-and-perf-hazards` compliant), and 7 legacy `transition-all` classes in `GsdStudioView.tsx` and `MensTrainingView.tsx` were eliminated.
4. **Schema Validation & CV Geometry Invariants (`PASS`)**
   - `parseWorkbenchLayoutV4` rejects malformed, out-of-range, or pre-v4 storage payloads before hydrating `MasterDetailWorkbench` split ratios.
   - Every surface retains a single `<RomanSurfaceHeader>` (`friezeHeaderCount === 1`) and `w-full` chart geometry (`chartPanelFillRatio >= 0.84`).
