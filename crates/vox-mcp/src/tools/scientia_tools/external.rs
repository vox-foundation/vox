use crate::{ServerState, ToolResult};
use schemars::JsonSchema;
use serde::Deserialize;

use super::common::{default_one_u32, no_voxdb_tool_string, REM_SCIENTIA_DB};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationExternalJobsDueParams {
    #[serde(default = "default_jobs_due_limit")]
    pub limit: i64,
}

fn default_jobs_due_limit() -> i64 {
    50
}

pub async fn vox_scientia_publication_external_jobs_due(
    state: &ServerState,
    params: VoxScientiaPublicationExternalJobsDueParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    let before_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    match db
        .list_external_submission_jobs_due(before_ms, params.limit)
        .await
    {
        Ok(jobs) => ToolResult::ok(serde_json::json!({
            "due_before_ms_inclusive": before_ms,
            "jobs": jobs,
        }))
        .to_json(),
        Err(e) => ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json(),
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationExternalJobsDeadLetterParams {
    #[serde(default = "default_jobs_dead_letter_limit")]
    pub limit: i64,
}

fn default_jobs_dead_letter_limit() -> i64 {
    50
}

pub async fn vox_scientia_publication_external_jobs_dead_letter(
    state: &ServerState,
    params: VoxScientiaPublicationExternalJobsDeadLetterParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    match db.list_external_submission_jobs_failed(params.limit).await {
        Ok(jobs) => ToolResult::ok(serde_json::json!({ "jobs": jobs })).to_json(),
        Err(e) => ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json(),
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationExternalJobsReplayParams {
    pub job_id: i64,
}

pub async fn vox_scientia_publication_external_jobs_replay(
    state: &ServerState,
    params: VoxScientiaPublicationExternalJobsReplayParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    match db
        .replay_failed_external_submission_job_to_queued(params.job_id)
        .await
    {
        Ok(job) => ToolResult::ok(serde_json::json!({
            "replayed": true,
            "job": job,
        }))
        .to_json(),
        Err(e) => ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json(),
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationExternalJobsTickParams {
    #[serde(default = "default_jobs_tick_limit")]
    pub limit: i64,
    #[serde(default = "default_jobs_tick_lock_ttl_ms")]
    pub lock_ttl_ms: i64,
    #[serde(default)]
    pub lock_owner: Option<String>,
    #[serde(default = "default_one_u32")]
    pub iterations: u32,
    #[serde(default)]
    pub interval_secs: u64,
    #[serde(default)]
    pub max_runtime_secs: Option<u64>,
    #[serde(default)]
    pub jitter_secs: u64,
}

fn default_jobs_tick_limit() -> i64 {
    10
}

fn default_jobs_tick_lock_ttl_ms() -> i64 {
    120_000
}

pub async fn vox_scientia_publication_external_jobs_tick(
    state: &ServerState,
    params: VoxScientiaPublicationExternalJobsTickParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    if params.iterations <= 1
        && params.interval_secs == 0
        && params.max_runtime_secs.is_none()
        && params.jitter_secs == 0
    {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        return match vox_publisher::scholarly_external_jobs::run_external_submit_jobs_tick(
            db,
            params.limit,
            params.lock_ttl_ms,
            params.lock_owner.as_deref(),
            now_ms,
        )
        .await
        {
            Ok(out) => ToolResult::ok(serde_json::json!({
                "now_ms": now_ms,
                "lock_owner": out.lock_owner,
                "lock_ttl_ms": out.lock_ttl_ms,
                "results": out.results,
            }))
            .to_json(),
            Err(e) => ToolResult::<String>::err_with_remediation(e.to_string(), REM_SCIENTIA_DB).to_json(),
        };
    }
    match vox_publisher::scholarly_external_jobs::run_external_submit_jobs_tick_loop(
        db,
        params.limit,
        params.lock_ttl_ms,
        params.lock_owner.as_deref(),
        params.iterations,
        params.interval_secs,
        params.max_runtime_secs,
        params.jitter_secs,
    )
    .await
    {
        Ok(v) => ToolResult::ok(v).to_json(),
        Err(e) => ToolResult::<String>::err_with_remediation(e.to_string(), REM_SCIENTIA_DB).to_json(),
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationExternalPipelineMetricsParams {
    /// Hours of history for attempts, snapshots, terminal latencies, and publication_attempts (0 = all time). Clamped 0–8760.
    #[serde(default = "default_metrics_since_hours")]
    pub since_hours: i64,
}

fn default_metrics_since_hours() -> i64 {
    168
}

pub async fn vox_scientia_publication_external_pipeline_metrics(
    state: &ServerState,
    params: VoxScientiaPublicationExternalPipelineMetricsParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let hours = params.since_hours.clamp(0, 8_760);
    let since_ms = if hours == 0 {
        0_i64
    } else {
        now_ms.saturating_sub(hours.saturating_mul(3_600_000))
    };
    match db.summarize_scholarly_external_pipeline_metrics(since_ms).await {
        Ok(v) => ToolResult::ok(v).to_json(),
        Err(e) => ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json(),
    }
}
