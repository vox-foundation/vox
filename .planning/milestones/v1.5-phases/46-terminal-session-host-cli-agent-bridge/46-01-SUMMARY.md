# Plan 46-01 Summary: `BlockOrigin` (`User` | `Agent`) & `submit_with_origin`

- **Phase**: `46-terminal-session-host-cli-agent-bridge`
- **Plan**: `01`
- **Requirements**: `TERM-01`
- **Status**: Complete

## Changes Made
- Added `BlockOrigin` (`User` | `Agent { agent_id: String }`) with `#[serde(tag = "kind", rename_all = "snake_case")]` and `#[default] User` in `crates/vox-terminal-core/src/block.rs`.
- Added `#[serde(default)] pub origin: BlockOrigin` and `Block::with_origin` to `Block`.
- Added `Session::submit_with_origin` in `crates/vox-terminal-core/src/session.rs` and `LocalHost::submit_with_origin` in `crates/vox-terminal-core/src/host.rs`, preserving `BlockOrigin::User` as the default on `submit`.
- Re-exported `BlockOrigin` from `crates/vox-terminal-core/src/lib.rs`.

## Verification
- RED → GREEN verified via `timeout 900s cargo test -p vox-terminal-core`:
  - `tests/block.rs`: 5 passed (`block_defaults_to_user_origin`, `block_origin_roundtrips_and_defaults_on_missing_field`)
  - `tests/session.rs`: 5 passed (`submit_with_agent_origin_tags_block`, `submit_default_remains_user_origin`)
  - Unit tests (`src/lib.rs`): 37 passed
