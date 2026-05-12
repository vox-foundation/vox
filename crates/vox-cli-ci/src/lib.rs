//! Workspace boundary marker for future extraction of `vox ci` implementations.
//!
//! Today, `vox ci …` lives under [`vox_cli::commands::ci`](../../vox-cli/src/commands/ci/mod.rs)
//! because of tight coupling to `VoxCliRoot`, registry contracts, and shared helpers.
//!
//! New **pure** validation helpers that do not need `clap` or `vox-cli` types should
//! land here first behind unit tests; `vox-cli` can call them via normal crate deps
//! once the dependency direction is one-way (`vox-cli` → `vox-cli-ci`).
//!
//! See [`docs/src/architecture/where-things-live.md`](../../../docs/src/architecture/where-things-live.md).

#![forbid(unsafe_code)]

/// Sentinel used by docs/architecture to describe the split boundary.
pub const CRATE_BOUNDARY_MARKER: &str = "vox-cli-ci";
