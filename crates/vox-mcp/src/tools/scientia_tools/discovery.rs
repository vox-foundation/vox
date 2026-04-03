//! MCP tools: deterministic SCIENTIA discovery scan / explain (parity with `vox db`).

use crate::{ServerState, ToolResult};
use schemars::JsonSchema;
use serde::Deserialize;

use super::common::{
    REM_PUBLICATION_ID, REM_SCIENTIA_DB, REM_SCIENTIA_METADATA, no_voxdb_tool_string,
};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationDiscoveryScanParams {
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default = "default_discovery_limit")]
    pub limit: i64,
}

fn default_discovery_limit() -> i64 {
    50
}

pub async fn vox_scientia_publication_discovery_scan(
    state: &ServerState,
    params: VoxScientiaPublicationDiscoveryScanParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    let ct = params
        .content_type
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let st = params
        .state
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let rows = match db.list_publication_manifests(ct, st, params.limit).await {
        Ok(r) => r,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(
                format!("DB error: {e}"),
                REM_SCIENTIA_DB,
            )
            .to_json();
        }
    };
    let scientia_h = vox_publisher::scientia_heuristics::ScientiaHeuristics::load_from_repo_root(
        &state.repository.root,
    );
    let mut candidates: Vec<serde_json::Value> = Vec::new();
    for row in rows {
        let evidence =
            vox_publisher::scientia_evidence::parse_scientia_evidence(row.metadata_json.as_deref())
                .unwrap_or_default();
        let rank = vox_publisher::scientia_discovery::rank_candidate_heuristics(
            row.publication_id.as_str(),
            row.source_ref.as_deref(),
            &evidence,
            &scientia_h,
        );
        candidates.push(serde_json::json!({
            "publication_id": row.publication_id,
            "content_type": row.content_type,
            "state": row.state,
            "updated_at_ms": row.updated_at_ms,
            "rank": rank,
        }));
    }
    candidates.sort_by(|a, b| {
        let sa = a["rank"]["rank_score"].as_u64().unwrap_or(0);
        let sb = b["rank"]["rank_score"].as_u64().unwrap_or(0);
        sb.cmp(&sa)
    });
    ToolResult::ok(serde_json::json!({
        "schema_kind": "scientia_discovery_scan",
        "candidates": candidates,
    }))
    .to_json()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationDiscoveryExplainParams {
    pub publication_id: String,
}

pub async fn vox_scientia_publication_discovery_explain(
    state: &ServerState,
    params: VoxScientiaPublicationDiscoveryExplainParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    let row = match db.get_publication_manifest(&params.publication_id).await {
        Ok(r) => r,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(
                format!("DB error: {e}"),
                REM_SCIENTIA_DB,
            )
            .to_json();
        }
    };
    let Some(row) = row else {
        return ToolResult::<String>::err_with_remediation(
            "publication not found",
            REM_PUBLICATION_ID,
        )
        .to_json();
    };
    let manifest = super::common::publication_manifest_from_row(&row);
    let evidence =
        vox_publisher::scientia_evidence::parse_scientia_evidence(row.metadata_json.as_deref())
            .unwrap_or_default();
    let scientia_h = vox_publisher::scientia_heuristics::ScientiaHeuristics::load_from_repo_root(
        &state.repository.root,
    );
    let mut rank = vox_publisher::scientia_discovery::rank_candidate_heuristics(
        params.publication_id.as_str(),
        row.source_ref.as_deref(),
        &evidence,
        &scientia_h,
    );
    let novelty_bundle =
        vox_publisher::scientia_prior_art::parse_novelty_bundle_from_metadata_json(
            row.metadata_json.as_deref(),
        );
    if let Some(ref b) = novelty_bundle {
        vox_publisher::scientia_discovery::merge_novelty_overlap_into_rank(&mut rank, b);
    }
    let completion = vox_publisher::scientia_discovery::manifest_completion_report(&manifest);
    let preview = vox_publisher::scientia_discovery::destination_transform_previews(
        &manifest,
        Some(&evidence),
    );
    let impact_readership_projection = novelty_bundle.as_ref().map(|b| {
        vox_publisher::scientia_finding_ledger::impact_readership_projection_v1(b, &scientia_h)
    });
    ToolResult::ok(serde_json::json!({
        "publication_id": params.publication_id,
        "discovery_rank": rank,
        "novelty_evidence_bundle": novelty_bundle,
        "manifest_completion": completion,
        "evidence_completeness_0_100": vox_publisher::scientia_discovery::evidence_completeness_score(&evidence, &scientia_h),
        "transform_preview": preview,
        "impact_readership_projection": impact_readership_projection,
    }))
    .to_json()
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct VoxScientiaPublicationDiscoveryRefreshEvidenceParams {
    pub publication_id: String,
}

