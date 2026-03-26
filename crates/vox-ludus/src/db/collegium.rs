//! Collegium (team) persistence.

use anyhow::Result;
use vox_db::Codex;

/// Create a new collegium.
pub async fn create_collegium(
    db: &Codex,
    id: &str,
    name: &str,
    description: Option<&str>,
    leader_id: &str,
) -> Result<()> {
    let now = crate::util::now_unix();
    db.insert_gamify_collegium(id, name, description, leader_id, now)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // Auto-join leader
    join_collegium(db, id, leader_id, "pontifex").await?;
    Ok(())
}

/// Join a collegium.
pub async fn join_collegium(
    db: &Codex,
    collegium_id: &str,
    user_id: &str,
    role: &str,
) -> Result<()> {
    let now = crate::util::now_unix();
    db.insert_gamify_collegium_member_or_ignore(collegium_id, user_id, role, now)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

/// Increment a collegium's lumens count.
pub async fn update_collegium_lumens(db: &Codex, collegium_id: &str, delta: i64) -> Result<()> {
    db.update_gamify_collegium_lumens(collegium_id, delta)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

/// List all collegiums with their total Lumens.
pub async fn list_collegiums(db: &Codex) -> Result<Vec<(String, String, i64, i64)>> {
    db.list_gamify_collegiums_with_counts()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

/// Get a specific collegium.
pub async fn get_collegium(db: &Codex, id: &str) -> Result<Option<(String, String, i64, i64)>> {
    db.get_gamify_collegium_with_count(id)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

/// Get the collegium a user belongs to.
pub async fn get_user_collegium(
    db: &Codex,
    user_id: &str,
) -> Result<Option<(String, String, i64, i64)>> {
    db.get_gamify_user_collegium_summary(user_id)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}
