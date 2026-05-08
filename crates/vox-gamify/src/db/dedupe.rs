//! Cross-process idempotency for orchestrator-driven Ludus events.

use anyhow::Result;
use vox_db::Codex;

/// Returns `true` if this `(user_id, dedupe_key)` was newly inserted (first time).
///
/// Duplicate keys return `false` so callers can skip reward processing for replays.
pub async fn try_claim_processed_event(
    db: &Codex,
    user_id: &str,
    dedupe_key: &str,
) -> Result<bool> {
    db.try_claim_gamify_processed_event(user_id, dedupe_key)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}