/// Live Socrates merge, repo sidecar hydration, `scientia_evidence` rebuild (CLI parity minus mens-only eval-gate `check_run`).
pub async fn vox_scientia_publication_discovery_refresh_evidence(
    state: &ServerState,
    params: VoxScientiaPublicationDiscoveryRefreshEvidenceParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    let repo_root = state.repository.root.clone();
    let repository_id = state.repository.repository_id.as_str();
    let row = match db.get_publication_manifest(&params.publication_id).await {
        Ok(r) => r,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(
                format!("DB error: {e}"),
                REM_SCIENTIA_DB,
            )
            .to_json();
        }
    };
    let Some(row) = row else {
        return ToolResult::<String>::err_with_remediation(
            "publication not found",
            REM_PUBLICATION_ID,
        )
        .to_json();
    };
    if row.content_type != "scientia" {
        return ToolResult::<()>::err_with_remediation(
            format!(
                "content_type must be scientia for discovery refresh (got `{}`)",
                row.content_type
            ),
            "Use a scientia manifest or run `vox db publication-discovery-refresh-evidence` for supported types when implemented.",
        )
        .to_json();
    }
    let mut manifest = super::common::publication_manifest_from_row(&row);
    let merged = match db
        .merge_scientia_live_socrates_into_metadata_json(
            manifest.metadata_json.as_deref(),
            repository_id,
        )
        .await
    {
        Ok(s) => s,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(
                format!("DB error: {e}"),
                REM_SCIENTIA_DB,
            )
            .to_json();
        }
    };
    manifest.metadata_json = Some(merged);
    match vox_publisher::scientia_evidence::enrich_metadata_json_with_repo_files(
        manifest.metadata_json.as_deref(),
        &repo_root,
    ) {
        Ok(Some(updated)) => manifest.metadata_json = Some(updated),
        Ok(None) => {}
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(
                format!("scientia_evidence file hydration: {e}"),
                REM_SCIENTIA_METADATA,
            )
            .to_json();
        }
    }
    let scientific = vox_publisher::publication_preflight::parse_scientific_from_metadata_json(
        manifest.metadata_json.as_deref(),
    )
    .ok()
    .flatten();
    let new_meta = match vox_publisher::scientia_evidence::rebuild_scientia_evidence_metadata_json(
        manifest.metadata_json.as_deref(),
        manifest.body_markdown.as_str(),
        manifest.abstract_text.as_deref(),
        manifest.citations_json.as_deref(),
        scientific.as_ref(),
        manifest
            .source_ref
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty()),
        Some("vox_scientia_publication_discovery_refresh_evidence"),
    ) {
        Ok(s) => s,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(
                format!("metadata_json: {e}"),
                REM_SCIENTIA_METADATA,
            )
            .to_json();
        }
    };
    manifest.metadata_json = Some(new_meta);
    let digest = manifest.content_sha3_256();
    if let Err(e) = db
        .upsert_publication_manifest(vox_db::PublicationManifestParams {
            publication_id: &manifest.publication_id,
            content_type: &manifest.content_type,
            source_ref: manifest.source_ref.as_deref(),
            title: &manifest.title,
            author: &manifest.author,
            abstract_text: manifest.abstract_text.as_deref(),
            body_markdown: &manifest.body_markdown,
            citations_json: manifest.citations_json.as_deref(),
            metadata_json: manifest.metadata_json.as_deref(),
            content_sha3_256: &digest,
            state: row.state.as_str(),
        })
        .await
    {
        return ToolResult::<String>::err_with_remediation(
            format!("DB error: {e}"),
            REM_SCIENTIA_DB,
        )
        .to_json();
    }
    let evidence = vox_publisher::scientia_evidence::parse_scientia_evidence(
        manifest.metadata_json.as_deref(),
    )
    .unwrap_or_default();
    let scientia_h = vox_publisher::scientia_heuristics::ScientiaHeuristics::load_from_repo_root(
        &repo_root,
    );
    let rank = vox_publisher::scientia_discovery::rank_candidate_heuristics(
        params.publication_id.as_str(),
        manifest.source_ref.as_deref(),
        &evidence,
        &scientia_h,
    );
    let detail = serde_json::json!({ "digest": digest, "rank": rank });
    if let Err(e) = db
        .append_publication_status_event(
            &params.publication_id,
            "discovery_evidence_refreshed",
            Some(&serde_json::to_string(&detail).unwrap_or_default()),
        )
        .await
    {
        return ToolResult::<String>::err_with_remediation(
            format!("DB error: {e}"),
            REM_SCIENTIA_DB,
        )
        .to_json();
    }
    ToolResult::ok(serde_json::json!({
        "publication_id": params.publication_id,
        "digest": digest,
        "rank": rank,
    }))
    .to_json()
}
