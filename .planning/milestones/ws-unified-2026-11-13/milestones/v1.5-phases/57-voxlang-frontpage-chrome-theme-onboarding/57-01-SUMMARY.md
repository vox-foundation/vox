---
phase: 57-voxlang-frontpage-chrome-theme-onboarding
plan: 01
type: execute
completed_at: 2026-10-14T11:55:00Z
files_modified:
  - docs-astro/src/components/Sidebar.astro
  - docs-astro/src/components/PageSidebar.astro
  - docs-astro/src/components/TableOfContents.astro
  - docs-astro/src/components/Banner.astro
  - docs-astro/src/components/VoxPlayground.astro
  - docs-astro/src/styles/reader-layout.css
  - docs-astro/astro.config.mjs
  - docs/src/index.mdx
  - README.md
  - docs-astro/tests/reader-layout.spec.ts
  - docs-astro/tests/unit/reader-layout.test.mjs
  - docs-astro/tests/trust-signals.spec.ts
  - crates/vox-gui/ui/src/components/layout/Sidebar.tsx
  - crates/vox-gui/ui/src/components/layout/AppShell.tsx
  - crates/vox-gui/ui/src/components/layout/Sidebar.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.tsx
requirements-completed:
  - CHROME-01
  - CHROME-02
  - CHROME-03
  - CHROME-04
  - CHROME-05
  - ONBOARD-01
  - ONBOARD-02
  - ONBOARD-03
---

# Phase 57 Plan 01–05 Summary: `voxlang.org` Visual Theme, Interactive Chrome, Universal Sidebar Hiders & Inspirational Frontpage

## Overview

Resolved all broken sidebar/TOC hiders on `voxlang.org` and made every sidebar across `voxlang.org` and Vox Axis GUI completely hideable with themed, tested buttons (`CHROME-01..05`). Replaced Starlight's drab default purple palette with the Vox brand design system (electric cyan `#38bdf8`, obsidian slate `#0b0f17`, warm amber `#f59e0b`), upgraded `VoxPlayground.astro` into a 3-tab interactive explorer, and crafted an inspirational human-first "Why Vox" narrative across `README.md` and `docs/src/index.mdx` (`ONBOARD-01..03`).

## Key Changes

1. **Universal Hideable Sidebars Across `voxlang.org` & Vox Axis GUI (`CHROME-01`, `CHROME-02`, `CHROME-05`)**:
   - [`docs-astro/src/components/Sidebar.astro`](file:///Users/brbrainerd/dev/vox/docs-astro/src/components/Sidebar.astro) & [`Banner.astro`](file:///Users/brbrainerd/dev/vox/docs-astro/src/components/Banner.astro): Added `#vox-sidebar-hide-btn` alongside `#vox-sidebar-pin-toggle` and mounted the floating `#vox-sidebar-restore-btn` pill outside `.sidebar-pane` so the left navigation sidebar can be collapsed to a 56px rail OR completely hidden (`data-sidebar-state="hidden"`) and restored with one click.
   - [`docs-astro/src/styles/reader-layout.css`](file:///Users/brbrainerd/dev/vox/docs-astro/src/styles/reader-layout.css): Hid `.sidebar-content > *:not(.vox-sidebar-toggle-container)` when collapsed, added zero-width collapse for both `html[data-sidebar-state="hidden"] .sidebar-pane` and `html[data-toc-state="hidden"] aside.right-sidebar`, and expanded `--sl-content-width` to `60rem` / `74rem`.
   - [`crates/vox-gui/ui/src/components/layout/Sidebar.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/layout/Sidebar.tsx), [`AppShell.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/layout/AppShell.tsx), & [`DiscoveryRail.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.tsx): Extended `SidebarMode` with `'hidden'` (`0px` width) and added themed `sidebar-hide-button` / `sidebar-restore-button` controls so the Axis GUI navigation sidebar and Console discovery rail are hideable and restorable.
2. **Vox Brand Theme & Multi-Tab Playground (`CHROME-03`, `CHROME-04`)**:
   - Replaced default Starlight purple CSS variables with the Vox dark/light brand palette in [`docs-astro/src/styles/reader-layout.css`](file:///Users/brbrainerd/dev/vox/docs-astro/src/styles/reader-layout.css).
   - Upgraded [`docs-astro/src/components/VoxPlayground.astro`](file:///Users/brbrainerd/dev/vox/docs-astro/src/components/VoxPlayground.astro) into a 3-tab interactive explorer (`1. Database + AI Tool`, `2. No Null / Safe Errors`, `3. Durable Workflow`) with a Copy button and deterministic offline/online execution preview.
3. **Inspirational "Why Vox" Narrative & Beginner Frontpage (`ONBOARD-01`, `ONBOARD-02`, `ONBOARD-03`)**:
   - Rewrote the hero pitch and `why_vox` synced section in [`README.md`](file:///Users/brbrainerd/dev/vox/README.md) and [`docs/src/index.mdx`](file:///Users/brbrainerd/dev/vox/docs/src/index.mdx) around *"Software Spoken in One Clear Voice"* — explaining the human and AI creative *why* before progressive technical stack details while keeping 1:1 `why_vox` anchor parity.

## Verification

- `cargo run -p vox-doc-pipeline -- --lint-only --paths README.md docs/src/index.mdx`: 0 errors.
- `node --test docs-astro/tests/unit/reader-layout.test.mjs`: passed.
- `pnpm --dir crates/vox-gui/ui exec vitest run src/components/layout/Sidebar.test.tsx src/components/surfaces/Console/DiscoveryRail.test.tsx`: passed.
- `pnpm --dir docs-astro exec playwright test tests/reader-layout.spec.ts`: passed.
