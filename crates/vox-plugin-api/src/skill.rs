//! Plain-Rust types for the skill side of plugin loading.
//! Skill payloads do not cross a dylib boundary, so no abi_stable here.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SkillManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub tools: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LoadedSkill {
    pub plugin_id: String,
    pub format_version: u32,
    pub manifest: SkillManifest,
    pub body: String,
    pub exposed_tools: Vec<String>,
}
