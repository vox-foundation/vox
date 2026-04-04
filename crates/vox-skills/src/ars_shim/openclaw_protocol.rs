//! Typed protocol envelopes for OpenClaw Gateway WebSocket transport.
//!
//! The protocol is request/response/event JSON frames over WebSocket text frames.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Gateway request frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayRequest {
    #[serde(rename = "type")]
    pub frame_type: String,
    pub id: String,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

impl GatewayRequest {
    /// Build a `req` frame.
    pub fn req(id: impl Into<String>, method: impl Into<String>, params: Value) -> Self {
        Self {
            frame_type: "req".to_string(),
            id: id.into(),
            method: method.into(),
            params,
        }
    }
}

/// Gateway response frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayResponse {
    #[serde(rename = "type")]
    pub frame_type: String,
    pub id: String,
    pub ok: bool,
    #[serde(default)]
    pub payload: Value,
    #[serde(default)]
    pub error: Option<Value>,
}

/// Gateway event frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayEvent {
    #[serde(rename = "type")]
    pub frame_type: String,
    pub event: String,
    #[serde(default)]
    pub payload: Value,
    #[serde(default)]
    pub seq: Option<u64>,
    #[serde(default, rename = "stateVersion")]
    pub state_version: Option<u64>,
}

/// Dynamic inbound frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum InboundFrame {
    #[serde(rename = "res")]
    Response {
        id: String,
        ok: bool,
        #[serde(default)]
        payload: Value,
        #[serde(default)]
        error: Option<Value>,
    },
    #[serde(rename = "event")]
    Event {
        event: String,
        #[serde(default)]
        payload: Value,
        #[serde(default)]
        seq: Option<u64>,
        #[serde(default, rename = "stateVersion")]
        state_version: Option<u64>,
    },
    #[serde(other)]
    Other,
}

/// Connect params for WS handshake.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConnectParams {
    #[serde(rename = "minProtocol")]
    pub min_protocol: u32,
    #[serde(rename = "maxProtocol")]
    pub max_protocol: u32,
    pub client: GatewayClientIdentity,
    pub role: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub caps: Vec<String>,
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub permissions: Value,
    #[serde(default)]
    pub auth: Value,
    #[serde(default, rename = "userAgent")]
    pub user_agent: Option<String>,
    #[serde(default)]
    pub locale: Option<String>,
}

/// Client identity metadata for connect params.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayClientIdentity {
    pub id: String,
    pub version: String,
    pub platform: String,
    pub mode: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_connect_challenge_event() {
        let raw = r#"{
            "type":"event",
            "event":"connect.challenge",
            "payload":{"nonce":"abc","ts":1737264000000}
        }"#;
        let frame: InboundFrame = serde_json::from_str(raw).expect("parse event frame");
        match frame {
            InboundFrame::Event { event, payload, .. } => {
                assert_eq!(event, "connect.challenge");
                assert_eq!(
                    payload
                        .get("nonce")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or(""),
                    "abc"
                );
            }
            _ => panic!("expected event frame"),
        }
    }

    #[test]
    fn parses_connect_request_fixture() {
        let raw =
            include_str!("../../../../contracts/openclaw/protocol/connect.request.operator.json");
        let frame: GatewayRequest = serde_json::from_str(raw).expect("parse connect request");
        assert_eq!(frame.frame_type, "req");
        assert_eq!(frame.method, "connect");
        assert!(frame.params.get("client").is_some());
    }
}
