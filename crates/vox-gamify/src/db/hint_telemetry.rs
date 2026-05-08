//! Hint delivery telemetry (shown / suppressed / dismissed).

use anyhow::Result;
use vox_db::Codex;

/// Log a hint-related action for KPI analysis (`action`: shown, dismissed, suppressed, acted).
pub async fn log_hint_event(
    db: &Codex,
    user_id: &str,
    kind: &str,
    action: &str,
    reason: Option<&str>,
) -> Result<()> {
    db.insert_gamify_hint_telemetry(user_id, kind, action, reason)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}
