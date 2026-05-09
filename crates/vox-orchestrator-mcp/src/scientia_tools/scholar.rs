use crate::params::ToolResult;
use crate::server_state::ServerState;
use schemars::JsonSchema;
use serde::Deserialize;
use vox_publisher::publication::PublicationManifest;
use vox_publisher::publication_preflight::PreflightProfile;
use vox_publisher::scholarly_external_jobs::{
    poll_scholarly_remote_status_all_submissions_for_publication,
    poll_scholarly_remote_status_batch, poll_scholarly_remote_status_persist,
    publication_scholarly_submit_with_ledger,
};
use vox_publisher::submission::{
    ScholarlyVenue, StagingExportError, ValidationFinding, validate_scholarly_staging,
    write_scholarly_staging,
};

use super::common::default_one_u32;
use super::common::{
    REM_PUBLICATION_ID, REM_SCIENTIA_ARXIV, REM_SCIENTIA_DB, REM_SCIENTIA_EXT_SUBMIT,
    REM_SCIENTIA_METADATA, REM_SCIENTIA_OUTPUT_DIR, REM_SCIENTIA_REMOTE, REM_SCIENTIA_STAGE,
    no_voxdb_tool_string,
};
use super::lifecycle::PreflightProfileParam;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationScholarlyRemoteStatusParams {
    pub publication_id: String,
    #[serde(default)]
    pub external_submission_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationScholarlyRemoteStatusSyncAllParams {
    pub publication_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationScholarlyRemoteStatusSyncBatchParams {
    #[serde(default = "default_scholarly_remote_sync_batch_limit")]
    pub limit: i64,
    #[serde(default = "default_one_u32")]
    pub iterations: u32,
    #[serde(default)]
    pub interval_secs: u64,
    #[serde(default)]
    pub max_runtime_secs: Option<u64>,
    #[serde(default)]
    pub jitter_secs: u64,
}

fn default_scholarly_remote_sync_batch_limit() -> i64 {
    25
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationScholarlyStagingExportParams {
    pub publication_id: String,
    /// Absolute or process-relative directory; created if missing.
    pub output_dir: String,
    /// `zenodo`, `openreview`, or `arxiv-assist` (same tokens as `ScholarlyVenue::parse`).
    pub venue: String,
}

pub async fn vox_scientia_publication_scholarly_staging_export(
    state: &ServerState,
    params: VoxScientiaPublicationScholarlyStagingExportParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    let publication_id = params.publication_id.trim();
    if publication_id.is_empty() {
        return ToolResult::<String>::err_with_remediation(
            "publication_id must not be empty".to_string(),
            REM_PUBLICATION_ID,
        )
        .to_json();
    }
    let out_s = params.output_dir.trim();
    if out_s.is_empty() {
        return ToolResult::<String>::err_with_remediation(
            "output_dir must not be empty".to_string(),
            REM_SCIENTIA_OUTPUT_DIR,
        )
        .to_json();
    }
    let venue_raw = params.venue.trim();
    let Some(venue) = ScholarlyVenue::parse(venue_raw) else {
        return ToolResult::<String>::err_with_remediation(
            format!("unknown venue {venue_raw:?}"),
            "Use `zenodo`, `openreview`, or `arxiv-assist` for `venue` (see `ScholarlyVenue::parse` in vox-publisher).",
        )
        .to_json();
    };
    let output_dir = std::path::PathBuf::from(out_s);
    let row = match db.get_publication_manifest(publication_id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return ToolResult::<String>::err_with_remediation(
                format!("publication not found: {publication_id}"),
                REM_PUBLICATION_ID,
            )
            .to_json();
        }
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(
                format!("DB error: {e}"),
                REM_SCIENTIA_DB,
            )
            .to_json();
        }
    };
    let manifest = PublicationManifest {
        publication_id: row.publication_id.clone(),
        content_type: row.content_type.clone(),
        source_ref: row.source_ref.clone(),
        title: row.title.clone(),
        author: row.author.clone(),
        abstract_text: row.abstract_text.clone(),
        body_markdown: row.body_markdown.clone(),
        citations_json: row.citations_json.clone(),
        metadata_json: row.metadata_json.clone(),
    };
    let written = match write_scholarly_staging(&manifest, venue, &output_dir) {
        Ok(w) => w,
        Err(e) => {
            let e: StagingExportError = e;
            return ToolResult::<String>::err_with_remediation(
                e.to_string(),
                REM_SCIENTIA_OUTPUT_DIR,
            )
            .to_json();
        }
    };
    if let Err(findings) = validate_scholarly_staging(&output_dir, venue, &manifest) {
        let findings: Vec<ValidationFinding> = findings;
        let msg: String = findings
            .iter()
            .map(|f: &ValidationFinding| format!("{}: {}", f.code, f.message))
            .collect::<Vec<String>>()
            .join("; ");
        return ToolResult::<String>::err_with_remediation(
            format!("staging validation failed: {msg}"),
            "Inspect `written` paths under output_dir; re-run export or fix files to match the venue plan (see vox-publisher `submission::staging_artifacts`).",
        )
        .to_json();
    }
    ToolResult::ok(serde_json::json!({
        "publication_id": publication_id,
        "output_dir": output_dir,
        "venue": venue.as_str().to_string(),
        "written": written,
    }))
    .to_json()
}

pub async fn vox_scientia_publication_scholarly_remote_status(
    state: &ServerState,
    params: VoxScientiaPublicationScholarlyRemoteStatusParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    let submissions = match db.list_scholarly_submissions(&params.publication_id).await {
        Ok(v) => v,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(
                format!("DB error: {e}"),
                REM_SCIENTIA_DB,
            )
            .to_json();
        }
    };
    let sub_row: &vox_db::ScholarlySubmissionRow = match params.external_submission_id.as_deref() {
        Some(e) => {
            let e = e.trim();
            if e.is_empty() {
                return ToolResult::<String>::err_with_remediation(
                    "external_submission_id must not be empty when provided".to_string(),
                    REM_SCIENTIA_EXT_SUBMIT,
                )
                .to_json();
            }
            let Some(row) = submissions.iter().find(|r| r.external_submission_id == e) else {
                return ToolResult::<String>::err_with_remediation(
                    format!("no scholarly submission with external_submission_id {e}"),
                    REM_SCIENTIA_REMOTE,
                )
                .to_json();
            };
            row
        }
        None => {
            let Some(row) = submissions.first() else {
                return ToolResult::<String>::err_with_remediation(
                    "no scholarly submissions for this publication".to_string(),
                    REM_SCIENTIA_REMOTE,
                )
                .to_json();
            };
            row
        }
    };
    match poll_scholarly_remote_status_persist(db, params.publication_id.as_str(), sub_row).await {
        Ok(v) => ToolResult::ok(v).to_json(),
        Err(e) => {
            ToolResult::<String>::err_with_remediation(e.to_string(), REM_SCIENTIA_DB).to_json()
        }
    }
}

