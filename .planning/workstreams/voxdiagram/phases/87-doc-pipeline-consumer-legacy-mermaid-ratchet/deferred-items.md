# Phase 87 deferred items

- 87-01: `cargo clippy -p vox-doc-pipeline --all-targets -- -D warnings` fails with 26 pre-existing
  findings (rust 1.98 lints: collapsible_if, io_other_error, manual_strip, never_loop,
  needless_range_loop, unnecessary clone) all under `crates/vox-doc-pipeline/src/claims/`
  (`mod.rs`, `gate.rs`, `ledger.rs`, `prune.rs`, `resolver.rs`, `suppressions.rs`). Untouched by
  this phase; owner decides whether to fix in a toolchain-lint sweep. Later plans in this phase
  should judge clippy by "no finding in files this phase edited".
