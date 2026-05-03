//! Shared API surface for Vox plugins. Both host and code-payload plugin
//! crates depend on this crate.
//!
//! See: docs/src/architecture/plugin-system-redesign-2026.md

pub const VOX_PLUGIN_ABI_VERSION: u32 = 1;

pub mod errors;
pub mod extensions;
pub mod manifest;
pub mod skill;
