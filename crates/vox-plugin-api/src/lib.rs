//! Shared API surface for Vox plugins. Both host and code-payload plugin
//! crates depend on this crate.
//!
//! See: docs/src/architecture/plugin-system-redesign-2026.md

pub const VOX_PLUGIN_ABI_VERSION: u32 = 6;

pub mod abi;
pub mod errors;
pub mod extensions;
pub mod host;
pub mod manifest;
pub mod skill;
