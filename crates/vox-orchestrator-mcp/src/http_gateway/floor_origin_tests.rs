//! Safety floor origin enforcement for HTTP gateway.

use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use std::collections::HashSet;
use std::net::SocketAddr;
use tower::ServiceExt;
use vox_foundation::protocol::{DispatchPayload, DispatchRequest};
use vox_orchestrator::orch_daemon::ExtraDispatch;

use super::{GatewayState, build_app};
use crate::ServerState;
use crate::daemon_extra::McpExtraDispatch;

fn req(method: &str, params: serde_json::Value) -> DispatchRequest {
    DispatchRequest {
        id: "1".to_string(),
        method: method.to_string(),
        params,
        auth_token: None,
        permission_mode: None,
    }
}

#[test]
fn gateway_cannot_approve_floored_park() {
    std::thread::Builder::new()
        .name("gateway_cannot_approve_floored_park".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime")
                .block_on(async {
                    let server_state = ServerState::new_test().await;
                    let extra = McpExtraDispatch::new(server_state.clone());

                    let mut allowed = HashSet::new();
                    allowed.insert("vox_skill_install".to_string());
                    allowed.insert("vox_resolve_approval".to_string());
                    let gateway_state =
                        GatewayState::for_test_with_state(server_state, allowed).await;
                    let app = build_app(gateway_state.clone());

                    // 1. Spawn POST /v1/tools/call vox_skill_install {}
                    let app_clone = app.clone();
                    let call = tokio::spawn(async move {
                        let peer: SocketAddr = "127.0.0.1:1234".parse().unwrap();
                        let mut req = Request::builder()
                            .method("POST")
                            .uri("/v1/tools/call")
                            .header("content-type", "application/json")
                            .body(Body::from(
                                serde_json::json!({
                                    "name": "vox_skill_install",
                                    "args": {}
                                })
                                .to_string(),
                            ))
                            .unwrap();
                        req.extensions_mut().insert(ConnectInfo(peer));
                        let resp = app_clone.oneshot(req).await.unwrap();
                        assert_eq!(resp.status(), StatusCode::OK);
                        let bytes = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
                        serde_json::from_slice::<Value>(&bytes).unwrap()
                    });

                    // Poll ExtraDispatch LIST_PENDING_APPROVALS until it parks
                    let deadline = tokio::time::Instant::now() + vox_config::timeouts::D_15S;
                    let approval_id = loop {
                        let resp = extra
            .try_handle(&req(
                vox_foundation::protocol::orch_daemon_method::LIST_PENDING_APPROVALS,
                serde_json::json!({}),
            ))
            .await
            .expect("list pending approvals");
                        let approvals = match resp.payload {
                            DispatchPayload::Result { value } => value,
                            other => panic!("expected Result payload, got {other:?}"),
                        };
                        if let Some(entry) = approvals
                            .get("approvals")
                            .and_then(|a| a.as_array())
                            .and_then(|arr| arr.iter().find(|e| e["tool"] == "vox_skill_install"))
                        {
                            let summary = entry["summary"].as_str().expect("summary string");
                            assert!(
                                summary.starts_with("[floor:external_effect]"),
                                "expected [floor:external_effect], got {summary}"
                            );
                            break entry["approval_id"]
                                .as_str()
                                .expect("approval_id")
                                .to_string();
                        }
                        assert!(
                            tokio::time::Instant::now() < deadline,
                            "vox_skill_install never parked via gateway"
                        );
                        tokio::time::sleep(vox_config::timeouts::D_20MS).await;
                    };

                    // 2. Send POST /v1/tools/call vox_resolve_approval approve
                    let peer: SocketAddr = "127.0.0.1:1234".parse().unwrap();
                    let mut req_approve = Request::builder()
                        .method("POST")
                        .uri("/v1/tools/call")
                        .header("content-type", "application/json")
                        .body(Body::from(
                            serde_json::json!({
                                "name": "vox_resolve_approval",
                                "args": {
                                    "approval_id": approval_id,
                                    "outcome": "approve",
                                    "origin": "human"
                                }
                            })
                            .to_string(),
                        ))
                        .unwrap();
                    req_approve.extensions_mut().insert(ConnectInfo(peer));
                    let app_approve = app.clone();
                    let resp_approve =
                        tokio::spawn(
                            async move { app_approve.oneshot(req_approve).await.unwrap() },
                        )
                        .await
                        .unwrap();
                    assert_eq!(resp_approve.status(), StatusCode::OK);
                    let bytes = to_bytes(resp_approve.into_body(), 64 * 1024).await.unwrap();
                    let res_val: Value = serde_json::from_slice(&bytes).unwrap();
                    let res_text = res_val.to_string();
                    assert!(
                        res_text.contains("FLOOR_SELF_APPROVAL_REFUSED"),
                        "expected FLOOR_SELF_APPROVAL_REFUSED, got: {res_text}"
                    );

                    // Parked approval is still listed
                    let list_resp = extra
                        .try_handle(&req(
                            vox_foundation::protocol::orch_daemon_method::LIST_PENDING_APPROVALS,
                            serde_json::json!({}),
                        ))
                        .await
                        .expect("list pending approvals");
                    let after_approvals = match list_resp.payload {
                        DispatchPayload::Result { value } => value,
                        other => panic!("expected Result payload, got {other:?}"),
                    };
                    assert!(
                        after_approvals["approvals"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|e| e["approval_id"] == approval_id),
                        "parked approval must still be listed"
                    );

                    // 3. Clean up through ExtraDispatch orch.resolve_approval rejected
                    let resolve_resp = extra
        .try_handle(&req(
            vox_foundation::protocol::orch_daemon_method::RESOLVE_APPROVAL,
            serde_json::json!({ "approval_id": approval_id, "outcome": "rejected" }),
        ))
        .await
        .expect("resolve approval");
                    let resolved = match resolve_resp.payload {
                        DispatchPayload::Result { value } => value,
                        other => panic!("expected Result payload, got {other:?}"),
                    };
                    assert_eq!(
                        resolved.get("resolved"),
                        Some(&serde_json::Value::Bool(true))
                    );

                    let raw = call.await.expect("join");
                    let is_err = raw["is_error"].as_bool().unwrap_or(false)
                        || !raw["success"].as_bool().unwrap_or(true)
                        || crate::server_state::tool_json_envelope_is_error(&raw.to_string());
                    assert!(is_err, "spawned call should fail after reject, got: {raw}");
                })
        })
        .expect("spawn")
        .join()
        .expect("join");
}
