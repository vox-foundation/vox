//! SCIENTIA Studio `/api/v2/scientia/*` routes: publication claims ledger,
//! class/venue routing recommendations, and DB-backed flywheel advancement.

use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use vox_scientia::class_routing::{
    FindingClass, PublicationTier, arxiv_subject_catalog, atlas_gate_applies_to,
    builtin_class_defaults, critic_allowed_for, default_arxiv_categories_for_class,
    effective_critic_allowed, negative_result_quota_for, recommended_venues_for,
    reply_window_days_for, valid_spdx_licenses, venue_critic_policy_for,
};
use vox_scientia::critic_gate::VenueCatalog;
use vox_scientia::review::flywheel_advance_from_db;
use vox_scientia::review_flow::{
    AdvancePublicationStageRequest, ClassifyPublicationPatch, advance_publication_stage_in_db,
    classify_publication_in_db, evaluate_arxiv_readiness_from_db, publication_session_id,
    resolve_manifest_or_promote,
};

use crate::gui_ops::{api_db, enforce_route};
use crate::http_gateway::GatewayState;
use crate::services::routes::{err, ok};

const SCIENTIA_SURFACES: &[&str] = &["scientia"];

#[derive(Debug, Deserialize)]
pub struct VenueRecommendQuery {
    pub candidate_class: Option<String>,
    pub venue_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FlywheelAdvanceQuery {
    pub target_stage: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct ClassifyPublicationQuery {
    pub candidate_class: Option<String>,
    pub publication_tier: Option<String>,
    pub arxiv_primary_category: Option<String>,
    pub arxiv_secondary_categories: Option<String>,
    pub acm_ccs: Option<String>,
    pub keywords: Option<String>,
    pub license_spdx: Option<String>,
    pub orcid: Option<String>,
    pub ror_affiliation: Option<String>,
    pub prereserved_doi: Option<String>,
    pub arxiv_id: Option<String>,
    pub abstract_text: Option<String>,
    pub autofill_imrad: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
pub struct StageAdvanceQuery {
    pub target_stage: Option<String>,
    pub orcid: Option<String>,
    pub prereserved_doi: Option<String>,
    pub arxiv_id: Option<String>,
    pub autofill_imrad: Option<bool>,
    pub reason: Option<String>,
}

fn split_csv(raw: Option<String>) -> Option<Vec<String>> {
    raw.map(|s| {
        s.split(',')
            .map(|part| part.trim().to_string())
            .filter(|part| !part.is_empty())
            .collect()
    })
}

fn parse_candidate_class(raw: Option<&str>) -> Option<FindingClass> {
    raw.map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(FindingClass::from_str)
}

fn extract_primary_source_url(citations_json: Option<&str>) -> Option<String> {
    let raw = citations_json.map(str::trim).filter(|s| !s.is_empty())?;
    if raw.starts_with("http://") || raw.starts_with("https://") {
        return Some(raw.to_string());
    }
    let val: Value = serde_json::from_str(raw).ok()?;
    if let Some(arr) = val.as_array() {
        for item in arr {
            if let Some(url) = item
                .get("url")
                .or_else(|| item.get("source_url"))
                .or_else(|| item.get("uri"))
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
            {
                return Some(url.to_string());
            }
            if let Some(s) = item.as_str().filter(|s| s.starts_with("http")) {
                return Some(s.to_string());
            }
        }
    }
    val.get("source_url")
        .or_else(|| val.get("url"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

/// `GET /api/v2/scientia/publications/{id}/claims`
pub async fn get_publication_claims(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(publication_id): Path<String>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, SCIENTIA_SURFACES) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    let manifest = match resolve_manifest_or_promote(&db, &publication_id).await {
        Ok(m) => m,
        Err(e) => return err("not_found", &format!("{e:#}")),
    };
    let sid = publication_session_id(&manifest.publication_id);
    let claims = match db.list_publication_claims(sid).await {
        Ok(c) => c,
        Err(e) => return err("db_error", &e.to_string()),
    };
    let default_source_url = extract_primary_source_url(manifest.citations_json.as_deref());
    let claim_rows: Vec<Value> = claims
        .into_iter()
        .map(|c| {
            json!({
                "claim_id": c.claim_id,
                "text": c.text,
                "is_numeric": c.is_numeric,
                "verifiability_score": c.verifiability_score,
                "verdict": c.verdict,
                "confidence": c.confidence,
                "verifier_model": c.verifier_model,
                "created_at_ms": c.created_at_ms,
                "source_url": default_source_url,
            })
        })
        .collect();
    ok(json!({
        "publication_id": manifest.publication_id,
        "session_id": sid,
        "claim_count": claim_rows.len(),
        "claims": claim_rows,
    }))
}

/// `GET /api/v2/scientia/venue-recommend?candidate_class=...`
pub async fn get_venue_recommend(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<VenueRecommendQuery>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, SCIENTIA_SURFACES) {
        return e;
    }
    let Some(class) = parse_candidate_class(q.candidate_class.as_deref()) else {
        return err(
            "invalid_candidate_class",
            "Provide a valid candidate_class (e.g. algorithmic_improvement, model_capability_atlas)",
        );
    };
    let defaults = builtin_class_defaults();
    let catalog = VenueCatalog::load_from_repo(&gs.server_state().repository.root).ok();
    let venue_id = q.venue_id.as_deref().filter(|s| !s.trim().is_empty());
    let policy = venue_critic_policy_for(&defaults, class, venue_id, catalog.as_ref());
    let (default_arxiv_primary, default_arxiv_secondary) =
        default_arxiv_categories_for_class(class);
    let recommended_tier = PublicationTier::recommended_for_class(class);
    ok(json!({
        "candidate_class": class.as_str(),
        "candidate_class_label": class.label(),
        "recommended_tier": recommended_tier.as_str(),
        "recommended_tier_label": recommended_tier.label(),
        "default_arxiv_primary_category": default_arxiv_primary,
        "default_arxiv_secondary_categories": default_arxiv_secondary,
        "recommended_venues": recommended_venues_for(&defaults, class),
        "reply_window_days": reply_window_days_for(&defaults, class),
        "negative_result_quota": negative_result_quota_for(&defaults, class),
        "critic_allowed": critic_allowed_for(&defaults, class),
        "effective_critic_allowed": effective_critic_allowed(&defaults, class, venue_id, catalog.as_ref()),
        "venue_critic_policy": policy.as_str(),
        "atlas_gate_applies": atlas_gate_applies_to(class),
    }))
}

/// `GET /api/v2/scientia/publications/{id}/arxiv-readiness`
pub async fn get_publication_arxiv_readiness(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(publication_id): Path<String>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, SCIENTIA_SURFACES) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    match evaluate_arxiv_readiness_from_db(&db, &publication_id).await {
        Ok(report) => ok(report),
        Err(e) => err("readiness_error", &format!("{e:#}")),
    }
}

/// `GET` / `POST /api/v2/scientia/publications/{id}/classify`
pub async fn post_publication_classify(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(publication_id): Path<String>,
    Query(q): Query<ClassifyPublicationQuery>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, SCIENTIA_SURFACES) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    let patch = ClassifyPublicationPatch {
        candidate_class: q.candidate_class,
        publication_tier: q.publication_tier,
        arxiv_primary_category: q.arxiv_primary_category,
        arxiv_secondary_categories: split_csv(q.arxiv_secondary_categories),
        acm_ccs: split_csv(q.acm_ccs),
        msc_classes: None,
        keywords: split_csv(q.keywords),
        license_spdx: q.license_spdx,
        orcid: q.orcid,
        ror_affiliation: q.ror_affiliation,
        prereserved_doi: q.prereserved_doi,
        arxiv_id: q.arxiv_id,
        abstract_text: q.abstract_text,
        autofill_imrad: q.autofill_imrad,
    };
    match classify_publication_in_db(&db, &publication_id, patch).await {
        Ok(report) => ok(report),
        Err(e) => err("classify_error", &format!("{e:#}")),
    }
}

/// `GET` / `POST /api/v2/scientia/publications/{id}/stage-advance`
pub async fn post_publication_stage_advance(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(publication_id): Path<String>,
    Query(q): Query<StageAdvanceQuery>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, SCIENTIA_SURFACES) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    let req = AdvancePublicationStageRequest {
        target_stage: q.target_stage,
        orcid: q.orcid,
        prereserved_doi: q.prereserved_doi,
        arxiv_id: q.arxiv_id,
        autofill_imrad: q.autofill_imrad,
        reason: q.reason,
    };
    match advance_publication_stage_in_db(&db, &publication_id, req).await {
        Ok(report) => ok(report),
        Err(e) => err("stage_advance_error", &format!("{e:#}")),
    }
}

/// `GET /api/v2/scientia/arxiv-categories`
pub async fn get_arxiv_categories(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, SCIENTIA_SURFACES) {
        return e;
    }
    let tiers: Vec<Value> = PublicationTier::all()
        .iter()
        .map(|t| {
            json!({
                "id": t.as_str(),
                "label": t.label(),
                "requires_full_imrad": t.requires_full_imrad(),
                "minimum_citations": t.minimum_citations(),
            })
        })
        .collect();
    let classes: Vec<Value> = FindingClass::all()
        .iter()
        .map(|c| {
            let (p, secs) = default_arxiv_categories_for_class(*c);
            json!({
                "id": c.as_str(),
                "label": c.label(),
                "default_arxiv_primary": p,
                "default_arxiv_secondary": secs,
            })
        })
        .collect();
    ok(json!({
        "arxiv_categories": arxiv_subject_catalog(),
        "publication_tiers": tiers,
        "finding_classes": classes,
        "spdx_licenses": valid_spdx_licenses(),
    }))
}

