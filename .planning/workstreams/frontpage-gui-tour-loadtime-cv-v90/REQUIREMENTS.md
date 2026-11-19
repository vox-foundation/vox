# Requirements: Workstream `frontpage-gui-tour-loadtime-cv-v90` (`v9.0`)

| ID | Category | Requirement | Phase | Status |
|---|---|---|---|---|
| `PERF-01` | Performance | Measure reproducible Before vs After frontpage (`/`) load-time, `LCP`, `CLS`, request count, and byte-transfer metrics via Playwright + CDP (`10 Mbps` simulated WAN + unthrottled localhost) | 533 | `[x]` |
| `PERF-02` | Performance | Eliminate eager below-the-fold image downloads, preload hero LCP banner with `fetchpriority="high"`, and set exact intrinsic `width`/`height` across all frontpage images (`CLS = 0`) | 533 | `[x]` |
| `PERF-03` | Performance | Optimize bloated WebP assets (`vox_language_suite_tour.webp` `< 450 KB`, `vox_hero_banner.webp` `< 120 KB`, `axis_hero_tour.webp` `< 450 KB`, static cards `< 110 KB` each) benefiting both `voxlang.org` and `README.md` | 533 | `[x]` |
| `PERF-04` | Reporting | Generate a graphical Before/After load-time & byte-budget comparison plate (`docs/src/assets/voxlang-frontpage-loadtime-before-after.webp`) and SSOT doc (`docs/src/architecture/voxlang-frontpage-perf-and-gui-tour-cv-ssot-2026.md`) | 533 | `[x]` |
| `TOUR-01` | GUI Tour | Refresh `urbs-civilization.webp` and `urbs-imperium-inspector.webp` at `16:10` (`1440x900`) with `LIVE` status bar and zero clipped file-list rows | 533 | `[x]` |
| `TOUR-02` | GUI Tour | Refresh `chat-multidock.webp` and `harness.webp` with clean formatted assistant prose, non-wrapped composer prompt, and untruncated braid task titles | 533 | `[x]` |
| `TOUR-03` | GUI Tour | Refresh `research-arcane.webp` and `memory-arcane.webp` fixing the right-clipped DAG node, toast overlay, boilerplate citations, and `example.com` fixture | 533 | `[x]` |
| `TOUR-04` | GUI Tour | Refresh `doc-reader-arcane.webp` and `publications-arcane.webp` with non-sliced dock content and diverse scholarly titles across all 7 lifecycle stages | 533 | `[x]` |
| `TOUR-05` | GUI Tour | Refresh `theme-arcane.webp`, `theme-slate.webp`, and animated `axis_hero_tour.webp` reflecting current `33 Views` and `AXIS STUDIO` chrome | 533 | `[x]` |
| `TOUR-06` | CV Flywheel | Verify zero cropped/stretched images, zero clipped interactive elements, `axeViolations: 0`, and WCAG `>= 4.5:1` contrast across `dark`/`light` themes and desktop/mobile viewports | 533 | `[x]` |
| `UI-ALIGN-01` | UI Layout | Eliminate Starlight sibling top-margin leaks (`not-content`) and unify heights across `VoxPlayground`, `VoxLeaderboardArena`, hero buttons, and `.vox-progressive-tiers` CTAs (`<= 2px` row/button delta) | 533.1 | `[x]` |
| `UI-ALIGN-02` | UI Layout | Equalize same-row card heights across `.vox-progressive-tiers`, `.vox-showcase-grid`, and `.card-grid` (`align-items: stretch; height: 100%;`) on frontpage and tutorials | 533.1 | `[x]` |
| `TOUR-CAROUSEL-01` | GUI Carousel | Replace buried 10-card showcase stack on `/` with an early interactive user-navigable `<VoxGuiTourCarousel />` above `Why Vox` supporting Prev/Next, 9 tabs, keyboard navigation, and Light/Dark toggle | 533.1 | `[x]` |
| `DOCS-UTIL-01` | Content Utility | Deduplicate redundant text diagrams under WebP infographics and trim marketing prose on `index.mdx`, `how-to-axis-workbench.md`, and flagship tutorials while preserving `SYNC-FROM-README: why_vox` and audit invariants | 533.1 | `[x]` |
| `DOCS-UTIL-02` | Tutorial Assets | Re-capture clean `LIVE` `vox-search-arcane.webp`, `discovery-arcane.webp` (`33-surface`), and unclipped `scientia-arcane.webp` (`16:10`), removing toast overlays and `OFFLINE` chrome | 533.1 | `[x]` |
| `CV-TUT-01` | CV Flywheel | Extend `suite-portal-showcase.spec.ts` with bounding-box alignment (`<= 2px`), carousel state navigation, WCAG `>= 4.5:1` contrast, and visual review captures across `/`, workbench guide, and tutorials | 533.1 | `[x]` |
