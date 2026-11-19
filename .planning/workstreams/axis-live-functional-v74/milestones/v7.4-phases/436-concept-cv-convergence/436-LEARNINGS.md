---
phase: 436
phase_name: "concept-cv-convergence"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 5
  lessons: 3
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 436 Learnings: concept-cv-convergence

## Decisions

### v74-concept-bento-workbench-carrara.jpg (154 KB) & v74-concept-bento-wor...
`v74-concept-bento-workbench-carrara.jpg` (`154 KB`) & `v74-concept-bento-workbench-slate.jpg` (`126 KB`)

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 436-01-SUMMARY.md

---

### v74-concept-studio-telemetry-carrara.jpg (140 KB) & v74-concept-studio-t...
`v74-concept-studio-telemetry-carrara.jpg` (`140 KB`) & `v74-concept-studio-telemetry-slate.jpg` (`137 KB`)

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 436-01-SUMMARY.md

---

### Upgraded conceptArtPlateForSurface(surfaceId, activeTheme) and evaluateV...
Upgraded `conceptArtPlateForSurface(surfaceId, activeTheme)` and `evaluateVlmSurfaceCritique` in `crates/vox-gui/ui/e2e/review/vlmVisionFlywheel.ts` to pair redesigned Studio (`oratio`, `populi`, `mesh`, `mens`, `mens-runs`) and Bento Workbench (`catalog`, `mercatus`, `approvals`, `gsd`, `vox-search`, `policies`, `models`, `scientia`, `publications`, `memory`, `repository`, `coderabbit`, `skills`, `settings`, `research`, `sub-agents`, `activity`) surfaces against their active-theme plate (`arcane`/`glacier` -> `carrara`, `void`/`high-contrast` -> `slate`) with dual-theme materiality scoring.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 436-01-SUMMARY.md

---

### Upgraded auditDomInPage in crates/vox-gui/ui/e2e/review/cv-flywheel.spec...
Upgraded `auditDomInPage` in `crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts` from `.include('main')` to full-document `new AxeBuilder({ page }).options({ resultTypes: ['violations'] }).analyze()`, scanning `document.body` (`<nav>`, `<main>`, `BottomStatusBar`, and portaled `[data-radix-popper-content-wrapper]` / `[role="dialog"]` / `[role="listbox"]` / `[role="menu"]` nodes) on every surface capture.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 436-01-SUMMARY.md

---

### Added Test 9 (Milestone v7.4
Added Test 9 (`Milestone v7.4: Full-Document Open Radix Portals across 4 Themes & 5 Resolutions`) auditing open `Command` (`Omnibar`), `Popover`, `DropdownMenu`, `VoxSelect` (`[role="listbox"]`), `Dialog`, and `<960px` `MasterDetailWorkbench` `Sheet` across `900×600`, `1100×720`, `1280×800`, `1440×900`, and `1920×1080` and all 4 themes (`arcane`, `glacier`, `void`, `high-contrast`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 436-01-SUMMARY.md

## Lessons

### IN-01: Error message sanitization for OS paths in mercatus.rs.
**`IN-01`:** Error message sanitization for OS paths in `mercatus.rs`.

**Context:** Surfaced in structured phase code review.
**Source:** 436-REVIEW.md

---

### IN-02: Empty-candidates guard in getActiveTransportRouteStatus (t3Wave3S...
**`IN-02`:** Empty-candidates guard in `getActiveTransportRouteStatus` (`t3Wave3Shared.ts`).

**Context:** Surfaced in structured phase code review.
**Source:** 436-REVIEW.md

---

### IN-03: Preview reset helper, guarded preview JSON.parse, and 'mens-qwen3...
**`IN-03`:** Preview reset helper, guarded preview `JSON.parse`, and `'mens-qwen3-8b'` slug alignment (`t3Wave3VcsRpc.ts`, `t3Wave3ChatSessionRpc.ts`).

**Context:** Surfaced in structured phase code review.
**Source:** 436-REVIEW.md

## Patterns

## Surprises

