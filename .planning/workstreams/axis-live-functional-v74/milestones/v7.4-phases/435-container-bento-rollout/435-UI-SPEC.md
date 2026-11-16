---
phase: 435
title: "@container Concept-Art Bento Compaction & Resizable Master-Detail Rollout — UI Specification"
status: approved
---

# UI Specification: Phase 435 (`AXIS-BENTO-01..04`)

## Container Breakpoints & Density Rules

- **Scroll Host (`SurfaceScrollHost.tsx`)**:
  - Viewport class: `@container/surface h-full min-h-0 min-w-0 overflow-x-hidden overflow-y-auto custom-scrollbar`
  - Preserves `role="region"`, `aria-label="Surface content"`, `tabIndex={0}`.
- **Bento Primitives (`crates/vox-gui/ui/src/components/ui/BentoGrid.tsx`)**:
  - `<BentoGrid preset="balanced" | "dense" | "hero" | "triad">`:
    - `< 640px` (including `320px` `DockWorkspaceShell` companion docks): `grid-cols-1 gap-2.5`
    - `@2xl/surface` (`>= 672px`): `grid-cols-2 gap-3`
    - `@5xl/surface` (`>= 1024px`): `grid-cols-3` (`triad` / `balanced`) or `grid-cols-4` (`dense`) or `grid-cols-12` (`hero`)
  - `<BentoCard>`: `min-w-0 min-h-0 flex flex-col`, theme-aware surface border/elevation, zero duplicate `data-roman-frieze`.
  - `<BentoScrollRegion ariaLabel="...">`: `min-w-0 min-h-0 overflow-auto custom-scrollbar`, `role="region"`, `tabIndex={0}`, `aria-label={ariaLabel}` (Axe `scrollable-region-focusable`).
  - `<MasterDetailWorkbench storageKey="vox.layout.<surface>.v4">`:
    - Schema: `{ version: 4, masterPct: number }` (`25 <= masterPct <= 75`), validated before hydration; invalid/legacy payloads fall back cleanly to `defaultMasterPct`.
    - Desktop (`width >= 960` and `!embedded`): `<ResizablePanelGroup orientation="horizontal">` with `<ResizableHandle withHandle />`.
    - Narrow / Dock (`width < 960` or `embedded === true`): Full-width master pane + `<Sheet side="right">` inspector drawer when an item is active or inspector button is clicked.

## CV & Static Audit Invariants

1. **Zero `transition-all` & Coalesced `ResizeObserver`**: Every `new ResizeObserver(` callback wraps state updates inside `requestAnimationFrame` within 8 lines (`vox/ui/layout-and-perf-hazards`).
2. **Single Frieze Header**: Exactly `1` `<RomanSurfaceHeader>` (`data-roman-frieze="true"`) per surface (`friezeHeaderCount === 1`).
3. **Full-Bleed Chart Fill**: Every `[data-cv-chart="true"]` and `.vox-marble-tablet` sparkline spans `w-full` inside its immediate parent (`chartPanelFillRatio >= 0.84`).
4. **Zero Clipped Compact Buttons**: Narrow combobox triggers carry `data-allow-ellipsis="true"` and action buttons use `shrink-0 whitespace-nowrap` or wrapping rows so `clippedInteractiveCount === 0` at `900×600`..`1920×1080` and `320px` docks.
5. **File Size Budget**: Newly created files and sub-500-line modified files remain strictly `< 500` non-blank lines (`toestub` `arch/god_object`).
