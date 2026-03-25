use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod dei_method {
    pub const AI_CHECK: &str = "ai.check";
    pub const AI_FIX: &str = "ai.fix";
    pub const AI_REVIEW: &str = "ai.review";
    pub const AI_GENERATE: &str = "ai.generate";
    pub const CONFIG_GET: &str = "config.get";
    pub const AI_PLAN_NEW: &str = "ai.plan.new";
    pub const AI_PLAN_REPLAN: &str = "ai.plan.replan";
    pub const AI_PLAN_STATUS: &str = "ai.plan.status";
    pub const AI_PLAN_EXECUTE: &str = "ai.plan.execute";
}

/// Outgoing request from thin clients to Dei-style JSON-line daemons.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchRequest {
    pub id: String,
    pub method: String,
    pub params: Value,
}

/// Incoming response envelope from Dei-style JSON-line daemons.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchResponse {
    pub id: String,
    pub payload: DispatchPayload,
}

/// Payload variants for streaming and final Dei responses.
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
