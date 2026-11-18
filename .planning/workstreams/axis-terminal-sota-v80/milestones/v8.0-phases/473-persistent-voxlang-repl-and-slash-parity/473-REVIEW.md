# Phase 473 Code Review

**Scope:** `crates/vox-terminal-core/src/{vox_interp.rs,commands.rs,session.rs}`, `crates/vox-term/Cargo.toml`

## Findings
- **Warning (`skeleton/untested-pub-api` & `rust/unwrap-call`) — `crates/vox-terminal-core/src/vox_interp.rs`:**
  - Inline `#[cfg(test)] mod tests` was missing inside `vox_interp.rs` itself (coverage only lived in `tests/vox_interp.rs` and `host.rs`).
  - `capabilities_for_session` used `.unwrap()` in fallback capability construction.
  - Empty `if is_decl_only ...` no-op branch remained in `eval_in_session`.
