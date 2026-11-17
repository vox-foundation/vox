use super::*;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

async fn read_body(resp: axum::response::Response) -> Value {
    let bytes = to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .expect("read body bytes");
    serde_json::from_slice(&bytes).expect("parse json body")
}

fn req_peer(method: &str, uri: &str, body: Body) -> Request<Body> {
    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(body)
        .expect("build request");
    req.extensions_mut().insert(ConnectInfo(
        "127.0.0.1:4242".parse::<SocketAddr>().expect("addr"),
    ));
    req
}

fn url_encode_payload(payload: &str) -> String {
    payload
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

#[tokio::test]
async fn concilium_router_snapshot_board_placement_and_all_pilot_actions() {
    let gs = GatewayState::for_test().await;
    let mut bus_rx = gs.server_state.orchestrator.event_bus().subscribe();
    let mut topic_rx = gs.topic_tx.subscribe();
    let app = super::super::build_app(gs);

    let r1 = app
        .clone()
        .oneshot(req_peer(
            "POST",
            "/api/v2/concilium/action",
            Body::from(
                serde_json::json!({"action":"park_agent","agent_id":"parser","channel":"parser"})
                    .to_string(),
            ),
        ))
        .await
        .expect("park request");
    assert_eq!(r1.status(), StatusCode::OK);
    assert_eq!(read_body(r1).await["data"]["seq"], 1);

    let payload = serde_json::json!({
        "action":"steer_agent",
        "agent_id":"parser",
        "channel":"#parser",
        "directive":"check ast"
    })
    .to_string();
    let enc = url_encode_payload(&payload);
    let r2 = app
        .clone()
        .oneshot(req_peer(
            "GET",
            &format!("/api/v2/concilium/action?payload={enc}"),
            Body::empty(),
        ))
        .await
        .expect("steer request");
    assert_eq!(read_body(r2).await["data"]["seq"], 2);

    let r3 = app
        .clone()
        .oneshot(req_peer(
            "POST",
            "/api/v2/concilium/action",
            Body::from(
                serde_json::json!({
                    "action":"fork_agent",
                    "agent_id":"parser",
                    "channel":"#parser",
                    "fork_name":"parser-b",
                    "workspace_mode":"share"
                })
                .to_string(),
            ),
        ))
        .await
        .expect("fork request");
    assert_eq!(read_body(r3).await["data"]["forked_agent_id"], "parser-b");

    let r4 = app
        .clone()
        .oneshot(req_peer(
            "POST",
            "/api/v2/concilium/action",
            Body::from(
                serde_json::json!({
                    "action":"swap_agent_compute",
                    "agent_id":"parser-b",
                    "channel":"#parser",
                    "target_tier":"populi_quic_peer",
                    "target_node_id":"iroh-peer-1",
                    "target_model_id":"qwen2.5-coder-32b"
                })
                .to_string(),
            ),
        ))
        .await
        .expect("swap request");
    assert_eq!(read_body(r4).await["data"]["ok"], true);

    let snap = read_body(
        app.clone()
            .oneshot(req_peer("GET", "/api/v2/concilium/snapshot", Body::empty()))
            .await
            .expect("snapshot request"),
    )
    .await;
    assert_eq!(snap["data"]["head_seq"], 4);
    assert!(
        snap["data"]["lanes"]
            .as_array()
            .expect("lanes")
            .iter()
            .any(|l| l["roster_name"] == "parser-b")
    );

    let board = read_body(
        app.clone()
            .oneshot(req_peer(
                "GET",
                "/api/v2/concilium/board?since_seq=1&channel=parser",
                Body::empty(),
            ))
            .await
            .expect("board request"),
    )
    .await;
    assert_eq!(board["data"].as_array().expect("board").len(), 3);

    let place = read_body(
        app.oneshot(req_peer(
            "GET",
            "/api/v2/concilium/placement",
            Body::empty(),
        ))
        .await
        .expect("placement request"),
    )
    .await;
    let nodes = place["data"]["nodes"].as_array().expect("nodes");
    assert!(nodes.iter().any(|n| n["tier"] == "local_mens_gpu"));
    assert!(nodes.iter().any(|n| n["node_id"] == "iroh-peer-1"));
    assert!(nodes.iter().any(|n| n["tier"] == "cloud_bandit"));
    assert_eq!(
        topic_rx.try_recv().expect("topic msg").topic,
        "concilium.delta"
    );
    let ev = bus_rx.try_recv().expect("stamped bus event");
    assert!(ev.causal_stamp.is_some());
}

#[tokio::test]
async fn concilium_action_enforces_write_token_and_blocks_cross_site_get() {
    let gs = GatewayState::for_test()
        .await
        .with_tokens(Some("write-secret"), Some("read-secret"));
    let app = super::super::build_app(gs);

    let mut read_req = req_peer("GET", "/api/v2/concilium/snapshot", Body::empty());
    read_req.headers_mut().insert(
        "authorization",
        "Bearer read-secret".parse().expect("auth header"),
    );
    let read_body_json = read_body(app.clone().oneshot(read_req).await.expect("read")).await;
    assert!(read_body_json.get("data").is_some(), "{read_body_json}");

    let mut write_denied = req_peer(
        "POST",
        "/api/v2/concilium/action",
        Body::from(r#"{"action":"park_agent","agent_id":"parser"}"#),
    );
    write_denied.headers_mut().insert(
        "authorization",
        "Bearer read-secret".parse().expect("auth header"),
    );
    let denied_json = read_body(app.clone().oneshot(write_denied).await.expect("write")).await;
    assert_eq!(denied_json["error"]["code"], "forbidden");

    let enc = url_encode_payload(r#"{"action":"park_agent","agent_id":"parser"}"#);
    let mut csrf_get = req_peer(
        "GET",
        &format!("/api/v2/concilium/action?payload={enc}"),
        Body::empty(),
    );
    csrf_get.headers_mut().insert(
        "authorization",
        "Bearer write-secret".parse().expect("auth header"),
    );
    csrf_get.headers_mut().insert(
        "sec-fetch-site",
        "cross-site".parse().expect("sec-fetch-site"),
    );
    let csrf_json = read_body(app.oneshot(csrf_get).await.expect("csrf")).await;
    assert_eq!(csrf_json["error"]["code"], "forbidden");
}
