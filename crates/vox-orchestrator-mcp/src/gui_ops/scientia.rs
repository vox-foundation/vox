//! Scientia publication routes and ops for Axis (plan 07 Task 7): manifests, the claim review
//! queue and decisions, offline nanopublication, evidence suggestions, novelty / completion /
//! autofill / archive status, and the discovery inbox.
//!
//! Review-flow logic stays in `vox_scientia::review_flow` (shared with `vox scientia
//! publication-*`). Like the CLI, this file carries NO production-network publishing symbols
//! (asserted by the guard test below).
use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use vox_scientia::review_flow;

use crate::gui_ops::dto::research::{
    ArchiveStatusDto, AutofillResultDto, ClaimAwaitingReviewDto, DiscoveryInboxDto,
    NanopubResultDto, PublicationManifestDto, ReviewDecisionDto,
};
use crate::gui_ops::{api_db, enforce_route, require_db};
use crate::http_gateway::GatewayState;
use crate::params::ToolResult;
use crate::server_state::ServerState;
use crate::services::routes::{err, ok};

const SCIENTIA_SURFACES: &[&str] = &["scientia"];
const DEFAULT_MANIFEST_LIMIT: i64 = 200;
const DEFAULT_DISCOVERY_LIMIT: i64 = 50;
#[cfg(not(feature = "news-publish"))]
const NEEDS_NEWS_PUBLISH: &str =
    "this Vox server was built without the `news-publish` feature (vox-publisher)";

fn anyhow_msg(e: anyhow::Error) -> String {
    format!("{e:#}")
}

async fn load_manifest_row(
    db: &vox_db::VoxDb,
    publication_id: &str,
) -> Result<vox_db::PublicationManifestRow, String> {
    review_flow::resolve_manifest_or_promote(db, publication_id)
        .await
        .map_err(|e| format!("{e:#}"))
}

// ─── Novelty / completion / autofill (vox-publisher) ─────────────────────────

#[cfg(feature = "news-publish")]
mod publisher {
    use std::path::Path;

    use crate::gui_ops::dto::research::{
        AutofillResultDto, CompletionReportDto, ConflictDto, ConflictHitDto, FieldProvenanceDto,
        NoveltyAssessmentDto, PlannedFillDto, PriorArtHitDto, SignalsDto,
    };

    /// Default user id for identity lookup — mirrors the CLI `publication-autofill` handler.
    const ARCHIVE_DEFAULT_USER_ID: &str = "local-user";

    pub fn manifest_from_row(
        row: &vox_db::PublicationManifestRow,
    ) -> vox_publisher::publication::PublicationManifest {
        vox_publisher::publication::PublicationManifest {
            publication_id: row.publication_id.clone(),
            content_type: row.content_type.clone(),
            source_ref: row.source_ref.clone(),
            title: row.title.clone(),
            author: row.author.clone(),
            abstract_text: row.abstract_text.clone(),
            body_markdown: row.body_markdown.clone(),
            citations_json: row.citations_json.clone(),
            metadata_json: row.metadata_json.clone(),
        }
    }

