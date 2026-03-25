//! User counters.

use anyhow::Result;
use turso::params;
use vox_db::Codex;

/// Get a specific counter for a user.
pub async fn get_counter(db: &Codex, user_id: &str, name: &str) -> Result<u32> {
    let mut rows = db
        .connection()
        .query(
            "SELECT count FROM gamify_counters WHERE user_id = ?1 AND counter_name = ?2",
            params![user_id, name],
        )
        .await?;
    if let Some(row) = rows.next().await? {
        Ok(row.get::<i64>(0)? as u32)
    } else {
        Ok(0)
    }
}

/// Increment a counter and return the new value.
pub async fn increment_counter(db: &Codex, user_id: &str, name: &str, amount: u32) -> Result<u32> {
    db.connection()
        .execute(
            "INSERT INTO gamify_counters (user_id, counter_name, count)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(user_id, counter_name) DO UPDATE SET
            count = count + excluded.count",
            params![user_id, name, amount as i64],
        )
        .await?;
    get_counter(db, user_id, name).await
}

/// Set a counter to a specific value.
pub async fn set_counter(db: &Codex, user_id: &str, name: &str, value: u32) -> Result<()> {
    db.connection()
        .execute(
            "INSERT INTO gamify_counters (user_id, counter_name, count)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(user_id, counter_name) DO UPDATE SET
            count = excluded.count",
            params![user_id, name, value as i64],
        )
        .await?;
    Ok(())
}