/// `GET` / `POST /api/v2/scientia/publications/{id}/flywheel-advance?target_stage=...`
pub async fn post_flywheel_advance(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(publication_id): Path<String>,
    Query(q): Query<FlywheelAdvanceQuery>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, SCIENTIA_SURFACES) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    let stage = q.target_stage.as_deref().unwrap_or("replay_verify");
    match flywheel_advance_from_db(&db, &publication_id, stage).await {
        Ok(res) => ok(res),
        Err(e) => err("flywheel_error", &format!("{e:#}")),
    }
}

const V76_BEFORE_BASELINE_REL: &str = "contracts/reports/scientia-v76-before-baseline.v1.json";
const V76_AFTER_CLOSURE_REL: &str = "contracts/reports/scientia-v76-after-closure.v1.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct V76AuditAxisDeltaDto {
    pub axis: String,
    pub label: String,
    pub metric: String,
    pub before_value: f64,
    pub after_value: f64,
    pub delta: f64,
    pub higher_is_better: bool,
    pub ci_low: Option<f64>,
    pub ci_high: Option<f64>,
    pub ni_margin: Option<f64>,
    pub ni_verdict: Option<String>,
    pub superior: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct V76BugStatusDto {
    pub id: String,
    pub before: Value,
    pub after: Value,
    pub resolved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct V76DiffHunkLineDto {
    pub index: usize,
    pub tag: String,
    pub old_line: Option<usize>,
    pub new_line: Option<usize>,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct V76LessonsSummaryDto {
    pub quarantined_findings_count: usize,
    pub corroborated_promotions_count: usize,
    pub proposed_effort_rules_count: usize,
    pub auto_dpo_pairs_minted: usize,
    pub unreviewed_ssot_mutations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct V76AuditDiffDto {
    pub before_stage: String,
    pub after_stage: String,
    pub before_path: String,
    pub after_path: String,
    pub axes: Vec<V76AuditAxisDeltaDto>,
    pub bugs: Vec<V76BugStatusDto>,
    pub hunks: Vec<V76DiffHunkLineDto>,
    pub lessons_summary: Option<V76LessonsSummaryDto>,
    pub after_available: bool,
}

fn round6_f64(v: f64) -> f64 {
    (v * 1_000_000.0).round() / 1_000_000.0
}

fn bug_is_resolved(id: &str, after: &Value) -> bool {
    match id {
        "composer_research_scope_overwritten"
        | "research_turn_waves_hardcoded_to_one"
        | "artifact_json_wiped_on_doc_publish" => after.as_bool() == Some(false),
        "none_to_novel_false_positive_rate"
        | "mock_verifier_fallback_rate"
        | "contradiction_false_support_rate" => after.as_f64().is_some_and(|v| v == 0.0),
        "kv_cache_prefix_stability_ratio" => after.as_f64().is_some_and(|v| v >= 0.90),
        "session_detail_ipc_payload_bytes" => after.as_u64().is_some_and(|v| v <= 32_768),
        "gui_cli_db_spawn_count" => after.as_u64() == Some(0),
        _ => false,
    }
}

/// Read locked before baseline + after closure reports under `repo_root` and compute inline diff DTO.
pub fn build_v76_audit_diff_from_root(
    repo_root: &std::path::Path,
) -> Result<V76AuditDiffDto, String> {
    let before_full = repo_root.join(V76_BEFORE_BASELINE_REL);
    let before_raw = std::fs::read_to_string(&before_full)
        .map_err(|e| format!("failed to read {}: {e}", before_full.display()))?;
    let before_json: Value = serde_json::from_str(&before_raw)
        .map_err(|e| format!("invalid before JSON {}: {e}", before_full.display()))?;

    let after_full = repo_root.join(V76_AFTER_CLOSURE_REL);
    let after_raw_opt = std::fs::read_to_string(&after_full).ok();
    let after_json_opt: Option<Value> = after_raw_opt
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());
    let after_available = after_json_opt.is_some();

    let paired_axes: Vec<&Value> = after_json_opt
        .as_ref()
        .and_then(|v| v.get("paired_comparison")?.get("axes")?.as_array())
        .map(|arr| arr.iter().collect())
        .unwrap_or_default();

    let axis_specs = [
        (
            "performance",
            "Performance & KV Stability",
            "kv_cache_prefix_stability_ratio",
        ),
        ("quality", "Quality & Groundedness", "groundedness_mean"),
        (
            "verification",
            "Verification & Contradiction",
            "contradiction_detection_recall",
        ),
        (
            "integration",
            "Continuum Integration",
            "composer_continuum_fidelity_rate",
        ),
        (
            "socrates_calibration",
            "Socrates Calibration",
            "selective_abstention_f1",
        ),
        (
            "novelty_detection",
            "Novelty Detection",
            "prior_art_recall_at_5",
        ),
    ];

    let axes = axis_specs
        .into_iter()
        .map(|(axis, label, metric)| {
            let before_value = before_json["six_axes"][axis][metric]
                .as_f64()
                .unwrap_or(0.0);
            let after_value = after_json_opt
                .as_ref()
                .and_then(|v| v["six_axes"][axis][metric].as_f64())
                .unwrap_or(before_value);
            let paired = paired_axes
                .iter()
                .find(|row| row.get("axis").and_then(Value::as_str) == Some(axis))
                .copied();
            V76AuditAxisDeltaDto {
                axis: axis.to_string(),
                label: label.to_string(),
                metric: metric.to_string(),
                before_value: round6_f64(before_value),
                after_value: round6_f64(after_value),
                delta: round6_f64(after_value - before_value),
                higher_is_better: true,
                ci_low: paired.and_then(|p| p.get("ci_low")).and_then(Value::as_f64),
                ci_high: paired
                    .and_then(|p| p.get("ci_high"))
                    .and_then(Value::as_f64),
                ni_margin: paired
                    .and_then(|p| p.get("ni_margin"))
                    .and_then(Value::as_f64),
                ni_verdict: paired
                    .and_then(|p| p.get("ni_verdict"))
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                superior: paired
                    .and_then(|p| p.get("superior"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            }
        })
        .collect();

    let bug_ids = [
        "composer_research_scope_overwritten",
        "research_turn_waves_hardcoded_to_one",
        "none_to_novel_false_positive_rate",
        "artifact_json_wiped_on_doc_publish",
        "mock_verifier_fallback_rate",
        "contradiction_false_support_rate",
        "kv_cache_prefix_stability_ratio",
        "session_detail_ipc_payload_bytes",
        "gui_cli_db_spawn_count",
    ];
    let bugs = bug_ids
        .into_iter()
        .map(|id| {
            let before = before_json["live_bug_reproductions"]
                .get(id)
                .cloned()
                .unwrap_or(Value::Null);
            let after = after_json_opt
                .as_ref()
                .and_then(|v| v["live_bug_reproductions"].get(id).cloned())
                .unwrap_or(Value::Null);
            let resolved = after_available && bug_is_resolved(id, &after);
            V76BugStatusDto {
                id: id.to_string(),
                before,
                after,
                resolved,
            }
        })
        .collect();

    let hunks = match after_raw_opt.as_deref() {
        Some(after_raw) if after_available => {
            similar::TextDiff::from_lines(before_raw.as_str(), after_raw)
                .iter_all_changes()
                .enumerate()
                .map(|(index, change)| V76DiffHunkLineDto {
                    index,
                    tag: match change.tag() {
                        similar::ChangeTag::Equal => "equal",
                        similar::ChangeTag::Delete => "delete",
                        similar::ChangeTag::Insert => "insert",
                    }
                    .to_string(),
                    old_line: change.old_index().map(|i| i + 1),
                    new_line: change.new_index().map(|i| i + 1),
                    text: change.value().trim_end_matches(['\r', '\n']).to_string(),
                })
                .collect()
        }
        _ => Vec::new(),
    };

    let lessons_summary = after_json_opt
        .as_ref()
        .and_then(|v| v.get("lessons_summary"))
        .and_then(|v| serde_json::from_value::<V76LessonsSummaryDto>(v.clone()).ok());

    Ok(V76AuditDiffDto {
        before_stage: before_json["stage"]
            .as_str()
            .unwrap_or("before")
            .to_string(),
        after_stage: after_json_opt
            .as_ref()
            .and_then(|v| v["stage"].as_str())
            .unwrap_or("after")
            .to_string(),
        before_path: V76_BEFORE_BASELINE_REL.to_string(),
        after_path: V76_AFTER_CLOSURE_REL.to_string(),
        axes,
        bugs,
        hunks,
        lessons_summary,
        after_available,
    })
}

/// `GET /api/v2/scientia/v76-audit-diff`
pub async fn get_v76_audit_diff(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, SCIENTIA_SURFACES) {
        return e;
    }
    let repo_root = {
        let gw_root = &gs.server_state().repository.root;
        if gw_root.join(V76_BEFORE_BASELINE_REL).is_file() {
            gw_root.clone()
        } else {
            vox_repository::resolve_repo_root_for_ci()
        }
    };
    match build_v76_audit_diff_from_root(&repo_root) {
        Ok(dto) => ok(dto),
        Err(e) => err("v76_audit_diff_error", &e),
    }
}

pub fn api_v2_router() -> Router<GatewayState> {
    Router::new()
        .route(
            "/scientia/publications/{id}/claims",
            get(get_publication_claims),
        )
        .route("/scientia/venue-recommend", get(get_venue_recommend))
        .route("/scientia/arxiv-categories", get(get_arxiv_categories))
        .route(
            "/scientia/publications/{id}/flywheel-advance",
            get(post_flywheel_advance).post(post_flywheel_advance),
        )
        .route(
            "/scientia/publications/{id}/arxiv-readiness",
            get(get_publication_arxiv_readiness),
        )
        .route(
            "/scientia/publications/{id}/classify",
            get(post_publication_classify).post(post_publication_classify),
        )
        .route(
            "/scientia/publications/{id}/stage-advance",
            get(post_publication_stage_advance).post(post_publication_stage_advance),
        )
        .route("/scientia/v76-audit-diff", get(get_v76_audit_diff))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_ops::test_support::{body, gateway_with_memory_db, peer};

    #[tokio::test]
    async fn studio_claims_and_flywheel_advance_hydrate_from_db() {
        let (gs, db) = gateway_with_memory_db().await;
        db.upsert_publication_manifest(vox_db::PublicationManifestParams {
            publication_id: "pub-studio-1",
            content_type: "scientia",
            source_ref: None,
            title: "Studio Flywheel Test",
            author: "Ada Lovelace",
            abstract_text: Some("Evaluates throughput gains."),
            body_markdown: "Throughput increased by 35% across 20 benchmarks.",
            citations_json: Some(r#"[{"url":"https://example.org/evidence"}]"#),
            metadata_json: None,
            revision_history_json: None,
            content_sha3_256: "deadbeef",
            state: "draft",
        })
        .await
        .expect("seed manifest");

        let adv = body(
            post_flywheel_advance(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                Path("pub-studio-1".into()),
                Query(FlywheelAdvanceQuery {
                    target_stage: Some("replay_verify".into()),
                }),
            )
            .await,
        )
        .await;
        assert_eq!(adv["data"]["publication_id"], "pub-studio-1", "{adv}");
        assert_eq!(adv["data"]["stage_completed"], "replay_verify", "{adv}");
        assert_eq!(
            adv["data"]["stages_done"].as_array().map(Vec::len),
            Some(8),
            "{adv}"
        );

        let claims = body(
            get_publication_claims(
                State(gs),
                peer(),
                HeaderMap::new(),
                Path("pub-studio-1".into()),
            )
            .await,
        )
        .await;
        assert_eq!(claims["data"]["publication_id"], "pub-studio-1", "{claims}");
        assert!(claims["data"]["session_id"].is_i64(), "{claims}");
        assert!(claims["data"]["claims"].is_array(), "{claims}");
    }

    #[tokio::test]
    async fn studio_venue_recommend_returns_class_and_atlas_policy() {
        let (gs, _db) = gateway_with_memory_db().await;
        let out = body(
            get_venue_recommend(
                State(gs),
                peer(),
                HeaderMap::new(),
                Query(VenueRecommendQuery {
                    candidate_class: Some("model_capability_atlas".into()),
                    venue_id: None,
                }),
            )
            .await,
        )
        .await;
        assert_eq!(
            out["data"]["candidate_class"], "model_capability_atlas",
            "{out}"
        );
        assert_eq!(out["data"]["atlas_gate_applies"], true, "{out}");
        assert!(
            out["data"]["recommended_venues"]
                .as_array()
                .is_some_and(|v| !v.is_empty()),
            "{out}"
        );
    }

    #[test]
    fn v76_audit_diff_computes_axes_bugs_and_line_hunks() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let reports_dir = tmp.path().join("contracts/reports");
        std::fs::create_dir_all(&reports_dir).expect("mkdir");

        let before_val = json!({
            "stage": "before",
            "six_axes": {
                "performance": { "kv_cache_prefix_stability_ratio": 0.544548 },
                "quality": { "groundedness_mean": 0.25 },
                "verification": { "contradiction_detection_recall": 0.333333 },
                "integration": { "composer_continuum_fidelity_rate": 0.0 },
                "socrates_calibration": { "selective_abstention_f1": 0.8 },
                "novelty_detection": { "prior_art_recall_at_5": 0.5 }
            },
            "live_bug_reproductions": {
                "composer_research_scope_overwritten": true,
                "research_turn_waves_hardcoded_to_one": true,
                "none_to_novel_false_positive_rate": 1.0,
                "artifact_json_wiped_on_doc_publish": true,
                "mock_verifier_fallback_rate": 1.0,
                "contradiction_false_support_rate": 0.666667,
                "kv_cache_prefix_stability_ratio": 0.544548,
                "session_detail_ipc_payload_bytes": 140517,
                "gui_cli_db_spawn_count": 3
            }
        });
        let before_str = serde_json::to_string_pretty(&before_val).expect("before json");
        std::fs::write(
            reports_dir.join("scientia-v76-before-baseline.v1.json"),
            &before_str,
        )
        .expect("write before");

        let before_only = build_v76_audit_diff_from_root(tmp.path()).expect("before-only diff");
        assert!(!before_only.after_available);
        assert_eq!(before_only.axes.len(), 6);
        assert_eq!(before_only.bugs.len(), 9);
        assert!(before_only.bugs.iter().all(|b| !b.resolved));
        assert!(before_only.hunks.is_empty());

        let after_val = json!({
            "stage": "after",
            "six_axes": {
                "performance": { "kv_cache_prefix_stability_ratio": 1.0 },
                "quality": { "groundedness_mean": 1.0 },
                "verification": { "contradiction_detection_recall": 1.0 },
                "integration": { "composer_continuum_fidelity_rate": 1.0 },
                "socrates_calibration": { "selective_abstention_f1": 1.0 },
                "novelty_detection": { "prior_art_recall_at_5": 1.0 }
            },
            "live_bug_reproductions": {
                "composer_research_scope_overwritten": false,
                "research_turn_waves_hardcoded_to_one": false,
                "none_to_novel_false_positive_rate": 0.0,
                "artifact_json_wiped_on_doc_publish": false,
                "mock_verifier_fallback_rate": 0.0,
                "contradiction_false_support_rate": 0.0,
                "kv_cache_prefix_stability_ratio": 1.0,
                "session_detail_ipc_payload_bytes": 12000,
                "gui_cli_db_spawn_count": 0
            },
            "paired_comparison": {
                "blinded_judge_protocol": true,
                "before_report_path": "contracts/reports/scientia-v76-before-baseline.v1.json",
                "axes": [
                    { "axis": "performance", "ci_low": 0.12, "ci_high": 0.99, "ni_margin": 0.10, "ni_verdict": "NonInferior", "superior": true }
                ]
            },
            "lessons_summary": {
                "quarantined_findings_count": 3,
                "corroborated_promotions_count": 2,
                "proposed_effort_rules_count": 2,
                "auto_dpo_pairs_minted": 3,
                "unreviewed_ssot_mutations": 0
            }
        });
        std::fs::write(
            reports_dir.join("scientia-v76-after-closure.v1.json"),
            serde_json::to_string_pretty(&after_val).expect("after json"),
        )
        .expect("write after");

        let full = build_v76_audit_diff_from_root(tmp.path()).expect("full diff");
        assert!(full.after_available);
        assert_eq!(full.axes.len(), 6);
        assert_eq!(full.axes[0].ni_verdict.as_deref(), Some("NonInferior"));
        assert!(full.axes[0].superior);
        assert_eq!(full.bugs.len(), 9);
        assert!(full.bugs.iter().all(|b| b.resolved));
        assert!(
            full.hunks
                .iter()
                .any(|h| h.tag == "insert" || h.tag == "delete")
        );
        assert_eq!(
            full.lessons_summary
                .as_ref()
                .map(|s| s.unreviewed_ssot_mutations),
            Some(0)
        );
    }

    #[tokio::test]
    async fn studio_v76_audit_diff_endpoint_answers_from_repo_root() {
        let (gs, _db) = gateway_with_memory_db().await;
        let out = body(get_v76_audit_diff(State(gs), peer(), HeaderMap::new()).await).await;
        assert_eq!(out["v"], 1, "{out}");
        assert_eq!(out["data"]["before_stage"], "before", "{out}");
        assert_eq!(
            out["data"]["axes"].as_array().map(Vec::len),
            Some(6),
            "{out}"
        );
        assert_eq!(
            out["data"]["bugs"].as_array().map(Vec::len),
            Some(9),
            "{out}"
        );
    }

    #[tokio::test]
    async fn studio_classify_stage_advance_readiness_and_venue_aliases_work() {
        let (gs, db) = gateway_with_memory_db().await;
        db.upsert_publication_manifest(vox_db::PublicationManifestParams {
            publication_id: "pub-arxiv-studio-1",
            content_type: "paper",
            source_ref: None,
            title: "Autonomous Stage & arXiv Readiness Control",
            author: "Ada Lovelace",
            abstract_text: None,
            body_markdown: "Throughput increased 28% across 10 trials.",
            citations_json: Some(
                r#"[{"title":"Benchmark Paper","url":"https://doi.org/10.1000/bench"}]"#,
            ),
            metadata_json: None,
            revision_history_json: None,
            content_sha3_256: "beefcafe",
            state: "draft",
        })
        .await
        .expect("seed");

        let rec = body(
            get_venue_recommend(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                Query(VenueRecommendQuery {
                    candidate_class: Some("paper".into()),
                    venue_id: None,
                }),
            )
            .await,
        )
        .await;
        assert_eq!(rec["data"]["candidate_class"], "empirical_science", "{rec}");
        assert_eq!(
            rec["data"]["default_arxiv_primary_category"], "cs.AI",
            "{rec}"
        );

        let classified = body(
            post_publication_classify(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                Path("pub-arxiv-studio-1".into()),
                Query(ClassifyPublicationQuery {
                    candidate_class: Some("algorithmic_improvement".into()),
                    publication_tier: Some("arxiv_preprint".into()),
                    arxiv_primary_category: Some("cs.SE".into()),
                    autofill_imrad: Some(true),
                    ..Default::default()
                }),
            )
            .await,
        )
        .await;
        assert_eq!(classified["data"]["arxiv_ready"], true, "{classified}");
        assert_eq!(classified["data"]["todo_stub_count"], 0, "{classified}");

        let advanced = body(
            post_publication_stage_advance(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                Path("pub-arxiv-studio-1".into()),
                Query(StageAdvanceQuery {
                    target_stage: Some("published".into()),
                    orcid: Some("0000-0002-1825-0097".into()),
                    ..Default::default()
                }),
            )
            .await,
        )
        .await;
        assert_eq!(advanced["data"]["current_stage"], "published", "{advanced}");
        assert!(
            advanced["data"]["prereserved_doi"].is_string(),
            "{advanced}"
        );
        assert!(advanced["data"]["arxiv_id"].is_string(), "{advanced}");

        let cats = body(get_arxiv_categories(State(gs), peer(), HeaderMap::new()).await).await;
        assert!(
            cats["data"]["arxiv_categories"]
                .as_array()
                .is_some_and(|a| !a.is_empty()),
            "{cats}"
        );
    }
}
