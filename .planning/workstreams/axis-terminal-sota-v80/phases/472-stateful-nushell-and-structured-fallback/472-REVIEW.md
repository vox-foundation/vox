# Phase 472 Code Review

**Scope:** `crates/vox-terminal-core/src/{pty.rs,host.rs,agent_exec.rs,session.rs}`

## Findings
- **Warning (path traversal safety) — `crates/vox-terminal-core/src/pty.rs` (`try_native_structured_pipeline` `"ls"`):** Relative directory argument allowed unvalidated `..` (`ParentDir`) components.
- **Warning (double encoding) — `crates/vox-terminal-core/src/pty.rs` & `host.rs`:** Wrapping pipelines already ending with `| to json` or `| to json --raw` in `do { ... } | to json --raw` risked double-encoding JSON output.
