//! Cross-process idempotency for orchestrator-driven Ludus events.

use anyhow::Result;
use turso::params;
use vox_db::Codex;

/// Returns `true` if this `(user_id, dedupe_key)` was newly inserted (first time).
///
/// Duplicate keys return `false` so callers can skip reward processing for replays.
pub async fn try_claim_processed_event(
    db: &Codex,
    user_id: &str,
    dedupe_key: &str,
) -> Result<bool> {
    let user_id = user_id.to_string();
    let dedupe_key = dedupe_key.to_string();
    let breaker = db.breaker().clone();
    let conn = db.connection().clone();
    breaker
        .call(|| async move {
            let n = conn
                .execute(
                    "INSERT OR IGNORE INTO gamify_processed_events (user_id, dedupe_key) VALUES (?1, ?2)",
                    params![user_id.as_str(), dedupe_key.as_str()],
                )
                .await?;
            Ok::<_, vox_db::StoreError>(n > 0)
        })
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}
