# Phase 471 Summary 01: Async PTY Notify, IPC Coalescing & Cross-Platform Parity

## Shipped Changes
- **`crates/vox-terminal-core/src/host.rs`:** Added `settled_notify: tokio::sync::Notify`, `wait_until_async`, `LocalHost::submit_async`, `submit_with_origin_async`, and fixed Rust 2024 `if let` `MutexGuard` temporary lifetimes.
- **`crates/vox-terminal-core/src/agent_exec.rs`:** Replaced `POLL_STEP_MS` sleep polling with race-free `tokio::select!` + `notified.as_mut().enable()` (`notify_waiters()` in EOF + `promote_bg`), plus shell-syntax-aware wrapping for `pwsh`/`powershell`, `nu`, `cmd`, and POSIX when `!osc633_capable`.
- **`crates/vox-gui/src/commands/pty/warp.rs`:** Added `COALESCE_MAX_BYTES = 32 * 1024` chunk coalescing inside `spawn_term_forwarder`.
- **`crates/vox-terminal-core/src/input.rs` & `crates/vox-gui/ui/src/components/surfaces/Console/intentRouter.ts`:** Routed POSIX binary paths (`/usr/bin/env`, `/bin/echo hi`) to `InputIntent::Shell` and skipped speculative Vox compilation in locked shell mode.
- **`crates/vox-terminal-core/src/pty.rs`, `session.rs` & `crates/vox-term/src/app.rs`:** Threaded `cwd` into `run_shell_capture_in_cwd`, added `submit_with_origin_and_level`, and wired headless `run_plain_with_host` dispatch.
