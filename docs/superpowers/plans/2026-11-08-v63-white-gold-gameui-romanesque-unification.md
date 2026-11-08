# Milestone v6.3 — White/Gold Game-UI Romanesque Unification, Medallion Sidebar Rail, Zero-Void/Zero-Truncation Layout & Built-In VLM Vision Flywheel (`Phases 368–372`)

## Objective
Unify all 32 Vox Axis GUI surfaces around the approved White/Cream Carrara Marble, Sunlit Travertine Tablet (`#F6F1E6` / `#EDE3D1`), and Burnished Gold (`#C59B27` / `#8C6214`) Game-UI aesthetic (`v63_concept_gameui_romanesque_unified_suite_1794153259509.jpg`) **throughout the entire page body and sidebar** (not only at the top header), eliminate repetitive column pilasters, convert `Sidebar.tsx` into a `92px` Sculptural Bronze Medallion Rail on a White Carrara Marble plinth, purge all remaining IBM dark-terminal / neon green-blue colors, fix all multi-resolution (`1280×800`, `1440×900`, `1920×1080`) clipping/truncation and empty voids, and verify via upgraded VLM + Playwright Computer Vision Flywheel passes and direct built-in vision inspection.

## Tasks

### Phase 368: Built-In VLM Vision Audit & Structural/Chromatic Flywheel Upgrade (`[SEQUENTIAL]`)
- **Files**:
  - `crates/vox-gui/ui/e2e/review/vlmVisionFlywheel.ts`
  - `crates/vox-gui/ui/e2e/review/audits.ts`
  - `crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts`
- **Steps**:
  1. Map `v63_concept_gameui_romanesque_unified_suite_1794153259509.jpg` in `conceptArtPlateForSurface` for non-simulator surfaces and add `digitalGreenBluePixelRatio` + `parchmentTravertineSlabRatio` to `extractChromaticSpatialVisionSignals`.
  2. Add `clippedChildInOverflowHiddenCount` and `truncatedTextElementCount` checks to `audits.ts` so child clipping inside `overflow-hidden` containers and mid-word text truncation are caught automatically.

### Phase 369: 92px White-Marble & Bronze Medallion Game-UI Sidebar Rail, Column De-Duplication & Status Bar Fix (`[SEQUENTIAL]`)
- **Files**:
  - `crates/vox-gui/ui/src/components/layout/Sidebar.tsx`
  - `crates/vox-gui/ui/src/components/layout/Layout.tsx`
  - `crates/vox-gui/ui/src/components/layout/StatusBar.tsx`
  - `crates/vox-gui/ui/src/components/ui/RomanSurfaceHeader.tsx`
- **Steps**:
  1. Convert `Sidebar.tsx` from a `216px` dark SaaS list into a `92px` (`w-[92px]`) **White Carrara Marble & Burnished Bronze Medallion Game-UI Rail** (`white-marble-frieze-banner.jpg` vertical slab, `40px` embossed 3D bronze medallions with `Cinzel` labels below, golden vertical active torch bar), preserving all `data-testid` attributes and keyboard accessibility while reclaiming `124px` of horizontal viewport width.
  2. Remove the repetitive outer `vox-roman-column-frame` left/right column pilaster strips on `<main>` in `Layout.tsx` and replace the repetitive left/right SVG column icons in `RomanSurfaceHeader.tsx` with domain-varied Romanesque corner ornaments (laurel wreath, astrolabe ring, wax seal, or bevelled brass rosette) and larger semicircular `<RomanBrassGauge>` chronometer dials.
  3. Fix `ROUTING Auto → anthr` truncation in `StatusBar.tsx` and style `StatusBar.tsx` with a warm Carrara marble & brass architrave trim and zero neon green/cyan dots.

### Phase 370: Full-Body White/Cream Carrara Marble & Gold Tablet Theme (`index.css`) & Zero-Digital-Color Purge (`[SEQUENTIAL]`)
- **Files**:
  - `crates/vox-gui/ui/src/index.css`
- **Steps**:
  1. Upgrade `.vox-roman-main-canvas` in `index.css` so that below the top frieze, the page body, cards, tables, list rows, and panels render as **Luminous White/Cream Carrara Marble & Sunlit Travertine Stone Tablets (`#F6F1E6`, `#EDE3D1`, `/assets/romanesque/travertine-parchment-tablet.jpg`)** framed by burnished gold/brass moldings (`#C59B27` / `#8C6214`) with deep espresso ink (`#1C140C` / `#3B2B18`).
  2. Remap all digital `emerald-*`, `teal-*`, `cyan-*`, and `sky-*` utility classes inside `.vox-roman-main-canvas` and `.vox-roman-shell` to **Olive-Laurel Bronze (`#4D5D1E` / `#E9F0D1`)**, **Burnished Gold (`#8C6214` / `#FBF3DC`)**, and **Imperial Crimson Wax Seal (`#881B1B` / `#FCE8E6`)** so zero digital green or blue remains.

### Phase 371: Multi-Resolution (`1280×800`, `1440×900`, `1920×1080`) Zero-Clipping & Empty-Void Elimination (`[SEQUENTIAL]`)
- **Files**:
  - `crates/vox-gui/ui/src/components/gamify/LudusSandbox.tsx`
  - `crates/vox-gui/ui/src/components/surfaces/Harness/HarnessRiverPanel.tsx`
  - `crates/vox-gui/ui/src/components/surfaces/Compute/VoxPopuliView.tsx`
  - `crates/vox-gui/ui/src/components/surfaces/Mercatus/MercatusView.tsx`
  - `crates/vox-gui/ui/src/components/surfaces/Workspace/TerminalSurface.tsx`
- **Steps**:
  1. In `LudusSandbox.tsx`, unify the bottom `urbs-developer-action-dock` and `HudPanels` into a compact, non-overlapping bottom console bar (`left-2.5 right-[206px] bottom-2`) so it never clips at `1280×800` and never collides with `urbs-minimap-radar` (`right-2.5 bottom-2`), and compact the top architrave toolbar so `LENS & LAYOUT ▾` and `ALERTS & HISTORY ▾` stay on a single line at `1280×800`.
  2. In `HarnessRiverPanel.tsx`, make the vertical rhythm responsive (`gap-2`, canvas `h-[175px] xl:h-[235px]`) so `VerificationFunnelFooter` is 100% visible without bottom clipping at `1280×800`.
  3. Fill the empty bottom viewport voids in `VoxPopuliView.tsx`, `MercatusView.tsx`, and `TerminalSurface.tsx` with White/Gold Roman Game-UI Mesh Node Telemetry Tablets, Arbitrage Ledgers, and Riveted Brass Console Bezels.

### Phase 372: Multi-Pass Built-In Vision + Playwright CV Flywheel Verification & Inline Before/After Presentation (`[SEQUENTIAL]`)
- **Steps**:
  1. Run `timeout 300s pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/cv-flywheel.spec.ts --project=chromium`.
  2. Inspect every captured `After` screenshot (`1280×800` and `1440×900`) with `read_file` (built-in vision) against `v63_concept_gameui_romanesque_unified_suite_1794153259509.jpg`.
  3. Run `timeout 300s pnpm --dir crates/vox-gui/ui build` and present the Concept Art + Before/After carousels inline.
