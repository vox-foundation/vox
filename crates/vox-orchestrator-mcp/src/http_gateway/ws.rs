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
                                let from_seq = req
                                    .args
                                    .as_ref()
                                    .and_then(|v| v.get("from_seq"))
                                    .and_then(|v| v.as_u64());
                                let session_id = req
                                    .args
                                    .as_ref()
                                    .and_then(|v| v.get("session_id"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                let mut replay_frames = Vec::new();
                                let out = match topic {
                                    Some(topic) => {
                                        if req.msg_type == "subscribe" {
                                            subscribed_topics.insert(topic.clone());
                                            if topic == super::scientia_feed::RESEARCH_SESSION_DELTA
                                                && from_seq.is_some()
                                            {
                                                replay_frames = replay_frames_for_subscription(
                                                    state.research_feed(),
                                                    session_id,
                                                    from_seq,
                                                );
                                            }
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
                                let mut send_failed = false;
                                for frame in replay_frames {
                                    if socket
                                        .send(Message::Text(
                                            serde_json::to_string(&frame)
                                                .unwrap_or_default()
                                                .into(),
                                        ))
                                        .await
                                        .is_err()
                                    {
                                        send_failed = true;
                                        break;
                                    }
                                }
                                if send_failed {
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
            recv_res = topic_rx.recv() => {
                match recv_res {
                    Ok(topic_msg) => {
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
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        let out = lagged_resync_frame(skipped);
                        if socket.send(Message::Text(
                            serde_json::to_string(&out).unwrap_or_default().into()
                        )).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
}

pub(super) fn lagged_resync_frame(skipped: u64) -> WsMessageOut {
    WsMessageOut {
        id: None,
        msg_type: "resync_required".to_string(),
        success: true,
        is_error: false,
        data: serde_json::json!({
            "skipped": skipped,
            "reason": "broadcast_lagged",
        }),
        error: None,
    }
}

pub(super) fn replay_frames_for_subscription(
    feed: &super::scientia_feed::ResearchFeedHub,
    session_id: &str,
    from_seq: Option<u64>,
) -> Vec<WsMessageOut> {
    match feed.replay_since(session_id, from_seq) {
        super::scientia_feed::ReplayWindow::Events(events) => events
            .into_iter()
            .map(|ev| WsMessageOut {
                id: None,
                msg_type: super::scientia_feed::RESEARCH_SESSION_DELTA.to_string(),
                success: true,
                is_error: false,
                data: serde_json::to_value(&ev).unwrap_or(Value::Null),
                error: None,
            })
            .collect(),
        super::scientia_feed::ReplayWindow::ResyncRequired {
            oldest_available_seq,
            latest_seq,
        } => vec![WsMessageOut {
            id: None,
            msg_type: "resync_required".to_string(),
            success: true,
            is_error: false,
            data: serde_json::json!({
                "session_id": session_id,
                "oldest_available_seq": oldest_available_seq,
                "latest_seq": latest_seq,
                "reason": "cursor_behind_replay_window",
            }),
            error: None,
        }],
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

    #[test]
    fn lagged_recv_error_maps_to_resync_required_frame() {
        let frame = super::lagged_resync_frame(17);
        let v = serde_json::to_value(&frame).unwrap();
        assert_eq!(v["type"], "resync_required");
        assert_eq!(v["data"]["skipped"], 17);
        assert_eq!(v["success"], true);
    }

    #[test]
    fn ws_research_delta_subscription_replays_or_requests_resync() {
        let (topic_tx, _) = tokio::sync::broadcast::channel(16);
        let hub = super::super::scientia_feed::ResearchFeedHub::with_ring_capacity(
            topic_tx,
            "repo".into(),
            2,
        );
        for w in 1..=4u8 {
            hub.publish_research_event(
                vox_research_events::ResearchEvent::WaveStarted {
                    session_id: "s-ws".into(),
                    wave: w,
                    max_waves: 5,
                    tier: "deep".into(),
                    subquery_count: 1,
                },
                100 + w as i64,
            );
        }
        let ok_frames = super::replay_frames_for_subscription(&hub, "s-ws", Some(2));
        assert_eq!(ok_frames.len(), 2);
        assert_eq!(ok_frames[0].msg_type, "research.session.delta");
        assert_eq!(ok_frames[0].data["seq"], 3);
        assert_eq!(ok_frames[1].data["seq"], 4);

        let lag_frames = super::replay_frames_for_subscription(&hub, "s-ws", Some(1));
        assert_eq!(lag_frames.len(), 1);
        assert_eq!(lag_frames[0].msg_type, "resync_required");
        assert_eq!(lag_frames[0].data["oldest_available_seq"], 3);
        assert_eq!(lag_frames[0].data["latest_seq"], 4);
    }
}
