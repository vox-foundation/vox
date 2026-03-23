//! JSON line protocol shared by the CLI dispatch client (`dispatch` module) and [`crate::compilerd`] (daemon).
//!
//! Each message is a single JSON object per line on stdout/stdin.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Outgoing request from the thin CLI to `vox-compilerd` / `vox-dei-d`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchRequest {
    /// Unique identifier for the request.
    pub id: String,
    /// RPC Method name.
    pub method: String,
    /// JSON value arguments for the parameter payload.
    pub params: Value,
}

/// Incoming response envelope (one per line).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchResponse {
    /// Unique identifier matching the original request.
    pub id: String,
    /// JSON payload of the response variant.
    pub payload: DispatchPayload,
}

/// Payload variants — keep in sync across all daemon implementations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DispatchPayload {
    /// Standard successful RPC return value.
    Result {
        /// Arbitrary JSON value result.
        value: Value,
    },
    /// An error occurred during dispatch.
    Error {
        /// Human readable error message.
        message: String,
        /// Numeric error code.
        code: i32,
    },
    /// Streaming text chunk for generation pipelines.
    Chunk {
        /// The streaming text fragment.
        text: String,
    },
    /// Execution progress update.
    Progress {
        /// Completion percentage (0.0 to 1.0) or unbounded (100.0).
        percent: f32,
        /// Description of the current step.
        status: String,
    },
    /// Console log stream message.
    Log {
        /// Level string (`info`, `warn`, `error`, `debug`).
        level: String,
        /// The log message string.
        msg: String,
    },
    /// Complier/Parser diagnostic event.
    Diag {
        /// Diagnostic severity (`error`, `warning`, `info`).
        severity: String,
        /// Human readable diagnostic output.
        message: String,
        /// Source file path where diagnostic occurred.
        file: String,
        /// Source line number (1-indexed).
        line: u32,
        /// Source column number (1-indexed).
        col: u32,
    },
    /// File artifact emitted by the process.
    Artifact {
        /// Absolute path to the emitted artifact.
        path: String,
    },
    /// Final stream completion.
    Done {
        /// Exit code (non-zero on semantic failure).
        exit: i32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_response_roundtrip_done() {
        let msg = DispatchResponse {
            id: "abc".into(),
            payload: DispatchPayload::Done { exit: 0 },
        };
        let json = serde_json::to_string(&msg).expect("serialize DispatchResponse");
        let back: DispatchResponse =
            serde_json::from_str(&json).expect("deserialize DispatchResponse");
        assert_eq!(back.id, "abc");
        match back.payload {
            DispatchPayload::Done { exit } => assert_eq!(exit, 0),
            other => panic!("expected Done payload, got {other:?}"),
        }
    }
}