    pub fn novelty(row: &vox_db::PublicationManifestRow) -> NoveltyAssessmentDto {
        use chrono::Datelike;

        let Some(bundle) =
            vox_publisher::scientia_prior_art::parse_novelty_bundle_from_metadata_json(
                row.metadata_json.as_deref(),
            )
        else {
            return NoveltyAssessmentDto::insufficient_evidence();
        };
        let claim_year = chrono::Utc::now().date_naive().year();
        let assessment = vox_publisher::scientia_novelty_assess::assess_novelty(
            &bundle,
            claim_year,
            &vox_scientia::inspect_bridge::NoveltyConfig::default(),
        );
        // Top-5 prior-art hits by semantic score (desc); unscored hits sort last.
        let mut hits = bundle.normalized_hits.clone();
        hits.sort_by(|a, b| {
            b.semantic_score
                .unwrap_or(f64::NEG_INFINITY)
                .partial_cmp(&a.semantic_score.unwrap_or(f64::NEG_INFINITY))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let prior_art = hits
            .into_iter()
            .take(5)
            .map(|h| PriorArtHitDto {
                work_uri: h.work_uri,
                title: h.title,
                year: h.year,
                cited_by_count: h.cited_by_count,
                semantic_score: h.semantic_score,
            })
            .collect();
        macro_rules! side {
            ($hits:expr) => {
                $hits
                    .into_iter()
                    .map(|h| ConflictHitDto {
                        work_uri: h.work_uri,
                        excerpt: h.excerpt,
                    })
                    .collect()
            };
        }
        let conflicts = assessment
            .conflicts
            .into_iter()
            .map(|c| ConflictDto {
                claim_text: c.claim_text,
                conflict_score: c.conflict_score,
                supporting: side!(c.supporting_hits),
                contradicting: side!(c.contradicting_hits),
            })
            .collect();
        NoveltyAssessmentDto {
            verdict_kind: assessment.verdict_kind,
            closest_hit_uri: assessment.closest_hit_uri,
            closest_score: assessment.closest_score,
            excluded_future_hits: assessment.excluded_future_hits,
            conflicts,
            signals: SignalsDto {
                max_semantic: assessment.signals.max_semantic,
                max_lexical: assessment.signals.max_lexical,
                near_hit_count: assessment.signals.near_hit_count,
                top_hit_citations: assessment.signals.top_hit_citations,
                sources_succeeded: assessment.signals.sources_succeeded,
            },
            prior_art,
            insufficient_evidence_reason: assessment.insufficient_evidence_reason,
        }
    }

    pub fn completion(row: &vox_db::PublicationManifestRow) -> CompletionReportDto {
        let r =
            vox_publisher::scientia_discovery::manifest_completion_report(&manifest_from_row(row));
        CompletionReportDto {
            completeness_0_100: r.completeness_0_100,
            required_missing: r.required_missing,
            inferred_ok: r.inferred_ok,
            human_only_pending: r.human_only_pending,
            field_provenance: r
                .field_provenance
                .into_iter()
                .map(|e| FieldProvenanceDto {
                    field: e.field,
                    origin: e.origin,
                    notes: e.notes,
                })
                .collect(),
        }
    }

    /// SPDX id from the first LICENSE file under `repo_root` (first 4 KB; "Apache" / "MIT").
    fn detect_repo_license(repo_root: &Path) -> Option<String> {
        for name in &["LICENSE", "LICENSE.md", "LICENSE.txt"] {
            if let Ok(text) = std::fs::read_to_string(repo_root.join(name)) {
                let mut end = text.len().min(4096);
                while end > 0 && !text.is_char_boundary(end) {
                    end -= 1;
                }
                let lower = text[..end].to_lowercase();
                if lower.contains("apache") {
                    return Some("Apache-2.0".into());
                }
                if lower.contains("mit") {
                    return Some("MIT".into());
                }
            }
        }
        None
    }

    fn detect_workspace_version(repo_root: &Path) -> Option<String> {
        let text = std::fs::read_to_string(repo_root.join("Cargo.toml")).ok()?;
        text.lines().find_map(|line| {
            let version = line
                .trim()
                .strip_prefix("version = ")?
                .trim()
                .trim_matches('"');
            (!version.is_empty()).then(|| version.to_string())
        })
    }

    async fn git_remote_origin(repo_root: &Path) -> Option<String> {
        let out = crate::git_exec::GitExec::new(repo_root)
            .run(&["remote", "get-url", "origin"])
            .await
            .ok()?;
        let url = out.stdout.trim();
        (out.exit_code == 0 && !url.is_empty()).then(|| url.to_string())
    }

    /// Compute the deterministic autofill plan and, when `apply`, persist it with the same
    /// sequence as the CLI `publication-autofill --apply` handler.
    pub async fn autofill(
        db: &vox_db::VoxDb,
        repo_root: &Path,
        row: vox_db::PublicationManifestRow,
        apply: bool,
    ) -> Result<AutofillResultDto, String> {
        let mut manifest = manifest_from_row(&row);
        let repo_license = detect_repo_license(repo_root);
        let git_remote = git_remote_origin(repo_root).await;
        let workspace_version = detect_workspace_version(repo_root);
        let identity_view = db
            .get_user_identity(ARCHIVE_DEFAULT_USER_ID)
            .await
            .ok()
            .flatten()
            .map(|r| vox_publisher::scientia_autofill::UserIdentityView {
                user_id: r.user_id,
                orcid_id: r.orcid_id,
            });
        let completeness_before =
            vox_publisher::scientia_discovery::manifest_completion_report(&manifest)
                .completeness_0_100;
        let plan = vox_publisher::scientia_autofill::compute_autofill(
            &manifest,
            identity_view.as_ref(),
            repo_license.as_deref(),
            git_remote.as_deref(),
            workspace_version.as_deref(),
        );
        let mut completeness_after = completeness_before;
        if apply && !plan.fills.is_empty() {
            let new_meta = vox_publisher::scientia_autofill::apply_autofill(
                manifest.metadata_json.as_deref(),
                &mut manifest.abstract_text,
                &plan,
            )
            .map_err(|e| format!("autofill apply: {e:#}"))?;
            manifest.metadata_json = Some(new_meta);
            let digest = manifest.content_sha3_256();
            db.upsert_publication_manifest(vox_db::PublicationManifestParams {
                publication_id: &manifest.publication_id,
                content_type: &manifest.content_type,
                source_ref: manifest.source_ref.as_deref(),
                title: &manifest.title,
                author: &manifest.author,
                abstract_text: manifest.abstract_text.as_deref(),
                body_markdown: &manifest.body_markdown,
                citations_json: manifest.citations_json.as_deref(),
                metadata_json: manifest.metadata_json.as_deref(),
                revision_history_json: row.revision_history_json.as_deref(),
                content_sha3_256: &digest,
                state: row.state.as_str(),
            })
            .await
            .map_err(|e| format!("{e:#}"))?;
            db.append_publication_status_event(
                &manifest.publication_id,
                "scientia_autofill_applied",
                Some(
                    &serde_json::json!({ "fills": plan.fills.len(), "digest": digest, "via": "gui" })
                        .to_string(),
                ),
            )
            .await
            .map_err(|e| format!("{e:#}"))?;
            completeness_after =
                vox_publisher::scientia_discovery::manifest_completion_report(&manifest)
                    .completeness_0_100;
        }
        Ok(AutofillResultDto {
            fills: plan
                .fills
                .into_iter()
                .map(|f| PlannedFillDto {
                    field: f.field,
                    value: f.value.to_string(),
                    origin: f.origin,
                    notes: f.notes,
                })
                .collect(),
            human_only_remaining: plan.human_only_remaining,
            completeness_before,
            completeness_after,
        })
    }
}

// ─── Archive status ──────────────────────────────────────────────────────────

/// SWHID / SWH task status from `metadata_json.scientia`, Zenodo DOI / state from the latest
/// persisted `zenodo` scholarly submission. Read-only; never deposits.
async fn archive_status(
    db: &vox_db::VoxDb,
    row: &vox_db::PublicationManifestRow,
) -> Result<ArchiveStatusDto, String> {
    let scientia: Option<Value> = row
        .metadata_json
        .as_deref()
        .and_then(|s| serde_json::from_str::<Value>(s).ok())
        .and_then(|v| v.get("scientia").cloned());
    let non_empty = |v: Option<&Value>| {
        v.and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    let swhid = non_empty(scientia.as_ref().and_then(|s| s.get("swhid")));
    let swh_task_status = non_empty(
        scientia
            .as_ref()
            .and_then(|s| s.get("swh_save"))
            .and_then(|s| s.get("task_status")),
    );
    let subs = db
        .list_scholarly_submissions(&row.publication_id)
        .await
        .map_err(|e| format!("{e:#}"))?;
    let (mut zenodo_doi, mut zenodo_state) = (None, None);
    if let Some(z) = subs.iter().rev().find(|s| s.adapter == "zenodo") {
        zenodo_state = Some(z.status.clone()).filter(|s| !s.is_empty());
        zenodo_doi = z
            .metadata_json
            .as_deref()
            .and_then(|s| serde_json::from_str::<Value>(s).ok())
            .and_then(|v| non_empty(v.get("doi").or_else(|| v.get("expected_doi_hint"))));
    }
    Ok(ArchiveStatusDto {
        swhid,
        swh_task_status,
        zenodo_doi,
        zenodo_state,
    })
}

// ─── Ops ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClaimReviewParams {
    pub publication_id: String,
    pub claim_id: i64,
    /// `approved` | `rejected` | `needs_revision` (validated by `review_flow`).
    pub decision: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NanopublishParams {
    pub publication_id: String,
    pub claim_id: i64,
    #[serde(default)]
    pub orcid: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AutofillParams {
    pub publication_id: String,
    /// Persist the plan; `false` only computes it.
    #[serde(default)]
    pub apply: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DiscoveryAckParams {
    pub id: i64,
}

/// Record a human review decision for ONE claim, bound to the publication's content digest.
pub async fn claim_review(state: &ServerState, p: ClaimReviewParams) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<ReviewDecisionDto>::err(e).to_json(),
    };
    match review_flow::record_claim_review(db, &p.publication_id, p.claim_id, &p.decision, p.reason)
        .await
    {
        Ok(row) => ToolResult::ok(ReviewDecisionDto {
            claim_id: row.claim_id,
            publication_id: row.publication_id,
            decision: row.decision,
            bound_digest: row.bound_digest,
            decided_at_ms: row.decided_at_ms,
        })
        .to_json(),
        Err(e) => ToolResult::<ReviewDecisionDto>::err(anyhow_msg(e)).to_json(),
    }
}

/// Build + sign + offline-validate a nanopublication for an APPROVED claim and persist it
/// locally. The approval token is minted from the review ledger; no approval → refusal.
pub async fn nanopublish(state: &ServerState, p: NanopublishParams) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<NanopubResultDto>::err(e).to_json(),
    };
    let token = match review_flow::approval_for(db, &p.publication_id, p.claim_id).await {
        Ok(t) => t,
        Err(e) => return ToolResult::<NanopubResultDto>::err(anyhow_msg(e)).to_json(),
    };
    match review_flow::nanopub_build(
        db,
        &p.publication_id,
        p.claim_id,
        p.orcid.as_deref(),
        &token,
    )
    .await
    {
        Ok(signed) => ToolResult::ok(NanopubResultDto {
            trusty_uri: signed.trusty_uri,
            published_state: "local".into(),
            validated_offline: true,
        })
        .to_json(),
        Err(e) => ToolResult::<NanopubResultDto>::err(anyhow_msg(e)).to_json(),
    }
}

pub async fn autofill(state: &ServerState, p: AutofillParams) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<AutofillResultDto>::err(e).to_json(),
    };
    let row = match load_manifest_row(db, &p.publication_id).await {
        Ok(r) => r,
        Err(e) => return ToolResult::<AutofillResultDto>::err(e).to_json(),
    };
    #[cfg(feature = "news-publish")]
    let out = publisher::autofill(db, &state.repository.root, row, p.apply).await;
    #[cfg(not(feature = "news-publish"))]
    let out: Result<AutofillResultDto, String> = {
        let _ = (row, p.apply);
        Err(NEEDS_NEWS_PUBLISH.to_string())
    };
    match out {
        Ok(r) => ToolResult::ok(r).to_json(),
        Err(e) => ToolResult::<AutofillResultDto>::err(e).to_json(),
    }
}