pub async fn vox_scientia_publication_scholarly_remote_status_sync_all(
    state: &ServerState,
    params: VoxScientiaPublicationScholarlyRemoteStatusSyncAllParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    let publication_id = params.publication_id.trim();
    if publication_id.is_empty() {
        return ToolResult::<String>::err_with_remediation(
            "publication_id must not be empty".to_string(),
            REM_PUBLICATION_ID,
        )
        .to_json();
    }
    match poll_scholarly_remote_status_all_submissions_for_publication(db, publication_id).await {
        Ok(v) => ToolResult::ok(v).to_json(),
        Err(e) => {
            ToolResult::<String>::err_with_remediation(e.to_string(), REM_SCIENTIA_DB).to_json()
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationScholarlyPipelineRunParams {
    pub publication_id: String,
    #[serde(default)]
    pub preflight_profile: Option<PreflightProfileParam>,
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default)]
    pub staging_output_dir: Option<String>,
    /// When `staging_output_dir` is set: `zenodo`, `openreview`, or `arxiv-assist`.
    #[serde(default)]
    pub venue: Option<String>,
    #[serde(default)]
    pub adapter: Option<String>,
    /// When true, emit compact JSON in the tool result (single line).
    #[serde(default)]
    pub json_compact: bool,
}

pub async fn vox_scientia_publication_scholarly_pipeline_run(
    state: &ServerState,
    params: VoxScientiaPublicationScholarlyPipelineRunParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    let publication_id = params.publication_id.trim();
    if publication_id.is_empty() {
        return ToolResult::<String>::err_with_remediation(
            "publication_id must not be empty".to_string(),
            REM_PUBLICATION_ID,
        )
        .to_json();
    }
    let profile: PreflightProfile = params.preflight_profile.unwrap_or_default().into();
    let row = match db.get_publication_manifest(publication_id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return ToolResult::<String>::err_with_remediation(
                format!("publication not found: {publication_id}"),
                REM_PUBLICATION_ID,
            )
            .to_json();
        }
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(
                format!("DB error: {e}"),
                REM_SCIENTIA_DB,
            )
            .to_json();
        }
    };
    let manifest = PublicationManifest {
        publication_id: row.publication_id.clone(),
        content_type: row.content_type.clone(),
        source_ref: row.source_ref.clone(),
        title: row.title.clone(),
        author: row.author.clone(),
        abstract_text: row.abstract_text.clone(),
        body_markdown: row.body_markdown.clone(),
        citations_json: row.citations_json.clone(),
        metadata_json: row.metadata_json.clone(),
    };
    let report = match super::lifecycle::publication_preflight_report_for_row(
        db,
        &row,
        &manifest,
        profile,
        state.orchestrator_config.news.dry_run,
        state.orchestrator_config.news.publish_armed,
        &state.repository.root,
        Some(state.repository.repository_id.as_str()),
        false,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(e, REM_SCIENTIA_METADATA).to_json();
        }
    };
    if !report.ok {
        return ToolResult::<String>::err_with_remediation(
            format!(
                "scholarly pipeline preflight failed (readiness {}): {}",
                report.readiness_score,
                serde_json::to_string(&report).unwrap_or_else(|_| "{}".into())
            ),
            "Fix preflight findings on the stored manifest or pass a different `preflight_profile`; compare with `vox scientia publication-preflight`.",
        )
        .to_json();
    }
    let digest = row.content_sha3_256.clone();
    let dual = match db
        .has_dual_publication_approval_for_digest(publication_id, &digest)
        .await
    {
        Ok(b) => b,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(
                format!("DB error: {e}"),
                REM_SCIENTIA_DB,
            )
            .to_json();
        }
    };
    if !dual {
        return ToolResult::<String>::err_with_remediation(
            "scholarly pipeline requires two distinct digest-bound approvers before staging export / submit",
            "Record two digest-bound approvers with `vox scientia publication-approve --publication-id ...` (distinct `--approver` values), then retry.",
        )
        .to_json();
    }
    let mut stages: Vec<String> = vec!["preflight_ok".into(), "dual_approval_ok".into()];

    let out_dir = params
        .staging_output_dir
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let venue_raw = params
        .venue
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    match (venue_raw, out_dir) {
        (Some(vs), Some(od)) => {
            let Some(venue) = ScholarlyVenue::parse(vs) else {
                return ToolResult::<String>::err_with_remediation(
                    format!("unknown venue {vs:?}"),
                    "Use `zenodo`, `openreview`, or `arxiv-assist` for `venue` when `staging_output_dir` is set.",
                )
                .to_json();
            };
            if params.dry_run {
                stages.push(format!("staging_skipped_dry_run venue={vs} dir={od}"));
            } else {
                let output_path = std::path::Path::new(od);
                if let Err(e) = write_scholarly_staging(&manifest, venue, output_path) {
                    let e: StagingExportError = e;
                    return ToolResult::<String>::err_with_remediation(
                        e.to_string(),
                        REM_SCIENTIA_OUTPUT_DIR,
                    )
                    .to_json();
                }
                if let Err(findings) = validate_scholarly_staging(output_path, venue, &manifest) {
                    let findings: Vec<ValidationFinding> = findings;
                    let msg: String = findings
                        .iter()
                        .map(|f: &ValidationFinding| format!("{}: {}", f.code, f.message))
                        .collect::<Vec<String>>()
                        .join("; ");
                    return ToolResult::<String>::err_with_remediation(
                        format!("staging validation failed: {msg}"),
                        "Inspect staging under output_dir; re-export with matching venue or fix validation codes reported above.",
                    )
                    .to_json();
                }
                stages.push("staging_exported".into());
            }
        }
        (None, Some(_)) => {
            return ToolResult::<String>::err_with_remediation(
                "staging_output_dir requires venue",
                "Pass `venue` (`zenodo`, `openreview`, or `arxiv-assist`) whenever `staging_output_dir` is set (matches CLI).",
            )
            .to_json();
        }
        (Some(_), None) => {
            return ToolResult::<String>::err_with_remediation(
                "venue requires staging_output_dir",
                "Set `staging_output_dir` to the directory that should receive staging files, or omit both for submit-only.",
            )
            .to_json();
        }
        (None, None) => {}
    }

    let compact = params.json_compact;
    if params.dry_run {
        let tr = ToolResult::ok(serde_json::json!({
            "dry_run": true,
            "publication_id": publication_id,
            "digest": digest,
            "stages": stages,
            "preflight_report": report,
        }));
        return if compact {
            tr.to_json_compact()
        } else {
            tr.to_json()
        };
    }

    match publication_scholarly_submit_with_ledger(db, publication_id, params.adapter.as_deref()).await
    {
        Ok(receipt) => {
            let tr = ToolResult::ok(serde_json::json!({
                "pipeline_completed": true,
                "publication_id": publication_id,
                "digest": digest,
                "stages": stages,
                "submission": {
                    "adapter": receipt.adapter,
                    "external_submission_id": receipt.external_submission_id,
                    "status": receipt.status,
                }
            }));
            if compact {
                tr.to_json_compact()
            } else {
                tr.to_json()
            }
        }
        Err(e) => ToolResult::<String>::err_with_remediation(
            e.to_string(),
            "Verify `VOX_SCHOLARLY_*` flags, adapter credentials (Secrets / env), dual approval, and that the stored digest matches the manifest.",
        )
        .to_json(),
    }
}

