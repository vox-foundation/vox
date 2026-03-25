//! YAML-driven synthetic templates (`mens/config/templates.yaml`).

use std::sync::LazyLock;

#[derive(serde::Deserialize)]
pub(crate) struct TemplatesConfig {
    pub(crate) synthetic: SyntheticTemplates,
}

#[derive(serde::Deserialize)]
pub(crate) struct SyntheticTemplates {
    pub(crate) tool_definitions: Vec<String>,
    pub(crate) a2a_messages: Vec<String>,
    pub(crate) skills: Vec<String>,
    pub(crate) orchestrator_commands: Vec<String>,
    pub(crate) workflows: Vec<ScenarioDef>,
    pub(crate) agents: Vec<ScenarioDef>,
}

#[derive(serde::Deserialize, Clone)]
pub(crate) struct ScenarioDef {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) snippet: String,
}

pub(crate) static TEMPLATES: LazyLock<SyntheticTemplates> = LazyLock::new(|| {
    let yaml = include_str!("../../../../mens/config/templates.yaml");
    let cfg: TemplatesConfig = serde_yaml::from_str(yaml).expect("Failed to parse templates.yaml");
    cfg.synthetic
});
