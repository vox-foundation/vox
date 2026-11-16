# Deferred items (phase 02)

- **Status:** resolved
- 02-05 (RESOLVED: added `#[cfg(feature = "tavily")]` guard in `crates/vox-search/src/policy.rs`): `cargo check -p vox-search --no-default-features` passes cleanly.
- 02-06 (RESOLVED in 02-07, `fix(02-07)`): moved the test SQL behind `VoxDb::library_chunk_positions` / `embedding_count` and existing readers; `vox ci sql-surface-guard --all` passes.
- 02-10 (RESOLVED in 02-12, `86d0ef43a`): ratchet tests retargeted at LibraryRetriever with a config-aware shape check; `std::slice::from_ref(&q)` in the `library.rs` unit test.
