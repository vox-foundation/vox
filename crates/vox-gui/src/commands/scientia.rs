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
use crate::remote::calls::{BackendCalls, api_data, encode_component, path_with_query};

fn publication_path(publication_id: &str, suffix: &str) -> String {
    format!(
        "/scientia/publications/{}/{suffix}",
        encode_component(publication_id)
    )
}

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

pub(crate) async fn promote_scientia_candidate_impl<B: BackendCalls>(
    b: &B,
    candidate_id: &str,
    venue: Option<&str>,
) -> Result<serde_json::Value, String> {
    let base = format!(
        "/scientia/candidates/{}/promote",
        encode_component(candidate_id)
    );
    let path = path_with_query(
        &base,
        &[("venue", venue.filter(|s| !s.is_empty()).map(str::to_string))],
    );
    api_data(b.api_v2_get(&path).await?)
}

pub(crate) async fn scientia_publication_manuscript_impl<B: BackendCalls>(
    b: &B,
    publication_id: &str,
) -> Result<serde_json::Value, String> {
    api_data(
        b.api_v2_get(&publication_path(publication_id, "manuscript"))
            .await?,
    )
}

pub(crate) async fn scientia_publication_latex_impl<B: BackendCalls>(
    b: &B,
    publication_id: &str,
) -> Result<serde_json::Value, String> {
    api_data(
        b.api_v2_get(&publication_path(publication_id, "latex"))
            .await?,
    )
}

pub(crate) async fn scientia_publication_finding_page_impl<B: BackendCalls>(
    b: &B,
    publication_id: &str,
) -> Result<serde_json::Value, String> {
    api_data(
        b.api_v2_get(&publication_path(publication_id, "finding-page"))
            .await?,
    )
}

pub(crate) async fn scientia_publication_extract_claims_impl<B: BackendCalls>(
    b: &B,
    publication_id: &str,
) -> Result<serde_json::Value, String> {
    api_data(
        b.api_v2_get(&publication_path(publication_id, "extract-claims"))
            .await?,
    )
}

pub(crate) async fn scientia_publication_replay_impl<B: BackendCalls>(
    b: &B,
    publication_id: &str,
    measured_score: Option<f64>,
) -> Result<serde_json::Value, String> {
    let base = publication_path(publication_id, "replay");
    let path = path_with_query(
        &base,
        &[("measured_score", measured_score.map(|s| s.to_string()))],
    );
    api_data(b.api_v2_get(&path).await?)
}

pub(crate) async fn scientia_publication_claims_impl<B: BackendCalls>(
    b: &B,
    publication_id: &str,
) -> Result<serde_json::Value, String> {
    api_data(
        b.api_v2_get(&publication_path(publication_id, "claims"))
            .await?,
    )
}

pub(crate) async fn scientia_publication_venue_recommend_impl<B: BackendCalls>(
    b: &B,
    candidate_class: &str,
) -> Result<serde_json::Value, String> {
    let path = path_with_query(
        "/scientia/venue-recommend",
        &[("candidate_class", Some(candidate_class.to_string()))],
    );
    api_data(b.api_v2_get(&path).await?)
}

pub(crate) async fn scientia_publication_flywheel_advance_impl<B: BackendCalls>(
    b: &B,
    publication_id: &str,
    target_stage: Option<&str>,
) -> Result<serde_json::Value, String> {
    let base = publication_path(publication_id, "flywheel-advance");
    let path = path_with_query(
        &base,
        &[(
            "target_stage",
            target_stage.filter(|s| !s.is_empty()).map(str::to_string),
        )],
    );
    api_data(b.api_v2_get(&path).await?)
}

pub(crate) async fn scientia_publication_arxiv_bundle_impl<B: BackendCalls>(
    b: &B,
    publication_id: &str,
) -> Result<serde_json::Value, String> {
    api_data(
        b.api_v2_get(&publication_path(publication_id, "arxiv-bundle"))
            .await?,
    )
}

