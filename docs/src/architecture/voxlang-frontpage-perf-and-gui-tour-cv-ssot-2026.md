---
title: "Frontpage Load-Time Optimization, GUI Tour Refresh & CV Flywheel (v9.0)"
description: "Measured Before vs. After frontpage load-time, Chrome DevTools Protocol wire payload, WebP asset compression, and Computer Vision / Accessibility Flywheel verification for voxlang.org and Vox Axis."
category: "Architecture SSOTs"
status: "current"
topic: "web-frontend"
domain: "gui"
authorship: "llm-assisted"
authorship_model: "claude-opus-4-6"
---

# Frontpage Load-Time Optimization, GUI Tour Refresh & CV Flywheel (v9.0)

<!-- kb:summary -->
Milestone v9.0 (Phases 533–533.2) refreshes all 16 Vox Axis 16:10 GUI screenshots and animated WebP tours, fixes four source layout and ARIA defects, promotes an early interactive 9-surface carousel on voxlang.org, enforces <= 2px button and card row alignment, adds hero preload with fetchpriority="high", cuts 10 Mbps WAN window.load latency by 94.5% (5,824.3 ms to 318.8 ms), and reduces full-scroll image payload by 90.5% (8.66 MiB to 839.5 KiB) with CLS = 0 and zero axe violations.
<!-- /kb:summary -->

<div class="vox-media-frame">
  <img src="/assets/voxlang-frontpage-loadtime-before-after.webp" alt="Before vs. After frontpage load time, Largest Contentful Paint, eager image payload, and CV Flywheel audit comparison across 10 Mbps WAN and localhost preview runs" width="1440" height="860" loading="lazy" decoding="async" />
  <div class="vox-media-caption">
    <strong>Phase 533 Frontpage Benchmark &amp; CV Flywheel Comparison</strong> — <em>Median of 3 cold uncached Chromium runs via Playwright + Chrome DevTools Protocol (`10 Mbps` down / `5 Mbps` up / `40 ms` RTT + unthrottled localhost) on [voxlang.org](../index.mdx).</em>
  </div>
</div>

## 1. Reproducible Before vs. After Frontpage Telemetry

Measurements were captured before and after asset optimization and HTML attribute updates using Playwright (`docs-astro/tests/frontpage-perf-and-cv-flywheel.spec.ts`) over an uncached `1440×900` Chromium viewport on built static output (`pnpm --dir docs-astro build`).

### Simulated 10 Mbps WAN (`40 ms` RTT, Cache Disabled — Median of 3 Runs)

| Metric | Before (`PERF_STAGE=before`) | After (`PERF_STAGE=after`) | Change |
|---|---:|---:|---:|
| **`window.load` Event (`loadEventMs`)** | `5,824.3 ms` | **`318.8 ms`** | **-94.5%** (`18.3×` faster) |
| **Largest Contentful Paint (`lcpMs`)** | `2,436.0 ms` | **`320.0 ms`** | **-86.9%** (`7.6×` faster) |
| **Initial Eager Image Transfer (`eagerImageBytes`)** | `7,048,418 B` (`6.72 MiB`) | **`89,028 B` (`86.9 KiB`)** | **-98.7%** |
| **Initial Total Page Transfer (`eagerTotalBytes`)** | `7,181,106 B` (`6.85 MiB`) | **`221,799 B` (`216.6 KiB`)** | **-96.9%** |
| **Full-Scroll Image Footprint (`totalScrolledImageBytes`)** | `9,081,540 B` (`8.66 MiB`) | **`2,144,976 B` (`2.05 MiB`)** | **-76.4%** |
| **Full-Scroll Total Transfer (`totalScrolledTransferBytes`)** | `9,214,228 B` (`8.79 MiB`) | **`2,277,747 B` (`2.17 MiB`)** | **-75.3%** |
| **Initial Eager Network Requests (`eagerRequestCount`)** | `20` | **`16`** | **-20.0%** |
| **`DOMContentLoaded` (`domContentLoadedMs`)** | `275.8 ms` | `312.3 ms` | `+36.5 ms` *(hero preload priority)* |
| **First Contentful Paint (`fcpMs`)** | `132.0 ms` | `156.0 ms` | `+24.0 ms` *(preload scheduling)* |
| **Post-Scroll CLS (`clsAfterScroll`)** | `0` | **`0`** | `0` *(no layout shift)* |
| **Hero `<link rel="preload" as="image">`** | `0` | **`1`** | Enabled |

