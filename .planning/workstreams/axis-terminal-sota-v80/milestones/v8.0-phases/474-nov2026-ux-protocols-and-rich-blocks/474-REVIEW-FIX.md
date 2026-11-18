# Phase 474 Review Fixes

- Added `#[cfg(test)] mod tests` (`input_box_ghost_completion_and_mode_badges_work`) to `crates/vox-term/src/ui/input.rs` covering `InputBox::new`, `push_char`, `backspace`, `update_ghost`, `accept_ghost`, `active_mode`, and `submit`.
- Verified: `cargo test -p vox-term` passed (`2` unit tests + `1` pre-existing ignored interactive test; `dispatch` 2, `headless` 8, `palette` 4, `vt` 3).
