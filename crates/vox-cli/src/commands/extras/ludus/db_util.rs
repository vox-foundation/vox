//! Shared DB access for Ludus CLI commands (`vox ludus`, `extras-ludus`).

use anyhow::Result;
use vox_db::Codex;

/// Get a local database instance.
pub async fn get_db() -> Result<Codex> {
    Codex::connect_default()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to local gamification DB: {}", e))
}
