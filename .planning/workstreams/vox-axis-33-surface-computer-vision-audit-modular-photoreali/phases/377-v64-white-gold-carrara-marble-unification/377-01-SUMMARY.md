---
phase: 377
plan: 01
subsystem: vox-gui
tags: [romanesque, carrara-marble, urbs-simulator, typography, cv-flywheel]
one_liner: "Unified all 32 Vox Axis GUI surfaces into the full-body Sunlit White/Gold Carrara Marble aesthetic with painted Roman monument simulator, single-line controls, zero-cutoff ORCHESTRATOR medallions, and zero-monospace Palatino/Cinzel Roman typography."
key_files:
  created:
    - crates/vox-gui/ui/public/assets/romanesque/urbs-white-gold-citadel-bg.jpg
    - crates/vox-gui/ui/public/assets/romanesque/carrara-page-watermark-bg.jpg
    - crates/vox-gui/ui/public/assets/romanesque/roman-architectural-frieze-footer.jpg
    - crates/vox-gui/ui/public/assets/romanesque/monument-pantheon-orchestrator.jpg
    - crates/vox-gui/ui/public/assets/romanesque/monument-temple-compiler.jpg
    - crates/vox-gui/ui/public/assets/romanesque/monument-library-knowledge.jpg
    - crates/vox-gui/ui/public/assets/romanesque/monument-colosseum-harness.jpg
  modified:
    - crates/vox-gui/ui/src/index.css
    - crates/vox-gui/ui/src/main.tsx
    - crates/vox-gui/ui/src/App.tsx
    - crates/vox-gui/ui/vite.config.ts
    - crates/vox-gui/ui/style-dictionary.config.mjs
    - crates/vox-gui/ui/src/components/ui/Backdrop.tsx
    - crates/vox-gui/ui/src/components/ui/Glass.tsx
    - crates/vox-gui/ui/src/components/ui/Button.tsx
    - crates/vox-gui/ui/src/components/surfaces/Flow/AgentFlow.tsx
    - crates/vox-gui/ui/src/components/surfaces/Explorer/CapabilityBadge.tsx
    - crates/vox-gui/ui/src/components/surfaces/Explorer/ExplorerTree.tsx
    - crates/vox-gui/ui/src/components/surfaces/Explorer/ExplorerRail.tsx
    - crates/vox-gui/ui/src/components/surfaces/NeedsYou/NeedsYouSurface.tsx
    - crates/vox-gui/ui/src/components/gamify/LudusSandbox.tsx
    - crates/vox-gui/ui/src/components/gamify/urbs/worldRenderer.ts
---

# Phase 373–377 (Milestone v6.4) Summary

## Accomplishments
1. Enlarged `ORCHESTRATOR` circular hub medallion (`r=66` in `AgentFlow.tsx`) and widened `ORCHESTRATOR` DAG node (`width=136` in `NeedsYouSurface.tsx`) for zero text cutoff, attaching live file/folder paths (`crates/vox-orchestrator/src/router.rs`, `crates/vox-compiler/src/typeck`, `crates/vox-mens/src/qlora.rs`, `crates/vox-db/src/turso.rs`).
2. Made `OPEN FORM`, `EXPLAIN CAPABILITY`, `OPEN DOCS` buttons strictly single-line (`ExplorerTree.tsx`, `Button.tsx`) and slimmed `= GENERIC FORM` (`CapabilityBadge.tsx`) into a compact single-line Roman pill.
3. Replaced procedural 2D polygon building drawing in `worldRenderer.ts` with painted Roman Acropolis & Monument artwork (`urbs-white-gold-citadel-bg.jpg` + `monument-*.jpg` portraits in `LudusSandbox.tsx`), live active file/folder badges, and animated SVG golden aqueduct streams.
4. Converted all 32 surfaces (`Backdrop.tsx`, `Glass.tsx`, `index.css`) to the full-body Sunlit White/Cream Carrara Marble & Burnished Gold aesthetic with subtle Vitruvian architectural watermark background and sculpted Roman frieze footer.
5. Purged all Courier/Arial/Menlo monospace font fallbacks across CSS, SVG, and Canvas renderers in favor of `'Palatino', 'Palatino Linotype', 'Book Antiqua', 'EB Garamond', Georgia, serif` (`tabular-nums`) and `'Cinzel', 'Palatino', Georgia, serif`, and verified all 32 surfaces across `1280×800`, `1440×900`, and `1920×1080` via Playwright + `@ngi/platform-ssot` CV Flywheel (`0` `axeViolations`, `0` `clippedInteractiveCount`, `0` `overlappingControlPairs`, `0` bundler warnings).
