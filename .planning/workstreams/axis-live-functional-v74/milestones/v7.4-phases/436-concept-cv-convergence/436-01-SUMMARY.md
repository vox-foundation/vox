---
phase: 436
plan: 01
title: "Concept-Art Plates, Full-Document Accessibility, Static Audit & Multi-Pass CV Flywheel Convergence"
status: completed
completed_at: "2026-11-14T19:22:00Z"
requirements_completed:
  - AXIS-CONCEPT-CV-01
  - AXIS-CONCEPT-CV-02
  - AXIS-CONCEPT-CV-03
  - AXIS-CONCEPT-CV-04
---

# Phase 436 Plan 01 Summary: Concept-Art Plates, Full-Document Accessibility, Static Audit & Multi-Pass CV Flywheel Convergence

## What Was Built

1. **Dual-Theme Bento & Studio Concept-Art Plates (`AXIS-CONCEPT-CV-01`)**:
   - Generated and JPEG-optimized (`sips -Z 1024 -s format jpeg -s formatOptions 78`, `126 KB`–`154 KB` each) four dual-theme reference plates under `crates/vox-gui/ui/public/assets/romanesque/`:
     - `v74-concept-bento-workbench-carrara.jpg` (`154 KB`) & `v74-concept-bento-workbench-slate.jpg` (`126 KB`)
     - `v74-concept-studio-telemetry-carrara.jpg` (`140 KB`) & `v74-concept-studio-telemetry-slate.jpg` (`137 KB`)
   - Upgraded `conceptArtPlateForSurface(surfaceId, activeTheme)` and `evaluateVlmSurfaceCritique` in `crates/vox-gui/ui/e2e/review/vlmVisionFlywheel.ts` to pair redesigned Studio (`oratio`, `populi`, `mesh`, `mens`, `mens-runs`) and Bento Workbench (`catalog`, `mercatus`, `approvals`, `gsd`, `vox-search`, `policies`, `models`, `scientia`, `publications`, `memory`, `repository`, `coderabbit`, `skills`, `settings`, `research`, `sub-agents`, `activity`) surfaces against their active-theme plate (`arcane`/`glacier` -> `carrara`, `void`/`high-contrast` -> `slate`) with dual-theme materiality scoring.

2. **Full-Document & Open Radix Portal Audit (`AXIS-CONCEPT-CV-02`)**:
   - Upgraded `auditDomInPage` in `crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts` from `.include('main')` to full-document `new AxeBuilder({ page }).options({ resultTypes: ['violations'] }).analyze()`, scanning `document.body` (`<nav>`, `<main>`, `BottomStatusBar`, and portaled `[data-radix-popper-content-wrapper]` / `[role="dialog"]` / `[role="listbox"]` / `[role="menu"]` nodes) on every surface capture.
   - Added Test 9 (`Milestone v7.4: Full-Document Open Radix Portals across 4 Themes & 5 Resolutions`) auditing open `Command` (`Omnibar`), `Popover`, `DropdownMenu`, `VoxSelect` (`[role="listbox"]`), `Dialog`, and `<960px` `MasterDetailWorkbench` `Sheet` across `900×600`, `1100×720`, `1280×800`, `1440×900`, and `1920×1080` and all 4 themes (`arcane`, `glacier`, `void`, `high-contrast`).
   - Updated `auditIconsInPage` in `crates/vox-gui/ui/e2e/review/audits.ts` to distinguish responsive CSS `display: none` containers (`max-xl:hidden`, `max-[1399px]:hidden`) from genuinely zero-sized visible SVGs while still auditing `!drawable` (`empty-svg`) across all SVGs.

3. **Automated Live-Control E2E Spec (`AXIS-CONCEPT-CV-03`)**:
   - Shipped `crates/vox-gui/ui/e2e/review/live-controls-v74.spec.ts` (`4/4` tests passing) recording real `window.__TAURI_INTERNALS__.invoke` IPC round-trips and reload persistence across `OratioStudioView` (`RomanRotaryKnob` + Push-to-Talk `oratio_transcribe`), `PopuliMeshBentoView` & `ImperiumInspector` (`RomanRotaryKnob` + overlay probes), `RunsView` clickable `<RomanBrassGauge>` dial filter + `ModelsView` `VoxSelect` + `Dashboard` Recharts `<Brush>` drag & bar drill-down, and `Mercatus` `<Switch>` + threshold `mercatus_save_config` with `checkHonestyInvariant` (`0` `_demo` tokens, `0` `fake_success` alerts, `0` `pageerror`s).

4. **Multi-Pass CV Flywheel Remediations & Convergence (`AXIS-CONCEPT-CV-04`)**:
   - Pass 1–3 full-document Axe & runtime sweeps surfaced 6 concrete defects across the 33 surfaces, all remediated at root cause:
     - `OratioStudioView.tsx`: added `role="img"` to `oratio-harmonic-bars` (`aria-prohibited-attr` -> `0`).
     - `PopuliMeshBentoView.tsx`: changed `populi-quic-topology-svg` from `role="img"` to `role="group"` so interactive peer `<g role="button">` nodes satisfy ARIA (`nested-interactive` -> `0`).
     - `SessionSidebarSection.tsx`: removed nested `role="button"` inside `<div role="tab">` and added keyboard `Enter`/`Space`/`Delete` handling directly on the `role="tab"` row (`nested-interactive` -> `0`, `10/10` unit tests green).
     - `Console/InputEditor.tsx` & `VoxGraph/CodeAtlasCanvas.tsx`: replaced legacy dark-only `bg-zinc-900`/`text-zinc-200..300` classes on `VoxSelect` and atlas cards with semantic theme-contract tokens (`bg-bg-surface`, `bg-bg-elevated`, `border-border-subtle`, `text-text-primary`), eliminating WCAG AA `color-contrast` leaks in `arcane` light mode.
     - `VoxGraph/VoxGraphStatusPanel.tsx`: guarded optional `warnings`, `stale_reasons`, and `corpora` DTO arrays (`?? []`), fixing a runtime `TypeError` caught on `#view=vox-search`.
   - Pass 4 converged `9/9` suites in `cv-flywheel.spec.ts` (`exit 0`) with `visualAfterScore` `0.9250`–`0.9665` (`>= 0.90` bar) and `vlmOverallScore` `0.9235`–`0.9535` (`>= 0.88` bar) across all 33 `SURFACE_REGISTRY` surfaces.
