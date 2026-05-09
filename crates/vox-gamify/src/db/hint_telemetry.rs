//! Hint delivery telemetry (shown / suppressed / dismissed).

use anyhow::Result;
use turso::params;
use vox_db::Codex;

/// Log a hint-related action for KPI analysis (`action`: shown, dismissed, suppressed, acted).
pub async fn log_hint_event(
    db: &Codex,
    user_id: &str,
    kind: &str,
    action: &str,
    reason: Option<&str>,
) -> Result<()> {
    let user_id = user_id.to_string();
    let kind = kind.to_string();
    let action = action.to_string();
    let reason = reason.unwrap_or("").to_string();
    let breaker = db.breaker().clone();
    let conn = db.connection().clone();
    breaker
        .call(|| async move {
            conn.execute(
                "INSERT INTO gamify_hint_telemetry (user_id, kind, action, reason)
                 VALUES (?1, ?2, ?3, ?4)",
                params![user_id.as_str(), kind.as_str(), action.as_str(), reason.as_str()],
            )
            .await?;
            Ok::<(), vox_db::StoreError>(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}
