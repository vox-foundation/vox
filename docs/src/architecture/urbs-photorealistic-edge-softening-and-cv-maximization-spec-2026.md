---
title: "Urbs v5.3 Photorealistic Edge-Softening, Monumental Aqueduct-Sewer Integration & CV Flywheel Spec"
description: "Canonical specification for Milestone v5.3 (Phases 295–300): eliminating sharp vector polygon edges, saw-tooth axis-aligned crenellations, and high-frequency PBR Laplacian noise while integrating continuous 3D Roman aqueducts, sub-surface plumbing/sewer conduits, and soft cylindrical/coffered architectural shading."
category: "Architecture SSOTs"
status: "current"
topic: "agent-orchestrator"
domain: "orchestration"
authorship: "llm-assisted"
authorship_model: "claude-opus-4-6"
---

# Urbs v5.3 Photorealistic Edge-Softening, Monumental Aqueduct-Sewer Integration & CV Flywheel Spec

<!-- kb:summary -->
Defines the Milestone v5.3 (`Phases 295–300`, `24` requirements) graphical and semiotic architecture for `VRBS ROMA`. Resolves five root causes of sharp non-photorealistic edges and CV noise penalties identified in `v5.2`: (1) hard vector polygon outlines on coastal shelves and terrace diamonds, (2) axis-aligned `fillRect` saw-tooth wall crenellations and unblended rectangular street overlays, (3) screen-space conduit lines crossing over 3D building rooftops, (4) cylindrical/rotunda wonders lacking smooth 3D normal-shaded gradients, and (5) 1px high-frequency Laplacian pixel chatter in `pbrMaterials.ts`.
<!-- /kb:summary -->

## 1. Visual Critique & Root-Cause Diagnosis (`v5.2` → `v5.3`)

Inspection of the live `v5.2` frame (`urbs-civilization-after.png`, `visualAfterScore: 0.9387`, `noiseFloor: 10.7`, `noiseReductionPct: -109.8%`) identified five concrete graphical defects preventing true photorealism and higher CV scores:

| # | Visual Defect in `v5.2` | Root Cause in Code | `v5.3` Photorealistic Engineering Fix |
|---|---|---|---|
| **1** | **Hard diamond polygon cutouts in the sea around cliff bases** | `paintFullBleedRomanLandscape` draws coastal turquoise shallows and cliff shadows using hard 4-vertex `fillPoly` diamonds with `1.1px` vector strokes. | Replace polygon coastal shelves with **feathered multi-stop elliptical radial coastal shoal & surf gradients** that blend smoothly into the deep bathymetric sea (`0` hard polygon edges). |
| **2** | **Jagged saw-tooth wall crenellations & harsh `#171310` cartoon ink outlines** | `drawDistrictCurtainWall` stamps axis-aligned 2D `ctx.fillRect` merlons along diagonal `2:1` isometric walls, and `drawIsoBox` outlines every wall with dark `#1a1613` ink strokes. | Replace 2D `fillRect` teeth with **true isometric molded travertine parapet copings (`Corona`)**, soft edge bevels, and subtle tonal mortar seams instead of black cartoon outlines. |
| **3** | **Translucent grey/brown rectangles pasted over terraces for streets & plazas** | `drawDistrictGroundDetails` draws overlapping semi-transparent `Cardo` and `Decumanus` polygons (`rgba(34,30,27,0.68)`) with visible alpha-compositing cross seams. | Render **continuous recessed stone street channels** with beveled curbstones, perspective-aligned polygonal basalt flagstones (`Via Silicea`), and recessed drainage gutters (`Canalis`) that meet cleanly at a central **coffered `Opus Sectile` Forum**. |
| **4** | **Monumental Aqueduct gap & screen-space neon lines crossing over 3D rooftops** | `drawAqueductComplex` draws 6 small `fillRect` posts that stop in mid-air, and `drawScreenSpaceHydraulicOverlay` draws straight conduit lines *on top of* 3D buildings in `Layer 6`. | Build a **continuous 3D Double-Tiered Roman Stone Aqueduct (`Aqua Claudia`)** bridging `AQVAE MCP` directly into the `Castellum Divisorium` header tank, and route all pressurized `Fistulae` and `Cloaca Maxima` conduits **in the ground/viaduct layer (`Layer 0.85` & `Layer 3.8`) with smooth beveled isometric elbows (`quadraticCurveTo`)**. |
| **5** | **High-frequency 1px PBR pixel chatter (`noiseFloor: 10.7`)** | `pbrMaterials.ts` uses 1px horizontal/vertical `fillRect` lines that trigger 4-neighbor Laplacian noise penalties in `computeRgbaFrameCvMetrics`. | Synthesize **smooth bilinear/radial macro-shaded stone, marble, and water tiles** in `pbrMaterials.ts` that eliminate 1px Laplacian chatter (`noiseFloor < 5.0`, positive `noiseReductionPct`) while increasing macro luminance contrast (`visualAfterScore >= 0.955`). |

---

