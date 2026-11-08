---
title: "Romanesque White Carrara Marble, Fluted Pillars, Typography & Animation UI-SPEC (2026)"
description: "GSD UI-SPEC contract locking luminous white/cream Carrara marble friezes, fluted Ionic pillars, Cinzel/Palatino Romanesque typography, 3D riveted brass buttons, and tasteful animations across all 32 Vox Axis surfaces."
category: "Architecture SSOTs"
status: "current"
topic: "desktop-app"
domain: "gui"
authorship: "llm-assisted"
authorship_model: "claude-opus-4-6"
---

# Romanesque White Carrara Marble, Fluted Pillars, Typography & Animation UI-SPEC (2026)

<!-- kb:summary -->
Defines the Phase 362–367 GSD UI-SPEC design contract for Vox Axis, resolving the visual gap between the Plate 2 Romanesque Concept Art plates and the live 32-surface GUI. Locks luminous white and cream Carrara marble bas-relief headers, fluted Ionic marble pillar pilasters, sunlit travertine card slabs, authentic Cinzel and Palatino Romanesque typography across all surfaces, 3D riveted brass and chiseled marble buttons, large analog semicircular dial gauges, semiotic codebase city architecture, and interactive Roman aqueduct LLM/orchestrator hydraulics.
<!-- /kb:summary -->

> [!NOTE]
> Extended in Milestone `v6.2` (`Phases 363–367`): see [`semiotic-codebase-and-aqueduct-orchestrator-simulator-research-2026.md`](semiotic-codebase-and-aqueduct-orchestrator-simulator-research-2026.md) for the November 2026 survey of software city semiotics (*CodeCharta*, *CodeCity*), visual multi-agent orchestrators (*LangGraph Studio*, *AutoGen Studio*, *Rivet*, *Arize Phoenix*), and hydraulic city-builder UX mechanics (*Anno 117: Pax Romana*, *Timberborn*, *Caesar III*).

## 1. Visual Diagnosis & Root-Cause Gap Analysis

Comparing the approved Plate 2 Concept Art plates (`v61_concept_compute_and_market_1794141010387.jpg`, `v61_concept_workspace_and_commands_1794141011579.jpg`, and `v61_concept_knowledge_explorer_urbs_refined_1794141055158.jpg`) against the Pass 3 screenshots identified five concrete gaps:

1. **Missing White/Cream Carrara Marble & Sunlit Travertine Elegance:**
   - The Concept Art achieves its unmistakable classical richness through high-contrast pairing: a dark antique-bronze outer shell (`#14100B`) framing **luminous white/cream veined Carrara marble bas-relief frieze headers (`#F4EFE6` → `#E4D8C3`)** and **warm sunlit travertine/marble tablets (`#EFE7D8` / `#E6DBC8`)**.
   - Previously, `<RomanSurfaceHeader>` overlaid a `92%` opaque black gradient over `marble-frieze-banner.jpg` (whose center was already dark obsidian), turning headers and cards into uniform dark brown-black (`#16120C`).
2. **Missing Fluted Ionic/Corinthian Marble Pillars & Bas-Relief Sculpture:**
   - The Concept Art frames headers and surface panels with **vertical fluted white marble columns/pilasters** with carved acanthus capitals and bases, plus white marble bas-relief senators/scholars.
3. **Missing Romanesque Fonts (`Cinzel` & Classical Roman Serif) on Most Pages:**
   - `crates/vox-gui/ui/src/styles/fonts.css` declared `@font-face { font-family: 'Cinzel'; font-weight: 600; }` for weight `600` only, while `crates/vox-gui/ui/src/index.css` omitted `'Cinzel'` from `--font-display` and forced `--font-sans` (`Inter`) onto `html, body, #root`, buttons, spans, and divs.
4. **Underwhelming Buttons, Gauges & Symbols:**
   - Concept Art buttons are **3D Riveted Burnished Brass Plates** (metallic gold bevel, 4 corner rivets, dark engraved `Cinzel` label) and **Chiseled Cream Marble Tablets**.
   - Concept Art gauges are **large physical analog semicircular dials (`72px × 44px`)** with cream marble dial faces, engraved tick marks, brass bezels, and animated sweeping needles.
5. **VLM Flywheel Blind Spot:**
   - `extractChromaticSpatialVisionSignals` previously rewarded dark brown (`#16120C`) as "warm" without checking for **luminous white/cream Carrara marble pixels (`ivoryMarbleLuminanceRatio`)**, **fluted marble pillars (`flutedPillarCount`)**, **Romanesque font family coverage (`romanFontCoverageRatio`)**, or **active CSS/SVG animations (`activeAnimationCount`)**.

