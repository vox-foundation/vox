use crate::otlp_json::merge_otlp_log_batch;
use crate::redact::RedactedRecord;
use std::fs;
use std::path::{Path, PathBuf};

/// Maximum number of pending spool files combined into a single OTLP/HTTP POST request.
pub const MAX_OTLP_BATCH_RECORDS: usize = 100;
/// Maximum serialized byte size of a single OTLP/HTTP POST batch (`256 KiB`).
pub const MAX_OTLP_BATCH_BYTES: usize = 256 * 1024;

/// Async OTLP/HTTP uploader (feature `remote` only).
///
/// Reads pending clean spool records and POSTs them to the central OTLP endpoint
/// when `is_remote_allowed()` returns true.
pub async fn upload_pending_otlp(spool_dir: &Path, endpoint: &str) -> std::io::Result<usize> {
    upload_pending_otlp_with_auth(spool_dir, endpoint, None).await
}

/// Async OTLP/HTTP uploader with optional Bearer authentication token (`TELEM-02`, `TELEM-03`).
///
/// Enforces `vox_telemetry::config::is_remote_allowed()`, reads `.json` files from `spool_dir`
/// (or `spool_dir/pending` if present), batches up to [`MAX_OTLP_BATCH_RECORDS`] records into a
/// single `resourceLogs` envelope via [`merge_otlp_log_batch`], POSTs to `endpoint`, and deletes
/// acknowledged files on HTTP 2xx.
pub async fn upload_pending_otlp_with_auth(
    spool_dir: &Path,
    endpoint: &str,
    bearer: Option<&str>,
) -> std::io::Result<usize> {
    if !vox_telemetry::config::is_remote_allowed() {
        return Ok(0);
    }
    let endpoint = endpoint.trim();
    if endpoint.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "telemetry OTLP endpoint is empty",
        ));
    }

    let pending_root = if spool_dir.join("pending").is_dir() {
        spool_dir.join("pending")
    } else {
        spool_dir.to_path_buf()
    };
    if !pending_root.is_dir() {
        return Ok(0);
    }

    let mut paths: Vec<PathBuf> = fs::read_dir(&pending_root)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "json"))
        .collect();
    paths.sort();
    if paths.is_empty() {
        return Ok(0);
    }

    let client = reqwest::Client::builder()
        // drift-allow(timeout-literal): bounded OTLP telemetry batch upload timeout
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(std::io::Error::other)?;

    let mut uploaded = 0usize;
    for chunk in paths.chunks(MAX_OTLP_BATCH_RECORDS) {
        let mut batch_paths = Vec::with_capacity(chunk.len());
        let mut batch_values = Vec::with_capacity(chunk.len());
        for p in chunk {
            if let Ok(raw) = fs::read_to_string(p)
                && let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw)
            {
                batch_paths.push(p.clone());
                batch_values.push(val);
            }
        }
        if batch_values.is_empty() {
            continue;
        }
        let merged = merge_otlp_log_batch(&batch_values);
        let body = serde_json::to_string(&merged).map_err(std::io::Error::other)?;
        if body.len() > MAX_OTLP_BATCH_BYTES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "OTLP batch payload ({} bytes) exceeds 256 KiB limit",
                    body.len()
                ),
            ));
        }

        let mut req = client.post(endpoint).body(body).header(
            reqwest::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        );
        if let Some(tok) = bearer.filter(|s| !s.trim().is_empty()) {
            req = req.header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", tok.trim()),
            );
        }

        let resp = req.send().await.map_err(std::io::Error::other)?;
        if resp.status().is_success() {
            for p in batch_paths {
                let _ = fs::remove_file(p);
                uploaded += 1;
            }
        } else {
            return Err(std::io::Error::other(format!(
                "OTLP upload rejected with HTTP {}",
                resp.status()
            )));
        }
    }

    Ok(uploaded)
}

/// A single pending record read from the spool, ready for upload.
#[allow(dead_code)]
pub struct PendingRecord {
    pub id: String,
    pub record: RedactedRecord,
}
