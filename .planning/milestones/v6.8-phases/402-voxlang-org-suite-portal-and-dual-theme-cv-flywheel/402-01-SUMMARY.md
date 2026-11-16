---
phase: 402-voxlang-org-suite-portal-and-dual-theme-cv-flywheel
plan: 01
subsystem: docs-and-gui
tags: [voxlang.org, vox-axis, cv-flywheel, dual-theme, wcag-contrast, ssot]
completed: 2026-11-10
---

# Phase 402 Plan 01 Summary: `voxlang.org` Suite Portal Showcase, Dual-Theme Inline Verification & CV Flywheel Zero-Contrast Gate

## Accomplishments

1. **Upgraded `voxlang.org` Frontpage (`docs/src/index.mdx`) & Axis Workbench Guide (`docs/src/how-to/how-to-axis-workbench.md`) as the Suite Entry Point:**
   - Added the **Vox Language Suite Tour** (`vox_language_suite_tour.webp`) and two-lane execution architecture plate (`vox_two_lanes_architecture.webp`).
   - Added a dedicated **Vox Axis (`vox-gui`) Native Tauri 2 Workbench & `VRBS ROMA` Simulator** section featuring `axis_hero_tour.webp`, `axis_dual_mode_architecture.webp`, `axis_five_lanes_diagram.webp`, and a 4-card operator showcase (`VRBS ROMA` 3D/2.5D Simulator, `Imperium Prefecture` Blast-Radius Inspector, Multi-Dock Chat + Posture Steering, and `AQVAEDUCTUS` 3-Tier Hydraulic Cascade).
   - Added **Inline Dual-Theme Verification** cards comparing **Sunlit Imperial Carrara White & Gold (`arcane`)** and **Minimalist Grey-Blue Slate (`slate`)** side by side.
   - Added the **4-Repository Ecosystem Separation of Concerns** table (`vox-foundation/vox`, `vox-foundation/vox-axis`, `vox-foundation/voxlang.org`, `vox-ops`) maintaining strict SSOT across all 4 repositories.
2. **Resolved All `axe-core` & WCAG 2.1 AA Contrast Issues Across `vox-gui` and `docs-astro`:**
   - Normalized light-mode (`arcane`) and dark-mode (`slate`) text/background contrast in `crates/vox-gui/ui/src/index.css` and `AgentTrajectoryBraid.tsx`.
   - Verified `e2e/review/cv-flywheel.spec.ts`: **7/7 passed** across all 32 `SURFACE_REGISTRY` surfaces (`axeViolations: 0`, `urbs-civilization` `visualAfterScore: 0.9557`, `chat-dock-multi` `visualAfterScore: 0.9330`, `mens` `visualAfterScore: 0.9279`).
   - Verified `docs-astro` build (`919 page(s) built`, `All internal links are valid`) and Playwright `tests/suite-portal-showcase.spec.ts` + `tests/reader-layout.spec.ts`: **15/15 passed** with zero contrast failures in both `dark` and `light` themes, and visually inspected all inline screenshots (`suite-portal-hero-{dark,light}.png`, `suite-portal-axis-showcase-{dark,light}.png`, `suite-portal-dual-theme-{dark,light}.png`).
