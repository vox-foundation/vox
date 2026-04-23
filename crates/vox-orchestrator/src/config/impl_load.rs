use std::path::Path;

use vox_socrates_policy::ConfidencePolicy;

use super::errors::ConfigError;
use super::merge_populi::apply_vox_populi_toml;
use super::orchestrator_fields::OrchestratorConfig;

impl OrchestratorConfig {
    /// Effective Socrates policy for gates and MCP tools (workspace default + optional overrides).
    #[must_use]
    pub fn effective_socrates_policy(&self) -> ConfidencePolicy {
        let base = ConfidencePolicy::workspace_default();
        match &self.socrates_policy {
            Some(o) => base.with_overrides(o),
            None => base,
        }
    }

    /// Effective web search policy (environment-driven fallback).
    #[must_use]
    pub fn effective_search_policy(&self) -> vox_search::policy::SearchPolicy {
        vox_search::policy::SearchPolicy::from_env()
    }

    /// Load configuration from a TOML file.
    ///
    /// Looks for an `[orchestrator]` section in the given file.
    /// Returns the default config if the section is missing.
    pub fn load_from_toml(path: &Path) -> Result<Self, ConfigError> {
        let content = vox_bounded_fs::read_utf8_path_capped(path)
            .map_err(|e| ConfigError::Io(std::io::Error::other(e.to_string())))?;
        let table: toml::Table = toml::from_str(&content).map_err(ConfigError::Parse)?;

        let mut config = if let Some(section) = table.get("orchestrator") {
            let section_str = toml::to_string(section).map_err(ConfigError::Serialize)?;
            toml::from_str(&section_str).map_err(ConfigError::Parse)?
        } else {
            Self::default()
        };

        match vox_repository::read_vox_populi_toml(path) {
            Ok(Some(mens)) => apply_vox_populi_toml(&mut config, &mens),
            Ok(None) => {}
            Err(e) => tracing::warn!("Vox.toml [mens] ignored (parse error): {e}"),
        }

        Ok(config)
    }
}
