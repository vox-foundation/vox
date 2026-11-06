---
phase: 295-feathered-bathymetry-and-smooth-pbr
plan: "01"
subsystem: vox-gui-urbs
status: Complete
completed: "2026-10-25T23:33:09Z"
requirements:
  - URBS-EDGE-PBR-01
  - URBS-EDGE-PBR-02
  - URBS-EDGE-PBR-03
  - URBS-EDGE-PBR-04
---

# Phase 295 Plan 01 Summary: Feathered Coastal Bathymetry, Smooth Macro-Shaded PBR Materials & Laplacian Noise Suppression

**Status:** Complete

## Requirements
- [x] `URBS-EDGE-PBR-01`
- [x] `URBS-EDGE-PBR-02`
- [x] `URBS-EDGE-PBR-03`
- [x] `URBS-EDGE-PBR-04`

## Files Changed
- `crates/vox-gui/ui/src/components/gamify/urbs/pbrMaterials.ts`
- `crates/vox-gui/ui/src/components/gamify/urbs/worldRenderer.ts`
- `crates/vox-gui/ui/src/components/gamify/urbs/__tests__/pbrMaterials.test.ts`

## Delivered
1. **URBS-EDGE-PBR-01** — Feathered elliptical radial coastal shoals + soft foam crescents in `paintFullBleedRomanLandscape` (no hard 4-vertex diamond cutouts).
2. **URBS-EDGE-PBR-02** — Soft overlapping radial ashlar synthesis (`fillSoftAshlarBlock`) for travertine / terracotta / plaza / cliff tiles (low-Laplacian mortar).
3. **URBS-EDGE-PBR-03** — Full-canvas diagonal twilight ocean swell ribbons.
4. **URBS-EDGE-PBR-04** — Soft tonal edge bevels on `drawIsoBox` / `drawIsoPitchedRoof` (no `#1a1613` cartoon ink).

## Verification
- `pnpm --dir crates/vox-gui/ui exec vitest run src/components/gamify` → **97/97 PASS**

