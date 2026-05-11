//! Integration tests for daemon dispatch JSON envelopes.

use serde_json::json;
use vox_protocol::{DispatchPayload, DispatchRequest, DispatchResponse, orch_daemon_method};

#[test]
fn dispatch_request_roundtrips_through_json() {
    let req = DispatchRequest {
        id: "1".into(),
        method: orch_daemon_method::PING.to_string(),
        params: json!({"repository_id": "repo"}),
    };
    let json = serde_json::to_string(&req).expect("serialize request");
    let back: DispatchRequest = serde_json::from_str(&json).expect("deserialize request");
    assert_eq!(back.id, "1");
    assert_eq!(back.method, orch_daemon_method::PING);
    assert_eq!(back.params["repository_id"], "repo");
}

#[test]
fn dispatch_response_result_variant_roundtrips() {
    let resp = DispatchResponse {
        id: "9".into(),
        payload: DispatchPayload::Result {
            value: json!({"ok": true}),
        },
    };
    let json = serde_json::to_string(&resp).expect("serialize response");
    let back: DispatchResponse = serde_json::from_str(&json).expect("deserialize response");
    match back.payload {
        DispatchPayload::Result { value } => assert_eq!(value["ok"], true),
        other => panic!("unexpected payload: {other:?}"),
    }
}

#[test]
fn dispatch_payload_snake_case_tagged_enum_decodes() {
    let raw = r#"{"type":"error","message":"bad","code":7}"#;
    let p: DispatchPayload = serde_json::from_str(raw).expect("decode error payload");
    match p {
        DispatchPayload::Error { message, code } => {
            assert_eq!(message, "bad");
            assert_eq!(code, 7);
        }
        other => panic!("unexpected: {other:?}"),
    }
}
