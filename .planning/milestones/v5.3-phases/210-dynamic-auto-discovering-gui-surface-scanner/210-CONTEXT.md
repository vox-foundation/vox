# Phase 210: Dynamic Auto-Discovering GUI Surface, Sub-Tab, Dock & Popover CV Scanner - Context

**Gathered:** 2026-10-25
**Status:** Ready for execution

<domain>
## Phase Boundary

Upgrade `cv-flywheel.spec.ts`, `states.ts`, `audits.ts`, and a new `src/lib/cvSurfaceDiscovery.ts` module so 100% of registered views (`SURFACE_REGISTRY`), decorator surfaces (`surfaceDecorators`), navigation routes (`PARENT_CHILD_MAP` + `TOP_LEVEL_VIEWS`), Chat dock panels (`CHAT_DOCK_PANEL_IDS`), and in-page sub-tabs (`[role="tab"]`) / popovers (`[data-cv-popover-trigger]`) are dynamically discovered and audited for clipping, composer toolbar row-wrapping, and control duplication without hardcoded surface arrays.
</domain>

<decisions>
## Implementation Decisions

- **Zero hardcoded surface arrays**: Derive `SURFACE_STATES` in `e2e/review/states.ts` and `ALL_SURFACES` in `e2e/review/cv-flywheel.spec.ts` dynamically from `discoverAllGuiSurfacesAndPanels()` (`src/lib/cvSurfaceDiscovery.ts`).
- **Export `CHAT_DOCK_PANEL_IDS`** from `ChatSurface.tsx` (`Object.keys(CHAT_DOCK_COMPONENTS)`) so the CV scanner automatically knows all registered dock panels.
- **In-page sub-tab & popover auto-discovery**: Add `discoverInPageSubSurfaces()` in `e2e/review/audits.ts` that queries visible `[role="tab"]`, `[data-cv-subsurface]`, and `[data-cv-popover-trigger]` elements inside `<main>`, and record discovered sub-tabs and popovers per surface in `cv-flywheel-all-surfaces.json`.
- **Structural clipping, row-wrap & duplication detectors**: Add `composerToolbarRowCount`, `composerToolbarHeightPx`, and `duplicateControlGroupCount` to `DomQualityReport` (`audits.ts`) and `GuiDomAuditMetrics` (`cvFlywheel.ts`), and strengthen `clippedInteractiveCount` to also detect right-edge rail overflow (`btnRect.right > railRect.right + 2`) and truncated compact action buttons (`el.scrollWidth > el.clientWidth + 4` on buttons `< 140px` without `data-allow-ellipsis="true"`).
</decisions>
