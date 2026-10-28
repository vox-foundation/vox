---
phase: 47-axis-console-warp-parity-webgl-blocks
plan: 02
subsystem: gui-ui
tags: [terminal, xterm, webgl, session-host]
provides:
  - WebglTerminalAddon with onContextLoss DOM fallback and renderTermBlockForAgent
  - SessionHost attach + snapshot replay in TerminalTab.tsx
  - Retirement of duplicate frontend osc633.ts parser
affects: [vox-gui]
key-files:
  created:
    - crates/vox-gui/ui/src/components/surfaces/Console/webglAddon.ts
  modified:
    - crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.test.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/Console.test.tsx
requirements-completed: [TERM-06]
completed: 2026-10-14
---

# Phase 47 Plan 02: TerminalTab SessionHost Replay & WebGL Addon Summary

Upgraded `TerminalTab.tsx` to attach to `SessionHost` (`termSessionOpen` + `termSessionAttach`), replay `replay_utf8` and `TermBlockDto[]` on mount, load `WebglTerminalAddon` (`webglAddon.ts`) with `onContextLoss` DOM fallback, and retired the duplicate frontend `osc633.ts` parser.

## Verification

- RED: `pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/TerminalTab.test.tsx` failed with `Failed to resolve import "./webglAddon"`.
- GREEN: `pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/TerminalTab.test.tsx src/components/surfaces/Console/Console.test.tsx` — 8 passed.