---

## 2. Locked Design System Contract (`Phase 362`)

### 2.1 Chromatic & Materiality Palette
| Surface Layer | Material | Hex / Gradient Token | Text / Ink Color |
| :--- | :--- | :--- | :--- |
| **Outer Shell & Left Rail** | Antique Obsidian Bronze | `#120E0A` → `#1B150E` | `#F3E5AB` (Gilded Cream) / `#D4AF37` (Brass) |
| **Frieze Header (`<RomanSurfaceHeader>`)** | White/Cream Carrara Marble Bas-Relief | `url(/assets/romanesque/white-marble-frieze-banner.jpg)` with luminous ivory veil `rgba(245,239,228,0.82)` | `#1E1710` (Deep Engraved Roman Ink) & `#5C3A10` (Bronze Subhead) |
| **Fluted Pillar Pilasters** | Carved White/Cream Ionic Marble Column | `linear-gradient(90deg, #D8CCB8 0%, #F7F2E8 28%, #DED2BE 50%, #F5EFE4 75%, #C9BBA3 100%)` | `#8C6825` (Capital Trim) |
| **Surface Cards & Tables** | Warm Veined Carrara Marble & Travertine | Cream Marble Header Bars (`#EBE1D0` with `#231C14` ink) + Warm Travertine Parapet Cards (`#211A12` / `#2A2117` with cream marble accents) | `#F5EFE4` / `#1E1710` |
| **Primary Action Buttons** | 3D Riveted Burnished Brass Plate | `linear-gradient(180deg, #F0D57A 0%, #D4AF37 48%, #9A741C 100%)` + 4 brass corner rivets | `#1A1206` (`Cinzel` bold engraved) |
| **Secondary / Tab Buttons** | Chiseled Cream Carrara Marble Tablet | `linear-gradient(180deg, #F6F0E4 0%, #E2D5BE 100%)` (active/pill) or `#282015` (inset) | `#231C14` / `#F3E5AB` |

### 2.2 Typography Contract (`fonts.css` & `index.css`)
- **Display / Headings / Buttons / Tabs / Column Headers (`--font-display`):**
  - `'Cinzel', 'Palatino', 'Palatino Linotype', 'Book Antiqua', Georgia, serif` (`@font-face` weight range `100 900`).
  - Applied globally to `h1, h2, h3, h4, h5, h6, button, [role="tab"], [role="columnheader"], th, nav, header, footer, [data-roman-frieze="true"]`.
- **Body / Prose / Table Cells / Cards (`--font-sans` & `--font-serif`):**
  - `'Palatino', 'Palatino Linotype', 'Book Antiqua', 'Georgia', serif` — classical Roman humanist book serif that is crisp and effortless to read for English-only speakers (no U→V substitutions).
- **Code / Diffs / Terminal (`--font-mono`):**
  - `'JetBrains Mono', 'SF Mono', 'Menlo', ui-monospace, monospace` — strictly scoped to `pre, code, .cm-editor, .cm-content`.

### 2.3 Tasteful Classical Animation Contract
1. **`vox-marble-sheen` (`7s infinite ease-in-out`)**: Soft diagonal sunlight shimmer gliding across the white Carrara marble bas-relief frieze and brass bezels.
2. **`vox-needle-sweep` & `vox-dial-breathe` (`3.2s infinite ease-in-out`)**: Analog semicircular brass gauge needles smoothly sweep to their live ratio and subtly oscillate (`±1.8 deg`) like a live mechanical pressure instrument.
3. **`vox-brass-glint` (hover + `4.5s` ambient)**: Specular highlight across 3D riveted brass buttons and embossed bronze medallions.
4. **`vox-tablet-rise` (`260ms cubic-bezier(0.22, 1, 0.36, 1)`)**: Tactile stone-tablet elevation when panels and cards mount.

### 2.4 Upgraded VLM Computer Vision Flywheel Gates (`vlmVisionFlywheel.ts`)
- **`ivoryMarbleLuminanceRatio >= 0.06`**: Every surface must exhibit genuine luminous white/cream Carrara marble & sunlit travertine highlights (`R >= 185, G >= 170, B >= 145`).
- **`flutedPillarCount >= 2`**: Every surface must render sculpted fluted Ionic marble pillars (`[data-roman-pillar="true"]`).
- **`romanFontCoverageRatio >= 0.85`**: At least 85% of visible headings, buttons, and tabs must compute to `Cinzel` or `Palatino` / Roman serif (`Inter` regression is flagged).
- **`activeAnimationCount >= 2`**: Every surface must have active CSS/SVG animations running (`vox-marble-sheen`, `vox-dial-breathe`, `vox-brass-glint`).
