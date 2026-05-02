//! Simplified DeI JSON-line RPC integration boundary for vox-mens.
//! Used primarily for AI-assisted corpus curation via vox-orchestrator-d.

use serde_json::Value;

pub const BINARY: &str = "vox-orchestrator-d";

// Method constants are owned by `vox-protocol` (single source of truth shared
// with vox-cli, vox-orchestrator, etc.).  Re-exported here so existing call
// sites (`crate::dei_daemon::method::*`) continue to resolve without a second
// definition that can drift.
pub use vox_protocol::dei_method as method;

#[derive(serde::Serialize)]
struct DispatchRequest {
    id: String,
    method: String,
    params: Value,
}

#[derive(serde::Deserialize)]
struct DispatchResponse {
    payload: DispatchPayload,
}

#[derive(serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum DispatchPayload {
    Log {
        level: String,
        msg: String,
    },
    Done {
        result: Value,
    },
    Error {
        code: i32,
        message: String,
    },
    #[serde(other)]
    Unknown,
}

pub async fn call(method: &str, params: Value, _auto_open: bool) -> anyhow::Result<Value> {
    use std::process::Stdio;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::process::Command;

    let mut child = Command::new(BINARY)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to spawn daemon '{}': {}", BINARY, e))?;

    let mut stdin = child.stdin.take().expect("stdin was piped");
    let stdout = child.stdout.take().expect("stdout was piped");

    let req = DispatchRequest {
        id: uuid::Uuid::new_v4().to_string(),
        method: method.into(),
        params,
    };

    let json = serde_json::to_string(&req)? + "\n";
    stdin.write_all(json.as_bytes()).await?;
    stdin.flush().await?;
    drop(stdin);

    let mut reader = BufReader::new(stdout).lines();
    let mut final_result = Value::Null;

    while let Ok(Some(line)) = reader.next_line().await {
        if let Ok(resp) = serde_json::from_str::<DispatchResponse>(&line) {
            match resp.payload {
                DispatchPayload::Log { level, msg } => {
                    eprintln!("[{}] {}", level.to_uppercase(), msg);
                }
                DispatchPayload::Done { result } => {
                    final_result = result;
                    break;
                }
                DispatchPayload::Error { code, message } => {
                    anyhow::bail!("Daemon error (code {}): {}", code, message);
                }
                DispatchPayload::Unknown => {}
            }
        }
    }

    Ok(final_result)
}