pub async fn vox_scientia_publication_scholarly_remote_status_sync_batch(
    state: &ServerState,
    params: VoxScientiaPublicationScholarlyRemoteStatusSyncBatchParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    let res = if params.iterations <= 1
        && params.interval_secs == 0
        && params.max_runtime_secs.is_none()
        && params.jitter_secs == 0
    {
        poll_scholarly_remote_status_batch(db, params.limit).await
    } else {
        vox_publisher::scholarly_external_jobs::poll_scholarly_remote_status_batch_loop(
            db,
            params.limit,
            params.iterations,
            params.interval_secs,
            params.max_runtime_secs,
            params.jitter_secs,
        )
        .await
    };
    match res {
        Ok(v) => ToolResult::ok(v).to_json(),
        Err(e) => {
            ToolResult::<String>::err_with_remediation(e.to_string(), REM_SCIENTIA_DB).to_json()
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationArxivHandoffRecordParams {
    pub publication_id: String,
    /// One of: staging_exported, operator_ack, bundle_validated, submitted, published.
    pub stage: String,
    #[serde(default)]
    pub operator: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub arxiv_id: Option<String>,
}

pub async fn vox_scientia_publication_arxiv_handoff_record(
    state: &ServerState,
    params: VoxScientiaPublicationArxivHandoffRecordParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    let publication_id = params.publication_id.trim();
    if publication_id.is_empty() {
        return ToolResult::<String>::err_with_remediation(
            "publication_id must not be empty".to_string(),
            REM_PUBLICATION_ID,
        )
        .to_json();
    }
    let stage = params.stage.trim().to_ascii_lowercase();
    let allowed = [
        "staging_exported",
        "operator_ack",
        "bundle_validated",
        "submitted",
        "published",
    ];
    if !allowed.contains(&stage.as_str()) {
        return ToolResult::<String>::err_with_remediation(
            format!(
                "invalid stage {stage:?}; expected one of {}",
                allowed.join(", ")
            ),
            REM_SCIENTIA_STAGE,
        )
        .to_json();
    }
    if stage == "published"
        && params
            .arxiv_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .is_none()
    {
        return ToolResult::<String>::err_with_remediation(
            "arxiv_id is required when stage is published".to_string(),
            REM_SCIENTIA_ARXIV,
        )
        .to_json();
    }
    match db.get_publication_manifest(publication_id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            return ToolResult::<String>::err_with_remediation(
                format!("publication not found: {publication_id}"),
                REM_PUBLICATION_ID,
            )
            .to_json();
        }
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(
                format!("DB error: {e}"),
                REM_SCIENTIA_DB,
            )
            .to_json();
        }
    }

    let op_trim = params
        .operator
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let note_trim = params
        .note
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let arxiv_trim = params
        .arxiv_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let status = format!("arxiv_handoff:{stage}");
    let detail = serde_json::json!({
        "schema_version": 1_u32,
        "workflow": "arxiv_operator_assist",
        "stage": stage,
        "operator": op_trim,
        "note": note_trim,
        "arxiv_id": arxiv_trim,
    });
    if let Err(e) = db
        .append_publication_status_event(publication_id, &status, Some(&detail.to_string()))
        .await
    {
        return ToolResult::<String>::err_with_remediation(
            format!("DB error: {e}"),
            REM_SCIENTIA_DB,
        )
        .to_json();
    }
    ToolResult::ok(serde_json::json!({
        "recorded": true,
        "publication_id": publication_id,
        "status": status,
        "detail": detail,
    }))
    .to_json()
}
