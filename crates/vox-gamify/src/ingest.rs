//! Canonical Ludus event ingestion contract.
//!
//! All producers (CLI, MCP, tests) should send normalized JSON payloads with a `type` field
//! matching [`crate::reward_policy`] snake_case event ids. Prefer [`crate::event_router::route_event`]
//! or [`crate::event_router::route_event_auto_user`] as the implementation of this contract.
//!
//! **Mesh / Populi:** correlating remote task outcomes with Ludus events is a separate design (see
//! `docs/src/reference/populi.md`); ingestion helpers here do not special-case A2A payloads.

use anyhow::Result;
use serde_json::Value;
use vox_db::Codex;

use crate::reward_policy::RouteResult;

/// Max serialized JSON size for routed Ludus events (protects Codex / agent_events rows).
pub const MAX_LUDUS_EVENT_PAYLOAD_BYTES: usize = 262_144;

/// Required shape: JSON object with string `type` (snake_case event id); size cap.
pub fn validate_event_payload(event_json: &Value) -> Result<()> {
    let t = event_json
        .get("type")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty());
    if t.is_none() {
        anyhow::bail!("ludus event missing non-empty \"type\" field");
    }
    let len = serde_json::to_string(event_json)
        .map(|s| s.len())
        .unwrap_or(usize::MAX);
    if len > MAX_LUDUS_EVENT_PAYLOAD_BYTES {
        anyhow::bail!(
            "ludus event JSON exceeds {} bytes (got {})",
            MAX_LUDUS_EVENT_PAYLOAD_BYTES,
            len
        );
    }
    Ok(())
}

/// Canonical ingestion: validates, then routes through the full Ludus pipeline.
pub async fn ingest_orchestrator_event(
    db: &Codex,
    user_id: &str,
    event_json: &Value,
) -> Result<RouteResult> {
    validate_event_payload(event_json)?;
    crate::event_router::route_event(db, user_id, event_json).await
}
