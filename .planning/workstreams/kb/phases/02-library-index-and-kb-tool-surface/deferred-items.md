# Deferred items (phase 02)

- 02-05: `cargo check -p vox-search --no-default-features` fails at HEAD: `policy.rs` (~line 493) uses `crate::tavily_research` without a `tavily` cfg guard. Pre-existing, unrelated to the Tantivy removal.

- 02-06: `vox ci ssot-drift` (sql-surface-guard) fails at HEAD: `crates/vox-search/tests/kb_library_index.rs` uses raw `query`/`execute` outside the allowlist (introduced by 02-01/02-04 tests, not 02-06). Fix: move the raw SQL into a `vox_db::VoxDb` method in `store/ops_*.rs`, or allowlist the test file.