Unthrottled localhost medians shifted from `fcpMs: 28 / lcpMs: 112 / loadEventMs: 97` (when lazy decoding deferred work) to `fcpMs: 76 / lcpMs: 208 / loadEventMs: 221.8` due to synchronous high-priority hero decode without network throttling. Under realistic `10 Mbps` WAN conditions, prioritizing and compressing the `1344×444` hero LCP image cut user-visible `LCP` by `2,116 ms` and `window.load` by `5,505.5 ms`.

## 2. Asset Pipeline & Payload Budgets

Root bottlenecks in the `before` snapshot stemmed from three sources: (1) five un-lazy-loaded frontpage images downloading `6.72 MiB` eagerly at startup, (2) `vox_language_suite_tour.webp` shipped lossless at `4,738,896 B` (`4.52 MiB`), and (3) `vox_hero_banner.webp` encoded as a `25`-frame animated WebP (`812.7 KiB`) blocking initial LCP.

| Asset | Before | After | Optimization Applied |
|---|---:|---:|---|
| `docs/src/assets/vox_hero_banner.webp` (`1344×444`) | `832,228 B` (`812.7 KiB`) | **`88,760 B` (`86.7 KiB`)** | Extracted frame 1 static WebP (`-q 76 -alpha_q 85 -m 6 -sharp_yuv`), `<link rel="preload">` + `fetchpriority="high"` |
| `docs/src/assets/vox_language_suite_tour.webp` (`1280×720`, 5f) | `4,738,896 B` (`4.52 MiB`) | **`385,286 B` (`376.3 KiB`)** | Rebuilt via `img2webp -min_size -lossy -q 76 -m 6` (`-91.9%`, speeds up both `voxlang.org` and `README.md`) |
| `docs/src/assets/axis/axis_hero_tour.webp` (`960×600`, 7f) | `1,124,818 B` (`1.07 MiB`) | **`423,884 B` (`413.9 KiB`)** | Rebuilt from 7 fresh `16:10` captures (`urbs-civilization`, `urbs-imperium-inspector`, `chat-multidock`, `harness`, `research-arcane`, `doc-reader-arcane`, `theme-slate`); synced to `crates/vox-gui/assets/tours/` |
| `docs/src/assets/schema-fragmentation.webp` (`1376×768`) | `152,346 B` (`148.8 KiB`) | **`98,458 B` (`96.2 KiB`)** | Lossy `-q 80 -m 6 -sharp_yuv`, added `width="760" height="424" loading="lazy" decoding="async"` across `index.mdx` and `README.md` (`why_vox` parity) |
| `docs/src/assets/vox_two_lanes_architecture.webp` (`1376×768`) | `198,776 B` (`194.1 KiB`) | **`113,296 B` (`110.6 KiB`)** | Lossy `-q 64 -m 6 -sharp_yuv`, fixed `width="1376" height="768"` attribute aspect match (`attrAspectDrift: 0`) |
| **13 Refreshed `docs/src/assets/axis/*.webp` (`1280×800`)** | Legacy screenshots | **`68.5 KiB` – `168.5 KiB`** | Downsampled from `1440×900` Playwright captures via `cwebp -resize 1280 800 -q 80 -m 6 -sharp_yuv` |

## 3. Vox Axis GUI Bugfixes & Fresh `16:10` Captures

Before regenerating the 13 Vox Axis captures (`urbs-civilization`, `urbs-imperium-inspector`, `chat-multidock`, `harness`, `research-arcane`, `research-slate`, `memory-arcane`, `memory-slate`, `doc-reader-arcane`, `doc-reader-slate`, `publications-arcane`, `theme-arcane`, `theme-slate`), Computer Vision inspection uncovered four UI/accessibility defects that were fixed at the source in `crates/vox-gui/ui/`:

1. **`ResearchDagCanvas.tsx`**: Removed fixed `min-w-[500px]` in favor of `className="w-full h-auto max-w-full"`, eliminating right-edge clipping of `c3` DAG nodes inside the multi-column research card at `1440×900`.
2. **`ArenaComparisonCard.tsx`**: Added inline `**...**` bold span rendering (`BOLD_SPAN_RE` + `hasBold` handling in `renderStructuredMessageText`) so raw markdown asterisks no longer leak into chat bubbles.
3. **`ImperiumInspector.tsx`**: Increased inspector list height ceiling (`max-h-28` -> `max-h-[120px]`) so 4 impacted file rows display without mid-row clipping.
4. **`SessionSidebarSection.tsx`**: Moved `role="tablist"` inside each `SessionRow` wrapping only the interactive `[role="tab"]` button while keeping badge/Archive siblings outside, bringing all 13 captures to `axeViolations: 0` with `outerOverflowPx: 0` and active `LIVE` status bar telemetry (see [Using the Vox Axis Desktop Workbench](../how-to/how-to-axis-workbench.md)).

## 4. CV & Accessibility Flywheel Gates

Every built page passes:
- **`frontpage-perf-and-cv-flywheel.spec.ts` (`PERF_STAGE=after`)**: Enforces `eagerImageBytes < 200,000`, `eagerTotalBytes < 500,000`, `clsAfterScroll <= 0.015`, `preloadCount >= 1`, `objectFit !== 'cover'` (with `.vox-home-banner-frame img, .vox-media-frame img, .vox-showcase-card img` locked to `object-fit: contain`), explicit `width` + `height` attributes (`aspectDrift <= 0.03`, `attrAspectDrift <= 0.03`), and zero horizontal overflow across desktop (`1440×900`) and mobile (`390×844`) under both `light` and `dark` Starlight themes.
- **Documentation & Repository Gates**: Validated via `vox-doc-pipeline --lint-only`, `docs_production_audit_e2e`, `vox ci check-links`, `node --test`, `suite-portal-showcase.spec.ts`, `v89-quality.spec.ts`, and `reader-layout.spec.ts`.

## 5. Phase 533.1 Addendum: Interactive GUI Carousel, Alignment Fixes & Tutorial CV Pass

- **Starlight `margin-top` leak & uneven row fixes:** Added `not-content` to `VoxPlayground.astro` and `VoxLeaderboardArena.astro`, unified button flexbox alignment (`height`, `line-height`, `margin: 0 !important`), stretched `.vox-progressive-tiers`, `.vox-showcase-grid`, and Starlight `.card-grid` row siblings (`align-items: stretch; height: 100%;`), and added equalized `.vox-tier-btn-emerald` CTAs — enforced via `<= 2px` bounding-box row checks across `/`, workbench, research, and KB tutorials in `suite-portal-showcase.spec.ts`.
- **Early navigable GUI tour carousel (`<VoxGuiTourCarousel />`):** Promoted above `#why-vox` right below `<VoxPlayground />` with 9 interactive tabs, `← Prev` / `Next →` controls, `ArrowLeft`/`ArrowRight` keybinds, and Light/Dark Slate theme switching over the refreshed `1280×800` captures, replacing the redundant bottom 10-card stack.
- **Content deduplication & refreshed tutorial captures:** Removed redundant `vox-diagram` blocks below matching WebP infographics (`index.mdx`, `tut-deep-research-and-scientia.mdx`, `tut-knowledgebase-and-openwiki.mdx`), trimmed repetitive prose, updated `32 -> 33` view references, and re-captured clean `LIVE` `vox-search-arcane.webp` (`67.6 KiB`, zero toast overlap), `discovery-arcane.webp` (`74.4 KiB`, `33-surface`), and `scientia-arcane.webp` (`69.8 KiB`, unclipped archive status).
- **Post-533.1 frontpage payload & Phase 533.2 code review (`CR-01..07`):** Replacing the buried 10-card showcase stack cut full-scroll frontpage image footprint from `2,144,976 B` to **`859,682 B` (`839.5 KiB`)**. `/gsd-code-review --fix` additionally removed hardcoded `/Users/...` test output fallbacks (`CR-01`), aligned tutorial JSON examples with `ResearchResult` / `PreflightReport` Rust structs (`CR-02`), corrected `--defect hallucinated_api`, numeric session ids, and boolean `--dry-run` CLI flags (`CR-03`, `CR-04`), added WAI-ARIA `tabpanel` linkage and unique `#vox-axis-workbench` / `#try-it` anchors (`CR-05`), and made `frontpage-tour-capture.spec.ts` fail closed on Axe runtime errors (`CR-06`).

