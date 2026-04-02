//! YAML document types for [`contracts/capability/capability-registry.yaml`](../../../contracts/capability/capability-registry.yaml).

use serde::{Deserialize, Serialize};

/// Root document deserialized from `capability-registry.yaml`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CapabilityRegistryDoc {
    pub schema_version: u32,
    #[serde(default)]
    pub auto_mcp_capabilities: bool,
    #[serde(default)]
    pub auto_cli_capabilities: bool,
    #[serde(default)]
    pub curated: Vec<CuratedCapability>,
    #[serde(default)]
    pub runtime_builtin_maps: Vec<RuntimeBuiltinMap>,
    #[serde(default)]
    pub exemptions: Option<Exemptions>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CuratedCapability {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description_human: Option<String>,
    #[serde(default)]
    pub description_model: Option<String>,
    #[serde(default)]
    pub intent_tags: Vec<String>,
    #[serde(default)]
    pub side_effect_class: Option<String>,
    #[serde(default)]
    pub scope_kind: Option<String>,
    #[serde(default)]
    pub reversible: Option<bool>,
    #[serde(default)]
    pub requires_repo: Option<bool>,
    #[serde(default)]
    pub requires_git: Option<bool>,
    #[serde(default)]
    pub preferred_for_models: Option<bool>,
    #[serde(default)]
    pub human_takeover_friendly: Option<bool>,
    #[serde(default)]
    pub mens_planner_visible: Option<bool>,
    #[serde(default)]
    pub mcp_tool: Option<String>,
    #[serde(default)]
    pub cli_path: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RuntimeBuiltinMap {
    pub namespace: String,
    pub method: String,
    pub capability_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Exemptions {
    #[serde(default)]
    pub cli_paths: Vec<Vec<String>>,
}
