# Phase 471 Context: Async `Notify` Non-Blocking PTY Host, `120 Hz` IPC Frame Coalescing & Cross-Platform Shell/CWD Parity

**Workstream:** `axis-terminal-sota-v80`
**Phase:** 471
**Requirements:** `TERM-PERF-01`, `TERM-PERF-02`, `TERM-PERF-03`, `TERM-PERF-04`, `TERM-XPLAT-01`, `TERM-XPLAT-02`, `TERM-XPLAT-03`, `TERM-XPLAT-04`
**SSOT:** [`terminal-surfaces-audit-and-benchmarks-research-2026.md`](../../../../docs/src/architecture/terminal-surfaces-audit-and-benchmarks-research-2026.md)

## Decisions
1. Replace `Condvar::wait_timeout` in `LocalHost::submit_with_origin` and `20 ms` `POLL_STEP_MS` polling in `exec_agent_command` with `tokio::sync::Notify` and `tokio::select!`.
2. Add chunk coalescing (`COALESCE_MAX_BYTES = 32 * 1024`) in `crates/vox-gui/src/commands/pty/warp.rs` (`spawn_term_forwarder`) to avoid saturating Tauri WebView IPC during high-throughput SGR bursts.
3. Prevent speculative Vox compiler parsing on bare shell inputs (`intentRouter.ts`, `input.rs`, `host.rs`) and route POSIX binary paths (`/usr/bin/env`, `/bin/echo hi`) to `InputIntent::Shell`.
4. Thread live session `cwd` into out-of-PTY capture (`run_shell_capture_in_cwd`, `run_nu_structured_capture_in_cwd`), adapt `format_agent_command_for_shell` across `pwsh`/`nu`/`cmd`/POSIX when `!osc633_capable`, and dispatch `vox-term` headless `run_plain_with_host`.
