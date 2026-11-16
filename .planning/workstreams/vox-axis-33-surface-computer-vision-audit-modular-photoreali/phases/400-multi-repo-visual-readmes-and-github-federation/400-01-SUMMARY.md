---
phase: 400-multi-repo-visual-readmes-and-github-federation
plan: 01
subsystem: multi-repo-documentation-and-github-federation
tags: [readme, vox-axis, voxlang-org, vox-ops, webp-tours, raster-diagrams, coolify, github-federation]
completed: 2026-11-10
---

# Phase 400 Plan 01 Summary: Multi-Repo Rich Visual READMEs, Animated WebP Tours, Real Generated Raster Diagrams, Cross-Repo Agent Awareness, Coolify & `vox-foundation` GitHub Federation

## Accomplishments

1. **Rich Visual Tour of All 32 `vox-axis` Screens + 3 Shell Modes + 5 Socrates HITL Stages (`README-VISUAL-01`)**:
   - Authored `/Users/brbrainerd/dev/vox-axis/README.md` (and synced `/Users/brbrainerd/dev/vox/crates/vox-gui/README.md`) modeled on `/Users/brbrainerd/dev/vox/README.md` (honest 4-state `Status` tables, `Tradeoffs` table, append-only `Log`, and narrative walkthrough).
   - Embedded **46 real Playwright screenshots** (`assets/screens/*.png`) covering all **32 registered `SURFACE_REGISTRY` views**, 3 shell modes (`_sidebar-wide.png`, `_sidebar-rail.png`, `_command-palette.png`), and 5 Socrates HITL review stages (`hitl-01-inbox-stage.png`..`hitl-05-digest-stage.png`), explaining **why each screen exists** and **what it can do**.
2. **7 Looping Animated `.webp` Tours & 6 Real Generated Raster Architecture Graphics — Zero SVGs (`README-VISUAL-02`)**:
   - Generated **7 looping animated `.webp` tour files** (`1440×900`, `img2webp`) across `vox-axis` (`axis_hero_tour.webp`, `axis_operate_lane_tour.webp`, `axis_develop_lane_tour.webp`, `axis_knowledge_hitl_tour.webp`, `axis_compute_system_tour.webp`), `voxlang.org` (`voxlang_site_tour.webp`), and `vox-ops` (`vox_ops_fleet_tour.webp`).
   - Generated **6 high-resolution raster architecture graphics** via `image-generator` subagents (`axis_hero_banner.webp`, `axis_dual_mode_architecture.webp`, `axis_five_lanes_diagram.webp`, `axis_trust_tiers_diagram.webp`, `voxlang_pipeline_diagram.webp`, `vox_ops_sovereignty_diagram.webp`), converted to `.webp` via `cwebp -q 88`, and removed all `.svg` diagram files per user directive (`dont use svgs generate real graphics`).
3. **Coolify (`eval.voxlang.org`, `telemetry.voxlang.org`) & Cloudflare Pages (`voxlang.org`) Multi-Repo Bindings (`README-VISUAL-03`)**:
   - Authored `/Users/brbrainerd/dev/voxlang.org/README.md` (synced to `/Users/brbrainerd/dev/vox/docs-astro/README.md`) and `/Users/brbrainerd/dev/vox-ops/README.md`.
   - Synced `vox-eval.compose.yml` into `/Users/brbrainerd/dev/voxlang.org/vox-eval.compose.yml` (`eval.voxlang.org` `/v1/eval` sandbox) and `docker/vox-telemetry.compose.yml` into `/Users/brbrainerd/dev/vox-ops/docker/vox-telemetry.compose.yml` (`telemetry.voxlang.org`).
4. **Cross-Repo Agent Documentation Awareness & `vox-foundation` GitHub Federation (`README-VISUAL-04`)**:
   - Updated `/Users/brbrainerd/dev/AGENTS.md`, `/Users/brbrainerd/dev/vox/AGENTS.md`, `/Users/brbrainerd/dev/vox-axis/AGENTS.md`, `/Users/brbrainerd/dev/voxlang.org/AGENTS.md`, and `/Users/brbrainerd/dev/vox-ops/AGENTS.md` (with `CLAUDE.md -> AGENTS.md` symlinks) so any agent working in any repository knows the cross-repo documentation structure, Coolify/Cloudflare bindings, and mandatory rules for keeping READMEs, docs, and screenshots up to date.
   - Configured and pushed public repositories under **`vox-foundation`** (`vox-foundation/vox`, `vox-foundation/vox-axis`, `vox-foundation/voxlang.org`).
