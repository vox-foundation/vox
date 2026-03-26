//! User counters.

use anyhow::Result;
use vox_db::Codex;

/// Get a specific counter for a user.
pub async fn get_counter(db: &Codex, user_id: &str, name: &str) -> Result<u32> {
    let v = db
        .get_gamify_counter(user_id, name)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(v.max(0) as u32)
}

/// Increment a counter and return the new value.
pub async fn increment_counter(db: &Codex, user_id: &str, name: &str, amount: u32) -> Result<u32> {
    let v = db
        .increment_gamify_counter_by(user_id, name, amount as i64)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(v.max(0) as u32)
}

/// Set a counter to a specific value.
pub async fn set_counter(db: &Codex, user_id: &str, name: &str, value: u32) -> Result<()> {
    db.set_gamify_counter(user_id, name, value as i64)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}
