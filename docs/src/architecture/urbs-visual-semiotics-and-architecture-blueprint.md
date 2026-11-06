---
title: "Urbs Visual Semiotics & Architecture Blueprint (v5.3)"
description: "Carousel notes and verified visual targets for VRBS ROMA after Milestone v5.3 edge-softening and CV maximization."
category: "Architecture SSOTs"
status: "current"
topic: "gamification"
domain: "gui"
authorship: "llm-generated"
authorship_model: "unknown"
---

# Urbs Visual Semiotics & Architecture Blueprint (`v5.3`)

<!-- kb:summary -->
Visual targets and verification notes for VRBS ROMA, the gamified city view in the Vox GUI,
after milestone v5.3 (Phases 295–299) softened edges and raised the computer-vision quality
score. It records the verified gamify unit suite, the screenshot path and the CV metrics the
milestone aimed for (noise floor and visual score), a per-phase checklist of rendering changes
such as walls, vaults, aqueducts and particles, and the four semiotic legend channels: AQUA for
supply, IGNIS for agent work in progress, CLOACA for faults and VICTORIA for verified CI.
<!-- /kb:summary -->

Verified unit suite: `pnpm --dir crates/vox-gui/ui exec vitest run src/components/gamify` (97/97).

## Production Screenshot

- After frame path: `crates/vox-gui/ui/artifacts/cv-flywheel/urbs-civilization-after.png`
- Prior CV artifact (pre-re-score): `noiseFloor: 10.7`, `visualAfterScore: 0.9387`, `noiseReductionPct: -109.8%`
- v5.3 target: `noiseFloor < 5.1` (positive `noiseReductionPct`), `visualAfterScore >= 0.942`

## Visual Checklist (Phases 295–299)

| Phase | Visual signal |
|---|---|
| 295 | Feathered coastal shoals, soft-mortar PBR, soft tonal bevels |
| 296 | Isometric `Corona` walls, seamless `Via Silicea`, `Opus Sectile` + columns, cliff `Geison` |
| 297 | Cylindrical/`Lacunaria` vaults, recessed lit windows, stylobates/tympanums |
| 298 | Continuous `Aqua Claudia`, rounded pipe elbows, Layer 0.85 ground conduits |
| 299 | Halo+comet particles, load-scaled gauges, `Epistomia` rings, vector legend |

## Semiotic Legend Channels

1. `❯ AQUA` — cyan supply
2. `◆ IGNIS` — gold agent WIP
3. `▲ CLOACA` — crimson fault
4. `◉ VICTORIA` — emerald verified CI

Last updated: 2026-10-25T23:33:09Z