/// Mark a discovery-inbox row acknowledged (now). No-op for an unknown id.
pub async fn discovery_ack(state: &ServerState, p: DiscoveryAckParams) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<()>::err(e).to_json(),
    };
    let now_ms = chrono::Utc::now().timestamp_millis();
    match db.acknowledge_discovery(p.id, now_ms).await {
        Ok(()) => ToolResult::ok(()).to_json(),
        Err(e) => ToolResult::<()>::err(format!("{e:#}")).to_json(),
    }
}

// ─── Routes ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct LimitQuery {
    pub limit: Option<i64>,
}

fn manifest_row_to_dto(m: &vox_db::PublicationManifestRow) -> PublicationManifestDto {
    let meta: Value = m
        .metadata_json
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| serde_json::from_str::<Value>(s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    let sci = meta.get("scientific_publication");

    let resolved_class = sci
        .and_then(|s| s.get("candidate_class"))
        .and_then(Value::as_str)
        .and_then(vox_scientia::class_routing::FindingClass::from_str)
        .or_else(|| vox_scientia::class_routing::FindingClass::from_str(&m.content_type))
        .unwrap_or(vox_scientia::class_routing::FindingClass::EmpiricalScience);
    let resolved_tier = sci
        .and_then(|s| s.get("publication_tier"))
        .and_then(Value::as_str)
        .and_then(vox_scientia::class_routing::PublicationTier::from_str)
        .unwrap_or_else(|| {
            vox_scientia::class_routing::PublicationTier::recommended_for_class(resolved_class)
        });
    let (default_primary, default_secs) =
        vox_scientia::class_routing::default_arxiv_categories_for_class(resolved_class);

    let arxiv_primary_category = sci
        .and_then(|s| s.get("arxiv_primary_category"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(default_primary)
        .to_string();

    let arxiv_secondary_categories = sci
        .and_then(|s| s.get("arxiv_secondary_categories"))
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| default_secs.into_iter().map(ToString::to_string).collect());

    let license_spdx = sci
        .and_then(|s| s.get("license_spdx"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("CC-BY-4.0")
        .to_string();

    let orcid = sci
        .and_then(|s| {
            s.get("orcid").or_else(|| {
                s.get("authors")
                    .and_then(Value::as_array)
                    .and_then(|a| a.first())
                    .and_then(|f| f.get("orcid"))
            })
        })
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string);

    let ror_affiliation = sci
        .and_then(|s| {
            s.get("ror_affiliation")
                .or_else(|| s.get("affiliation"))
                .or_else(|| {
                    s.get("authors")
                        .and_then(Value::as_array)
                        .and_then(|a| a.first())
                        .and_then(|f| f.get("affiliation"))
                })
        })
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string);

    let prereserved_doi = sci
        .and_then(|s| s.get("prereserved_doi").or_else(|| s.get("doi")))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string);

    let arxiv_id = sci
        .and_then(|s| s.get("arxiv_id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string);

    let flywheel_stages_done: Vec<String> = meta
        .pointer("/scientia_evidence/flywheel/stages_done")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default();

    let has_abs = m
        .abstract_text
        .as_deref()
        .is_some_and(|s| s.split_whitespace().count() >= 20);
    let has_body =
        m.body_markdown.contains("## Introduction") && m.body_markdown.contains("## Methods");
    let passed_signals = [
        resolved_class != vox_scientia::class_routing::FindingClass::Other,
        has_abs,
        has_body,
        m.citations_json
            .as_deref()
            .is_some_and(|s| !s.trim().is_empty()),
        orcid.is_some(),
        prereserved_doi.is_some(),
        flywheel_stages_done.len() >= 4,
        matches!(m.state.as_str(), "approved" | "submitted" | "published"),
    ]
    .into_iter()
    .filter(|b| *b)
    .count();
    let readiness_score = ((passed_signals * 100) / 8).clamp(25, 100) as u8;
    let arxiv_ready = has_abs && has_body;

    PublicationManifestDto {
        publication_id: m.publication_id.clone(),
        content_type: m.content_type.clone(),
        state: m.state.clone(),
        created_at_ms: m.created_at_ms,
        updated_at_ms: m.updated_at_ms,
        title: Some(vox_scientia::review_flow::synthesize_scholarly_title(
            Some(&m.title),
            Some(&m.body_markdown),
            resolved_class.as_str(),
            &m.publication_id,
        )),
        author: (!m.author.trim().is_empty()).then(|| m.author.clone()),
        abstract_text: m.abstract_text.clone(),
        candidate_class: Some(resolved_class.as_str().to_string()),
        publication_tier: Some(resolved_tier.as_str().to_string()),
        arxiv_primary_category: Some(arxiv_primary_category),
        arxiv_secondary_categories: Some(arxiv_secondary_categories),
        license_spdx: Some(license_spdx),
        orcid,
        ror_affiliation,
        prereserved_doi,
        arxiv_id,
        readiness_score: Some(readiness_score),
        arxiv_ready: Some(arxiv_ready),
        flywheel_stages_done: Some(flywheel_stages_done),
    }
}

/// GET /api/v2/scientia/manifests?limit=
pub async fn get_manifests(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<LimitQuery>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, &["scientia", "publications"]) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    match db
        .list_publication_manifests(None, None, q.limit.unwrap_or(DEFAULT_MANIFEST_LIMIT))
        .await
    {
        Ok(rows) => ok(rows.iter().map(manifest_row_to_dto).collect::<Vec<_>>()),
        Err(e) => err("db_error", &e.to_string()),
    }
}

async fn review_queue(
    db: &vox_db::VoxDb,
    publication_id: &str,
) -> Result<Vec<ClaimAwaitingReviewDto>, String> {
    let effective_id = review_flow::resolve_manifest_or_promote(db, publication_id)
        .await
        .map(|m| m.publication_id)
        .unwrap_or_else(|_| publication_id.to_string());
    let sid = review_flow::publication_session_id(&effective_id);
    let rows = db
        .list_claims_awaiting_review(sid, &effective_id)
        .await
        .map_err(|e| format!("{e:#}"))?;
    Ok(rows
        .into_iter()
        .map(|r| ClaimAwaitingReviewDto {
            claim_id: r.claim_id,
            text: r.text,
            is_numeric: r.is_numeric,
            verdict: r.verdict,
            confidence: r.confidence,
            verifier_model: r.verifier_model,
            created_at_ms: r.created_at_ms,
        })
        .collect())
}

/// GET /api/v2/scientia/publications/{id}/review-queue
pub async fn get_review_queue(
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
    match review_queue(&db, &publication_id).await {
        Ok(rows) => ok(rows),
        Err(e) => err("db_error", &e),
    }
}

/// GET /api/v2/scientia/publications/{id}/claims/{claim_id}/evidence — ADVISORY LLM suggestions
/// via `vox_scientia::evidence_assist`; never mutates; empty on LLM error.
pub async fn get_evidence_suggestions(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path((publication_id, claim_id)): Path<(String, i64)>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, SCIENTIA_SURFACES) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    let claims = match review_queue(&db, &publication_id).await {
        Ok(c) => c,
        Err(e) => return err("db_error", &e),
    };
    let Some(c) = claims.into_iter().find(|c| c.claim_id == claim_id) else {
        return err(
            "not_found",
            &format!("claim {claim_id} not in review queue"),
        );
    };
    ok(vox_scientia::evidence_assist::suggest(&c.text, c.verdict.as_deref(), c.confidence).await)
}

async fn manifest_row_for_route(
    gs: &GatewayState,
    publication_id: &str,
) -> Result<
    (
        std::sync::Arc<vox_db::VoxDb>,
        vox_db::PublicationManifestRow,
    ),
    Json<Value>,
> {
    let db = api_db(gs)?;
    match load_manifest_row(&db, publication_id).await {
        Ok(row) => Ok((db, row)),
        Err(e) => Err(err("not_found", &e)),
    }
}

/// GET /api/v2/scientia/publications/{id}/novelty — `insufficient_evidence` when no bundle.
pub async fn get_novelty(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(publication_id): Path<String>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, SCIENTIA_SURFACES) {
        return e;
    }
    let (_db, _row) = match manifest_row_for_route(&gs, &publication_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    #[cfg(feature = "news-publish")]
    return ok(publisher::novelty(&_row));
    #[cfg(not(feature = "news-publish"))]
    err("feature_disabled", NEEDS_NEWS_PUBLISH)
}

/// GET /api/v2/scientia/publications/{id}/completion
pub async fn get_completion(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(publication_id): Path<String>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, SCIENTIA_SURFACES) {
        return e;
    }
    let (_db, _row) = match manifest_row_for_route(&gs, &publication_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    #[cfg(feature = "news-publish")]
    return ok(publisher::completion(&_row));
    #[cfg(not(feature = "news-publish"))]
    err("feature_disabled", NEEDS_NEWS_PUBLISH)
}

/// GET /api/v2/scientia/publications/{id}/archive
pub async fn get_archive(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(publication_id): Path<String>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, SCIENTIA_SURFACES) {
        return e;
    }
    let (db, row) = match manifest_row_for_route(&gs, &publication_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    match archive_status(&db, &row).await {
        Ok(s) => ok(s),
        Err(e) => err("db_error", &e),
    }
}

/// GET /api/v2/scientia/discoveries?limit= — unacknowledged discoveries, newest first.
pub async fn get_discoveries(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<LimitQuery>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, SCIENTIA_SURFACES) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    let limit = q.limit.unwrap_or(DEFAULT_DISCOVERY_LIMIT).clamp(1, 500);
    match db.list_unacknowledged_discoveries(limit).await {
        Ok(rows) => ok(rows
            .into_iter()
            .map(DiscoveryInboxDto::from)
            .collect::<Vec<_>>()),
        Err(e) => err("db_error", &format!("{e:#}")),
    }
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct PromoteCandidateQuery {
    pub venue: Option<String>,
}

/// POST /api/v2/scientia/candidates/{id}/promote
pub async fn post_promote_candidate(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(candidate_id): Path<String>,
    Query(q): Query<PromoteCandidateQuery>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, SCIENTIA_SURFACES) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    match review_flow::promote_candidate_to_publication(&db, &candidate_id, q.venue.as_deref())
        .await
    {
        Ok(promoted) => ok(serde_json::json!({
            "ok": true,
            "publication_id": promoted.publication_id,
            "candidate_id": promoted.candidate_id,
            "session_id": promoted.session_id,
            "title": promoted.title,
            "claims_seeded": promoted.claims_seeded,
            "stage": promoted.state,
        })),
        Err(e) => err("not_found", &anyhow_msg(e)),
    }
}

