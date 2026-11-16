---
title: "Vox Axis v7.2 Visual Audit, Photorealistic Diegetic FX & Multi-Resolution CV Flywheel SSOT"
description: "Architecture and computer-vision audit contract for Vox Axis Milestone v7.2 covering all 33 surfaces, all 14 Chat docks, small-resolution layouts (900x600+), panel-filling telemetry charts, and the modular PhotorealisticCanvasFx compositing engine."
category: "Architecture SSOTs"
status: "current"
topic: "desktop-app"
domain: "gui"
authorship: "llm-assisted"
authorship_model: "gemini-3.1-pro"
---

# Vox Axis v7.2 Visual Audit, Photorealistic Diegetic FX & Multi-Resolution CV Flywheel SSOT

<!-- kb:summary -->
Defines the computer-vision audit findings, layout invariants, and modular rendering architecture for Vox Axis Milestone v7.2 across all 33 `SURFACE_REGISTRY` views and all 14 Chat dock panels. Establishes the reusable `PhotorealisticCanvasFx` compositing stage for perspective-integrated Roman legionaries, sunlit crepuscular rays, multi-octave water caustics, and torch pools; mandates edge-to-edge panel-filling SVG/canvas telemetry charts; and enforces zero-clipping responsive layouts down to `900x600`.
<!-- /kb:summary -->

## 1. Scope & Computer-Vision Defect Taxonomy

A multimodal computer-vision pass across all **33** `SURFACE_REGISTRY` surfaces (including `gsd`), all **14** `DockWorkspaceShell` panels, and 5 viewport classes (`900x600`, `1100x720`, `1280x800`, `1440x900`, `1920x1080`) identified five systemic visual defect families that escaped prior DOM-only checks:

| Defect Family | Root Cause | Affected Surfaces | v7.2 CV Metric Gate |
| --- | --- | --- | --- |
| **Under-filled Line Graphs & Braids** | Fixed `viewBox="0 0 520 72"` without `ResizeObserver` or width scaling (`~35%` fill) and `56px` corner sparklines (`~20%` fill). | `Harness` (`AgentTrajectoryBraid`), `Dashboard` (`Kpi`), `NeedsYou`, `Runs`, `Models`, `Memory`, `CodeRabbit` | `chartPanelFillRatio >= 0.84` |
| **Pasted-On Soldiers & Wireframe Ghosting** | Flat DOM `<RomanCitizenSvg />` stickers without perspective foreshortening or contact shadows; `0.58`-alpha procedural wireframe buildings drawn over painted artwork; letterboxed sidebands on wide `Dashboard` strips. | `Dashboard`, `Gamify` (`LudusSandbox`), `HarnessRiverPanel`, `AgentFlow` | Zero cartoon DOM stickers; full-width panorama cover; perspective `scale(v)` + contact shadow |
| **Static / Subtle Canvas FX** | Single-frame `animations: 'disabled'` audits never verified live frame-to-frame motion; water/ray layers lacked multi-octave contrast. | `Gamify` (`Urbs`), `Dashboard`, `Harness`, `AgentFlow`, `Oratio` | `animationMotionDeltaPct >= 0.15` (`animations: 'allow'`, 420ms delta) |
| **Small-Resolution Offsets & Dock Cutoffs** | Fixed `440px` transcript min-width, 3–4 column dock splits below `1100px`, 2-row wrapped `Loquela` toolbar in narrow docks, `184px` sidebar truncating `CONTRO..`, and `BottomStatusBar` clipping `SPE` mid-word. | `AppShell`, `Sidebar`, `BottomStatusBar`, `ChatSurface`, `Loquela`, `ChatTranscript`, `WaveformTierStrip`, all 14 docks | `outerOverflowPx == 0`, `clippedInteractiveCount == 0`, `statusBarClippedSegmentCount == 0`, `sidebarTruncatedLabelCount == 0`, `composerToolbarRowCount == 1` at `900x600`..`1920x1080` |
| **Raw Emojis, Backticks & ASCII Brackets** | Narrow `/🧍/g` regex missed `⚔`, `🌊`, `🔍`, `🏆`, `📌`, `🏛`, `🔬`, `😊` and `[Dispatch Task]` button brackets. | `LudusSandbox`, `CitizenSprite`, `AgentFlow`, `HarnessRiverPanel`, `Explorer`, `Research`, `ChatSurface`, `Loquela` | `rawEmojiCount == 0`, `bracketedLabelCount == 0`, `rawBacktickCount == 0` |

## 2. Modular Photorealistic Diegetic FX Engine (`crates/vox-gui/ui/src/components/fx/`)

All animated Roman architectural scenes share one composable, GPU-friendly 2D canvas FX library (`PhotorealisticCanvasFx.ts` + `<DiegeticFxStage />`) obeying `prefers-reduced-motion` and the 4-profile GUI theme contract (`contracts/gui/theme-contract.v1.json`):

