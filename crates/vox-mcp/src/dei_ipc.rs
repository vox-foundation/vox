//! Minimal JSON-line RPC client for `vox-dei-d` (planning and other DeI surfaces).
//!
//! Wire shape `{ id, method, params }` matches [`vox_cli::dispatch_protocol::DispatchRequest`] and
//! `contracts/dei/rpc-methods.schema.json` (JSON Schema `$id`: `https://vox-lang.org/schemas/dei/rpc-methods.schema.json`).
//! No `vox-cli` dependency here; keep structs in sync.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

const DAEMON_BINARY: &str = "vox-dei-d";
const SPAWN_ERR: &str = "Failed to spawn daemon";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DispatchRequest {
    id: String,
    method: String,
    params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DispatchResponse {
    #[allow(dead_code)]
    id: String,
    payload: DispatchPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum DispatchPayload {
    Result {
        value: Value,
    },
    Error {
        message: String,
        code: i32,
    },
    Chunk {
        text: String,
    },
    Progress {
        percent: f32,
        status: String,
    },
    Log {
        level: String,
        msg: String,
    },
    Diag {
        severity: String,
        message: String,
        file: String,
        line: u32,
        col: u32,
    },
    Artifact {
        path: String,
    },
    Done {
        exit: i32,
    },
}

fn resolve_daemon_path(daemon: &str) -> std::path::PathBuf {
    if let Ok(p) = std::env::current_exe() {
        if let Some(dir) = p.parent() {
            let sibling = dir.join(if cfg!(windows) {
                format!("{daemon}.exe")
            } else {
                daemon.to_string()
            });
            if sibling.exists() {
                return sibling;
            }
        }
    }
    std::path::PathBuf::from(daemon)
}

/// Call `vox-dei-d` with `method` / `params`; returns the final `Result` JSON value or an error.
pub async fn call_dei_daemon(method: &str, params: Value) -> anyhow::Result<Value> {
    let daemon_path = resolve_daemon_path(DAEMON_BINARY);
    let mut child = Command::new(&daemon_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| anyhow::anyhow!("{} '{}': {}", SPAWN_ERR, daemon_path.display(), e))?;

    let mut stdin = child.stdin.take().expect("stdin piped");
    let stdout = child.stdout.take().expect("stdout piped");

    let req = DispatchRequest {
        id: format!(
            "mcp_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ),
        method: method.to_string(),
        params,
    };
    let json = serde_json::to_string(&req)? + "\n";
    stdin.write_all(json.as_bytes()).await?;
    stdin.flush().await?;
    drop(stdin);

    let mut reader = BufReader::new(stdout).lines();
    let mut final_result = Value::Null;
    let mut had_error: Option<String> = None;
    let mut exit_code = 0i32;

    while let Ok(Some(line)) = reader.next_line().await {
        if let Ok(resp) = serde_json::from_str::<DispatchResponse>(&line) {
            match resp.payload {
                DispatchPayload::Result { value } => final_result = value,
                DispatchPayload::Error { message, code } => {
                    had_error = Some(format!("Daemon error (code {code}): {message}"));
                }
                DispatchPayload::Done { exit } => {
                    exit_code = exit;
                    break;
                }
                _ => {}
            }
        }
    }

    if let Some(err) = had_error {
        anyhow::bail!(err);
    }
    if exit_code != 0 {
        anyhow::bail!("vox-dei-d reported exit code {exit_code}");
    }

    let status = child.wait().await?;
    if !status.success() {
        anyhow::bail!("vox-dei-d process exited with {}", status);
    }

    Ok(final_result)
}