async fn hydrated_scientific_spec(
    db: &vox_db::VoxDb,
    publication_id: &str,
) -> Result<
    (
        vox_scientia::manuscript::scaffold::ScaffoldInput,
        vox_scientia::manuscript::latex::ScientificPublicationSpec,
    ),
    String,
> {
    let manifest = review_flow::resolve_manifest_or_promote(db, publication_id)
        .await
        .map_err(|e| format!("{e:#}"))?;
    let input = review_flow::assemble_scaffold_from_db(db, &manifest.publication_id)
        .await
        .map_err(|e| format!("{e:#}"))?;
    let mut spec =
        vox_scientia::manuscript::latex::ScientificPublicationSpec::hydrate_from_manifest(
            &manifest.publication_id,
            &manifest.title,
            &manifest.author,
            manifest.abstract_text.as_deref(),
            &manifest.body_markdown,
            &manifest.content_type,
            manifest.metadata_json.as_deref(),
            &manifest.content_sha3_256,
            &input,
        );
    spec.autofill_missing_sections(&input);
    Ok((input, spec))
}

/// POST /api/v2/scientia/publications/{id}/manuscript
pub async fn post_publication_manuscript(
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
    let (input, spec) = match hydrated_scientific_spec(&db, &publication_id).await {
        Ok(v) => v,
        Err(e) => return err("not_found", &e),
    };
    let markdown = spec.to_scientific_markdown(&input);
    ok(serde_json::json!({
        "publication_id": spec.publication_id,
        "title": input.title_hint,
        "markdown": markdown,
        "scaffold": input,
        "spec": spec,
    }))
}

