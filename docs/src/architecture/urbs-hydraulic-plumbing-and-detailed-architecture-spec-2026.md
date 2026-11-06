---
title: "Urbs v5.2: Smooth Physically-Grounded Lighting, High-Detail Architecture & Grounds, and Integrated Roman Hydraulic/Sewer Conduit System"
description: "Architecture specification for Milestone v5.2 (Phases 289–294): eliminating alpha-overlay lighting artifacts, enriching building and terrace ground architecture, integrating the Aquaeductus/Fistulae/Cloaca Maxima hydraulic-sewer network, and encoding live activities via animated shape+color conduits."
category: "Architecture SSOTs"
status: "current"
topic: "agent-orchestrator"
domain: "orchestration"
authorship: "llm-assisted"
authorship_model: "claude-opus-4-6"
---

# Urbs v5.2: Smooth Physically-Grounded Lighting, High-Detail Architecture & Grounds, and Integrated Roman Hydraulic/Sewer Conduit System

<!-- kb:summary -->
Defines the Milestone v5.2 (Phases 289–294) architectural upgrades for the Urbs Roma codebase civilization simulator (`worldRenderer.ts`, `lightingAndFx.ts`, `pbrMaterials.ts`, and `hydraulicNetwork.ts`). Replaces alpha-overlay lighting blobs and tiled water seams with pre-building ground-plane bounce illumination, feathered Gaussian atmospheric light shafts, and seamless trigonometric wave synthesis; upgrades terrace grounds (`Cardo`/`Decumanus` streets, `Opus Sectile` plazas, `Substructio` cliff arcades) and T0–T3 buildings (rusticated arcades, stringcourse cornices, porticoes, impluvium courtyards); and promotes disconnected lines into an integrated SimCity-style Roman hydraulic supply (`Aquaeductus`, `Castellum Divisorium`, `Fistulae`) and `Cloaca Maxima` sewer network with animated color-and-shape-coded activity conduits.
<!-- /kb:summary -->

## 1. Visual Critique of `v5.0` & Root-Cause Diagnosis

Inspection of the `v5.0` production render (`urbs-photorealistic-v50-prod.png`) identified four structural visual weaknesses:

1. **Alpha-Overlay Lighting Artifacts (`URBS-SMOOTH-LIGHT-01..04`)**:
   - `drawVolumetricGodRays` previously drew 4-vertex polygon strips with linear gradients that faded only longitudinally while leaving hard lateral polygon edges across roofs and sea.
   - `drawAdditiveLightPools` painted radial gradients at `Layer 5` *after* all 3D buildings were rendered, placing semi-opaque yellow/red ellipses over rooftops rather than illuminating the terrace stone *beneath* the structures.
   - `drawAnimatedWorldEffects` drew a hard-edged pulsing circle (`ctx.arc(..., radius)`) over every district center.
   - `pbrMaterials.ts` generated `seaWater` using non-periodic noise coordinates, creating visible `64×64` rectangular tile boundaries across the Mediterranean sea.

2. **Under-Detailed Terrace Grounds & Cliff Substructures (`URBS-GROUNDS-01..04`)**:
   - District terraces were rendered as uniform cobblestone parallelograms without orthogonal Roman street grids (`Cardo Maximus` and `Decumanus Maximus`), curbstones (`Crepido`), central `Opus Sectile` marble forum inlays around District Wonders, or vaulted `Substructio` retaining wall arcades along cliff faces.

3. **Simplified Secondary Building Massing (`URBS-ARCH-DETAIL-01..04`)**:
   - File buildings (`T0` Taberna, `T1` Domus, `T2` Insula, `T3` Basilica) relied on basic `drawIsoBox` primitives without ground-floor rusticated arches (`Fornices`), inter-story stringcourse belt cornices (`Marcapiano`), recessed `Impluvium` rainwater courtyards, or sculpted rooftop `Acroteria` and hypocaust brick chimneys.

4. **Disconnected Aqueduct & Ambiguous Floating Connection Lines (`URBS-HYDRAULIC-01..04`, `URBS-CONDUIT-ANIM-01..04`)**:
   - The peripheral `AQVAE MCP` aqueduct terminated in isolation on the upper-right margin without feeding the city's districts.
   - Dependency, blast-radius, and agent-activity connections were drawn as raw screen-space straight lines cutting across rooftops, lacking physical grounding and multi-channel shape+color differentiation.

