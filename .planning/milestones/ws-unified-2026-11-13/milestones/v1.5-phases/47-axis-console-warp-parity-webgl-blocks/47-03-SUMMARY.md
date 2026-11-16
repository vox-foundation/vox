---
phase: 47-axis-console-warp-parity-webgl-blocks
plan: 03
subsystem: gui-ui
tags: [terminal, warp-blocks, block-stream, console]
provides:
  - BlockStreamView.tsx Warp-style interactive command block cards
  - Console.tsx integration with Copy cmd, Copy output, Re-run, Pin, and Ask Agent actions
affects: [vox-gui]
key-files:
  created:
    - crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.test.tsx
  modified:
    - crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/Console.test.tsx
requirements-completed: [TERM-07]
completed: 2026-10-14
---

# Phase 47 Plan 03: Warp-Style Interactive BlockStreamView Summary

Created `BlockStreamView.tsx` rendering Warp-style interactive `Block` cards (`BlockStatus` pill, exit code, duration, `user` vs `agent` origin badge, Copy cmd, Copy output, Re-run, Pin, and Ask Agent) and wired it into `Console.tsx`.

## Verification

- RED: `pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/BlockStreamView.test.tsx src/components/surfaces/Console/Console.test.tsx` failed with missing `./BlockStreamView` and missing `block-stream-view`.
- GREEN: `pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/BlockStreamView.test.tsx src/components/surfaces/Console/Console.test.tsx` — 5 passed.
