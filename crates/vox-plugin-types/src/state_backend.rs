//! Plugin state-backend trait — abstracts the persistent store the plugin
//! host uses for skill publish/unpublish and hydration.
//!
//! Existed to break the `vox-plugin-host` → `vox-db` layering inversion: the
//! host now depends on this trait (defined here at L1) and a concrete
//! implementation is supplied by the integrating crate (orchestrator/CLI).

use serde::{Deserialize, Serialize};

/// One row from the underlying skill-manifest store, returned by
/// [`PluginStateBackend::list_skill_manifests`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginStateSkillEntry {
    pub id: String,
    pub version: String,
    pub manifest_json: String,
    pub skill_md: String,
}

/// Errors a state backend may return. Backends stringify their internal
/// errors so the host doesn't depend on backend-specific error types.
#[derive(Debug, thiserror::Error)]
#[error("plugin state backend: {0}")]
pub struct PluginStateError(pub String);

impl PluginStateError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

/// Trait the plugin host uses for persistent skill state. Implemented by
/// `vox-db` (concrete) and any test/in-memory double.
#[async_trait::async_trait]
pub trait PluginStateBackend: Send + Sync {
    /// Persist a skill manifest. Idempotent — re-publishing the same
    /// (id, version) updates `manifest_json` and `skill_md`.
    async fn publish_skill(
        &self,
        id: &str,
        version: &str,
        manifest_json: &str,
        skill_md: &str,
    ) -> Result<(), PluginStateError>;

    /// Remove a skill manifest by id (any version).
    async fn unpublish_skill(&self, id: &str) -> Result<(), PluginStateError>;

    /// List all stored skill manifests.
    async fn list_skill_manifests(&self) -> Result<Vec<PluginStateSkillEntry>, PluginStateError>;
}