---

## 2. Six-Subsystem Architecture (`Phases 289–294`)

```vox-diagram
graph {
    direction: down
    container l0: "Layer 0–0.9: Seamless Sea, Detailed Grounds & Recessed Hydraulic Grid" {
        node p289: "Phase 289: Seamless Periodic Sea & Pre-Building Ground-Plane Illumination"
        node p290: "Phase 290: Cardo/Decumanus Streets, Opus Sectile Plazas & Substructio Cliff Arcades"
        node p292: "Phase 292: Integrated Aquaeductus, Castellum Divisorium, Fistulae Pipes & Cloaca Maxima Sewers"
    }
    container l1: "Layer 1–3: High-Detail Vitruvian Buildings & Wonders" {
        node p291: "Phase 291: Rusticated Arcades, Belt Cornices, Atrium Impluvia & Acroteria Roofs"
    }
    container l4: "Layer 4–5: Animated Shape+Color Activity Conduits & Feathered Atmospheric Light" {
        node p293: "Phase 293: Multi-Channel Color+Shape Animated Activity Conduits"
        node p294: "Phase 294: Semiotic Consistency Audit, Unit Tests & Playwright CV Flywheel"
    }
    l0.p289 -> l0.p290
    l0.p290 -> l0.p292
    l0.p292 -> l1.p291
    l1.p291 -> l4.p293
    l4.p293 -> l4.p294
}
```

### Phase 289: Smooth Ground-Plane-Recessed Lighting & Seamless Periodic Water (`URBS-SMOOTH-LIGHT-01..04`)
- **Pre-Building Ground-Plane Bounce Illumination**: Render district warm brazier/forge bounce pools on the terrace ground plane *before* 3D building walls and roofs are painted so buildings sit *on* illuminated stone and occlude light naturally.
- **2D-Feathered Elliptical Sun-Shafts**: Replace hard-edged polygon god-rays with rotated multi-stop elliptical radial gradients whose alpha reaches `0.0` around the entire perimeter (`stop 1.0 = rgba(..., 0)`), eliminating visible strip edges.
- **Eliminate Hard-Edged Pulsing Circles**: Replace the solid `ctx.arc` district pulse in `drawAnimatedWorldEffects` with soft emissive window/archway rim-lighting and localized ground-plane torch flicker.
- **Periodic Seamless `seaWater` Synthesis**: Use `2π`-periodic trigonometric wave interference (`Math.sin(2 * Math.PI * x / size)`) in `pbrMaterials.ts` so the `64×64` water texture tiles with zero boundary discontinuity.

### Phase 290: High-Detail Terrace Grounds, `Cardo`/`Decumanus` Streets, `Opus Sectile` Plazas & `Substructio` Cliff Arcades (`URBS-GROUNDS-01..04`)
- **Orthogonal `Cardo` & `Decumanus` Street Network**: Carve recessed basalt-paved Roman thoroughfares with raised travertine curbstones (`Crepido`) and stepping-stone crosswalks across every district terrace.
- **Central `Opus Sectile` Marble Forum Plaza**: Frame each District Wonder with a sunken geometric marble pavement featuring concentric travertine/porphyry borders and corner bronze brazier plinths.
- **Vaulted `Substructio` Retaining Cliff Arcades**: Articulate district cliff retaining walls with recessed blind-arch niches, projecting stone buttress pilasters, and carved drainage weep-spouts.
- **Perimeter Stone Balustrades & Cypress/Olive Horti**: Add carved stone parapet coping, corner boundary obelisks (`Termini`), and manicured cypress/olive planter beds along terrace margins.

### Phase 291: High-Detail Vitruvian Building Architecture (`T0`–`T3` Articulation) (`URBS-ARCH-DETAIL-01..04`)
- **Ground-Floor Rusticated Arcades (`Fornices`) & Keystones**: Add recessed arched shopfront/loggia openings along the ground stories of `T0` Tabernae and `T2` Insulae with warm interior lamplight.
- **Inter-Story Stringcourse Belt Cornices (`Marcapiano`) & Quoins**: Articulate multi-story `T1`/`T2`/`T3` facades with projecting horizontal stone belt courses and corner ashlar quoining.
- **Recessed Atrium `Impluvium` Courtyards (`T1` Domus)**: Render a sunken rainwater basin (`Impluvium`) with sky-blue water reflection inside the L-shaped courtyard of every `T1` Domus.
- **Sculpted Rooftop `Acroteria`, Ridge Cresting & Hypocaust Chimneys**: Crown pitched terracotta roofs with golden-ochre gable `Acroteria` finials, stepped tympanum mouldings, and brick hypocaust vent flues.

