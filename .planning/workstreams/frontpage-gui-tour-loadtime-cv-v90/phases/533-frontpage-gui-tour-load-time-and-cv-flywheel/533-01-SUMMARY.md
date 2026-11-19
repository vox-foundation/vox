# Phase 533 Plan 01 Summary: Frontpage GUI Tour Refresh, Load-Time Optimization & CV Flywheel

**Status:** Complete (`2026-11-19`)  
**Workstream:** `frontpage-gui-tour-loadtime-cv-v90` (`v9.0`)

## Delivered Outcomes

### 1. Vox Axis UI Defect Fixes & 13 Fresh `16:10` Captures
Fixed four UI/accessibility defects before recapturing all frontpage and workbench screenshots at `1440×900` (`16:10`, downsampled cleanly to `1280×800` WebP):
- [`crates/vox-gui/ui/src/components/surfaces/Research/ResearchDagCanvas.tsx`](../../crates/vox-gui/ui/src/components/surfaces/Research/ResearchDagCanvas.tsx): Replaced `min-w-[500px]` with `w-full h-auto max-w-full` and clamped height so the rightmost Epistemic DAG node (`c3`) renders unclipped.
- [`crates/vox-gui/ui/src/components/surfaces/Chat/ArenaComparisonCard.tsx`](../../crates/vox-gui/ui/src/components/surfaces/Chat/ArenaComparisonCard.tsx): Added inline `**...**` bold span rendering (`BOLD_SPAN_RE`, `hasBold`) so raw markdown asterisks do not leak in assistant bubbles.
- [`crates/vox-gui/ui/src/components/gamify/urbs/ImperiumInspector.tsx`](../../crates/vox-gui/ui/src/components/gamify/urbs/ImperiumInspector.tsx): Adjusted impacted file list scroll container (`max-h-28` -> `max-h-[120px]`) so 4 rows render without mid-row slicing.
- [`crates/vox-gui/ui/src/components/layout/SessionSidebarSection.tsx`](../../crates/vox-gui/ui/src/components/layout/SessionSidebarSection.tsx) + [`crates/vox-gui/ui/e2e/session-rail-actions.spec.ts`](../../crates/vox-gui/ui/e2e/session-rail-actions.spec.ts): Moved `role="tablist"` inside `SessionRow` around `[role="tab"]` with `data-testid` targeting so action buttons are no longer nested inside interactive tabs (`axeViolations: 0`).
- [`crates/vox-gui/ui/e2e/review/frontpage-tour-capture.spec.ts`](../../crates/vox-gui/ui/e2e/review/frontpage-tour-capture.spec.ts): Captured all 13 surfaces (`urbs-civilization`, `urbs-imperium-inspector`, `chat-multidock`, `harness`, `research-arcane`, `research-slate`, `memory-arcane`, `memory-slate`, `doc-reader-arcane`, `doc-reader-slate`, `publications-arcane`, `theme-arcane`, `theme-slate`) with active `vox://orch-status` `LIVE` pulse, `0` horizontal overflow, and `0` `axe-core` violations.

### 2. Asset Compression & Animated Tour Optimization
- Encoded 13 refreshed `1280×800` WebP surfaces (`68.5 KiB` – `168.5 KiB`) under `docs/src/assets/axis/`.
- Rebuilt animated `docs/src/assets/axis/axis_hero_tour.webp` (7 representative `960×600` frames via `img2webp -min_size -lossy -q 70 -m 6`: `1,124,818 B` -> `423,884 B` / `413.9 KiB`) and synced it to `crates/vox-gui/assets/tours/axis_hero_tour.webp`.
- Rebuilt animated `docs/src/assets/vox_language_suite_tour.webp` (5 `1280×720` frames via `img2webp -min_size -lossy -q 76 -m 6`: `4,738,896 B` -> `385,286 B` / `376.3 KiB`, `-91.9%`), accelerating both `voxlang.org` and `README.md`.
- Extracted static Frame 1 (`1344×444`) from the 25-frame animated `vox_hero_banner.webp` (`832,228 B` -> `88,760 B` / `86.7 KiB`, `-89.3%`), plus lossy-optimized `schema-fragmentation.webp` (`152,346 B` -> `98,458 B`) and `vox_two_lanes_architecture.webp` (`198,776 B` -> `113,296 B`).

### 3. Frontpage, Shared Docs, CSS Framing & Reporting
- Updated [`docs/src/index.mdx`](../../docs/src/index.mdx): Added Starlight YAML `head:` preload (`rel="preload" as="image" href="./assets/vox_hero_banner.webp" fetchpriority="high"`), hero `width="1344" height="444" fetchpriority="high" decoding="async"`, `loading="lazy" decoding="async"` and matched intrinsic dimensions on all 15 below-fold images, updated stale `v9.0` / `33` surfaces / `424` MCP / `56` BLAKE3 / `VoxDb (v99)` / `60-row` `2026-11-18` counts, and cross-linked the new SSOT doc.
- Updated [`README.md`](../../README.md) (`why_vox` image attributes synced to `width="760" height="424" loading="lazy" decoding="async"`, plus line 8 `33 catalog-generated GUI surfaces`), [`docs/src/how-to/how-to-axis-workbench.md`](../../docs/src/how-to/how-to-axis-workbench.md) (`33` surfaces with `Operate (12 views): ..., gsd`, `v99`, `axis_hero_tour.webp` `960×600`, `axis_dual_mode_architecture.webp` `1376×768`, `decoding="async"`), and [`docs-astro/src/styles/reader-layout.css`](../../docs-astro/src/styles/reader-layout.css) (`object-fit: contain;` on showcase frames).
- Created [`docs-astro/tests/frontpage-perf-and-cv-flywheel.spec.ts`](../../docs-astro/tests/frontpage-perf-and-cv-flywheel.spec.ts), rendered [`docs/src/assets/voxlang-frontpage-loadtime-before-after.webp`](../../docs/src/assets/voxlang-frontpage-loadtime-before-after.webp), and authored [`docs/src/architecture/voxlang-frontpage-perf-and-gui-tour-cv-ssot-2026.md`](../../docs/src/architecture/voxlang-frontpage-perf-and-gui-tour-cv-ssot-2026.md).
