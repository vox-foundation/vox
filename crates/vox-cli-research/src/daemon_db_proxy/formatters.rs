//! Formatting and payload-extraction helpers for `daemon_db_proxy`.

use anyhow::Result;
use serde_json::Value;

pub(crate) fn write_output_file(path: &std::path::Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok(())
}

pub(crate) fn unwrap_api_v2_envelope(envelope: Value) -> Result<Value> {
    if let Some(err) = envelope.get("error") {
        let code = err.get("code").and_then(Value::as_str).unwrap_or("error");
        let msg = err
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("daemon api_v2_get failed");
        anyhow::bail!("{code}: {msg}");
    }
    Ok(envelope.get("data").cloned().unwrap_or(Value::Null))
}

pub(crate) fn format_research_history_rows(data: &Value) -> String {
    let Some(rows) = data.as_array() else {
        return "No research sessions found.".to_string();
    };
    if rows.is_empty() {
        return "No research sessions found.".to_string();
    }
    rows.iter()
        .map(|row| {
            format!(
                "{}\t{}\t{}\t{}",
                row.get("id").and_then(Value::as_i64).unwrap_or(0),
                row.get("status").and_then(Value::as_str).unwrap_or(""),
                row.get("started_at_ms")
                    .and_then(Value::as_i64)
                    .unwrap_or(0),
                row.get("query_text").and_then(Value::as_str).unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn format_research_show_detail(session_id: i64, detail: &Value) -> String {
    let sess = detail.get("session").unwrap_or(detail);
    let id = sess.get("id").and_then(Value::as_i64).unwrap_or(session_id);
    let status = sess.get("status").and_then(Value::as_str).unwrap_or("");
    let started = sess
        .get("started_at_ms")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let query = sess.get("query_text").and_then(Value::as_str).unwrap_or("");
    let mut lines = vec![
        format!("session_id: {id}"),
        format!("status: {status}"),
        format!("started_at_ms: {started}"),
    ];
    if let Some(finished) = sess.get("finished_at_ms").and_then(Value::as_i64) {
        lines.push(format!("finished_at_ms: {finished}"));
    }
    lines.push(format!("query: {query}"));
    let body = detail
        .get("report_markdown")
        .and_then(Value::as_str)
        .map(|md| format!("\n{md}"))
        .unwrap_or_else(|| "\nNo durable research artifact found for this session.".to_string());
    lines.push(body);
    lines.join("\n")
}

pub(crate) fn extract_research_result_payload(
    session_id: i64,
    detail: &Value,
    format: &str,
) -> Result<String> {
    match format {
        "markdown" | "md" => detail
            .get("report_markdown")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| {
                anyhow::anyhow!("no durable research artifact found for session {session_id}")
            }),
        "json" => {
            if let Some(raw) = detail
                .get("artifact_json")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
            {
                Ok(raw.to_string())
            } else {
                Ok(serde_json::to_string_pretty(detail)?)
            }
        }
        other => anyhow::bail!("unsupported result format {other:?}: use markdown|json"),
    }
}

pub(crate) fn format_retrieval_status_proxy(
    engine_status: &Value,
    repo_index: Option<&Value>,
) -> String {
    let mut lines = vec!["Retrieval diagnostics (via daemon IPC)".to_string()];
    if let Some(lane) = engine_status.get("active_lane").and_then(Value::as_str) {
        lines.push(format!("  ActiveLane      : {lane}"));
    }
    if let Some(fast_ms) = engine_status.get("fast_timeout_ms").and_then(Value::as_u64) {
        lines.push(format!("  FastTimeoutMs   : {fast_ms}"));
    }
    if let Some(deep_ms) = engine_status.get("deep_timeout_ms").and_then(Value::as_u64) {
        lines.push(format!("  DeepTimeoutMs   : {deep_ms}"));
    }
    if let Some(providers) = engine_status.get("providers").and_then(Value::as_array) {
        let enabled = providers
            .iter()
            .filter(|p| p.get("is_enabled").and_then(Value::as_bool) == Some(true))
            .count();
        lines.push(format!(
            "  Providers       : {enabled}/{} enabled",
            providers.len()
        ));
    }
    if let Some(idx) = repo_index {
        lines.push(format!("  RepoIndex       : {idx}"));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unwrap_api_v2_envelope_handles_data_and_error() {
        let ok = unwrap_api_v2_envelope(json!({ "v": 1, "data": { "x": 7 } })).expect("ok");
        assert_eq!(ok, json!({ "x": 7 }));
        let err = unwrap_api_v2_envelope(json!({
            "v": 1,
            "error": { "code": "db_error", "message": "boom" }
        }))
        .expect_err("err");
        assert!(err.to_string().contains("db_error: boom"), "{err}");
    }

    #[test]
    fn format_research_history_and_show_detail_round_trip() {
        assert_eq!(
            format_research_history_rows(&json!([])),
            "No research sessions found."
        );
        let detail = json!({
            "session": {
                "id": 42,
                "status": "completed",
                "started_at_ms": 100,
                "finished_at_ms": 200,
                "query_text": "arrhenius conductivity"
            },
            "report_markdown": "# Report\nDone.",
            "artifact_json": "{\"ok\":true}"
        });
        let shown = format_research_show_detail(42, &detail);
        assert!(shown.contains("session_id: 42"));
        assert_eq!(
            extract_research_result_payload(42, &detail, "markdown").expect("md"),
            "# Report\nDone."
        );
        assert_eq!(
            extract_research_result_payload(42, &detail, "json").expect("json"),
            "{\"ok\":true}"
        );
        let fallback_json =
            extract_research_result_payload(42, &json!({ "session": { "id": 42 } }), "json")
                .expect("fallback json");
        assert!(fallback_json.contains("\"id\": 42"));
        assert!(extract_research_result_payload(42, &detail, "yaml").is_err());
    }

    #[test]
    fn format_retrieval_status_proxy_renders_lane_timeouts_and_providers() {
        let status = json!({
            "active_lane": "deep",
            "fast_timeout_ms": 2500,
            "deep_timeout_ms": 12000,
            "providers": [
                { "name": "tavily", "is_enabled": true },
                { "name": "arxiv", "is_enabled": false }
            ]
        });
        let report = format_retrieval_status_proxy(&status, Some(&json!({ "docs": 17 })));
        assert!(report.contains("ActiveLane      : deep"));
        assert!(report.contains("FastTimeoutMs   : 2500"));
        assert!(report.contains("DeepTimeoutMs   : 12000"));
        assert!(report.contains("Providers       : 1/2 enabled"));
        assert!(report.contains("RepoIndex       :"));
    }

    #[test]
    fn write_output_file_handles_bare_filename_and_nested_directory() {
        assert_eq!(
            std::path::Path::new("report.md").parent(),
            Some(std::path::Path::new(""))
        );
        let dir = tempfile::tempdir().expect("tempdir");
        let nested = dir.path().join("nested").join("dir").join("report.md");
        write_output_file(&nested, "# Report").expect("nested write succeeds");
        assert_eq!(std::fs::read_to_string(&nested).expect("read"), "# Report");

        let bare = dir.path().join("report.md");
        write_output_file(&bare, "# Direct").expect("direct write succeeds");
        assert_eq!(std::fs::read_to_string(&bare).expect("read"), "# Direct");
    }
}
