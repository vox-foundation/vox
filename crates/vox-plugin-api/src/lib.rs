//! Shared API surface for Vox plugins. Both host and code-payload plugin
//! crates depend on this crate.
//!
//! See: docs/src/architecture/plugin-system-redesign-2026.md
//
// The abi_stable `#[sabi_trait]` macro generates unsafe blocks for FFI vtable
// dispatch and impl blocks for the generated trait-object types in the same
// expansion site. Both are necessary and correct for the ABI boundary.
#![allow(unsafe_code, non_local_definitions)]

pub const VOX_PLUGIN_ABI_VERSION: u32 = 12;

pub mod abi;
pub mod errors;
pub mod extensions;
pub mod host;
pub mod manifest;
pub mod skill;
