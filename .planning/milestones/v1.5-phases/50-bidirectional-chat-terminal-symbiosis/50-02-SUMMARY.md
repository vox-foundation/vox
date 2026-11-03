---
phase: 50-bidirectional-chat-terminal-symbiosis-security-levels-l1-l5
plan: "02"
subsystem: vox-gui
tags: [console, security-levels, control-mode, terminal-bridge]
provides:
  - terminalBridge.ts shared state bus between Console and Loquela/Chat
  - term-security-level-select (L1..L5) and term-control-mode-pill (Manual | Co-Pilot | Autopilot) in Console.tsx
key-files:
  created:
    - crates/vox-gui/ui/src/lib/terminalBridge.ts
  modified:
    - crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/Console.test.tsx
completed: 2026-10-15
---

# Phase 50 Plan 02: Console L1..L5 Security Selector & Control Handoff Pill Summary

## Accomplishments
- Created [`crates/vox-gui/ui/src/lib/terminalBridge.ts`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/lib/terminalBridge.ts) to synchronize active terminal session, blocks, pinned block IDs, `L1..L5` security level, and `Manual | Co-Pilot | Autopilot` control mode with `Loquela`.
- Added `term-security-level-select` and `term-control-mode-pill` to [`Console.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx) with automatic `Autopilot -> Manual` drop on user submission.

## Verification
- `pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/Console.test.tsx` -> `6 passed; 0 failed`.
