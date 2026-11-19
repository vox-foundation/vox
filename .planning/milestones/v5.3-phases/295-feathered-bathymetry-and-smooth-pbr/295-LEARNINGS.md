---
phase: 295
phase_name: "feathered-bathymetry-and-smooth-pbr"
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

# Phase 295 Learnings: feathered-bathymetry-and-smooth-pbr

## Decisions

### Eliminate hard diamond coastal cutouts, 1px Laplacian PBR pixel chatter ...
Eliminate hard diamond coastal cutouts, 1px Laplacian PBR pixel chatter (`noiseFloor: 10.7`), and `#1a1613` cartoon ink outlines by introducing feathered elliptical radial coastal shoals, smooth bilinear/radial macro-shaded PBR stone/terracotta/water tiles, full-canvas diagonal ocean swells, and soft tonal masonry/roof edge bevels.

**Rationale:** Primary phase objective established in plan.
**Source:** 295-01-PLAN.md

## Lessons

## Patterns

## Surprises

