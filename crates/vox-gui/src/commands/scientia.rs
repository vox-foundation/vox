//! Typed Scientia-domain read commands (research sessions, publication manifests, queue and
//! cost dashboards), served by the daemon (`GET /api/v2/research/*`, `/api/v2/scientia/*`) so
//! they work against a local or a remote Vox server, plus the push bridges that turn those reads
//! into Tauri events.

use std::sync::Arc;

use tauri::Emitter;
use tauri::State;
pub use vox_orchestrator_mcp::gui_ops::dto::research::{
    DiscoveryInboxDto, PublicationManifestDto, ResearchDetailDto, ResearchSessionDto,
};

use crate::remote::backend::BackendState;
use crate::remote::calls::{BackendCalls, api_data, path_with_query};

pub(crate) async fn list_sessions_impl<B: BackendCalls>(
    b: &B,
    limit: Option<u32>,
) -> Result<Vec<ResearchSessionDto>, String> {
    let path = path_with_query(
        "/research/sessions",
        &[("limit", limit.map(|l| l.to_string()))],
    );
    api_data(b.api_v2_get(&path).await?)
}

pub(crate) async fn session_detail_impl<B: BackendCalls>(
    b: &B,
    session_id: i64,
) -> Result<ResearchDetailDto, String> {
    api_data(
        b.api_v2_get(&format!("/research/sessions/{session_id}"))
            .await?,
    )
}

pub(crate) async fn list_manifests_impl<B: BackendCalls>(
    b: &B,
    limit: Option<u32>,
) -> Result<Vec<PublicationManifestDto>, String> {
    let path = path_with_query(
        "/scientia/manifests",
        &[("limit", limit.map(|l| l.to_string()))],
    );
    api_data(b.api_v2_get(&path).await?)
}

pub(crate) async fn scientia_dashboard_snapshot_impl<B: BackendCalls>(
    b: &B,
) -> Result<vox_scientia::dashboard::QueueSnapshot, String> {
    api_data(b.api_v2_get("/scientia/queue").await?)
}

pub(crate) async fn scientia_cost_rollup_impl<B: BackendCalls>(
    b: &B,
) -> Result<vox_scientia::dashboard::cost::CostRollup, String> {
    api_data(b.api_v2_get("/scientia/cost").await?)
}

#[tauri::command]
pub async fn list_research_sessions(
    backend: State<'_, Arc<BackendState>>,
    limit: Option<u32>,
) -> Result<Vec<ResearchSessionDto>, String> {
    list_sessions_impl(&backend.current().await, limit).await
}

#[tauri::command]
pub async fn get_research_session_detail(
    backend: State<'_, Arc<BackendState>>,
    session_id: i64,
) -> Result<ResearchDetailDto, String> {
    session_detail_impl(&backend.current().await, session_id).await
}

#[tauri::command]
pub async fn list_publication_manifests(
    backend: State<'_, Arc<BackendState>>,
    limit: Option<u32>,
) -> Result<Vec<PublicationManifestDto>, String> {
    list_manifests_impl(&backend.current().await, limit).await
}

/// The `QueueSnapshot` (candidates, claims-pending, retraction queue, stall detection) the
/// server assembles for `vox scientia dashboard`.
#[tauri::command]
pub async fn scientia_dashboard_snapshot(
    backend: State<'_, Arc<BackendState>>,
) -> Result<vox_scientia::dashboard::QueueSnapshot, String> {
    scientia_dashboard_snapshot_impl(&backend.current().await).await
}

/// This quarter's `CostRollup` as reported by `vox scientia cost` on the server.
#[tauri::command]
pub async fn scientia_cost_rollup(
    backend: State<'_, Arc<BackendState>>,
) -> Result<vox_scientia::dashboard::cost::CostRollup, String> {
    scientia_cost_rollup_impl(&backend.current().await).await
}

// ── Live Scientia-queue push bridge (F2) ─────────────────────────────────────

/// Tauri event channel carrying a lightweight "the Scientia queue changed" ping
/// to the UI. The payload is a compact signal object
/// (`{ signal: u64, manifest_count, research_count }`); on receipt the UI
/// refetches via the typed read commands above.
pub const SCIENTIA_QUEUE_EVENT: &str = "vox://scientia-queue";

/// Tauri event channel for newly-surfaced discovery inbox rows (mirrors the
/// orchestrator-mcp `scientia.discovery.surfaced` WS topic at the desktop boundary).
/// The payload is one [`DiscoveryInboxDto`].
pub const SCIENTIA_DISCOVERY_SURFACED_EVENT: &str = "vox://scientia-discovery-surfaced";

