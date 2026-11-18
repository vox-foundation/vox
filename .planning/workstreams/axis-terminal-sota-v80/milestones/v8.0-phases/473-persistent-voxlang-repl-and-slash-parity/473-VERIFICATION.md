# Phase 473 Verification

- `cargo test -p vox-compiler --lib eval::caps::tests::production_embedders_assign_caps_explicitly`: `1 passed; 0 failed`.
- `cargo test -p vox-terminal-core --features vox-lang`: `89/89` unit tests passed + `tests/vox_interp.rs` (`2/2`) + `tests/session.rs` (`7/7`).
- `cargo test -p vox-terminal-core` (default without `vox-lang`): `88/88` unit tests passed + `tests/vox_interp.rs` (`1/1`: `returns_vox_lang_diagnostic_when_disabled`).
- `cargo test -p vox-term` (with `default = ["vox-lang"]`): all compiled and ran cleanly (`lib`: 1 passed / 1 pre-existing ignored, `dispatch`: 2, `headless`: 8, `palette`: 4, `vt`: 3).
