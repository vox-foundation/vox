//! JSON line protocol shared by the CLI dispatch client (`dispatch` module) and [`crate::compilerd`] (daemon).
//!
//! Each message is a single JSON object per line on stdout/stdin.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Outgoing request from the thin CLI to `vox-compilerd` / `vox-dei-d`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchRequest {
    pub id: String,
    pub method: String,
    pub params: Value,
}

/// Incoming response envelope (one per line).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchResponse {
    pub id: String,
    pub payload: DispatchPayload,
}

/// Payload variants — keep in sync across all daemon implementations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DispatchPayload {
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
