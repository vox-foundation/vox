use super::*;
pub(super) async fn http_ws(
    State(state): State<GatewayState>,
    ws: WebSocketUpgrade,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = enforce_request_guards(&state, &connect.0, &headers).await {
        return resp;
    }
    let role = match resolve_access_role(&state, &headers) {
        Ok(r) => r,
        Err(msg) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": msg })),
            )
                .into_response();
        }
    };
    let identity = request_identity(&state, &connect.0, &headers);
    ws.on_upgrade(move |socket| async move {
        handle_ws(socket, state, connect.0, identity, role).await;
    })
}
pub(super) async fn handle_ws(
    mut socket: WebSocket,
    state: GatewayState,
    peer: SocketAddr,
    identity: String,
    role: AccessRole,
) {
    let mut rx = state.server_state.orchestrator.event_bus().subscribe();

    loop {
        tokio::select! {
            msg = socket.recv() => {
                let Some(Ok(msg)) = msg else { break };
                match msg {
                    Message::Text(text) => {
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
                        let parsed: Result<WsMessageIn, _> = serde_json::from_str(&text);
                        let reply = match parsed {
                            Ok(req) => ws_handle_message(&state, req, peer, role).await,
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
        }
    }
}
pub(super) async fn ws_handle_message(
    state: &GatewayState,
    req: WsMessageIn,
    peer: SocketAddr,
    role: AccessRole,
) -> WsMessageOut {
    match req.msg_type.as_str() {
        "list_tools" => {
            let mut tools: Vec<String> = visible_tools_for_role(state, role).into_iter().collect();
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
            let result = call_tool_response_value(state, name, args, Some(peer), role).await;
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
