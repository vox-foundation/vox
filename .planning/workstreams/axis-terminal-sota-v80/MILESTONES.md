# Milestones

## v8.0 Non-Blocking Cross-Platform PTY, Stateful Nushell & Persistent VoxLang REPL, Nov 2026 UX, Spoke 9 Agentic Terminal & Competitor Benchmarks (Shipped: 2026-11-17)

**Phases completed:** 6 phases (`471–476`), 6 plans, 28 requirements (`TERM-PERF-01..04`, `TERM-XPLAT-01..04`, `TERM-NU-01..04`, `TERM-VOX-01..04`, `TERM-UX26-01..04`, `TERM-SPOKE-01..04`, `TERM-BENCH-01..04`)

**Key accomplishments:**

- Replaced blocking `Condvar::wait_timeout` submission and fixed `20 ms` `POLL_STEP_MS` agent polling with `tokio::sync::Notify` + `tokio::select!` (`wait_until_async`), added `32 KiB` IPC chunk coalescing in `vox-gui::commands::pty::warp::spawn_term_forwarder`, and eliminated speculative `vox-compiler` evaluation on POSIX paths (`/usr/bin/env`) and locked shell inputs (`intentRouter.ts`, `input.rs`, `host.rs`).
- Restored cross-platform parity across `pwsh`/`powershell`, `nu`, `cmd`, and POSIX shells (`format_agent_command_for_shell`), threaded session `cwd` into out-of-PTY capture (`run_shell_capture_in_cwd`, `run_nu_structured_capture_in_cwd`), and wired headless `vox-term` `run_plain_with_host` dispatch.
- Activated `detect_ai_shell()` session routing, stateful live-PTY `!nu` pipelines (guarded by `format_nu_structured_command`), `NuStructuredBackend` (`ShellKind::Nushell`), and path-traversal-guarded `try_native_structured_pipeline` JSON fallback when `nu` is absent on `PATH`.
- Replaced single-shot `fn main() -> Int` wrapping with `VoxReplSession` supporting arbitrary Vox value types, persistent declarations/`let` bindings, `/reset`, `TermSecurityLevel`-mapped `CapabilitySet`, live slash commands (`/budget`, `/skills`, `/memory`, `/context`), and `default = ["vox-lang"]` in `vox-term`.
- Added Nov 2026 UX & protocol parity across Console and `vox-term`: clickable `OSC 8` links, bounded `OSC 52` clipboard payloads, Kitty Keyboard Protocol `CSI u` encoding, per-block `rusage` badges, inline `std.frame` tables + `std.plot` SVG charts, and `InputBox` ghost-text autosuggestions.
- Added Tier-3B `ShellSpokeTier` / `OrganicSpokeKind::AGENTIC_TERMINAL_TASK_FAMILY` (`"agentic_terminal"`) and `verify_terminal_session_trajectory` while preserving 5-spoke and 8-spoke corpus invariants, plus `vox-terminal-core::bench`, `contracts/reports/terminal-bench.v1.json`, and `scripts/quality/terminal-bench-flywheel.vox`.
