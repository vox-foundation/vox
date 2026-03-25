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

    #[test]
    fn dispatch_request_validates_against_dei_rpc_schema() {
        use std::path::PathBuf;

        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let schema_path = manifest.join("../../contracts/dei/rpc-methods.schema.json");
        let schema_src = std::fs::read_to_string(&schema_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", schema_path.display()));
        let schema_val: serde_json::Value =
            serde_json::from_str(&schema_src).expect("parse DeI RPC schema");
        let validator =
            jsonschema::validator_for(&schema_val).expect("compile DeI RPC schema");

        let req = DispatchRequest {
            id: "req-1".into(),
            method: crate::dei_daemon::method::AI_GENERATE.into(),
            params: serde_json::json!({ "prompt": "hello" }),
        };
        let instance = serde_json::to_value(&req).expect("serialize DispatchRequest");
        validator
            .validate(&instance)
            .expect("DispatchRequest must match contracts/dei/rpc-methods.schema.json");
    }

    #[test]
    fn dei_schema_method_enum_matches_daemon_constants() {
        use std::collections::HashSet;
        use std::path::PathBuf;

        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let schema_path = manifest.join("../../contracts/dei/rpc-methods.schema.json");
        let schema_val: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&schema_path).expect("read DeI RPC schema"),
        )
        .expect("parse DeI RPC schema");
        let methods = schema_val["properties"]["method"]["enum"]
            .as_array()
            .expect("schema properties.method.enum");
        let as_set: HashSet<&str> = methods
            .iter()
            .map(|v| v.as_str().expect("enum string"))
            .collect();

        use crate::dei_daemon::method;
        for m in [
            method::AI_CHECK,
            method::AI_FIX,
            method::AI_REVIEW,
            method::AI_GENERATE,
            method::CONFIG_GET,
            method::AI_PLAN_NEW,
            method::AI_PLAN_REPLAN,
            method::AI_PLAN_STATUS,
            method::AI_PLAN_EXECUTE,
        ] {
            assert!(as_set.contains(m), "schema missing method {m}");
        }
        assert_eq!(as_set.len(), 9, "schema vs daemon method count drift");
    }
}
