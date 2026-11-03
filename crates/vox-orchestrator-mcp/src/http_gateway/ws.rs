use super::*;
pub(super) async fn http_ws(
    State(state): State<GatewayState>,
    ws: WebSocketUpgrade,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = enforce_request_guards(&state, &connect.0, &headers).await {
        return *resp;
    }

    let role_res = resolve_access_role(&state, &headers, Some(&connect.0));

    let role_opt = role_res.ok();
    let identity = request_identity(&state, &connect.0, &headers);
    ws.on_upgrade(move |socket| async move {
        handle_ws(socket, state, connect.0, identity, role_opt).await;
    })
}
pub(super) async fn handle_ws(
    mut socket: WebSocket,
    state: GatewayState,
    peer: SocketAddr,
    identity: String,
    mut role_opt: Option<AccessRole>,
) {
    let mut rx = state.server_state.orchestrator.event_bus().subscribe();
    let mut topic_rx = state.topic_tx.subscribe();
    let mut subscribed_topics: std::collections::HashSet<String> = std::collections::HashSet::new();

    loop {
        tokio::select! {
            msg = socket.recv() => {
                let Some(Ok(msg)) = msg else { break };
                match msg {
                    Message::Text(text) => {
                        let parsed: Result<WsMessageIn, _> = serde_json::from_str(&text);

                        if role_opt.is_none() {
                            let mut authenticated = false;
                            if let Ok(ref req) = parsed {
                                if req.msg_type == "auth" {
                                    if let Some(token_val) = req.args.as_ref().and_then(|v| v.get("token")).and_then(|t| t.as_str()) {
                                        if caller_is_loopback(&state, &peer) {
                                            if let Some(dt) = state.dashboard_token.as_ref() {
                                                if constant_time_eq(token_val.as_bytes(), dt.0.as_bytes()) {
                                                    role_opt = Some(AccessRole::Write);
                                                    authenticated = true;
                                                }
                                            }
                                        }
                                        if !authenticated {
                                            if let Some(bt) = state.bearer_token.as_ref() {
                                                if constant_time_eq(token_val.as_bytes(), bt.as_bytes()) {
                                                    role_opt = Some(AccessRole::Write);
                                                    authenticated = true;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if !authenticated {
                                let _ = socket.send(Message::Close(Some(axum::extract::ws::CloseFrame {
                                    code: 4401,
                                    reason: "Unauthorized".into(),
                                }))).await;
                                break;
                            } else {
                                continue;
                            }
                        }

                        let role = role_opt.unwrap();
                        if let Err(msg) = enforce_rate_limit(&state, &identity) {
                            let _ = socket
                                .send(Message::Text(
                                    serde_json::json!({
                                        "type": "error",
                                        "success": false,
                                        "is_error": true,
                                        "error": msg
                                    })
                                    .to_string()
                                    .into(),
                                ))
                                .await;
                            break;
                        }
                        // Topic subscription control messages are handled inline
                        // (they mutate per-connection state rather than producing a tool result).
                        if let Ok(ref req) = parsed {
                            if req.msg_type == "subscribe" || req.msg_type == "unsubscribe" {
                                let topic = req
                                    .args
                                    .as_ref()
                                    .and_then(|v| v.get("topic"))
                                    .and_then(|t| t.as_str())
                                    .map(str::to_string);
                                let out = match topic {
                                    Some(topic) => {
                                        if req.msg_type == "subscribe" {
                                            subscribed_topics.insert(topic.clone());
                                        } else {
                                            subscribed_topics.remove(&topic);
                                        }
                                        WsMessageOut {
                                            id: req.id.clone(),
                                            msg_type: format!("{}_result", req.msg_type),
                                            success: true,
                                            is_error: false,
                                            data: serde_json::json!({
                                                "topic": topic,
                                                "subscribed": subscribed_topics.contains(&topic),
                                            }),
                                            error: None,
                                        }
                                    }
                                    None => WsMessageOut {
                                        id: req.id.clone(),
                                        msg_type: "error".to_string(),
                                        success: false,
                                        is_error: true,
                                        data: Value::Null,
                                        error: Some("missing 'topic' field".to_string()),
                                    },
                                };
                                if socket
                                    .send(Message::Text(
                                        serde_json::to_string(&out).unwrap_or_default().into(),
                                    ))
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                                continue;
                            }
                        }

                        let reply = match parsed {
                            Ok(req) => {
                                let ctx = call_context(&state, &peer, role);
                                ws_handle_message(&state, req, peer, ctx).await
                            }
                            Err(e) => WsMessageOut {
                                id: None,
                                msg_type: "error".to_string(),
                                success: false,
                                is_error: true,
                                data: Value::Null,
                                error: Some(format!("invalid websocket payload: {e}")),
                            },
                        };
                        if socket
                            .send(Message::Text(
                                serde_json::to_string(&reply)
                                    .unwrap_or_else(|_| {
                                        "{\"type\":\"error\",\"success\":false,\"is_error\":true}"
                                            .to_string()
                                    })
                                    .into(),
                            ))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Message::Ping(bytes) => {
                        if socket.send(Message::Pong(bytes)).await.is_err() {
                            break;
                        }
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
            Ok(event) = rx.recv() => {
                let out = WsMessageOut {
                    id: None,
                    msg_type: "agent_event".to_string(),
                    success: true,
                    is_error: false,
                    data: serde_json::to_value(&event).unwrap_or(Value::Null),
                    error: None,
                };
                if socket.send(Message::Text(
                    serde_json::to_string(&out).unwrap_or_default().into()
                )).await.is_err() {
                    break;
                }
            }
            Ok(topic_msg) = topic_rx.recv() => {
                if !super::scientia_feed::should_forward(&subscribed_topics, &topic_msg.topic) {
                    continue;
                }
                let out = WsMessageOut {
                    id: None,
                    msg_type: topic_msg.topic.clone(),
                    success: true,
                    is_error: false,
                    data: topic_msg.data,
                    error: None,
                };
                if socket.send(Message::Text(
                    serde_json::to_string(&out).unwrap_or_default().into()
                )).await.is_err() {
                    break;
                }
            }
        }
    }
}
pub(super) async fn ws_handle_message(
    state: &GatewayState,
    req: WsMessageIn,
    peer: SocketAddr,
    ctx: CallContext,
) -> WsMessageOut {
    match req.msg_type.as_str() {
        "list_tools" => {
            let mut tools: Vec<String> = visible_tools_for(state, ctx).into_iter().collect();
            tools.sort();
            WsMessageOut {
                id: req.id,
                msg_type: "list_tools_result".to_string(),
                success: true,
                is_error: false,
                data: serde_json::json!({ "tools": tools }),
                error: None,
            }
        }
        "call_tool" => {
            let Some(name) = req.name else {
                return WsMessageOut {
                    id: req.id,
                    msg_type: "call_tool_result".to_string(),
                    success: false,
                    is_error: true,
                    data: Value::Null,
                    error: Some("missing tool name".to_string()),
                };
            };
            let args = req
                .args
                .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
            let result = call_tool_response_value(state, name, args, Some(peer), ctx).await;
            WsMessageOut {
                id: req.id,
                msg_type: "call_tool_result".to_string(),
                success: result.success,
                is_error: result.is_error,
                data: result.result,
                error: None,
            }
        }
        other => WsMessageOut {
            id: req.id,
            msg_type: "error".to_string(),
            success: false,
            is_error: true,
            data: Value::Null,
            error: Some(format!("unsupported message type: {other}")),
        },
    }
}

#[cfg(test)]
mod envelope_parity_tests {
    use std::sync::Arc;

    use vox_foundation::protocol::orch_daemon_method;
    use vox_orchestrator::orch_daemon::{self, ExtraDispatch};

    use super::ws_handle_message;
    use crate::daemon_extra::McpExtraDispatch;
    use crate::http_gateway::{GatewayState, WsMessageIn};
    use crate::profiles::CallContext;
    use crate::server_state::ServerState;

    /// `/v1/ws` `call_tool` → `data` must equal `orch.tool_call` → `result` for the same tool, for a
    /// success envelope and for a tool-level error envelope.
    #[tokio::test(flavor = "multi_thread")]
    async fn ws_call_tool_data_equals_local_orch_tool_call_result() {
        let state = ServerState::new_test().await;
        let mut gs = GatewayState::for_inproc_loopback(state.clone());
        let tools: std::collections::HashSet<String> = ["vox_git_status", "vox_get_active_model"]
            .map(String::from)
            .into();
        gs.allowed_tools = Arc::new(tools.clone());
        gs.read_role_eligible_tools = Arc::new(tools);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let extra: Arc<dyn ExtraDispatch> = Arc::new(McpExtraDispatch::new(state.clone()));
        tokio::spawn(orch_daemon::serve_listener_with_extra(
            listener,
            addr.clone(),
            "parity-repo".into(),
            state.orchestrator.clone(),
            Some(extra),
            None,
        ));
        let client = orch_daemon::OrchDaemonClient::new(addr);
        for _ in 0..750 {
            if client.ping().await.is_ok() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }

        for (tool, args) in [
            ("vox_git_status", serde_json::json!({})),
            (
                "vox_get_active_model",
                serde_json::json!({ "unexpected_arg": true }),
            ),
        ] {
            let local = client
                .call(
                    orch_daemon_method::TOOL_CALL,
                    serde_json::json!({ "name": tool, "args": args.clone() }),
                )
                .await
                .expect("orch.tool_call");
            let req: WsMessageIn = serde_json::from_value(serde_json::json!({
                "id": "1", "type": "call_tool", "name": tool, "args": args,
            }))
            .expect("ws frame");
            let remote = ws_handle_message(
                &gs,
                req,
                "127.0.0.1:9".parse().unwrap(),
                CallContext::LOCAL_WRITE,
            )
            .await;
            assert_eq!(
                remote.data, local,
                "{tool}: /v1/ws data differs from orch.tool_call result"
            );
        }
    }
}
