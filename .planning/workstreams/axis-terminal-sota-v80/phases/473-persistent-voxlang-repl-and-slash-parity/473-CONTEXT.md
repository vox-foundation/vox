# Phase 473 Context: Persistent Multi-Type `VoxLang` REPL Session State, Tier-Mapped Capabilities & Unified Slash Commands

**Workstream:** `axis-terminal-sota-v80`
**Phase:** 473
**Requirements:** `TERM-VOX-01`, `TERM-VOX-02`, `TERM-VOX-03`, `TERM-VOX-04`
**SSOT:** [`terminal-surfaces-audit-and-benchmarks-research-2026.md`](../../../../docs/src/architecture/terminal-surfaces-audit-and-benchmarks-research-2026.md)

## Decisions
1. Replace the hardcoded `fn main() -> Int` wrapper across `vox_interp.rs`, `host.rs`, and `session.rs` with persistent per-session `VoxReplSession` (`decl_buffer`, `let_buffer`, `script_mode: true`, `vox_value_display`).
2. Keep literal `"Interpreter::new"` and `"CapabilitySet::parse"` substrings in `crates/vox-terminal-core/src/vox_interp.rs` to satisfy `vox-compiler::eval::caps::tests::production_embedders_assign_caps_explicitly`.
3. Map `TermSecurityLevel` + live session `cwd` via `capabilities_for_session`, and filter bare shell inputs cleanly via `should_try_vox_eval`.
4. Enable `vox-lang` by default in `crates/vox-term/Cargo.toml` and wire live structured slash commands (`/budget`, `/skills`, `/memory`, `/context`, `/reset`).
