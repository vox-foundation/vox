use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

use crate::params::ToolResult;
use crate::server_state::ServerState;

const REM_TOESTUB_IO: &str = "Ensure `.vox/` is writable and `toestub_findings.jsonl` is not locked by another process (AV, sync).";

/// Arguments for `vox_code_audit_findings_upsert`.
#[derive(Debug, Deserialize)]
pub struct ToestubFindingsParams {
    /// List of findings to upsert.
    pub findings: Vec<vox_code_audit::rules::Finding>,
    /// Optional session ID to associate with these findings.
    pub session_id: Option<String>,
}

/// Upsert TOESTUB findings into the repository-local queue.
pub async fn toestub_findings_upsert(
    _state: &ServerState,
    params: ToestubFindingsParams,
) -> String {
    let repo_root = if let Some(p) =
        vox_secrets::resolve_secret(vox_secrets::SecretId::VoxRepositoryRoot).expose()
    {
        PathBuf::from(p)
    } else {
        std::env::current_dir().unwrap_or_default()
    };

    let dot_vox = repo_root.join(".vox");
    if !dot_vox.exists() {
        let _ = fs::create_dir_all(&dot_vox);
    }

    let findings_path = dot_vox.join("toestub_findings.jsonl");

    // Append findings as JSONL for high-concurrency safety (lock-free append)
    let mut data = String::new();
    for finding in &params.findings {
        if let Ok(json) = serde_json::to_string(finding) {
            data.push_str(&json);
            data.push('\n');
        }
    }

    match fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&findings_path)
    {
        Ok(mut file) => {
            use std::io::Write;
            if let Err(e) = file.write_all(data.as_bytes()) {
                return ToolResult::<String>::err_with_remediation(
                    format!("Write failed: {e}"),
                    REM_TOESTUB_IO,
                )
                .to_json();
            }
            ToolResult::ok(format!(
                "Upserted {} findings to {}",
                params.findings.len(),
                findings_path.display()
            ))
            .to_json()
        }
        Err(e) => ToolResult::<String>::err_with_remediation(
            format!("Failed to open findings file: {e}"),
            REM_TOESTUB_IO,
        )
        .to_json(),
    }
}