/// POST /api/v2/scientia/publications/{id}/latex
pub async fn post_publication_latex(
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
    let (input, spec) = match hydrated_scientific_spec(&db, &publication_id).await {
        Ok(v) => v,
        Err(e) => return err("not_found", &e),
    };
    let latex = vox_scientia::manuscript::latex::render_scientific_arxiv_latex(&input, &spec);
    let references_bib = vox_scientia::manuscript::latex::render_references_bibtex(
        &input.cited_facts,
        &input.results_rows,
        &spec,
    );
    let todo_stub_count = vox_scientia::manuscript::latex::count_latex_todo_stubs(&latex);
    ok(serde_json::json!({
        "publication_id": spec.publication_id,
        "title": input.title_hint,
        "latex": latex,
        "references_bib": references_bib,
        "todo_stub_count": todo_stub_count,
    }))
}

/// POST /api/v2/scientia/publications/{id}/arxiv-bundle
pub async fn post_publication_arxiv_bundle(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(publication_id): Path<String>,
) -> Json<Value> {
    use base64::Engine as _;

    if let Err(e) = enforce_route(&gs, &connect.0, &headers, SCIENTIA_SURFACES) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    let (input, spec) = match hydrated_scientific_spec(&db, &publication_id).await {
        Ok(v) => v,
        Err(e) => return err("not_found", &e),
    };
    match vox_scientia::manuscript::latex::render_scientific_arxiv_bundle(&input, &[], &spec) {
        Ok(bundle) => {
            let entries = bundle
                .entries
                .iter()
                .map(|(p, sz)| serde_json::json!({ "path": p, "bytes": sz }))
                .collect::<Vec<_>>();
            let validation_findings =
                vox_scientia::manuscript::latex::validate_arxiv_bundle_tar_gz(&bundle.tar_gz_bytes);
            let tar_gz_base64 =
                base64::engine::general_purpose::STANDARD.encode(&bundle.tar_gz_bytes);
            ok(serde_json::json!({
                "publication_id": spec.publication_id,
                "bytes_len": bundle.tar_gz_bytes.len(),
                "entries": entries,
                "main_tex": bundle.main_tex,
                "references_bib": bundle.references_bib,
                "arxiv_metadata_json": bundle.arxiv_metadata_json,
                "upload_checklist_md": bundle.upload_checklist_md,
                "validation_findings": validation_findings,
                "tar_gz_base64": tar_gz_base64,
            }))
        }
        Err(e) => err("bundle_error", &e.to_string()),
    }
}

