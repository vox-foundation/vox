# Phase 473 Review Fixes

- Removed the dead `if is_decl_only ...` no-op branch in `crates/vox-terminal-core/src/vox_interp.rs`.
- Replaced fallback `.unwrap()` calls with `.expect("empty capability spec is valid")` while preserving literal `Interpreter::new` and `CapabilitySet::parse` markers required by `vox-compiler`.
- Added inline `#[cfg(test)] mod tests` in `vox_interp.rs` (`vox_syntax_filter_and_repl_state_behave_cleanly` and `repl_evaluates_multi_type_and_persists_state_until_reset`).
- Verified `cargo test -p vox-terminal-core --features vox-lang` (`91/91` unit tests passed) and `cargo clippy --no-deps -D warnings` exited `0`.
