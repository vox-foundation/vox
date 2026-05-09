//! User counters.

use anyhow::Result;
use turso::params;
use vox_db::Codex;

/// Get a specific counter for a user.
pub async fn get_counter(db: &Codex, user_id: &str, name: &str) -> Result<u32> {
    let mut rows = db
        .connection()
        .query(
            "SELECT count FROM gamify_counters WHERE user_id=?1 AND name=?2",
            params![user_id, name],
        )
        .await?;
    let v = rows
        .next()
        .await?
        .map(|r| r.get::<i64>(0).unwrap_or(0))
        .unwrap_or(0);
    Ok(v.max(0) as u32)
}

/// Increment a counter and return the new value.
pub async fn increment_counter(db: &Codex, user_id: &str, name: &str, amount: u32) -> Result<u32> {
    if amount == 0 {
        return get_counter(db, user_id, name).await;
    }
    let delta = amount as i64;
    let user_id_s = user_id.to_string();
    let name_s = name.to_string();
    let breaker = db.breaker().clone();
    let conn = db.connection().clone();
    breaker
        .call(|| async move {
            conn.execute(
                "INSERT INTO gamify_counters (user_id, name, count) VALUES (?1, ?2, ?3)
                 ON CONFLICT(user_id, name) DO UPDATE SET count=count+excluded.count",
                params![user_id_s.as_str(), name_s.as_str(), delta],
            )
            .await?;
            Ok::<(), vox_db::StoreError>(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    get_counter(db, user_id, name).await
}

/// Set a counter to a specific value.
pub async fn set_counter(db: &Codex, user_id: &str, name: &str, value: u32) -> Result<()> {
    let user_id = user_id.to_string();
    let name = name.to_string();
    let v = value as i64;
    let breaker = db.breaker().clone();
    let conn = db.connection().clone();
    breaker
        .call(|| async move {
            conn.execute(
                "INSERT INTO gamify_counters (user_id, name, count) VALUES (?1, ?2, ?3)
                 ON CONFLICT(user_id, name) DO UPDATE SET count=excluded.count",
                params![user_id.as_str(), name.as_str(), v],
            )
            .await?;
            Ok::<(), vox_db::StoreError>(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}
