---
phase: 210
phase_name: "dynamic-cv-scanner"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 1
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "VERIFICATION.md"
  - "UAT.md"
  - "STATE.md"
---

# Phase 210 Learnings: dynamic-cv-scanner

## Decisions

### Replace static CV surface enumeration with a dynamic discovery manifest ...
Replace static CV surface enumeration with a dynamic discovery manifest (`discoverAllGuiSurfacesAndPanels`) that derives all 32 `SURFACE_REGISTRY` views, all 14 `ChatSurface` dock panels (`CORE_PANEL_IDS` + `OPT_IN_PANEL_IDS`), and interactive `[data-cv-subsurface]` / `[data-cv-popover-trigger]` state markers, while adding DOM layout density metrics (`composerToolbarRowCount`, `composerToolbarHeightPx`, `duplicateControlGroupCount`) to `auditDomQualityInPage` and `evaluateGuiSurfaceRubric`.

**Rationale:** Primary phase objective established in plan.
**Source:** 210-01-PLAN.md

## Lessons

## Patterns

## Surprises