## 6. Phase 533.3 Visual Audit, Anti-Redundancy & Readability Pass

- **Eliminated `11×` slogan repetition (`docs/src/index.mdx`, `VoxPlayground.astro`):** Removed duplicated "single `.vox` file gives SQL + Rust + React + MCP" descriptions across frontmatter, hero lead, hero alt text, onboarding cards, playground comments, and feature tables while preserving `{/* SYNC-FROM-README: why_vox */}` byte-for-byte with `README.md` as the sole explanation of 4-layer schema projection.
- **Desktop (`1280×800`) & (`1440×900`) layout and theme fixes:**
  - Added dark obsidian backdrop (`background: #070a12;`) on `.vox-home-banner-frame` in dark and light themes so the alpha-feathered `vox_hero_banner.webp` renders cleanly without a grey halo in Light Mode.
  - Reduced `.vox-home-actions` to 3 primary CTAs (`Try Sandbox ↓`, `Explore GUI Tour ↓`, `5-Min Quickstart →`) so buttons never wrap `2+2` at `1280px`.
  - Switched `.vox-progressive-tiers` to `repeat(3, minmax(0, 1fr))` at `@media (min-width: 1024px)` with `white-space: nowrap` inline command chips and button labels, fixing the `2+1` orphan card wrap at `1280px`.
  - Shortened `VoxPlayground.astro` preset pills and target tabs (`Runtime`, `SQL (Turso)`, `Rust (Axum)`, `TSX & MCP`) and grouped `VoxGuiTourCarousel.astro` controls into a dedicated `.carousel-topbar` above `.carousel-tabs`, eliminating tab label clipping (`scrollWidth <= clientWidth + 2`).
- **Compacted near-fold carousel image & updated telemetry (`PERF_STAGE=after`):** Compressing `urbs-civilization.webp` (`960×600`, `83,170 B` / `81.2 KiB`) brought post-compaction eager initial transfer (hero + near-fold first carousel slide) to `172,466 B` (`168.4 KiB`, `< 200,000 B` budget) and full-scroll image footprint to **`770,314 B` (`752.3 KiB`, `-91.5%` vs. `8.66 MiB` baseline)** with `lcpMs: 316 ms`, `loadEventMs: 442.1 ms`, `clsAfterScroll: 0`, and `axeViolations: 0`. Also replaced `'google/gemini-2.5-pro'` with family key `'google/gemini-pro'` in `crates/vox-gui/ui/e2e/review/frontpage-tour-capture.spec.ts` to satisfy `noVersionedModelIds.test.ts`.

## 7. v9.3 Playground, HD Captures, Subsystems Carousel & Editorial Rewrite

- **Playground output is real.** `VoxPlayground.astro` no longer synthesises SQL, Rust and TSX with a regex emitter. Its output is read from `docs-astro/src/data/playground/<example>/`, captured with `vox build` and `vox run --interp` (see the README in that directory). The earlier page showed a hook named `useVoxQuery`, which the compiler never generates; the real pieces are the typed fetch function in `vox-client.ts` and the optional `useVoxServerQuery` in `vox-tanstack-query.tsx`. Edited code shows a banner saying the browser cannot run the compiler. The old first example did not pass `vox build` ("async main cannot return a value") and was replaced.
- **One output tab strip** (Run output, SQL, Rust server, TS client, React UI, MCP tools) replaced the two overlapping strips. A fourth example shows the Vox UI vocabulary (VUV).
- **HD, two-theme captures.** `frontpage-tour-capture.spec.ts` takes `VOX_TOUR_DSF=2` (2880x1800 frames) and captures a dark (`-slate`) variant of every frame. `docs/src/assets/axis/hd/<id>-<light|dark>-<1280|2560>.webp` holds the encoded set (`cwebp -q 82`). `VoxCarousel.astro` fails the build if a captured slide lacks any of the four files.
- **Two carousels, one component.** `VoxCarousel.astro` renders both the Axis tour and the core-subsystems tour from `docs-astro/src/data/carousels.ts`. Each slide states what a part is, why it is built that way, and what it means for AI-written code and for people. Terminal blocks are real command output; window captures use the harness sample data and say so.
- **Not yet measured:** the v9.0 payload budget (section 2) was not re-run after the HD set was added. The new images are lazy-loaded with `srcset`, but the full-scroll footprint has not been recomputed.