/// POST /api/v2/scientia/publications/{id}/finding-page
pub async fn post_publication_finding_page(
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
    let page = match review_flow::assemble_finding_page_from_db(&db, &publication_id).await {
        Ok(p) => p,
        Err(e) => return err("not_found", &anyhow_msg(e)),
    };
    let html = vox_scientia::findings_site::render_finding_page(&page);
    let meta_tags = vox_scientia::findings_site::build_highwire_meta_tags(&page);
    ok(serde_json::json!({
        "publication_id": publication_id,
        "html": html,
        "meta_tags": meta_tags,
    }))
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct ReplayInputParams {
    pub measured_score: Option<f64>,
    pub wall_ms: Option<u64>,
}

/// POST /api/v2/scientia/publications/{id}/replay
pub async fn post_publication_replay(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(publication_id): Path<String>,
    Query(q): Query<ReplayInputParams>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, SCIENTIA_SURFACES) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    let score = q.measured_score.unwrap_or(1.0).clamp(0.0, 1.0);
    let report = vox_scientia::replay::ReplayReport {
        outcome: if score >= 0.5 {
            vox_scientia::replay::ReplayOutcome::Pass
        } else {
            vox_scientia::replay::ReplayOutcome::NonZeroExit { exit_code: 1 }
        },
        wall_ms: q.wall_ms.unwrap_or(42),
        measured_score: score,
        diagnostics: vec![],
        stdout: String::new(),
        stderr: String::new(),
    };
    match review_flow::record_replay_for_publication(&db, &publication_id, &report).await {
        Ok(recorded) => ok(serde_json::json!({
            "publication_id": publication_id,
            "artifact_replayability": recorded,
            "report": report,
        })),
        Err(e) => err("not_found", &anyhow_msg(e)),
    }
}

/// POST /api/v2/scientia/publications/{id}/extract-claims
pub async fn post_publication_extract_claims(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(publication_id): Path<String>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, SCIENTIA_SURFACES) {
        return e;
    }
    let (db, row) = match manifest_row_for_route(&gs, &publication_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    let context_str = row
        .citations_json
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(&row.body_markdown);
    let pipeline = vox_scientia::claim_extractor::ExtractionPipeline::new_calibrated(
        vox_scientia::claim_extractor::ExtractionConfig::default(),
    );
    let extracted = match pipeline.extract(&row.body_markdown, &[context_str]).await {
        Ok(r) => r,
        Err(e) => return err("extract_error", &e.to_string()),
    };
    let sid = review_flow::publication_session_id(&row.publication_id);
    let mut supported = 0u32;
    for (claim, verdict) in extracted.claims.iter().zip(extracted.verdicts.iter()) {
        let is_num = matches!(
            claim.verifiability,
            vox_scientia::claim_extractor::VerifiabilityClass::Numeric
        );
        let (vlabel, conf) = match verdict {
            vox_scientia::claim_extractor::ClaimVerdict::Supported { confidence } => {
                supported += 1;
                ("Supported", *confidence)
            }
            vox_scientia::claim_extractor::ClaimVerdict::Contested { confidence } => {
                ("Contested", *confidence)
            }
            vox_scientia::claim_extractor::ClaimVerdict::Contradicted { confidence } => {
                ("Contradicted", *confidence)
            }
            vox_scientia::claim_extractor::ClaimVerdict::Abstain { .. } => ("Abstain", 0.0),
        };
        let _ = db
            .store_claim(sid, claim.id, &claim.text, is_num, false, false)
            .await;
        let _ = db
            .store_claim_verdict(claim.id, vlabel, conf, "calibrated-hybrid:lexical")
            .await;
    }
    let _ = db
        .insert_scientia_cost_telemetry(
            "scientia-extractor",
            &row.publication_id,
            "vox-scientia",
            "extraction",
            None,
            Some("calibrated-hybrid:lexical"),
            None,
            None,
            0.0,
            None,
        )
        .await;
    ok(serde_json::json!({
        "publication_id": row.publication_id,
        "total_atomic": extracted.claims.len(),
        "supported": supported,
        "verifier_model": "calibrated-hybrid:lexical",
    }))
}

