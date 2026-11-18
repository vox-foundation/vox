# Phase 471 Review Fixes

- Simplified `submit_with_origin_async` and `submit_with_origin` in `crates/vox-terminal-core/src/host.rs` to return `lock(&entry.session).open_block_id()` directly (`clippy::let_and_return` resolved).
- Routed `host.rs` Nushell command formatting through shared `crate::pty::format_nu_structured_command`.
- Verified: `cargo clippy -p vox-terminal-core -p vox-term -p vox-corpus --no-deps --features vox-terminal-core/vox-lang -- -D warnings` exited `0`; `cargo test -p vox-terminal-core --features vox-lang` passed (`91/91` unit + `11` integration test binaries); `toestub` strict check passed cleanly.