## 2. Phase Breakdown (`Phases 295–300`, `24` Requirements)

### Phase 295: Feathered Coastal Bathymetry, Smooth Macro-Shaded PBR Materials & Laplacian Noise Suppression (`URBS-EDGE-PBR-01..04`)
- **`URBS-EDGE-PBR-01` (Feathered Elliptical Coastal Shoals & Surf Gradients):** Replace the hard 4-vertex polygon coastal shelf in `paintFullBleedRomanLandscape` with multi-stop feathered elliptical radial gradients (`rgba(14, 116, 144, 0.34)` → `rgba(6, 182, 212, 0.14)` → `rgba(5, 16, 27, 0)`) and soft organic foam crests around cliff bases.
- **`URBS-EDGE-PBR-02` (Low-Laplacian Macro-Shaded PBR Stone & Cobble Synthesis):** Upgrade `buildTravertineTile`, `buildTerracottaTile`, `buildPlazaCobbleTile`, and `buildCliffTile` in `pbrMaterials.ts` to use smooth gradient-shaded ashlar courses and soft mortar transitions instead of 1px high-contrast noise lines.
- **`URBS-EDGE-PBR-03` (Smooth Specular Ocean Swell Ribbons):** Add full-canvas diagonal isometric twilight ocean swell gradients in `paintFullBleedRomanLandscape` that create natural macro water depth without 64px tile seams or 1px Laplacian noise.
- **`URBS-EDGE-PBR-04` (Soft Tonal Edge Bevels Replacing Cartoon Ink Outlines):** Upgrade `drawIsoBox` and `drawIsoPitchedRoof` in `worldRenderer.ts` so wall and roof polygons use subtle tonal stone/terracotta edge strokes and soft specular bevels instead of dark `#171310` cartoon ink outlines.

### Phase 296: True Isometric Molded Curtain Walls, Seamless `Via Silicea` Street Grid & Coffered `Opus Sectile` Forums (`URBS-TERRACE-CRAFT-01..04`)
- **`URBS-TERRACE-CRAFT-01` (True Isometric Molded Parapet Copings & Beveled Bastions):** Replace the jagged 2D `ctx.fillRect` crenellations in `drawDistrictCurtainWall` with true `2:1` isometric molded travertine parapet capstones (`Corona`), recessed arrow-loops (`Ballistaria`), and 3D pyramidal-roofed corner bastions.
- **`URBS-TERRACE-CRAFT-02` (Seamless Intersection-Free `Cardo` & `Decumanus` Basalt Paving):** Refactor `drawDistrictGroundDetails` so the `Cardo Maximus` and `Decumanus Maximus` render as a unified recessed stone cross-plaza (`Via Silicea`) with zero overlapping alpha seams, flanked by continuous 3D travertine curbstones (`Crepido`) and storm-drain grates.
- **`URBS-TERRACE-CRAFT-03` (Recessed Multi-Ring `Opus Sectile` Marble Forum & Peristyle Columns):** Upgrade the central district plaza with a recessed travertine border step, concentric porphyry/verde-antico marble geometry, and 4 miniature corner Corinthian peristyle columns (`Columnae`) with soft ground shadows.
- **`URBS-TERRACE-CRAFT-04` (Smooth Cliff-Edge Travertine Cornice & Buttress Shading):** Upgrade the terrace cliff rim in `paintFullBleedRomanLandscape` with a continuous 3D overhanging travertine cornice (`Geison`) that casts a soft drop shadow down the `Substructio` arcade wall.

### Phase 297: Cylindrical & Coffered 3D Architectural Realism for Wonders & `T0`–`T3` Insulae (`URBS-ARCH-REALISM-01..04`)
- **`URBS-ARCH-REALISM-01` (3D Cylindrical Normal-Shaded `Tabularium` Rotunda & Coffered Half-Dome):** Upgrade `drawIsoBedrockHarborVault` and `drawIsoPantheonAcropolis` with continuous multi-stop horizontal cylindrical lighting gradients, recessed coffered half-dome ribs (`Lacunaria`), and warm interior brazier bounce light.
- **`URBS-ARCH-REALISM-02` (Recessed Isometric Window Reveals & Warm Interior Lamplight Gradients):** Upgrade `drawIsoBox` windows from flat 2D `fillRect` squares to isometric-skewed recessed stone window reveals with warm golden-hour interior radial glow on active/lit buildings.
- **`URBS-ARCH-REALISM-03` (Sculpted Pediment Tympanums, Portico Colonnades & Step Stylobates):** Enrich `T2` Basilica and `T3` Temple buildings in `drawIsoFileBuilding` with a 2-stepped stone stylobate base, shaded portico columns, and a recessed triangular pediment tympanum with bronze relief boss.
- **`URBS-ARCH-REALISM-04` (3D Monumental `AQVAE`, `CASTRVM`, `PORTVS` & `PORTA` Peripheral Complexes):** Upgrade all four peripheral landmarks to use 3D isometric masonry piers, arched stone harbor moles, and smooth water/torch reflections instead of flat 2D `fillRect` primitives.

