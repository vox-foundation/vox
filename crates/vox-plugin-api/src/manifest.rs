//! PluginManifest — typed deserialization of Plugin.toml files.
//!
//! Types live in `vox-plugin-types::plugin_manifest`. Re-exported here so
//! existing call sites (`vox_plugin_api::manifest::PluginManifest`) keep
//! compiling.

pub use vox_plugin_types::plugin_manifest::{
    CodePayload, CompositePayload, HostRequirement, NativeLib, PayloadProvides,
    PayloadRequires, PluginHeader, PluginManifest, PluginPayload, SkillPayload, SkillTools,
};
