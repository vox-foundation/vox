# Phase 47: Axis Console Warp-Parity Terminal, WebGL Renderer & Block Chrome (`Task H6`) - Context

**Gathered:** 2026-10-14
**Status:** Ready for execution
**Mode:** Autonomous (`/gsd-autonomous`)

<domain>
## Phase Boundary

Complete the Axis GUI Console cutover (`Task H6` of `docs/src/architecture/terminal-session-host-design-2026.md`) and Warp-parity block chrome:
- `47-01` (`TERM-05`): Cut `crates/vox-gui/src/commands/pty.rs` over to `SessionHost` (`DaemonHost` over `PersistentDaemon` + `LocalHost` fallback), expose `term_session_open/list/attach/input/submit/resize/close` Tauri commands, retire `crates/vox-gui/src/commands/terminal_core.rs`, and wire `crates/vox-gui/ui/src/transport.ts`.
- `47-02` (`TERM-06`): Cut `TerminalTab.tsx` over to `termSessionOpen`/`termSessionAttach` snapshot replay + live `vox://term-output`/`vox://term-event` streams, add `WebglTerminalAddon` (`webglAddon.ts`) with `onContextLoss` DOM fallback, and retire `osc633.ts` (`+ osc633.test.ts`).
- `47-03` (`TERM-07`): Create `BlockStreamView.tsx` rendering Warp-style interactive `Block` cards (`BlockStatus` pill, exit code, duration, `user`/`agent` origin badge, Copy cmd, Copy output, Re-run, Pin, Ask Agent) and integrate into `Console.tsx`.
- `47-04` (`TERM-08`): Add multi-session tab bar (`termSessionList` / `termSessionOpen`) and "continue in vox term" (`vox term --attach <id>`) clipboard handoff to `Console.tsx`, plus E2E mock parity in `operatorShellMock.ts` and `tauriMock.ts`.

</domain>

<decisions>
## Implementation Decisions

- **Zero duplicate OSC-633 state machine**: `crates/vox-gui/src/commands/terminal_core.rs` and `crates/vox-gui/ui/src/components/surfaces/Console/osc633.ts` are retired; `vox-terminal-core` is the single source of truth for blocks.
- **Self-contained WebGL2 addon with DOM fallback**: `webglAddon.ts` implements xterm's `ITerminalAddon` with `onContextLoss` fallback to DOM rendering and zero external network installs.

</decisions>
