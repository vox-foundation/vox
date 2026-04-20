//! Shared DB access for Ludus CLI commands (`vox ludus`, `extras-ludus`).

use anyhow::Result;
use vox_db::{Codex, DbConfig};

/// Get a database instance (supports remote sync if configured).
pub async fn get_db() -> Result<Codex> {
    // resolve_for_mesh prioritizes EmbeddedReplica if URL+TOKEN+PATH are set,
    // which is the ideal state for federated Ludus profiles.
    let config = DbConfig::resolve_for_mesh()
        .map_err(|e| anyhow::anyhow!("Failed to resolve database configuration: {}", e))?;

    Codex::connect(config)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to gamification DB: {}", e))
}
