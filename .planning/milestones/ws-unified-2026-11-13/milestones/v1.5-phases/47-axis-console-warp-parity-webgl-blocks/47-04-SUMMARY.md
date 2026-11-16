---
phase: 47-axis-console-warp-parity-webgl-blocks
plan: 04
subsystem: gui-ui
tags: [terminal, session-host, cli-handoff, e2e-mocks]
provides:
  - Multi-session tab bar / session picker in Console.tsx
  - continue in vox term CLI handoff button copying vox term --attach <id>
  - E2E mock parity for term_session_* commands in operatorShellMock.ts and tauriMock.ts
affects: [vox-gui]
key-files:
  modified:
    - crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/Console.test.tsx
    - crates/vox-gui/ui/e2e/lib/operatorShellMock.ts
    - crates/vox-gui/ui/e2e/lib/tauriMock.ts
requirements-completed: [TERM-08]
completed: 2026-10-14
---

# Phase 47 Plan 04: Session Picker & CLI Handoff Summary

Added a live multi-session tab bar (`termSessionList` / `termSessionOpen`) and a `continue in vox term` (`vox term --attach <id>`) clipboard handoff action to `Console.tsx`, and added `term_session_*` handlers to `operatorShellMock.ts` and `tauriMock.ts`.

## Verification

- RED: `pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/Console.test.tsx` failed with missing `term-session-tab-s-1`.
- GREEN:
  - `pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/ src/transport.console.test.ts src/guards/commandBackendsMigration.test.ts src/guards/transportIpcGuard.test.ts` — 11 test files, 47 tests passed.
  - `pnpm --dir crates/vox-gui/ui exec tsc --noEmit` — 0 errors.
