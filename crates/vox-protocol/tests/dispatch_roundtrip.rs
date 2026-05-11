//! Serde round-trip for daemon wire types (`vox-protocol`).

use serde_json::json;
use vox_protocol::{DispatchPayload, DispatchRequest, DispatchResponse, orch_daemon_method};

#[test]
fn dispatch_request_roundtrips_json() {
    let req = DispatchRequest {
        id: "req-1".to_string(),
        method: orch_daemon_method::PING.to_string(),
        params: json!({"repository_id": "repo"}),
    };
    let s = serde_json::to_string(&req).expect("serialize DispatchRequest");
    let back: DispatchRequest = serde_json::from_str(&s).expect("deserialize DispatchRequest");
    assert_eq!(back.id, req.id);
    assert_eq!(back.method, orch_daemon_method::PING);
    assert_eq!(back.params, req.params);
}

#[test]
fn dispatch_response_payload_variants_roundtrip() {
    let cases = vec![
        DispatchPayload::Result {
            value: json!({"ok": true}),
        },
        DispatchPayload::Error {
            message: "nope".into(),
            code: 42,
        },
        DispatchPayload::Chunk {
            text: "partial".into(),
        },
        DispatchPayload::Progress {
            percent: 50.0,
            status: "running".into(),
        },
        DispatchPayload::Done { exit: 0 },
    ];

    for payload in cases {
        let resp = DispatchResponse {
            id: "r1".into(),
            payload: payload.clone(),
        };
        let s = serde_json::to_string(&resp).unwrap();
        let back: DispatchResponse = serde_json::from_str(&s).unwrap();
        assert_eq!(back.id, "r1");
        assert_eq!(
            serde_json::to_string(&back.payload).unwrap(),
            serde_json::to_string(&payload).unwrap()
        );
    }
}
