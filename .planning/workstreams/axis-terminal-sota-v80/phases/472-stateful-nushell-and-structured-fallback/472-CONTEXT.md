# Phase 472 Context: Stateful PTY Nushell Execution, `detect_ai_shell` Profile Routing & Native Structured Pipeline Fallback

**Workstream:** `axis-terminal-sota-v80`
**Phase:** 472
**Requirements:** `TERM-NU-01`, `TERM-NU-02`, `TERM-NU-03`, `TERM-NU-04`
**SSOT:** [`terminal-surfaces-audit-and-benchmarks-research-2026.md`](../../../../docs/src/architecture/terminal-surfaces-audit-and-benchmarks-research-2026.md)

## Decisions
1. Activate `detect_ai_shell()` when session `OpenSpec.shell` is `Some("auto")` or `Some("ai")` (`agent_exec.rs`, `host.rs`).
2. When session shell is `nu`, execute `!nu` / `InputIntent::StructuredShell` directly through the live stateful PTY (`host.rs`) preserving `cwd` and environment state; otherwise run `run_nu_structured_capture_in_cwd` with session `cwd`.
3. Implement `try_native_structured_pipeline` in `pty.rs` (`ls`, `pwd`, `env`, `version`) so hosts without `nu` on `PATH` still receive valid structured JSON tables instead of passing Nushell syntax into `/bin/sh`.
4. Implement `NuStructuredBackend` (`ShellBackend` for `ShellKind::Nushell`), eliminating stale `"Track 6"` stub drift.
