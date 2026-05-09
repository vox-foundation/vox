//! Pure-types surface for the vox plugin system.
//!
//! Designed to be a leaf dep: no async runtime, no DB client, no abi_stable.
//! Crates that need only the manifest/skill/state-backend shapes can depend
//! here without pulling in `vox-plugin-api`'s full ABI machinery or `vox-db`.
//!
//! Re-exported by `vox-plugin-api` (manifest types) and `vox-plugin-host`
//! (skill manifest + state-backend trait) for backwards compatibility.

pub mod plugin_manifest;
pub mod skill_manifest;
pub mod state_backend;

pub use plugin_manifest::{
    CodePayload, CompositePayload, HostRequirement, NativeLib, PayloadProvides, PayloadRequires,
    PluginHeader, PluginManifest, PluginPayload, SkillPayload, SkillTools,
};
pub use skill_manifest::{SkillCategory, SkillManifest, SkillPermission};
pub use state_backend::{PluginStateBackend, PluginStateError, PluginStateSkillEntry};
