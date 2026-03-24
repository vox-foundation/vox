//! Ephemeral task execution used by `vox skill eval`.

use serde_json::Value;

use crate::manifest::ResourceLimits;

/// Errors from the lightweight task executor.
#[derive(Debug, thiserror::Error)]
pub enum ExecutorError {
    /// Serialization or shape error.
    #[error("executor: {0}")]
    Message(String),
}

/// Run a user-supplied task body with JSON input under advisory limits.
///
/// Current behavior: returns a structured envelope with input echo and body statistics.
/// A full workflow VM may replace this without changing the CLI signature.
pub async fn execute_vox_task(
    body: &str,
    input: &Value,
    limits: &ResourceLimits,
    _ctx: Option<Value>,
) -> Result<Value, ExecutorError> {
    let _ = limits;
    Ok(serde_json::json!({
        "status": "ok",
        "body_char_len": body.chars().count(),
        "input": input,
    }))
}
