# Phase 471 Verification

- `cargo test -p vox-terminal-core --features vox-lang`: `89/89` unit tests + all `11` integration test suites passed (includes `submit_async_does_not_block_current_thread_tokio_executor`, `format_agent_command_wraps_five_shells_without_subshell`, `classify_nu_structured_shell_and_help`, `shell_init_and_stateful_pty` 6 tests, `streaming_utf8_and_alt_screen` 9 tests).
- `cargo test -p vox-terminal-core`: `88/88` unit tests + all `12` integration test suites passed.
- `cargo test -p vox-term`: unit (`1` passed, `1` pre-existing ignored interactive test), `dispatch` (`2`), `headless` (`8`), `palette` (`4`), `vt` (`3`) all passed.
- `cargo test -p vox-gui --bin vox-gui commands::pty::warp::tests`: `1/1` passed (`warp_helpers_scrub_history_complete_tokens_list_runbooks_and_voxify_session`).
- `pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console`: `23/23` files, `141/141` tests passed.
- **Note:** Non-Windows cross-platform POSIX PTY/pipe behavior ran live on `macos/aarch64`; Windows ConPTY runtime execution was verified via cross-platform command-formatting/shim unit tests rather than on a Windows host runner.