### Phase 298: Continuous 3D Monumental `Aqua Claudia` Viaduct & Ground-Integrated Beveled Hydraulic/Sewer Grid (`URBS-PLUMBING-3D-01..04`)
- **`URBS-PLUMBING-3D-01` (Continuous 3D Multi-Tiered `Aqua Claudia` Aqueduct Bridge):** Upgrade `drawAqueductComplex` and `drawRomanHydraulicInfrastructure` so `AQVAE MCP` forms a continuous 3D isometric stone arcade viaduct with arched piers, spandrel masonry, and an open illuminated water channel (`Specus`) connecting directly into the Acropolis `Castellum Divisorium`.
- **`URBS-PLUMBING-3D-02` (Curved Beveled Isometric Pipe Elbows & 3D Shaded `Fistulae` / `Cloaca`):** Upgrade `drawConduitTrack` in `hydraulicNetwork.ts` to render smooth rounded isometric bends (`quadraticCurveTo` elbow fillets), 3D cylindrical bronze/travertine pipe shading, and soft recessed ground shadow trenches.
- **`URBS-PLUMBING-3D-03` (Sub-Roof Ground-Layer Routing for All Conduits):** Move blast-radius and dependency conduit rendering out of the post-building screen-space overlay so conduits run **on the terrace ground / viaduct plane beneath 3D buildings**, keeping rooftops 100% clean and unobstructed while highlighting active junction valves at district perimeters.
- **`URBS-PLUMBING-3D-04` (Stepped Stone `Cataracta` Sluice-Drops & Vaulted `Cloaca` Sea Outfalls):** Render 3D stepped stone water cascades (`Cataracta`) where aqueducts drop between `L3 → L2 → L1 → L0` elevation terraces, and vaulted stone `Cloaca Maxima` portals at cliff bases.

### Phase 299: Illuminated Halo-Backed Shape+Color Activity Particles & Interactive Conduit Semiotics (`URBS-FLOW-SEMIOTICS-01..04`)
- **`URBS-FLOW-SEMIOTICS-01` (Soft Radial Halo Backlighting & Comet Trails on All 4 Shape Channels):** Enhance `drawChevronWaveParticle`, `drawDiamondIngotParticle`, `drawWarningTriangleParticle`, and `drawLaurelSealParticle` in `hydraulicNetwork.ts` with soft radial glow halos and tapered comet tails so every moving particle reads Effortlessly at a glance without harsh jagged edges.
- **`URBS-FLOW-SEMIOTICS-02` (Dynamic Flow Rate, Pipe Gauge & Pressure Pulsing by Activity Load):** Scale conduit track width, inner stream luminance, and particle cadence proportionally to live activity intensity (MCP call count, git diff churn lines, diagnostic error count, CI runner count).
- **`URBS-FLOW-SEMIOTICS-03` (Diegetic `Castellum` Sluice-Gate Manifold & Perimeter Junction Rings):** Render glowing bronze manifold rings (`Epistomia`) at district street entry/exit ports so developers can visually trace which upstream provider feeds which downstream consumer without lines crossing over buildings.
- **`URBS-FLOW-SEMIOTICS-04` (Unified Semiotic Legend Pill with Shape Icons & Flow Counters):** Upgrade the top-left Roman Hydraulic & Sewer Semiotic Legend Pill in `drawScreenSpaceHydraulicOverlay` to render actual vector geometric shapes (`❯` Chevron, `◆` Diamond, `▲` Triangle, `◉` Laurel Seal) alongside live active channel counts.

### Phase 300: Comprehensive Unit Tests, Semiotic Critique Audit & Playwright CV Flywheel Score Maximization (`URBS-CV-MAX-01..04`)
- **`URBS-CV-MAX-01` (Unit Tests for Smooth Beveled Conduits, Low-Noise PBR & Edge-Softening):** Expand `pbrMaterials.test.ts`, `lightingAndFx.test.ts`, `hydraulicNetwork.test.ts`, and `worldRenderer.test.ts` to verify curved elbow fillets, ground-layer conduit routing, and low Laplacian noise floor (`computeRgbaFrameCvMetrics` `noiseFloor <= 6.5`).
- **`URBS-CV-MAX-02` (Positive CV Noise Reduction & Contrast Optimization):** Tune macro lighting contrast (`luminanceContrast >= 56`) and flat-region Laplacian smoothness (`noiseFloor <= 5.0`) so `urbs-civilization` achieves **positive `noiseReductionPct`** and **`visualAfterScore >= 0.945+`** in the `@ngi/platform-ssot` CV Flywheel.
- **`URBS-CV-MAX-03` (32-Surface Zero-Regression Verification):** Verify all 32 registered GUI surfaces in `e2e/review/cv-flywheel.spec.ts` pass with `0` overflow, `0` overlapping controls, and `0` axe-core violations.
- **`URBS-CV-MAX-04` (Visual Critique & Before/After Blueprint Documentation):** Capture the verified `v5.3` production screenshot and update the visual blueprint carousel.