1. **Full-Bleed Panorama Framing (`computeSafePanoramaRect`):** Covers `100%` of wide/embedded container width (`drawW >= cw`) anchored on the architectural horizon (`0.42` vertical focal point), eliminating letterboxed sky/stone sidebands on `Dashboard` and docked `Urbs` views while anchoring monument callouts to `panoRect` UV coordinates.
2. **Sunlit Volumetric God-Rays (`renderSunlitGodRays`):** Composite `screen` / `lighter` feathered crepuscular shafts drifting slowly across marble colonnades with warm dust motes catching sunlight.
3. **Multi-Octave Water Caustics & Buoyant Mist (`renderPhotorealWaterCausticsAndMist`):** Layered phase-shifted specular caustic ribbons along aqueduct channels and harbour basins paired with rising translucent waterfall mist plumes at cascade drops.
4. **Warm Flickering Torch & Brazier Pools (`renderDiegeticTorchAndBrazierPools`):** Radial amber/gold light pools (`screen` blend) with dual-sine flicker around active agent monuments and sluice gates.
5. **Perspective-Integrated Legionary & Citizen Procession (`renderPerspectiveIntegratedProcession`):** Replaces flat DOM cartoon SVGs with canvas-composited silhouettes scaled by horizon depth (`scale(v)`), grounded by soft directional contact shadows, lit by warm sunlit rim highlights, and blended with aerial-perspective haze along architectural walkways and bridges.
6. **Laminar Telemetry Filaments (`renderAtmosphericTelemetryPulse`):** High-visibility travelling golden/emerald energy pulses along `AgentFlow` DAG conduits, `AgentTrajectoryBraid` splines, and `Oratio` voice harmonic bands.

## 3. Responsive Shell, Chat & 14-Dock Layout Contract (`900x600` to `1920x1080`)

- **Adaptive Sidebar (`AppShell.tsx`, `Sidebar.tsx`):** Viewports narrower than `960px` default to the `64px` sculptural icon rail unless explicitly expanded in-session, reclaiming `+120px` horizontal workspace for Chat and Docks at `900x600`. Sidebar header badges use compact spacing with zero truncated brand text.
- **Zero-Clip Status Bar (`BottomStatusBar.tsx`):** Priority-tiered segment visibility hides secondary telemetry labels cleanly below `1280px` and `1024px` without mid-word scroll clipping.
- **Single-Row Container-Responsive Composer (`Loquela.tsx`):** `@container` rules collapse secondary pill captions (`Run on`, `Quick chat`, `@kb:auto`, cost readout) into compact pills when composer width `< 640px`, keeping `composerToolbarRowCount === 1` (`<= 48px`) across full-screen and docked layouts.
- **14-Panel Adaptive Docking (`ChatSurface.tsx`, `WaveformTierStrip.tsx`):** Transcript minimum width scales dynamically (`280px`..`440px`) with dock container width; secondary panels tabify into a single companion stack below `1100px`; `WaveformTierStrip` switches to a 1-column stack when panel width `< 560px`.

## 4. Verification & Multi-Pass CV Flywheel Convergence

Verification runs iteratively via `crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts` (`CV_STAGE=before` baseline followed by multi-pass `CV_STAGE=after` refinement) coupled with `tsc --noEmit` and Vitest (`themeContract.test.ts`, `cvFlywheel.test.ts`, `audits.test.ts`). Convergence requires every surface and resolution to satisfy `visualAfterScore >= 0.90`, `vlmOverallScore >= 0.88`, `chartPanelFillRatio >= 0.84`, positive live `animationMotionDeltaPct` on animated views, and zero overflow, clipping, emoji, backtick, bracket, or axe defects.

### Empirical Multi-Pass Convergence (`Pass 0` Baseline → `Pass 3` Golden Convergence)

| Surface / Resolution | Pass 0 (`Before`) | Pass 1 (`5/8` Suites) | Pass 2 (`7/8` Suites) | Pass 3 (`8/8` Green) | Key Measured Delta |
| --- | --- | --- | --- | --- | --- |
| `chat-multidock-900x600` (`compact`) | `0.8138` (`2` rows, `4` clips) | `0.9012` (`5` sb clips) | `0.9204` (`0` clips) | **`0.9208`** (`+0.1070`) | `1`-row composer, `64px` rail, `0` clipped segments |
| `chat-multidock-1100x720` (`laptop`) | `0.8209` (`2` rows, `3` clips) | `0.9185` | `0.9225` | **`0.9227`** (`+0.1018`) | Tabbed companion dock, `0` clips |
| `chat-dock-multi` (`1440x900`) | `0.8225` (`5` brackets, `2` emojis) | `0.9382` | `0.9396` | **`0.9400`** (`+0.1175`) | Clean `z-50` `Panels` popover, `0` brackets/emojis |
| `chat-default` / `chat` (`1440x900`) | `0.8327` (`vlm 0.854`) | `0.9256` (`vlm 0.8625`) | `0.9265` (`vlm 0.9535`) | **`0.9265`** (`vlm 0.9535`) | Romanesque frieze, medallion & brass gauge header |
| `loquela-knobs` (`1440x900`) | `0.8331` (upward overlap) | Failed (`y=612.5`) | `0.9268` (inline) | **`0.9268`** (`+0.0937`) | Unobstructed inline posture drawer below composer |
| `dashboard` (`1440x900`) | `0.9158` (`20.4%` chart fill) | `0.9479` | `0.9483` | **`0.9483`** (`vlm 0.9490`) | `96.3%` KPI chart fill, full-bleed map, `0.97%` live FX |
| `harness` (`1440x900`) | `0.9099` (`46.2%` braid fill) | `0.9412` | `0.9417` | **`0.9417`** (`vlm 0.9311`) | `99.3%` braid fill, `3.76%` live cascade/god-ray FX |
| `urbs-civilization` (`gamify`) | `0.8807` (cartoon stickers) | `0.9531` | `0.9537` | **`0.9537`** (`vlm 0.9204`) | Perspective legionaries + shadows, `3.14%` live FX |
| `flow` (`1440x900`) | `0.8518` (`4` emojis, `4` brackets) | `0.9369` | `0.9374` | **`0.9374`** (`vlm 0.9269`) | Laminar golden/emerald conduits, `1.58%` live FX |
| **All 33 Surfaces Mean** | **`0.9192`** | **`0.9296`** (`32/33`) | **`0.9306`** (`33/33`) | **`0.9308`** (`vlm 0.9461`) | **`33/33` surfaces, `14/14` docks, `0` defects** |
