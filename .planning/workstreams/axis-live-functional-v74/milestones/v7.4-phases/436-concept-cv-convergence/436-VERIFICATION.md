---
phase: 436
title: "Phase 436 Verification — Concept-Art Plates, Full-Document Accessibility, Static Audit & Multi-Pass CV Flywheel Convergence"
status: passed
verified_at: "2026-11-14T19:23:00Z"
requirements_verified:
  - AXIS-CONCEPT-CV-01
  - AXIS-CONCEPT-CV-02
  - AXIS-CONCEPT-CV-03
  - AXIS-CONCEPT-CV-04
---

# Phase 436 Verification Report

## Verdict: `passed` (4/4 requirements verified with raw empirical proof)

| Requirement | Status | Empirical Evidence |
|-------------|--------|--------------------|
| `AXIS-CONCEPT-CV-01` | PASSED | 4 dual-theme reference plates (`v74-concept-bento-workbench-{carrara,slate}.jpg` `154K`/`126K`, `v74-concept-studio-telemetry-{carrara,slate}.jpg` `140K`/`137K`) committed under `crates/vox-gui/ui/public/assets/romanesque/` and paired dynamically by surface + `data-theme` in `crates/vox-gui/ui/e2e/review/vlmVisionFlywheel.ts`. |
| `AXIS-CONCEPT-CV-02` | PASSED | `auditDomInPage` in `crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts` executes full-document `new AxeBuilder({ page }).options({ resultTypes: ['violations'] }).analyze()` across `document.body` + open Radix portals (`Command` `Omnibar`, `Popover`, `DropdownMenu`, `Select`, `Dialog`, `Sheet`) across 4 themes (`arcane`, `glacier`, `void`, `high-contrast`) and 5 resolutions (`900×600`..`1920×1080`) with `axeViolations === 0`, `rawEmojiCount === 0`, `iconIssueCount === 0`, and `outerOverflowPx === 0`. |
| `AXIS-CONCEPT-CV-03` | PASSED | `pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/live-controls-v74.spec.ts --project=chromium` -> **`4 passed (26.5s)` (`exit 0`)**, asserting real `set_gui_preference`, `oratio_transcribe`, `mercatus_save_config`, `execute_command`, `<RomanBrassGauge>` dial filter, `VoxSelect`, and Recharts `<Brush>` interactions + reload persistence with `0` `sentryHonesty` violations. |
| `AXIS-CONCEPT-CV-04` | PASSED | Multi-pass `CV_STAGE=after CV_PASS=4 VOX_CV_FLYWHEEL=1` converged **`9 passed (5.6m)` (`exit 0`)** across all 33 `SURFACE_REGISTRY` surfaces and 14 docks (`visualAfterScore`: `0.9250`–`0.9665` vs `>= 0.90` gate; `vlmOverallScore`: `0.9235`–`0.9535` vs `>= 0.88` gate; `composerToolbarRowCount === 1`; `chartPanelFillRatio >= 0.84`). `pnpm build` (`4.67s`), `pnpm knip` (`exit 0`), `vitest` (`370/370` suites, `2,310/2,310` tests), `toestub` (`0` issues), `vox ci gui-catalog-parity` (`exit 0`), `vox-drift-check` (`exit 0`), and `vox-arch-check` (`exit 0`) all green. |

## Representative Empirical CV Flywheel Scores (`cv-flywheel-all-surfaces.json` & `cv-flywheel-report.json`)

| Surface / Scenario | Pre-v7.4 `visualBeforeScore` | Converged `visualAfterScore` | `vlmOverallScore` | `axeViolations` | `rawEmojiCount` | `outerOverflowPx` |
|--------------------|------------------------------|------------------------------|-------------------|-----------------|-----------------|-------------------|
| `chat-default` | `0.8327` | **`0.9266`** (`+0.0939`) | **`0.9462`** | `0` | `0` | `0` |
| `chat-dock-multi` | `0.8225` | **`0.9398`** (`+0.1173`) | **`0.9462`** | `0` | `0` | `0` |
| `chat-multidock-900x600` | `0.8138` | **`0.9363`** (`+0.1225`) | — | `0` | `0` | `0` |
| `chat-multidock-1100x720` | `0.8209` | **`0.9378`** (`+0.1169`) | — | `0` | `0` | `0` |
| `oratio` (Bento Studio) | `0.9304` (CLI cards) | **`0.9301`** (Live Studio) | **`0.9504`** | `0` | `0` | `0` |
| `populi` (QUIC Mesh Studio) | `0.9304` (CLI cards) | **`0.9445`** (`+0.0141`, `+63.3%` contrast) | **`0.9444`** | `0` | `0` | `0` |
| `dashboard` | `0.9158` | **`0.9476`** (`+0.0318`) | **`0.9453`** | `0` | `0` | `0` |
| `gamify` (`urbs-civilization`) | `0.8807` | **`0.9531`** (`+0.0724`) | **`0.9264`** | `0` | `0` | `0` |
| `flow` | `0.8518` | **`0.9372`** (`+0.0854`) | **`0.9419`** | `0` | `0` | `0` |
| `harness` | `0.9099` | **`0.9411`** (`+0.0312`) | **`0.9490`** | `0` | `0` | `0` |
| `mens` | `0.8878` | **`0.9290`** (`+0.0412`) | **`0.9374`** | `0` | `0` | `0` |
| `mercatus` | `0.9300` | **`0.9293`** | **`0.9476`** | `0` | `0` | `0` |
| `catalog` | `0.9264` | **`0.9261`** | **`0.9467`** | `0` | `0` | `0` |
| `console` | `0.9660` | **`0.9665`** | **`0.9535`** | `0` | `0` | `0` |
| `research` | `0.8950` | **`0.9264`** (`+0.0314`) | **`0.9473`** | `0` | `0` | `0` |
| `vox-search` (`CodeAtlasCanvas`) | `0.9250` | **`0.9270`** (`+0.0020`) | **`0.9491`** | `0` | `0` | `0` |