/// How often the push bridges sample the backend for a change. The UI keeps its own
/// (longer) interval as a fallback, so this only governs push latency.
const SCIENTIA_POLL_INTERVAL_MS: u64 = crate::config::SCIENTIA_QUEUE_POLL_SECS * 1000;
const SIGNAL_MANIFEST_LIMIT: u32 = 500;
const SIGNAL_SESSION_LIMIT: u32 = 200;
const DISCOVERY_POLL_LIMIT: u32 = 500;

fn fnv1a_mix(mut acc: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        acc ^= b as u64;
        acc = acc.wrapping_mul(0x00000100_000001B3);
    }
    acc
}

/// A compact change signal over the Scientia queue: a hash folded from each publication
/// manifest's `(publication_id, state, updated_at_ms)` plus each research session's
/// `(id, status, finished_at_ms)`. Any add / state transition / timestamp bump flips it.
fn queue_signal(manifests: &[PublicationManifestDto], sessions: &[ResearchSessionDto]) -> u64 {
    let mut acc: u64 = 0xcbf29ce484222325; // FNV offset basis
    for m in manifests {
        acc = fnv1a_mix(acc, m.publication_id.as_bytes());
        acc = fnv1a_mix(acc, m.state.as_bytes());
        acc = fnv1a_mix(acc, &m.updated_at_ms.to_le_bytes());
    }
    for s in sessions {
        acc = fnv1a_mix(acc, &s.id.to_le_bytes());
        acc = fnv1a_mix(acc, s.status.as_bytes());
        acc = fnv1a_mix(acc, &s.finished_at_ms.unwrap_or(-1_i64).to_le_bytes());
    }
    acc
}

/// `(signal, manifest_count, research_count)` for the current backend.
async fn scientia_queue_signal<B: BackendCalls>(b: &B) -> Result<(u64, usize, usize), String> {
    let manifests = list_manifests_impl(b, Some(SIGNAL_MANIFEST_LIMIT)).await?;
    let sessions = list_sessions_impl(b, Some(SIGNAL_SESSION_LIMIT)).await?;
    Ok((
        queue_signal(&manifests, &sessions),
        manifests.len(),
        sessions.len(),
    ))
}

/// Rows newer than `last_max` in ascending id order, plus the new high-water mark. The first
/// observation (`last_max == None`) only records the mark, so pre-existing rows never fire.
fn new_discoveries(
    mut rows: Vec<DiscoveryInboxDto>,
    last_max: Option<i64>,
) -> (Vec<DiscoveryInboxDto>, Option<i64>) {
    let seen_max = rows.iter().map(|r| r.id).max();
    let Some(prev) = last_max else {
        return (Vec::new(), Some(seen_max.unwrap_or(0)));
    };
    rows.retain(|r| r.id > prev);
    rows.sort_by_key(|r| r.id);
    (rows, Some(seen_max.map_or(prev, |m| m.max(prev))))
}

/// Spawn a background task that polls the backend for Scientia-queue changes and emits a
/// [`SCIENTIA_QUEUE_EVENT`] ping when the queue signal flips. Resilient by design: a backend
/// error is logged and retried on the next tick; the task never crashes the app.
// toestub-ignore(skeleton/untested-pub-api) — spawns a background poll task bridging Scientia-queue changes to Tauri events; the signal fold is unit-tested
pub fn spawn_scientia_queue_stream(app_handle: tauri::AppHandle, backend: Arc<BackendState>) {
    tokio::spawn(async move {
        let mut last_signal: Option<u64> = None;
        loop {
            match scientia_queue_signal(&backend.current().await).await {
                Ok((signal, manifest_count, research_count)) => {
                    if last_signal != Some(signal) {
                        last_signal = Some(signal);
                        let _ = app_handle.emit(
                            SCIENTIA_QUEUE_EVENT,
                            serde_json::json!({
                                "signal": signal,
                                "manifest_count": manifest_count,
                                "research_count": research_count,
                            }),
                        );
                    }
                }
                Err(e) => tracing::debug!("scientia queue signal failed: {e}"),
            }
            tokio::time::sleep(std::time::Duration::from_millis(SCIENTIA_POLL_INTERVAL_MS)).await;
        }
    });
}

