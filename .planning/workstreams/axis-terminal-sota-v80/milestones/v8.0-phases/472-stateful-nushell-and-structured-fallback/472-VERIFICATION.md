# Phase 472 Verification

- Observed host status: `nu` is not installed on this macOS test host (`which nu` returned exit 1), exercising the native structured pipeline fallback path directly.
- `cargo test -p vox-terminal-core --features vox-lang` & default features both passed:
  - `pty::tests::detect_ai_shell_is_nonempty`
  - `pty::tests::run_nu_structured_capture_executes_or_falls_back_cleanly`
  - `pty::tests::run_shell_capture_in_cwd_and_native_structured_fallback_respect_cwd`
  - `pty::tests::nu_snippet_emits_osc633_markers`
  - `host::tests::open_with_auto_shell_resolves_ai_shell_cleanly`
  - `tests/commands.rs`: `2/2` (`help_includes_nu_slash_command`, `nu_command_help_and_dispatch`)
  - `tests/session.rs`: `7/7` (includes `openspec_custom_shell_and_structured_shell_submission`)
