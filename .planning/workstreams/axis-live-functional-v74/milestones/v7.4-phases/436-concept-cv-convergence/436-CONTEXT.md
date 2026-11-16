---
phase: 436
title: "Concept-Art Plates, Full-Document Accessibility, Static Audit & Multi-Pass CV Flywheel Convergence"
status: ready
discussed_at: "2026-11-14"
requirements:
  - AXIS-CONCEPT-CV-01
  - AXIS-CONCEPT-CV-02
  - AXIS-CONCEPT-CV-03
  - AXIS-CONCEPT-CV-04
---

# Phase 436 Context: Concept-Art Plates, Full-Document Accessibility, Static Audit & Multi-Pass CV Flywheel Convergence

## Locked User Decisions (Smart Discuss `3/3`)

1. **Dual-Theme Bento & Studio Concept-Art Plates (`AXIS-CONCEPT-CV-01`)**:
   - Generate 4 optimized (`<= 160 KB` JPEG, `1024px` max dimension) reference concept-art plates under `crates/vox-gui/ui/public/assets/romanesque/`:
     - `v74-concept-bento-workbench-carrara.jpg` (`arcane` / `glacier` light ivory Carrara marble & `#D4AF37` burnished brass master-detail Bento workbench)
     - `v74-concept-bento-workbench-slate.jpg` (`void` / `high-contrast` dark honed basalt-slate & `#D4AF37` gold master-detail Bento workbench)
     - `v74-concept-studio-telemetry-carrara.jpg` (`arcane` / `glacier` acoustic & QUIC mesh telemetry Bento studio)
     - `v74-concept-studio-telemetry-slate.jpg` (`void` / `high-contrast` acoustic & QUIC mesh telemetry Bento studio)
   - Update `conceptArtPlateForSurface(surfaceId, activeTheme?)` and `evaluateVlmSurfaceCritique` in `crates/vox-gui/ui/e2e/review/vlmVisionFlywheel.ts` so redesigned Bento workbenches (`catalog`, `mercatus`, `approvals`, `gsd`, `vox-search`, `policies`, `models`, `scientia`, `publications`, `memory`, `repository`, `coderabbit`, `skills`, `settings`, `research`) and live acoustic/mesh studios (`oratio`, `populi`, `mesh`, `mens`, `mens-runs`) resolve theme-appropriate plates (`arcane`/`glacier` -> `-carrara.jpg`, `void`/`slate`/`high-contrast` -> `-slate.jpg`) and score Q1 materiality accurately across both light-marble and dark-slate themes.

2. **Full-Document & Open Radix Portal Auditing (`AXIS-CONCEPT-CV-02`)**:
   - Upgrade `auditDomInPage` in `crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts` from `.include('main')` to full-document `new AxeBuilder({ page }).analyze()` (`document.body` including portaled overlays).
   - Add an explicit Radix Portal & Multi-Theme/Multi-Resolution audit test in `cv-flywheel.spec.ts` exercising open `VoxSelect` (`[role="listbox"]`), `Dialog` (`[role="dialog"]`), `DropdownMenu` (`[role="menu"]`), `Popover`, `Sheet`, and `Command` (`Omnibar`) across all 4 themes (`arcane`, `glacier`, `void`, `high-contrast`) and 5 resolutions (`900×600`, `1100×720`, `1280×800`, `1440×900`, `1920×1080`), asserting `axeViolations === 0`, `rawEmojiCount === 0`, `iconIssueCount === 0`, and `outerOverflowPx === 0`.

3. **Live-Control Playwright E2E Round-Trips & Multi-Pass CV Flywheel Convergence (`AXIS-CONCEPT-CV-03..04`)**:
   - Create `crates/vox-gui/ui/e2e/review/live-controls-v74.spec.ts` exercising:
     - `<RomanRotaryKnob>` keyboard/pointer adjustment + `set_gui_preference` persistence round-trip (`OratioStudioView`, `PopuliMeshBentoView`, `ImperiumInspector`)
     - `VoxSelect` Radix combobox selection + IPC filter/config round-trip
     - Clickable `<RomanBrassGauge>` header dial filter actuation
     - Recharts `<Brush>` timeline handles + datum click drill-down on `Dashboard`
     - `Mercatus` `<Switch>` + threshold save (`mercatus_save_config`)
     - `Oratio` Push-to-Talk (`oratio_transcribe`) transcript append + `0` `sentryHonesty` `fake_success` alerts
   - Preserve genuine pre-v7.4 `before` baselines (`all-surfaces-before.json`, `before-metrics.json`, `*-before.png`) and execute multi-pass `CV_STAGE=after` (`CV_PASS=1..N`) across all 33 `SURFACE_REGISTRY` surfaces and 14 `DockWorkspaceShell` docks until 100% converge (`visualAfterScore >= 0.90`, `vlmOverallScore >= 0.88`, `composerToolbarRowCount === 1`, `chartPanelFillRatio >= 0.84`), followed by workspace CI gates and inline Before/After chat carousels.
