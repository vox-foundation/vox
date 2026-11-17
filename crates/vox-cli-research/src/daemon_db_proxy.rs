//! Transparent daemon IPC fallback when canonical `vox.db` holds a Turso/POSIX `flock`.
//!
//! When `VOX_DB_PATH` is not overridden and `VoxDb::connect` fails with a file-lock
//! error, `vox research` and `vox scientia` commands route through the running
//! `vox-orchestrator-d` daemon (`orch.api_v2_get`, `orch.tool_call`, `research.run`).

use std::io::Write;

use anyhow::{Context, Result};
use serde_json::{Value, json};
use vox_foundation::protocol::{dei_method, orch_daemon_method};

#[path = "daemon_db_proxy/formatters.rs"]
mod formatters;
#[path = "daemon_db_proxy/scientia_ops.rs"]
mod scientia_ops;

pub(crate) use formatters::{
    extract_research_result_payload, format_research_history_rows, format_research_show_detail,
    unwrap_api_v2_envelope, write_output_file,
};
pub use scientia_ops::{
    retrieval_status_via_daemon, scientia_claims_via_daemon, scientia_cost_via_daemon,
    scientia_dashboard_via_daemon,
};

/// Returns `true` when `err_text` indicates Turso / SQLite / POSIX `flock` contention.
#[must_use]
pub fn is_db_lock_error_message(err_text: &str) -> bool {
    let lower = err_text.to_ascii_lowercase();
    lower.contains("failed locking file")
        || lower.contains("databasebusy")
        || lower.contains("database is locked")
        || lower.contains("resource temporarily unavailable")
        || lower.contains("wouldblock")
        || lower.contains("file is locked")
        || lower.contains("is locked")
}

/// Pure helper checking both `VOX_DB_PATH` override absence and lock error text.
#[must_use]
pub fn is_canonical_db_lock_error_with_override(err_text: &str, vox_db_path: Option<&str>) -> bool {
    if vox_db_path.is_some_and(|p| !p.trim().is_empty()) {
        return false;
    }
    is_db_lock_error_message(err_text)
}

/// Returns `true` when `VOX_DB_PATH` is unset/empty and `err_text` matches a DB lock failure.
#[must_use]
pub fn is_canonical_db_lock_error(err_text: &str) -> bool {
    let override_path = std::env::var("VOX_DB_PATH").ok();
    is_canonical_db_lock_error_with_override(err_text, override_path.as_deref())
}

pub(crate) async fn api_v2_get_with_client(
    client: &vox_orchestrator::orch_daemon::OrchDaemonClient,
    path_and_query: &str,
) -> Result<Value> {
    let env = client
        .call(
            orch_daemon_method::API_V2_GET,
            json!({ "path": path_and_query }),
        )
        .await
        .with_context(|| format!("daemon GET /api/v2{path_and_query}"))?;
    unwrap_api_v2_envelope(env)
}

pub(crate) async fn api_v2_get(path_and_query: &str) -> Result<Value> {
    let client = crate::daemon_tool::daemon_client().await?;
    api_v2_get_with_client(&client, path_and_query).await
}

pub async fn research_history_via_daemon(limit: u32) -> Result<()> {
    let data = api_v2_get(&format!("/research/sessions?limit={limit}")).await?;
    let text = format_research_history_rows(&data);
    writeln!(std::io::stdout(), "{text}")?;
    Ok(())
}

pub async fn research_show_via_daemon(session_id: i64) -> Result<()> {
    let detail = api_v2_get(&format!("/research/sessions/{session_id}")).await?;
    let text = format_research_show_detail(session_id, &detail);
    writeln!(std::io::stdout(), "{text}")?;
    Ok(())
}

pub async fn research_result_via_daemon(
    session_id: i64,
    format: &str,
    output: Option<std::path::PathBuf>,
) -> Result<()> {
    let detail = api_v2_get(&format!("/research/sessions/{session_id}")).await?;
    let rendered = extract_research_result_payload(session_id, &detail, format)?;
    if let Some(path) = output {
        write_output_file(&path, &rendered)?;
    } else {
        writeln!(std::io::stdout(), "{rendered}")?;
    }
    Ok(())
}

pub async fn research_watch_via_daemon(session_id: i64) -> Result<()> {
    let client = crate::daemon_tool::daemon_client().await?;
    loop {
        let detail =
            api_v2_get_with_client(&client, &format!("/research/sessions/{session_id}")).await?;
        let sess = detail.get("session").unwrap_or(&detail);
        let id = sess.get("id").and_then(Value::as_i64).unwrap_or(session_id);
        let status = sess.get("status").and_then(Value::as_str).unwrap_or("");
        let query = sess.get("query_text").and_then(Value::as_str).unwrap_or("");
        writeln!(std::io::stdout(), "{id}\t{status}\t{query}")?;
        if matches!(status, "completed" | "failed" | "orphaned") {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}

/// Attempt to run a research query via the running daemon when `vox.db` is `flock`-locked.
pub async fn try_run_research_via_daemon(params: &Value, json_out: bool) -> Result<Option<()>> {
    let Ok(client) = crate::daemon_tool::daemon_client().await else {
        return Ok(None);
    };
    let Ok(submitted) = client.call(dei_method::RESEARCH_RUN, params.clone()).await else {
        return Ok(None);
    };
    let Some(session_id) = submitted.get("session_id").and_then(Value::as_i64) else {
        return Ok(None);
    };
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(180);
    loop {
        let detail =
            api_v2_get_with_client(&client, &format!("/research/sessions/{session_id}")).await?;
        let status = detail
            .get("session")
            .and_then(|s| s.get("status"))
            .and_then(Value::as_str)
            .unwrap_or("");
        if status == "completed" {
            let out = if !json_out
                && let Some(md) = detail.get("report_markdown").and_then(Value::as_str)
            {
                md.to_string()
            } else {
                serde_json::to_string_pretty(&detail)?
            };
            writeln!(std::io::stdout(), "{out}")?;
            return Ok(Some(()));
        }
        if matches!(status, "failed" | "orphaned") {
            anyhow::bail!("daemon research session {session_id} ended with status={status}");
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!(
                "timed out waiting for daemon research session {session_id} (last status={status})"
            );
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_canonical_db_lock_error_matches_flock_and_turso_errors_only_without_override() {
        for msg in [
            "Failed locking file `/home/user/.vox/vox.db`: Resource temporarily unavailable",
            "DatabaseBusy: database is locked",
            "WouldBlock on flock",
            "IO error: file is locked by another process",
        ] {
            assert!(is_canonical_db_lock_error_with_override(msg, None));
            assert!(is_canonical_db_lock_error_with_override(msg, Some("   ")));
            assert!(!is_canonical_db_lock_error_with_override(
                msg,
                Some("/tmp/custom.db")
            ));
        }
        assert!(!is_canonical_db_lock_error_with_override(
            "connection refused on 127.0.0.1",
            None
        ));
    }
}
