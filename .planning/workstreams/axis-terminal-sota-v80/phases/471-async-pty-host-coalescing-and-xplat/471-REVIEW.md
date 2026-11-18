# Phase 471 Code Review

**Scope:** `crates/vox-terminal-core/src/{host.rs,agent_exec.rs,input.rs,pty.rs,session.rs}`, `crates/vox-gui/src/commands/pty/warp.rs`, `crates/vox-gui/ui/src/components/surfaces/Console/intentRouter.ts`, `crates/vox-term/src/app.rs`
**Checks:** `cargo clippy --no-deps`, `toestub`, `surfaceHonesty.guard.test.ts`

## Findings
- **Warning (`clippy::let_and_return`) — `crates/vox-terminal-core/src/host.rs:880,921`:** Unnecessary `let open_id = { lock(&entry.session).open_block_id() }; open_id` wrapper flagged under `-D warnings`.
- **Info — `crates/vox-terminal-core/src/host.rs` & `pty.rs`:** Inline `do { ... } | to json --raw` formatting duplicated between `host.rs` and `pty.rs` without checking existing `| to json` tails.
- **Confirmed clean:** Non-blocking `Notify` wakeup pinning (`notified.as_mut().enable()`), IPC `32 KiB` coalescing loop, `toestub` strict TDD/fs/dep-hygiene check, and `surfaceHonesty.guard.test.ts`.