### Phase 292: Integrated `Aquaeductus`, `Castellum Divisorium`, `Fistulae` Supply Plumbing & `Cloaca Maxima` Sewer System (`hydraulicNetwork.ts`) (`URBS-HYDRAULIC-01..04`)
- **End-to-End Aqueduct Trunk Integration**: Connect the peripheral `AQVAE MCP` multi-tier aqueduct bridge directly into the upper terrace's `Castellum Divisorium` (Main Hydraulic Distribution Reservoir) and branch raised `Specus` stone channels into adjacent districts.
- **Pressurized `Fistulae` Supply Conduits & Bronze Valves (`Epistomia`)**: Route isometric twin-rail bronze/lead supply pipes along terrace street curbs from district hydraulic headers to active file worksites, complete with bronze junction manifolds (`Calices`) and pressure gauge wheels.
- **Recessed `Cloaca Maxima` Sewer Trenches & Inspection Grates**: Carve stone-lined drainage channels with circular bronze manhole grates (`Spiracula`) along district streets to carry build runoff and error backpressure.
- **Cliff-Wall `Cloaca` Sea Outfalls & Sluice Cascades (`Cataracta`)**: Terminate lower-terrace sewer and aqueduct channels in arched stone `Cloaca Maxima` portals carved into the sea-cliff base, discharging animated water/error plumes into the coastal surf.

### Phase 293: Multi-Channel Color + Shape Animated Activity Conduits (`URBS-CONDUIT-ANIM-01..04`)
To make concurrent activities in progress immediately legible without hovering, every active conduit in `hydraulicNetwork.ts` combines a **physical Roman conduit style**, a **semantic color palette**, and a **distinct animated geometric particle shape**:

| Conduit Class | Activity / Telemetry Source | Physical Roman Structure | Color Channel | Animated Particle Shape |
|---|---|---|---|---|
| **`AQUA PURA`** (`supply`) | `AQVAE MCP` & Upstream Provider Crates (`⬆ PROVIDER`) | Raised stone `Specus` channel & cerulean `Fistula` pipe | Crystalline Cyan (`#38bdf8` → `#e0f2fe`) | **Chevron Wave-Crests (`❯❯`)** & water droplets |
| **`IGNIS OPERIS`** (`work_in_progress`) | Active Agent (`LEGIO`) → Target Worksite (`parser.rs`, etc.) | Riveted bronze hypocaust duct with junction collars | Molten Imperial Gold (`#f59e0b` → `#fef08a`) | **Diamond / Rhombus Ingots (`◆◆`)** |
| **`CLOACA FAULT`** (`error_backpressure`) | Diagnostic Error Worksite (`🔥 ERR`) → Downstream Blast Radius (`⚡`) | Recessed `Cloaca` sewer trench with hazard bars & sluice gate | Crimson Vermilion (`#ef4444` → `#fca5a5`) | **Warning Triangles (`▲`)** & turbulent eddies (`◎`) |
| **`VIA VICTORIAE`** (`verified_commit`) | Clean Build / Passing Tests → `CASTRVM CI` & `PORTA GIT` | Inlaid marble & verdigris brass triumphal channel | Emerald Verdigris (`#10b981` → `#a7f3d0`) | **Circular Laurel Seals (`◉`)** |

### Phase 294: Semiotic Consistency Audit, Unit Test Suite & Playwright CV Flywheel Convergence (`URBS-SEMIOTICS-CV-01..04`)
- Replace the floating screen-space yellow polygon arrow (`drawTacticalErrorDispatchOverlay`) and ungrounded center-to-center lines with diegetic **Roman Hydraulic Sluice-Gate Callouts & Isometric Conduit Routing** so graphics and actions remain 100% consistent with the Roman engineering theme.
- Add comprehensive unit tests in `crates/vox-gui/ui/src/components/gamify/urbs/__tests__/hydraulicNetwork.test.ts` and update `pbrMaterials.test.ts` and `lightingAndFx.test.ts`.
- Verify full convergence with `playwright test e2e/review/cv-flywheel.spec.ts`.
