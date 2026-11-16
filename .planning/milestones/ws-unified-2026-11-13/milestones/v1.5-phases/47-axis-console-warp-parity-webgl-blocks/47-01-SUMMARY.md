---
phase: 47-axis-console-warp-parity-webgl-blocks
plan: 01
subsystem: gui
tags: [terminal, session-host, tauri, transport]
provides:
  - SessionHost-backed term_session_* Tauri commands in crates/vox-gui/src/commands/pty.rs
  - Typed termSession* IPC wrappers and event listeners in crates/vox-gui/ui/src/transport.ts
  - Retirement of crates/vox-gui/src/commands/terminal_core.rs
affects: [vox-gui, vox-terminal-core]
key-files:
  modified:
    - crates/vox-gui/src/commands/pty.rs
    - crates/vox-gui/src/commands/mod.rs
    - crates/vox-gui/src/main.rs
    - crates/vox-gui/src/remote/manifest.rs
    - contracts/gui/command-backends.v1.yaml
    - crates/vox-gui/ui/src/transport.ts
    - crates/vox-gui/ui/src/transport.console.test.ts
    - crates/vox-gui/ui/src/guards/commandBackendsMigration.test.ts
requirements-completed: [TERM-05]
completed: 2026-10-14
---

# Phase 47 Plan 01: Axis GUI SessionHost Backend Cutover Summary

Cut over `crates/vox-gui/src/commands/pty.rs` to `vox_terminal_core::SessionHost` (`DaemonHost` over `PersistentDaemon` with `LocalHost` fallback), retired `crates/vox-gui/src/commands/terminal_core.rs`, and added typed `termSession*` IPC wrappers and `listenTermOutput` / `listenTermEvent` listeners in `crates/vox-gui/ui/src/transport.ts`.

## Verification

- RED:
  - `pnpm --dir crates/vox-gui/ui exec vitest run src/transport.console.test.ts` failed with `TypeError: termSessionOpen is not a function`.
  - `cargo test -p vox-gui commands::pty::tests` failed with missing `resolve_local_host` and `term_attachment_dto_from_parts`.
- GREEN:
  - `cargo test -p vox-gui --bin vox-gui commands::pty::tests` — 10 passed.
  - `cargo test -p vox-gui --bin vox-gui remote::manifest::tests` — 8 passed.
  - `pnpm --dir crates/vox-gui/ui exec vitest run src/transport.console.test.ts src/guards/commandBackendsMigration.test.ts src/guards/transportIpcGuard.test.ts` — 15 passed.
