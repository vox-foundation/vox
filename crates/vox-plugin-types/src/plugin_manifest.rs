//! PluginManifest — typed deserialization of Plugin.toml files.
//!
//! See docs/src/reference/plugin-manifest.md for the canonical schema.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub plugin: PluginHeader,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PluginHeader {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    pub host: HostRequirement,
    pub payload: PluginPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct HostRequirement {
    pub min_vox_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum PluginPayload {
    Code(CodePayload),
    Skill(SkillPayload),
    Composite(CompositePayload),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CodePayload {
    pub abi_version: u32,
    #[serde(default)]
    pub provides: PayloadProvides,
    #[serde(default)]
    pub requires: PayloadRequires,
    #[serde(default)]
    pub artifacts: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PayloadProvides {
    #[serde(default)]
    pub extension_points: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PayloadRequires {
    #[serde(default)]
    pub os: Vec<String>,
    #[serde(default)]
    pub arch: Vec<String>,
    #[serde(default)]
    pub native_libs: Vec<NativeLib>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct NativeLib {
    pub name: String,
    #[serde(default)]
    pub min_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SkillPayload {
    pub format_version: u32,
    pub skill_md: String,
    #[serde(default)]
    pub tools: SkillTools,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SkillTools {
    #[serde(default)]
    pub exposes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CompositePayload {
    pub code: CodePayload,
    pub skill: SkillPayload,
}
