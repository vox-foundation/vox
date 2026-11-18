# Phase 473 Summary 01: Persistent VoxLang REPL & Slash Command Parity

## Shipped Changes
- **`crates/vox-terminal-core/src/vox_interp.rs`:** Upgraded single-shot wrapper into `VoxReplSession` with persistent `decl_buffer`/`let_buffer`, multi-type display formatting, security-tier capability mapping (`capabilities_for_session`), and `should_try_vox_eval` filtering while retaining literal `Interpreter::new` / `CapabilitySet::parse`.
- **`crates/vox-terminal-core/src/session.rs` & `host.rs`:** Embedded `VoxReplSession`, replaced `fn main() -> Int` wrapping with `eval_vox_line`, and added `/reset`.
- **`crates/vox-terminal-core/src/commands.rs`:** Structured `/budget`, `/skills`, `/memory`, `/context` output and registered `/reset`.
- **`crates/vox-term/Cargo.toml`:** Enabled `default = ["vox-lang"]`.