pub(crate) async fn scientia_publication_arxiv_readiness_impl<B: BackendCalls>(
    b: &B,
    publication_id: &str,
) -> Result<serde_json::Value, String> {
    api_data(
        b.api_v2_get(&publication_path(publication_id, "arxiv-readiness"))
            .await?,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn scientia_publication_classify_impl<B: BackendCalls>(
    b: &B,
    publication_id: &str,
    candidate_class: Option<&str>,
    publication_tier: Option<&str>,
    arxiv_primary_category: Option<&str>,
    arxiv_secondary_categories: Option<&str>,
    license_spdx: Option<&str>,
    orcid: Option<&str>,
    ror_affiliation: Option<&str>,
    prereserved_doi: Option<&str>,
    arxiv_id: Option<&str>,
    autofill_imrad: Option<bool>,
) -> Result<serde_json::Value, String> {
    let base = publication_path(publication_id, "classify");
    let clean = |v: Option<&str>| {
        v.map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
    };
    let path = path_with_query(
        &base,
        &[
            ("candidate_class", clean(candidate_class)),
            ("publication_tier", clean(publication_tier)),
            ("arxiv_primary_category", clean(arxiv_primary_category)),
            (
                "arxiv_secondary_categories",
                clean(arxiv_secondary_categories),
            ),
            ("license_spdx", clean(license_spdx)),
            ("orcid", clean(orcid)),
            ("ror_affiliation", clean(ror_affiliation)),
            ("prereserved_doi", clean(prereserved_doi)),
            ("arxiv_id", clean(arxiv_id)),
            ("autofill_imrad", autofill_imrad.map(|v| v.to_string())),
        ],
    );
    api_data(b.api_v2_get(&path).await?)
}

pub(crate) async fn scientia_publication_stage_advance_impl<B: BackendCalls>(
    b: &B,
    publication_id: &str,
    target_stage: Option<&str>,
    orcid: Option<&str>,
    prereserved_doi: Option<&str>,
    arxiv_id: Option<&str>,
    autofill_imrad: Option<bool>,
) -> Result<serde_json::Value, String> {
    let base = publication_path(publication_id, "stage-advance");
    let clean = |v: Option<&str>| {
        v.map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
    };
    let path = path_with_query(
        &base,
        &[
            ("target_stage", clean(target_stage)),
            ("orcid", clean(orcid)),
            ("prereserved_doi", clean(prereserved_doi)),
            ("arxiv_id", clean(arxiv_id)),
            ("autofill_imrad", autofill_imrad.map(|v| v.to_string())),
        ],
    );
    api_data(b.api_v2_get(&path).await?)
}

pub(crate) async fn scientia_v76_audit_diff_impl<B: BackendCalls>(
    b: &B,
) -> Result<serde_json::Value, String> {
    api_data(b.api_v2_get("/scientia/v76-audit-diff").await?)
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

#[tauri::command]
pub async fn promote_scientia_candidate(
    backend: State<'_, Arc<BackendState>>,
    candidate_id: String,
    venue: Option<String>,
) -> Result<serde_json::Value, String> {
    promote_scientia_candidate_impl(&backend.current().await, &candidate_id, venue.as_deref()).await
}

#[tauri::command]
pub async fn scientia_publication_manuscript(
    backend: State<'_, Arc<BackendState>>,
    publication_id: String,
) -> Result<serde_json::Value, String> {
    scientia_publication_manuscript_impl(&backend.current().await, &publication_id).await
}

#[tauri::command]
pub async fn scientia_publication_latex(
    backend: State<'_, Arc<BackendState>>,
    publication_id: String,
) -> Result<serde_json::Value, String> {
    scientia_publication_latex_impl(&backend.current().await, &publication_id).await
}

#[tauri::command]
pub async fn scientia_publication_finding_page(
    backend: State<'_, Arc<BackendState>>,
    publication_id: String,
) -> Result<serde_json::Value, String> {
    scientia_publication_finding_page_impl(&backend.current().await, &publication_id).await
}

#[tauri::command]
pub async fn scientia_publication_extract_claims(
    backend: State<'_, Arc<BackendState>>,
    publication_id: String,
) -> Result<serde_json::Value, String> {
    scientia_publication_extract_claims_impl(&backend.current().await, &publication_id).await
}

#[tauri::command]
pub async fn scientia_publication_replay(
    backend: State<'_, Arc<BackendState>>,
    publication_id: String,
    measured_score: Option<f64>,
) -> Result<serde_json::Value, String> {
    scientia_publication_replay_impl(&backend.current().await, &publication_id, measured_score)
        .await
}

#[tauri::command]
pub async fn scientia_publication_claims(
    backend: State<'_, Arc<BackendState>>,
    publication_id: String,
) -> Result<serde_json::Value, String> {
    scientia_publication_claims_impl(&backend.current().await, &publication_id).await
}

#[tauri::command]
pub async fn scientia_publication_venue_recommend(
    backend: State<'_, Arc<BackendState>>,
    candidate_class: String,
) -> Result<serde_json::Value, String> {
    scientia_publication_venue_recommend_impl(&backend.current().await, &candidate_class).await
}

#[tauri::command]
pub async fn scientia_publication_flywheel_advance(
    backend: State<'_, Arc<BackendState>>,
    publication_id: String,
    target_stage: Option<String>,
) -> Result<serde_json::Value, String> {
    scientia_publication_flywheel_advance_impl(
        &backend.current().await,
        &publication_id,
        target_stage.as_deref(),
    )
    .await
}

#[tauri::command]
pub async fn scientia_publication_arxiv_bundle(
    backend: State<'_, Arc<BackendState>>,
    publication_id: String,
) -> Result<serde_json::Value, String> {
    scientia_publication_arxiv_bundle_impl(&backend.current().await, &publication_id).await
}

#[tauri::command]
pub async fn scientia_publication_arxiv_readiness(
    backend: State<'_, Arc<BackendState>>,
    publication_id: String,
) -> Result<serde_json::Value, String> {
    scientia_publication_arxiv_readiness_impl(&backend.current().await, &publication_id).await
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn scientia_publication_classify(
    backend: State<'_, Arc<BackendState>>,
    publication_id: String,
    candidate_class: Option<String>,
    publication_tier: Option<String>,
    arxiv_primary_category: Option<String>,
    arxiv_secondary_categories: Option<String>,
    license_spdx: Option<String>,
    orcid: Option<String>,
    ror_affiliation: Option<String>,
    prereserved_doi: Option<String>,
    arxiv_id: Option<String>,
    autofill_imrad: Option<bool>,
) -> Result<serde_json::Value, String> {
    scientia_publication_classify_impl(
        &backend.current().await,
        &publication_id,
        candidate_class.as_deref(),
        publication_tier.as_deref(),
        arxiv_primary_category.as_deref(),
        arxiv_secondary_categories.as_deref(),
        license_spdx.as_deref(),
        orcid.as_deref(),
        ror_affiliation.as_deref(),
        prereserved_doi.as_deref(),
        arxiv_id.as_deref(),
        autofill_imrad,
    )
    .await
}

#[tauri::command]
pub async fn scientia_publication_stage_advance(
    backend: State<'_, Arc<BackendState>>,
    publication_id: String,
    target_stage: Option<String>,
    orcid: Option<String>,
    prereserved_doi: Option<String>,
    arxiv_id: Option<String>,
    autofill_imrad: Option<bool>,
) -> Result<serde_json::Value, String> {
    scientia_publication_stage_advance_impl(
        &backend.current().await,
        &publication_id,
        target_stage.as_deref(),
        orcid.as_deref(),
        prereserved_doi.as_deref(),
        arxiv_id.as_deref(),
        autofill_imrad,
    )
    .await
}

#[tauri::command]
pub async fn scientia_v76_audit_diff(
    backend: State<'_, Arc<BackendState>>,
) -> Result<serde_json::Value, String> {
    scientia_v76_audit_diff_impl(&backend.current().await).await
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
            title: None,
            author: None,
            abstract_text: None,
            candidate_class: None,
            publication_tier: None,
            arxiv_primary_category: None,
            arxiv_secondary_categories: None,
            license_spdx: None,
            orcid: None,
            ror_affiliation: None,
            prereserved_doi: None,
            arxiv_id: None,
            readiness_score: None,
            arxiv_ready: None,
            flywheel_stages_done: None,
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

    #[tokio::test]
    async fn studio_publication_pipeline_commands_encode_paths_and_hit_daemon_routes() {
        let b = FakeBackend::new()
            .on_get(
                "/scientia/candidates/cand%2F1/promote?venue=neurips_workshop",
                Ok(json!({ "v": 1, "data": { "ok": true, "publication_id": "pub-1" } })),
            )
            .on_get(
                "/scientia/publications/pub%2F1/manuscript",
                Ok(json!({ "v": 1, "data": { "publication_id": "pub/1", "markdown": "# IMRaD" } })),
            )
            .on_get(
                "/scientia/publications/pub%2F1/latex",
                Ok(json!({ "v": 1, "data": { "publication_id": "pub/1", "latex": "\\documentclass{article}" } })),
            )
            .on_get(
                "/scientia/publications/pub%2F1/finding-page",
                Ok(json!({ "v": 1, "data": { "publication_id": "pub/1", "html": "<html></html>" } })),
            )
            .on_get(
                "/scientia/publications/pub%2F1/extract-claims",
                Ok(json!({ "v": 1, "data": { "publication_id": "pub/1", "total_atomic": 2, "supported": 2 } })),
            )
            .on_get(
                "/scientia/publications/pub%2F1/replay?measured_score=0.95",
                Ok(json!({ "v": 1, "data": { "publication_id": "pub/1", "artifact_replayability": 0.95 } })),
            )
            .on_get(
                "/scientia/publications/pub%2F1/claims",
                Ok(json!({ "v": 1, "data": { "publication_id": "pub/1", "session_id": 1, "claims": [] } })),
            )
            .on_get(
                "/scientia/venue-recommend?candidate_class=benchmark_report",
                Ok(json!({ "v": 1, "data": { "candidate_class": "benchmark_report", "recommended_venues": ["arxiv_cs_se"] } })),
            )
            .on_get(
                "/scientia/publications/pub%2F1/flywheel-advance?target_stage=replay_verify",
                Ok(json!({ "v": 1, "data": { "publication_id": "pub/1", "stage_completed": "replay_verify", "stages_done": ["extract_claims", "replay_verify"] } })),
            );

        let promoted = promote_scientia_candidate_impl(&b, "cand/1", Some("neurips_workshop"))
            .await
            .unwrap();
        assert_eq!(promoted["publication_id"], "pub-1");

        let ms = scientia_publication_manuscript_impl(&b, "pub/1")
            .await
            .unwrap();
        assert_eq!(ms["markdown"], "# IMRaD");

        let tex = scientia_publication_latex_impl(&b, "pub/1").await.unwrap();
        assert_eq!(tex["latex"], "\\documentclass{article}");

        let fp = scientia_publication_finding_page_impl(&b, "pub/1")
            .await
            .unwrap();
        assert_eq!(fp["html"], "<html></html>");

        let ext = scientia_publication_extract_claims_impl(&b, "pub/1")
            .await
            .unwrap();
        assert_eq!(ext["total_atomic"], 2);

        let rep = scientia_publication_replay_impl(&b, "pub/1", Some(0.95))
            .await
            .unwrap();
        assert_eq!(rep["artifact_replayability"], 0.95);

        let claims = scientia_publication_claims_impl(&b, "pub/1").await.unwrap();
        assert_eq!(claims["publication_id"], "pub/1");

        let venue = scientia_publication_venue_recommend_impl(&b, "benchmark_report")
            .await
            .unwrap();
        assert_eq!(venue["recommended_venues"][0], "arxiv_cs_se");

        let fw = scientia_publication_flywheel_advance_impl(&b, "pub/1", Some("replay_verify"))
            .await
            .unwrap();
        assert_eq!(fw["stage_completed"], "replay_verify");
    }

    #[tokio::test]
    async fn v76_audit_diff_command_hits_daemon_route() {
        let b = FakeBackend::new().on_get(
            "/scientia/v76-audit-diff",
            Ok(json!({
                "v": 1,
                "data": {
                    "before_stage": "before",
                    "after_stage": "after",
                    "before_path": "contracts/reports/scientia-v76-before-baseline.v1.json",
                    "after_path": "contracts/reports/scientia-v76-after-closure.v1.json",
                    "after_available": true,
                    "axes": [],
                    "bugs": [],
                    "hunks": [],
                    "lessons_summary": {
                        "quarantined_findings_count": 3,
                        "corroborated_promotions_count": 1,
                        "proposed_effort_rules_count": 2,
                        "auto_dpo_pairs_minted": 3,
                        "unreviewed_ssot_mutations": 0
                    }
                }
            })),
        );
        let diff = scientia_v76_audit_diff_impl(&b).await.unwrap();
        assert_eq!(diff["before_stage"], "before");
        assert_eq!(diff["after_stage"], "after");
        assert_eq!(diff["after_available"], true);
        assert_eq!(diff["lessons_summary"]["unreviewed_ssot_mutations"], 0);
    }

    #[tokio::test]
    async fn arxiv_readiness_classify_stage_advance_and_bundle_hit_daemon_routes() {
        let b = FakeBackend::new()
            .on_get(
                "/scientia/publications/pub%2F1/arxiv-readiness",
                Ok(json!({ "v": 1, "data": { "publication_id": "pub/1", "readiness_score": 91, "arxiv_ready": true } })),
            )
            .on_get(
                "/scientia/publications/pub%2F1/classify?candidate_class=algorithmic_improvement&publication_tier=arxiv_preprint&arxiv_primary_category=cs.SE&autofill_imrad=true",
                Ok(json!({ "v": 1, "data": { "publication_id": "pub/1", "arxiv_primary_category": "cs.SE", "readiness_score": 100 } })),
            )
            .on_get(
                "/scientia/publications/pub%2F1/stage-advance?target_stage=doi_reserved&autofill_imrad=true",
                Ok(json!({ "v": 1, "data": { "publication_id": "pub/1", "current_stage": "doi_reserved" } })),
            )
            .on_get(
                "/scientia/publications/pub%2F1/arxiv-bundle",
                Ok(json!({ "v": 1, "data": { "publication_id": "pub/1", "bytes_len": 2048, "references_bib": "@misc{vox}" } })),
            );

        let r = scientia_publication_arxiv_readiness_impl(&b, "pub/1")
            .await
            .unwrap();
        assert_eq!(r["readiness_score"], 91);

        let c = scientia_publication_classify_impl(
            &b,
            "pub/1",
            Some("algorithmic_improvement"),
            Some("arxiv_preprint"),
            Some("cs.SE"),
            None,
            None,
            None,
            None,
            None,
            None,
            Some(true),
        )
        .await
        .unwrap();
        assert_eq!(c["arxiv_primary_category"], "cs.SE");

        let s = scientia_publication_stage_advance_impl(
            &b,
            "pub/1",
            Some("doi_reserved"),
            None,
            None,
            None,
            Some(true),
        )
        .await
        .unwrap();
        assert_eq!(s["current_stage"], "doi_reserved");

        let bundle = scientia_publication_arxiv_bundle_impl(&b, "pub/1")
            .await
            .unwrap();
        assert_eq!(bundle["bytes_len"], 2048);
    }
}
