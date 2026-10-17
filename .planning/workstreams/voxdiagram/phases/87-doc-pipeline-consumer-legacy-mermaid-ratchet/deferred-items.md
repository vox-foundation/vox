# Phase 87 deferred items

- 87-01: `cargo clippy -p vox-doc-pipeline --all-targets -- -D warnings` fails with 26 pre-existing
  findings (rust 1.98 lints: collapsible_if, io_other_error, manual_strip, never_loop,
  needless_range_loop, unnecessary clone) all under `crates/vox-doc-pipeline/src/claims/`
  (`mod.rs`, `gate.rs`, `ledger.rs`, `prune.rs`, `resolver.rs`, `suppressions.rs`). Untouched by
  this phase; owner decides whether to fix in a toolchain-lint sweep. Later plans in this phase
  should judge clippy by "no finding in files this phase edited".
- 87-03: `vox ci pre-push` (fast tier) fails at `ssot-drift` with
  `contracts/capabilities/capability-index.v1.json is stale; run vox ci capability-index --write`.
  Cause: the index has no `crate.vox-diagram` row (the crate landed in an earlier v1.7 phase; the
  row has never been in the committed index at any commit). `contracts/capabilities/` and the
  generator are byte-identical between `2f0bd1d73` and HEAD, so it predates this plan. The only
  diff `--write` produces is that one row (plus `capabilityIndex.generated.ts`). Verified by
  regenerating uncommitted, running the full fast tier green, then reverting both files. Hosted CI's
  `ssot-autoregen` regenerates it; not hand-committed here.
- 87-03: `vox-cli` lib tests did not compile on this branch (`OperationRow` test initializer in
  `operations_catalog_exposure.rs` lacked `owner_crate`/`domain`; main already has them). Fixed in
  `17f84c579` because the new gate test needs that test binary.
