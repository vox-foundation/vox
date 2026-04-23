use super::*;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::time::Duration;
use tokio::time::timeout;

#[derive(Debug, Deserialize)]
pub(super) struct EvalRequest {
    pub code: String,
}

#[derive(Debug, Serialize)]
pub(super) struct EvalResponse {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub error: Option<String>,
}

pub(super) async fn http_eval(
    State(state): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<EvalRequest>,
) -> Response {
    let identity = request_identity(&state, &connect.0, &headers);

    if state.public_eval_enabled {
        if let Err(msg) = enforce_https_requirement(&state, &headers) {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": msg })),
            )
                .into_response();
        }
        if state.public_eval_rate_limiter.check_key(&identity).is_err() {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                Json(serde_json::json!({ "error": "rate limit exceeded for public eval (10/min)" })),
            )
                .into_response();
        }
    } else {
        if let Err(resp) = enforce_request_guards(&state, &connect.0, &headers).await {
            return resp;
        }
        
        let _role = match resolve_access_role(&state, &headers, Some(&connect.0)) {
            Ok(r) => r,
            Err(msg) => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({ "error": msg })),
                )
                    .into_response();
            }
        };
    }

    let dir = match tempfile::tempdir() {
        Ok(d) => d,
        Err(e) => return Json(EvalResponse {
            success: false,
            stdout: String::new(),
            stderr: String::new(),
            error: Some(format!("Failed to create tempdir: {}", e)),
        }).into_response(),
    };

    let file_path = dir.path().join("eval.vox");
    if let Err(e) = tokio::fs::write(&file_path, &req.code).await {
        return Json(EvalResponse {
            success: false,
            stdout: String::new(),
            stderr: String::new(),
            error: Some(format!("Failed to write file: {}", e)),
        }).into_response();
    }

    let mut cmd = tokio::process::Command::new(std::env::current_exe().unwrap_or_else(|_| "vox".into()));
    // Use standard interpreter for pure computation (fastest)
    cmd.arg("run").arg("--interp").arg(&file_path);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    
    // Hard execution boundary: 5 seconds
    let exec = timeout(Duration::from_secs(5), cmd.output()).await;
    
    let _ = dir.close();

    match exec {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Json(EvalResponse {
                success: output.status.success(),
                stdout,
                stderr,
                error: None,
            }).into_response()
        }
        Ok(Err(e)) => {
            Json(EvalResponse {
                success: false,
                stdout: String::new(),
                stderr: String::new(),
                error: Some(format!("Execution failed: {}", e)),
            }).into_response()
        }
        Err(_) => {
            Json(EvalResponse {
                success: false,
                stdout: String::new(),
                stderr: String::new(),
                error: Some("Execution timed out after 5 seconds.".to_string()),
            }).into_response()
        }
    }
}
