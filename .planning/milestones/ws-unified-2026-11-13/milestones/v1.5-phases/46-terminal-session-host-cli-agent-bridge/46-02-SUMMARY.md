# Plan 46-02 Summary: `vox-term` Daemon Auto-Detect, `--attach <id>`, `--local` & Session Palette

- **Phase**: `46-terminal-session-host-cli-agent-bridge`
- **Plan**: `02`
- **Requirements**: `TERM-02`
- **Status**: Complete

## Changes Made
- Added `resolve_session_host(force_local: bool) -> Arc<dyn SessionHost>` in `crates/vox-terminal-core/src/daemon_host.rs` that pings `vox-orchestrator-d` with a 250ms timeout and falls back to `LocalHost::default()`.
- Added `TermCliArgs` (`--attach <session_id>`, `--attach=<session_id>`, `--local`) and `resolve_host(force_local: bool)` in `crates/vox-term/src/app.rs`.
- Added `Palette::with_sessions(&mut self, sessions: &[SessionInfo])` in `crates/vox-term/src/ui/palette.rs` surfacing `/attach <id>` entries in the fuzzy palette.

## Verification
- RED → GREEN verified via `timeout 900s cargo test -p vox-term`:
  - `tests/headless.rs`: 4 passed (`parse_cli_args_handles_attach_and_local`, `resolve_host_falls_back_to_local_when_daemon_unreachable`, `term_dumb_piped_stdin_no_ansi_no_crash`, `non_tty_piped_stdin_no_ansi_no_crash`)
  - `tests/palette.rs`: 4 passed (`palette_includes_live_sessions`, `fuzzy_mdl_matches_model`, `empty_query_returns_all`, `no_match_returns_empty`)
