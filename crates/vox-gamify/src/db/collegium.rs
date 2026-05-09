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
    let id_s = id.to_string();
    let name_s = name.to_string();
    let description_s = description.map(str::to_string);
    let leader_s = leader_id.to_string();
    let breaker = db.breaker().clone();
    let conn = db.connection().clone();
    breaker
        .call(|| async move {
            conn.execute(
                "INSERT INTO gamify_collegium (id, name, description, leader_id, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    id_s.as_str(), name_s.as_str(), description_s.as_deref(),
                    leader_s.as_str(), now
                ],
            )
            .await?;
            Ok::<(), vox_db::StoreError>(())
        })
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
    let collegium_id = collegium_id.to_string();
    let user_id = user_id.to_string();
    let role = role.to_string();
    let breaker = db.breaker().clone();
    let conn = db.connection().clone();
    breaker
        .call(|| async move {
            conn.execute(
                "INSERT OR IGNORE INTO gamify_collegium_members (collegium_id, user_id, role, joined_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![collegium_id.as_str(), user_id.as_str(), role.as_str(), now],
            )
            .await?;
            Ok::<(), vox_db::StoreError>(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

/// Increment a collegium's lumens count.
pub async fn update_collegium_lumens(db: &Codex, collegium_id: &str, delta: i64) -> Result<()> {
    let collegium_id = collegium_id.to_string();
    let breaker = db.breaker().clone();
    let conn = db.connection().clone();
    breaker
        .call(|| async move {
            conn.execute(
                "UPDATE gamify_collegium SET lumens=COALESCE(lumens, 0)+?1 WHERE id=?2",
                params![delta, collegium_id.as_str()],
            )
            .await?;
            Ok::<(), vox_db::StoreError>(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

/// List all collegiums with their total Lumens.
pub async fn list_collegiums(db: &Codex) -> Result<Vec<(String, String, i64, i64)>> {
    let mut rows = db
        .connection()
        .query(
            "SELECT id, name, lumens, (SELECT COUNT(*) FROM gamify_collegium_members WHERE collegium_id = id)
             FROM gamify_collegium ORDER BY lumens DESC",
            (),
        )
        .await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        out.push((
            row.get(0)?,
            row.get(1)?,
            row.get::<i64>(2).unwrap_or(0),
            row.get::<i64>(3).unwrap_or(0),
        ));
    }
    Ok(out)
}

/// Get a specific collegium.
pub async fn get_collegium(db: &Codex, id: &str) -> Result<Option<(String, String, i64, i64)>> {
    let mut rows = db
        .connection()
        .query(
            "SELECT id, name, lumens, (SELECT COUNT(*) FROM gamify_collegium_members WHERE collegium_id = id)
             FROM gamify_collegium WHERE id = ?1",
            params![id],
        )
        .await?;
    Ok(rows.next().await?.map(|row| {
        (
            row.get(0).unwrap_or_default(),
            row.get(1).unwrap_or_default(),
            row.get::<i64>(2).unwrap_or(0),
            row.get::<i64>(3).unwrap_or(0),
        )
    }))
}

/// Get the collegium a user belongs to.
pub async fn get_user_collegium(
    db: &Codex,
    user_id: &str,
) -> Result<Option<(String, String, i64, i64)>> {
    let mut rows = db
        .connection()
        .query(
            "SELECT c.id, c.name, c.lumens, (SELECT COUNT(*) FROM gamify_collegium_members WHERE collegium_id = c.id)
             FROM gamify_collegium c
             JOIN gamify_collegium_members m ON m.collegium_id = c.id
             WHERE m.user_id = ?1",
            params![user_id],
        )
        .await?;
    Ok(rows.next().await?.map(|row| {
        (
            row.get(0).unwrap_or_default(),
            row.get(1).unwrap_or_default(),
            row.get::<i64>(2).unwrap_or(0),
            row.get::<i64>(3).unwrap_or(0),
        )
    }))
}
