//! Coverage for `dei_method` ids and `DispatchPayload` variants not exercised elsewhere.

use serde_json::json;
use vox_protocol::{
    DispatchPayload, DispatchRequest, DispatchResponse, dei_method, orch_daemon_method,
};

#[test]
fn dei_method_constants_are_stable_non_empty() {
    assert_eq!(dei_method::AI_CHECK, "ai.check");
    assert_eq!(dei_method::AI_FIX, "ai.fix");
    assert_eq!(dei_method::AI_REVIEW, "ai.review");
    assert_eq!(dei_method::AI_GENERATE, "ai.generate");
    assert_eq!(dei_method::CONFIG_GET, "config.get");
    assert_eq!(dei_method::AI_PLAN_NEW, "ai.plan.new");
    assert_eq!(dei_method::AI_PLAN_REPLAN, "ai.plan.replan");
    assert_eq!(dei_method::AI_PLAN_STATUS, "ai.plan.status");
    assert_eq!(dei_method::AI_PLAN_EXECUTE, "ai.plan.execute");
}

#[test]
fn dispatch_request_accepts_dei_style_method_id() {
    let req = DispatchRequest {
        id: "r-dei".into(),
        method: dei_method::AI_CHECK.to_string(),
        params: json!({}),
    };
    let s = serde_json::to_string(&req).expect("serialize");
    let back: DispatchRequest = serde_json::from_str(&s).expect("deserialize");
    assert_eq!(back.method, dei_method::AI_CHECK);
    assert_eq!(back.params, json!({}));
}

#[test]
fn orch_daemon_method_subset_matches_expected_prefix() {
    // Spot-check a few frequently used ids so refactors keep the `orch.*` namespace stable.
    assert!(orch_daemon_method::PING.starts_with("orch."));
    assert!(orch_daemon_method::SUBMIT_TASK.starts_with("orch."));
}

#[test]
fn dispatch_payload_log_diag_artifact_roundtrip() {
    let cases = [
        DispatchPayload::Log {
            level: "info".into(),
            msg: "hello".into(),
        },
        DispatchPayload::Diag {
            severity: "warn".into(),
            message: "unused".into(),
            file: "src/lib.rs".into(),
            line: 10,
            col: 3,
        },
        DispatchPayload::Artifact {
            path: "/tmp/out.txt".into(),
        },
    ];

    for payload in cases {
        let resp = DispatchResponse {
            id: "id-1".into(),
            payload: payload.clone(),
        };
        let json = serde_json::to_string(&resp).expect("serialize response");
        let back: DispatchResponse = serde_json::from_str(&json).expect("deserialize response");
        assert_eq!(
            serde_json::to_value(&back.payload).unwrap(),
            serde_json::to_value(&payload).unwrap()
        );
    }
}
