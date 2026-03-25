//! Collegium (team) persistence.

use anyhow::Result;
use turso::params;
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
    db.connection()
        .execute(
            "INSERT INTO gamify_collegiums (id, name, description, leader_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, name, description, leader_id, now],
        )
        .await?;

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
    db.connection().execute(
        "INSERT OR IGNORE INTO gamify_collegium_members (collegium_id, user_id, role, joined_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![collegium_id, user_id, role, now],
    ).await?;
    Ok(())
}

/// Increment a collegium's lumens count.
pub async fn update_collegium_lumens(db: &Codex, collegium_id: &str, delta: i64) -> Result<()> {
    db.connection()
        .execute(
            "UPDATE gamify_collegiums SET lumens = lumens + ?1 WHERE id = ?2",
            params![delta, collegium_id],
        )
        .await?;
    Ok(())
}

/// List all collegiums with their total Lumens.
pub async fn list_collegiums(db: &Codex) -> Result<Vec<(String, String, i64, i64)>> {
    let mut rows = db.connection().query(
        "SELECT id, name, lumens, (SELECT COUNT(*) FROM gamify_collegium_members WHERE collegium_id = id) FROM gamify_collegiums ORDER BY lumens DESC",
        params![],
    ).await?;

    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        out.push((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?));
    }
    Ok(out)
}

/// Get a specific collegium.
pub async fn get_collegium(db: &Codex, id: &str) -> Result<Option<(String, String, i64, i64)>> {
    let mut rows = db.connection().query(
        "SELECT id, name, lumens, (SELECT COUNT(*) FROM gamify_collegium_members WHERE collegium_id = id) FROM gamify_collegiums WHERE id = ?1",
        params![id],
    ).await?;
    if let Some(row) = rows.next().await? {
        Ok(Some((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
    } else {
        Ok(None)
    }
}

/// Get the collegium a user belongs to.
pub async fn get_user_collegium(
    db: &Codex,
    user_id: &str,
) -> Result<Option<(String, String, i64, i64)>> {
    let mut rows = db.connection().query(
        "SELECT c.id, c.name, c.lumens, (SELECT COUNT(*) FROM gamify_collegium_members WHERE collegium_id = c.id)
         FROM gamify_collegiums c
         JOIN gamify_collegium_members m ON m.collegium_id = c.id
         WHERE m.user_id = ?1",
        params![user_id],
    ).await?;
    if let Some(row) = rows.next().await? {
        Ok(Some((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
    } else {
        Ok(None)
    }
}
