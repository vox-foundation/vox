//! Optional populi control-plane **join** + background **heartbeat** when env points at an HTTP base.
//!
//! Used by **`vox-mcp`** and **`vox run`** (feature **`transport`**). See `docs/src/architecture/populi-ssot.md`.

use std::time::Duration;

use crate::http_client::PopuliHttpClient;
use crate::{NodeRecord, PopuliRegistryError};

/// Result of [`populi_http_join_best_effort`].
#[derive(Debug)]
pub enum PopuliHttpJoinSpawnOutcome {
    /// `VOX_MESH_HTTP_JOIN` disabled, or no suitable control URL in env.
    Skipped,
    /// `POST /v1/populi/join` succeeded; heartbeat loop spawned when the heartbeat interval is non-zero.
    Joined {
        /// Normalized control plane origin (no trailing slash).
        base: String,
        /// Node id returned / used for join.
        node_id: String,
    },
    /// Join HTTP call failed (URL was configured).
    Failed {
        /// Normalized control plane origin used for the request.
        base: String,
        /// Node id from the join payload.
        node_id: String,
        /// Transport or HTTP-layer error from the client.
        err: PopuliRegistryError,
    },
}

/// `VOX_MESH_HTTP_JOIN` `0` / `false` disables join and heartbeat.
#[must_use]
pub fn populi_http_join_disabled_from_env() -> bool {
    std::env::var("VOX_MESH_HTTP_JOIN")
        .map(|v| {
            let v = v.trim();
            v == "0" || v.eq_ignore_ascii_case("false")
        })
        .unwrap_or(false)
}

/// First non-empty URL from **`VOX_ORCHESTRATOR_MESH_CONTROL_URL`** then **`VOX_MESH_CONTROL_ADDR`**, normalized for clients.
#[must_use]
pub fn populi_http_control_base_from_env() -> Option<String> {
    for var in ["VOX_ORCHESTRATOR_MESH_CONTROL_URL", "VOX_MESH_CONTROL_ADDR"] {
        if let Ok(v) = std::env::var(var) {
            let t = v.trim();
            if !t.is_empty()
                && let Some(b) = crate::normalize_http_control_base(t)
            {
                return Some(b);
            }
        }
    }
    None
}

/// Request timeout for populi HTTP client (**`VOX_ORCHESTRATOR_MESH_HTTP_TIMEOUT_MS`**, min 500, default 15000).
#[must_use]
pub fn populi_http_timeout_ms_from_env() -> u64 {
    std::env::var("VOX_ORCHESTRATOR_MESH_HTTP_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .filter(|n| *n >= 500)
        .unwrap_or(15_000)
}

/// Heartbeat interval (**`VOX_MESH_HTTP_HEARTBEAT_SECS`**, default 30; `0` = join only).
#[must_use]
pub fn populi_heartbeat_interval_secs_from_env() -> u64 {
    std::env::var("VOX_MESH_HTTP_HEARTBEAT_SECS")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(30)
}

async fn populi_http_heartbeat_loop(
    base: String,
    mut record: NodeRecord,
    timeout_ms: u64,
    interval_secs: u64,
    component: &'static str,
) {
    let mut tick = tokio::time::interval(Duration::from_secs(interval_secs.max(1)));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    tick.tick().await;
    loop {
        tick.tick().await;
        let client = PopuliHttpClient::new_with_timeout(&base, Duration::from_millis(timeout_ms))
            .with_env_token();
        match client.heartbeat(&record).await {
            Ok(u) => {
                record = u;
            }
            Err(e) => {
                tracing::debug!(
                    target: "vox.populi",
                    error = %e,
                    component,
                    "populi HTTP heartbeat failed (best-effort)"
                );
            }
        }
    }
}

/// `POST /v1/populi/join` and optionally spawn `POST /v1/populi/heartbeat` loop.
pub async fn populi_http_join_best_effort(
    record: NodeRecord,
    component: &'static str,
) -> PopuliHttpJoinSpawnOutcome {
    if populi_http_join_disabled_from_env() {
        return PopuliHttpJoinSpawnOutcome::Skipped;
    }
    let Some(base) = populi_http_control_base_from_env() else {
        return PopuliHttpJoinSpawnOutcome::Skipped;
    };
    let node_id = record.id.clone();
    let timeout_ms = populi_http_timeout_ms_from_env();
    let client =
        PopuliHttpClient::new_with_timeout(&base, Duration::from_millis(timeout_ms)).with_env_token();
    match client.join(&record).await {
        Ok(updated) => {
            tracing::info!(
                target: "vox.populi",
                node_id = %updated.id,
                control_base = %base,
                component,
                "populi HTTP join"
            );
            let secs = populi_heartbeat_interval_secs_from_env();
            if secs > 0 {
                let base_clone = base.clone();
                tokio::spawn(populi_http_heartbeat_loop(
                    base_clone,
                    updated.clone(),
                    timeout_ms,
                    secs,
                    component,
                ));
            }
            PopuliHttpJoinSpawnOutcome::Joined {
                base,
                node_id: updated.id,
            }
        }
        Err(e) => {
            tracing::debug!(
                target: "vox.populi",
                error = %e,
                control_base = %base,
                component,
                "populi HTTP join failed (best-effort)"
            );
            PopuliHttpJoinSpawnOutcome::Failed {
                base,
                node_id,
                err: e,
            }
        }
    }
}
