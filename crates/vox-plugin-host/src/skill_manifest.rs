//! Skill manifest types — re-exported from `vox-plugin-types::skill_manifest`.
//!
//! Types live in the L1 leaf crate `vox-plugin-types` so tooling that needs
//! only the manifest schema can avoid the plugin host runtime.

pub use vox_plugin_types::skill_manifest::{SkillCategory, SkillManifest, SkillPermission};
