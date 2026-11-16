---
phase: "441"
slug: "stateful-pty-and-lossless-streaming"
status: validated
nyquist_compliant: true
wave_0_complete: true
created: "2026-11-13"
---

# Phase 441 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (`libtest` + `tokio::test` 1.x) |
| **Config file** | `crates/vox-terminal-core/Cargo.toml`, `crates/vox-term/Cargo.toml`, `crates/vox-orchestrator-d/Cargo.toml`, `crates/vox-cli/Cargo.toml`, `crates/vox-gui/Cargo.toml` |
| **Quick run command** | `timeout 300s cargo test -p vox-terminal-core` |
| **Full suite command** | `timeout 300s cargo test -p vox-terminal-core -p vox-term -p vox-orchestrator-d` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `timeout 300s cargo test -p vox-terminal-core`
- **After every plan wave:** Run `timeout 300s cargo test -p vox-terminal-core -p vox-term` and `timeout 300s cargo test -p vox-orchestrator-d --bin vox-orchestrator-d term_dispatch`
- **Before `/gsd-verify-work`:** Full suite + `timeout 300s cargo test -p vox-cli --test command_catalog_paths_baseline` + `timeout 300s cargo test -p vox-gui --lib commands::pty` + `timeout 300s cargo run -p vox-arch-check` must be green
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 441-01-01 | 01 | 1 | TERM-PTY-01, TERM-PTY-02 | T-441-01, T-441-02 | `tempfile::TempDir` (`0700`) owner-only shim dir unlinked on `PtyHandle` drop; `cols.max(1)`/`rows.max(1)` clamped before `MasterPty::resize`; zero stdin hook echo | unit + integration | `timeout 300s cargo test -p vox-terminal-core --lib shell_init` | ❌ W1 (TDD) | ⬜ pending |
| 441-01-02 | 01 | 1 | TERM-PTY-01, TERM-PTY-02 | T-441-01, T-441-02, T-441-03 | Stateful persistent PTY `cd`/`export`, real `zsh` `$?` exit codes, `OSC 7`/`633;P` `cwd`, `\x03` `SIGINT`, `16 KiB` incomplete OSC cap | integration | `timeout 300s cargo test -p vox-terminal-core --test shell_init_and_stateful_pty` | ❌ W1 (TDD) | ⬜ pending |
| 441-02-01 | 02 | 2 | TERM-PTY-03 | T-441-03 | `Utf8StreamDecoder` 3-byte trailing tail prevents `U+FFFD` across prime-chunk splits; live `OutputAppended` streaming; `\r` progress overwrite normalization | unit + integration | `timeout 300s cargo test -p vox-terminal-core --test streaming_utf8_and_alt_screen` | ❌ W2 (TDD) | ⬜ pending |
| 441-02-02 | 02 | 2 | TERM-PTY-03 | T-441-02 | `VtShadow` `scrolling_history: 0` + `1×1` clamping tracks `TermMode::ALT_SCREEN` (`?1049h`/`?1049l`/`\x1bc`) and suppresses `OSC 633` block splitting | unit + integration | `timeout 300s cargo test -p vox-terminal-core --test streaming_utf8_and_alt_screen` | ❌ W2 (TDD) | ⬜ pending |
| 441-02-03 | 02 | 2 | TERM-PTY-03, TERM-PTY-04 | T-441-03, T-441-04 | `LocalHost` `2 MiB` `OutputFrame` ring serves `after_seq` deltas + `resync_required`; `DaemonHost` routes `alt_screen`/`cwd_changed`/`term.signal` & auto-resubscribes on lag | unit + integration | `timeout 300s cargo test -p vox-terminal-core` | ✅ | ⬜ pending |
| 441-03-01 | 03 | 3 | TERM-PTY-04 | T-441-02 | `TermDispatch` `120×32` `term.open` defaults, `after_seq` delta snapshots, and `alt_screen`/`cwd_changed` stream events over loopback TCP | integration | `timeout 300s cargo test -p vox-orchestrator-d --bin vox-orchestrator-d term_dispatch` | ✅ | ⬜ pending |
| 441-03-02 | 03 | 3 | TERM-PTY-04 | T-441-04 | `refuse_agent_origin` fail-closed on `term.signal`/`term.submit`/`term.input`; `DaemonHost::signal` interrupts running job without killing shell | integration | `timeout 300s cargo test -p vox-orchestrator-d --bin vox-orchestrator-d term_dispatch` | ✅ | ⬜ pending |
| 441-04-01 | 04 | 3 | TERM-PTY-04 | T-441-02, T-441-03, T-441-04 | `vox-term` `VtGrid` `1×1` clamping + `attach_or_open_session` (`--attach` numeric `"latest"`, explicit `<id>`, missing-ID error, detach continuity, lag resync) | integration | `timeout 300s cargo test -p vox-term` | ✅ | ⬜ pending |
| 441-04-02 | 04 | 3 | TERM-PTY-04 | — | `Cli::Term` `--attach [SESSION_ID]` + `--local` clap flags and async `run_with_args` dispatch without nested Tokio runtime or catalog drift | unit + integration | `timeout 300s cargo test -p vox-cli --test command_catalog_paths_baseline` | ✅ | ⬜ pending |
| 441-04-03 | 04 | 3 | TERM-PTY-04 | T-441-03 | `vox-gui` `term_session_attach` `after_seq` delta DTO, `Utf8StreamDecoder` chunk forwarding, and `RecvError::Lagged` delta recovery | unit + integration | `timeout 300s cargo test -p vox-gui --lib commands::pty` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] Every task embeds TDD RED→GREEN test creation (`crates/vox-terminal-core/tests/shell_init_and_stateful_pty.rs` in `441-01`, `crates/vox-terminal-core/tests/streaming_utf8_and_alt_screen.rs` in `441-02`, and existing `term_dispatch.rs`/`headless.rs`/`command_catalog.rs`/`pty.rs` test targets in `441-03..04`) — no separate pre-wave scaffold plan required.

---

## Manual-Only Verifications

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 60s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** validated 2026-11-14

---

## Validation Audit 2026-11-14

| Metric | Count |
|--------|-------|
| Gaps found | 0 |
| Resolved | 10 |
| Escalated | 0 |
