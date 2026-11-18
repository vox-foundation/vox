# Phase 472 Review Fixes

- Extracted `pub fn format_nu_structured_command(pipeline: &str) -> String` in `crates/vox-terminal-core/src/pty.rs` (skipping redundant `to json` wrapping) and called it from both `run_nu_structured_capture_in_cwd` and `host.rs`.
- Added quote trimming and `Component::ParentDir` traversal rejection in `try_native_structured_pipeline` (`"ls"`), plus unit assertions in `pty::tests`.
- Verified: `cargo test -p vox-terminal-core --features vox-lang` passed (`91/91` unit + `11` integration test binaries).