/// Spawn a background task that polls the discovery inbox and emits
/// [`SCIENTIA_DISCOVERY_SURFACED_EVENT`] for each newly-surfaced candidate.
// toestub-ignore(skeleton/untested-pub-api) — spawns a background poll task; the diff logic is unit-tested
pub fn spawn_discovery_surfaced_stream(app_handle: tauri::AppHandle, backend: Arc<BackendState>) {
    tokio::spawn(async move {
        let mut last_max: Option<i64> = None;
        loop {
            let path = format!("/scientia/discoveries?limit={DISCOVERY_POLL_LIMIT}");
            let polled = match backend.current().await.api_v2_get(&path).await {
                Ok(envelope) => api_data::<Vec<DiscoveryInboxDto>>(envelope),
                Err(e) => Err(e),
            };
            match polled {
                Ok(rows) => {
                    let (fresh, next) = new_discoveries(rows, last_max);
                    last_max = next;
                    for row in &fresh {
                        let _ = app_handle.emit(SCIENTIA_DISCOVERY_SURFACED_EVENT, row);
                    }
                }
                Err(e) => tracing::debug!("discovery surfaced poll failed: {e}"),
            }
            tokio::time::sleep(std::time::Duration::from_millis(SCIENTIA_POLL_INTERVAL_MS)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::remote::calls::fake::FakeBackend;

    fn manifest(id: &str, state: &str, ts: i64) -> PublicationManifestDto {
        PublicationManifestDto {
            publication_id: id.into(),
            content_type: "finding".into(),
            state: state.into(),
            created_at_ms: 1,
            updated_at_ms: ts,
        }
    }

    fn discovery(id: i64) -> DiscoveryInboxDto {
        DiscoveryInboxDto {
            id,
            publication_id: format!("pub-{id}"),
            surfaced_at_ms: id,
            intake_tier: "auto".into(),
            signal_codes: vec![],
            origin: "commit_watcher".into(),
        }
    }

    #[test]
    fn queue_signal_is_deterministic_and_state_sensitive() {
        let a = queue_signal(&[manifest("pub-1", "draft", 100)], &[]);
        let b = queue_signal(&[manifest("pub-1", "draft", 100)], &[]);
        let c = queue_signal(&[manifest("pub-1", "approved", 100)], &[]);
        assert_eq!(a, b, "same tuples -> same signal");
        assert_ne!(a, c, "a state transition flips the signal");
    }

    #[test]
    fn discovery_diff_primes_silently_then_emits_only_newer_rows_ascending() {
        let (fresh, mark) = new_discoveries(vec![discovery(3), discovery(1)], None);
        assert!(fresh.is_empty());
        assert_eq!(mark, Some(3));
        let (fresh, mark) = new_discoveries(vec![discovery(5), discovery(4), discovery(3)], mark);
        assert_eq!(fresh.iter().map(|r| r.id).collect::<Vec<_>>(), vec![4, 5]);
        assert_eq!(mark, Some(5));
        let (fresh, mark) = new_discoveries(vec![], mark);
        assert!(fresh.is_empty());
        assert_eq!(mark, Some(5), "an empty poll keeps the high-water mark");
    }

    #[tokio::test]
    async fn reads_go_to_the_research_and_scientia_routes() {
        let session = json!({ "id": 7, "status": "done", "query_text": "q", "started_at_ms": 1, "finished_at_ms": 2 });
        let b = FakeBackend::new()
            .on_get(
                "/research/sessions?limit=5",
                Ok(json!({ "v": 1, "data": [session.clone()] })),
            )
            .on_get(
                "/research/sessions/7",
                Ok(json!({ "v": 1, "data": {
                    "session": session, "report_markdown": null, "artifact_json": null,
                    "confidence_tier": null, "claims": [], "source_count": null, "citation_precision": null
                }})),
            )
            .on_get(
                "/scientia/manifests?limit=9",
                Ok(json!({ "v": 1, "data": [{
                    "publication_id": "pub-1", "content_type": "finding", "state": "draft",
                    "created_at_ms": 1, "updated_at_ms": 2
                }]})),
            );
        assert_eq!(list_sessions_impl(&b, Some(5)).await.unwrap()[0].id, 7);
        assert_eq!(
            session_detail_impl(&b, 7).await.unwrap().session.status,
            "done"
        );
        assert_eq!(
            list_manifests_impl(&b, Some(9)).await.unwrap()[0].publication_id,
            "pub-1"
        );
    }

    #[tokio::test]
    async fn dashboards_pass_through_the_existing_queue_and_cost_routes() {
        let b = FakeBackend::new()
            .on_get("/scientia/queue", Err("down".into()))
            .on_get("/scientia/cost", Err("down".into()));
        assert!(scientia_dashboard_snapshot_impl(&b).await.is_err());
        assert!(scientia_cost_rollup_impl(&b).await.is_err());
        let keys: Vec<String> = b.calls().into_iter().map(|(k, _)| k).collect();
        assert_eq!(keys, vec!["get:/scientia/queue", "get:/scientia/cost"]);
    }
}