/// GET /api/v2/scientia/dashboard
pub async fn get_dashboard(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, SCIENTIA_SURFACES) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    let queue = match review_flow::assemble_scientia_queue_from_db(&db).await {
        Ok(q) => q,
        Err(e) => return err("db_error", &anyhow_msg(e)),
    };
    let cost = match review_flow::assemble_scientia_cost_from_db(&db).await {
        Ok(c) => c,
        Err(e) => return err("db_error", &anyhow_msg(e)),
    };
    ok(serde_json::json!({
        "queue": queue,
        "cost": cost,
    }))
}

pub fn api_v2_router() -> Router<GatewayState> {
    Router::new()
        .route("/scientia/manifests", get(get_manifests))
        .route("/scientia/dashboard", get(get_dashboard))
        .route(
            "/scientia/candidates/{id}/promote",
            get(post_promote_candidate).post(post_promote_candidate),
        )
        .route(
            "/scientia/publications/{id}/review-queue",
            get(get_review_queue),
        )
        .route(
            "/scientia/publications/{id}/claims/{claim_id}/evidence",
            get(get_evidence_suggestions),
        )
        .route("/scientia/publications/{id}/novelty", get(get_novelty))
        .route(
            "/scientia/publications/{id}/completion",
            get(get_completion),
        )
        .route("/scientia/publications/{id}/archive", get(get_archive))
        .route(
            "/scientia/publications/{id}/manuscript",
            get(post_publication_manuscript).post(post_publication_manuscript),
        )
        .route(
            "/scientia/publications/{id}/latex",
            get(post_publication_latex).post(post_publication_latex),
        )
        .route(
            "/scientia/publications/{id}/arxiv-bundle",
            get(post_publication_arxiv_bundle).post(post_publication_arxiv_bundle),
        )
        .route(
            "/scientia/publications/{id}/finding-page",
            get(post_publication_finding_page).post(post_publication_finding_page),
        )
        .route(
            "/scientia/publications/{id}/replay",
            get(post_publication_replay).post(post_publication_replay),
        )
        .route(
            "/scientia/publications/{id}/extract-claims",
            get(post_publication_extract_claims).post(post_publication_extract_claims),
        )
        .route("/scientia/discoveries", get(get_discoveries))
        .merge(super::scientia_studio::api_v2_router())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::gui_ops::test_support::{body, gateway_with_memory_db, peer, tool_json};

    #[test]
    fn router_builds() {
        let _ = api_v2_router();
    }

    /// Guard: the review surface must carry NO production-network publishing symbols. The
    /// needles are assembled at runtime so this file cannot trip its own assertion.
    #[test]
    fn no_network_publish_symbol_in_review_ops() {
        let src = include_str!("scientia.rs");
        let publish = format!("{}{}", "publish_to_", "network");
        let test_server = format!("{}{}", "use_test_", "server");
        assert!(!src.to_lowercase().contains(&publish));
        assert!(!src.contains(&test_server));
    }

    #[tokio::test]
    async fn nanopublish_requires_prior_approval() {
        let (gs, _db) = gateway_with_memory_db().await;
        let out = tool_json(
            &nanopublish(
                gs.server_state(),
                NanopublishParams {
                    publication_id: "pub-x".into(),
                    claim_id: 1,
                    orcid: None,
                },
            )
            .await,
        );
        assert_eq!(out["success"], false, "{out}");
        assert!(
            out["error"]
                .as_str()
                .unwrap()
                .contains("no review decision"),
            "{out}"
        );
    }

    #[tokio::test]
    async fn discovery_inbox_list_then_acknowledge() {
        let (gs, db) = gateway_with_memory_db().await;
        let id = db
            .insert_discovery_inbox(
                "commit-abc",
                1_000,
                "strong_candidate",
                r#"["research_pipeline.supported_claims"]"#,
            )
            .await
            .expect("insert");
        let list = |gs: GatewayState| async move {
            body(
                get_discoveries(
                    State(gs),
                    peer(),
                    HeaderMap::new(),
                    Query(LimitQuery { limit: None }),
                )
                .await,
            )
            .await
        };
        let rows = list(gs.clone()).await;
        assert_eq!(rows["data"][0]["publication_id"], "commit-abc", "{rows}");
        assert_eq!(rows["data"][0]["origin"], "research");

        let acked = tool_json(&discovery_ack(gs.server_state(), DiscoveryAckParams { id }).await);
        assert_eq!(acked["success"], true, "{acked}");
        assert!(list(gs).await["data"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn publication_routes_answer_on_an_empty_db() {
        let (gs, _db) = gateway_with_memory_db().await;
        let manifests = body(
            get_manifests(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                Query(LimitQuery { limit: None }),
            )
            .await,
        )
        .await;
        assert_eq!(manifests["data"], serde_json::json!([]), "{manifests}");
        let queue = body(
            get_review_queue(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                Path("pub-x".into()),
            )
            .await,
        )
        .await;
        assert_eq!(queue["data"], serde_json::json!([]), "{queue}");
        let archive = body(
            get_archive(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                Path("pub-x".into()),
            )
            .await,
        )
        .await;
        assert_eq!(archive["error"]["code"], "not_found", "{archive}");
        let evidence = body(
            get_evidence_suggestions(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                Path(("pub-x".into(), 7)),
            )
            .await,
        )
        .await;
        assert_eq!(evidence["error"]["code"], "not_found", "{evidence}");
    }

    #[tokio::test]
    async fn scientia_routes_are_profile_gated() {
        let named = ServerState::new_test().await.with_profile(
            crate::profiles::ActiveProfile::resolve(&["mesh"]).expect("mesh profile"),
        );
        let gs = GatewayState::for_test_with_state(named, HashSet::new()).await;
        let denied = body(
            get_discoveries(
                State(gs),
                peer(),
                HeaderMap::new(),
                Query(LimitQuery { limit: None }),
            )
            .await,
        )
        .await;
        assert_eq!(denied["error"]["code"], "tool_not_in_profile", "{denied}");
    }

    #[tokio::test]
    async fn ops_report_missing_db() {
        let st = ServerState::new_test().await;
        let out = tool_json(&discovery_ack(&st, DiscoveryAckParams { id: 1 }).await);
        assert!(
            out["error"]
                .as_str()
                .unwrap()
                .starts_with("VoxDb not attached")
        );
        let out = tool_json(
            &autofill(
                &st,
                AutofillParams {
                    publication_id: "p".into(),
                    apply: false,
                },
            )
            .await,
        );
        assert_eq!(out["success"], false);
    }

    #[tokio::test]
    async fn promote_manuscript_latex_finding_page_replay_and_dashboard_routes_work() {
        let (gs, db) = gateway_with_memory_db().await;
        let cand_row = vox_db::store::FindingCandidateRow {
            candidate_id: "finding-phase53-test".to_string(),
            candidate_class: vox_db::store::FindingCandidateClass::AlgorithmicImprovement,
            publication_id: None,
            title_hint: Some("Multi-Wave Claim Re-Verification Study".to_string()),
            internal_signals_json: serde_json::json!([
                {
                    "code": "research_pipeline.supported_claims",
                    "summary": "Supported claim: lexical overlap improves precision by 24% [1]. Supported claim: contradiction detection catches sign flips [2]."
                }
            ])
            .to_string(),
            novelty_evidence_bundle_id: None,
            worthiness_decision_ref: None,
            confidence_json: Some(
                serde_json::json!({
                    "signal_strength": 0.88,
                    "contradiction_risk": 0.05,
                    "reproducibility_support": 0.8
                })
                .to_string(),
            ),
            repository_id: None,
            producer_name: "vox-research-shim".to_string(),
            signal_fingerprint: "finding-phase53-test".to_string(),
            created_at_ms: 1_700_000_000_000,
            updated_at_ms: 1_700_000_000_000,
        };
        db.insert_finding_candidate(&cand_row)
            .await
            .expect("insert candidate");

        // 1. Promote candidate to publication manifest + seed claims
        let promoted = body(
            post_promote_candidate(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                Path("finding-phase53-test".into()),
                Query(PromoteCandidateQuery {
                    venue: Some("neurips_workshop".into()),
                }),
            )
            .await,
        )
        .await;
        assert_eq!(promoted["data"]["ok"], true, "{promoted}");
        assert_eq!(
            promoted["data"]["candidate_id"], "finding-phase53-test",
            "{promoted}"
        );
        assert!(
            promoted["data"]["publication_id"]
                .as_str()
                .unwrap_or("")
                .starts_with("pub-"),
            "{promoted}"
        );
        assert!(
            promoted["data"]["claims_seeded"].as_u64().unwrap_or(0) >= 1,
            "{promoted}"
        );

        // 2. Manuscript IMRaD from DB
        let ms = body(
            post_publication_manuscript(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                Path("finding-phase53-test".into()),
            )
            .await,
        )
        .await;
        assert!(
            ms["data"]["markdown"]
                .as_str()
                .unwrap_or("")
                .contains("Multi-Wave Claim Re-Verification Study"),
            "{ms}"
        );

        // 3. LaTeX from DB
        let tex = body(
            post_publication_latex(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                Path("finding-phase53-test".into()),
            )
            .await,
        )
        .await;
        assert!(
            tex["data"]["latex"]
                .as_str()
                .unwrap_or("")
                .contains("\\documentclass"),
            "{tex}"
        );

        // 4. Finding page HTML from DB
        let fp = body(
            post_publication_finding_page(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                Path("finding-phase53-test".into()),
            )
            .await,
        )
        .await;
        assert!(
            fp["data"]["html"]
                .as_str()
                .unwrap_or("")
                .to_ascii_lowercase()
                .contains("<!doctype html>"),
            "{fp}"
        );

        // 5. Replay recording
        let rep = body(
            post_publication_replay(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                Path("finding-phase53-test".into()),
                Query(ReplayInputParams {
                    measured_score: Some(0.95),
                    wall_ms: Some(120),
                }),
            )
            .await,
        )
        .await;
        assert_eq!(rep["data"]["artifact_replayability"], 0.95, "{rep}");

        // 6. Dashboard snapshot
        let dash = body(get_dashboard(State(gs.clone()), peer(), HeaderMap::new()).await).await;
        assert!(
            dash["data"]["queue"]["candidates"]["total"]
                .as_u64()
                .unwrap_or(0)
                >= 1,
            "{dash}"
        );
    }
}
