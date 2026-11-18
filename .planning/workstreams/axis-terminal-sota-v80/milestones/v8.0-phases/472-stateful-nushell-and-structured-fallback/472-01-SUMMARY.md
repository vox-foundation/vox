# Phase 472 Summary 01: Nushell Backend & Native Structured Pipelines

## Shipped Changes
- **`crates/vox-terminal-core/src/agent_exec.rs` & `host.rs`:** Resolved `shell: Some("auto" | "ai")` via `detect_ai_shell()`, and routed `InputIntent::StructuredShell` through the live PTY whenever `shell_name == "nu"`.
- **`crates/vox-terminal-core/src/pty.rs` & `session.rs`:** Implemented `run_nu_structured_capture_in_cwd`, `try_native_structured_pipeline` (in-process JSON fallback for `ls`, `pwd`, `env`, `version` when `nu` is absent on `PATH`), and `NuStructuredBackend` (`ShellBackend` for `ShellKind::Nushell`, removing `"Track 6"` stub drift).
