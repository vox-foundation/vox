#![allow(missing_docs)]
#![cfg(feature = "transport")]

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Mutex;
use std::time::Duration;

use vox_populi::http_client::PopuliHttpClient;
use vox_populi::transport::{AdminQuarantineRequest, PopuliHttpAuth, PopuliTransportState};
use vox_populi::{node_record_for_current_process, transport};

static ENV_MUTEX: Mutex<()> = Mutex::new(());

#[tokio::test]
async fn join_list_heartbeat_roundtrip() {
    let state = PopuliTransportState::new();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();

    let app = transport::populi_http_app_with_auth(state, PopuliHttpAuth::Open);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let base = format!("http://{}", bound);
    let client = PopuliHttpClient::new(&base);
    let empty = client.list_nodes().await.unwrap();
    assert!(empty.nodes.is_empty());

    let node = node_record_for_current_process("node-a".into(), Some(bound.to_string()));
    let joined = client.join(&node).await.unwrap();
    assert_eq!(joined.id, "node-a");

    let listed = client.list_nodes().await.unwrap();
    assert_eq!(listed.nodes.len(), 1);

    let beat = client.heartbeat(&node).await.unwrap();
    assert!(beat.last_seen_unix_ms >= node.last_seen_unix_ms);

    server.abort();
}

#[tokio::test]
async fn leave_removes_node() {
    let state = PopuliTransportState::new();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();

    let app = transport::populi_http_app_with_auth(state, PopuliHttpAuth::Open);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let base = format!("http://{}", bound);
    let client = PopuliHttpClient::new(&base);
    let node = node_record_for_current_process("gone".into(), None);
    client.join(&node).await.unwrap();
    assert_eq!(client.list_nodes().await.unwrap().nodes.len(), 1);

    assert!(client.leave("gone").await.unwrap());
    assert!(client.list_nodes().await.unwrap().nodes.is_empty());
    assert!(!client.leave("gone").await.unwrap());

    server.abort();
}

#[tokio::test]
async fn mesh_token_requires_bearer() {
    let state = PopuliTransportState::new();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();

    let app = transport::populi_http_app_with_auth(
        state,
        PopuliHttpAuth::Bearer("unit-test-populi-token".into()),
    );
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let base = format!("http://{}", bound);
    let no_auth = PopuliHttpClient::new(&base);
    assert!(no_auth.list_nodes().await.is_err());

    let authed = PopuliHttpClient::new(&base).with_bearer("unit-test-populi-token");
    assert!(authed.list_nodes().await.is_ok());

    server.abort();
}

#[tokio::test]
async fn health_ok_without_bearer_when_token_required() {
    let state = PopuliTransportState::new();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();

    let app = transport::populi_http_app_with_auth(
        state,
        transport::PopuliHttpAuth::Bearer("unit-test-populi-token".into()),
    );
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let base = format!("http://{}", bound);
    let url = format!("{base}/health");
    let r = reqwest::get(&url).await.unwrap();
    assert_eq!(r.status(), reqwest::StatusCode::OK);
    assert_eq!(r.text().await.unwrap(), "ok\n");

    server.abort();
}

#[tokio::test]
async fn join_rejected_when_scope_mismatch() {
    let state = PopuliTransportState::with_required_scope(Some("cluster-a".into()));
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();

    let app = transport::populi_http_app_with_auth(state, PopuliHttpAuth::Open);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let base = format!("http://{}", bound);
    let client = PopuliHttpClient::new(&base);
    let mut node = node_record_for_current_process("wrong-scope".into(), None);
    node.scope_id = Some("cluster-b".into());
    let err = client.join(&node).await.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("403") || msg.contains("Forbidden"),
        "unexpected error: {msg}"
    );

    server.abort();
}

#[tokio::test]
async fn join_ok_when_scope_matches() {
    let state = PopuliTransportState::with_required_scope(Some("cluster-a".into()));
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();

    let app = transport::populi_http_app_with_auth(state, PopuliHttpAuth::Open);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let base = format!("http://{}", bound);
    let client = PopuliHttpClient::new(&base);
    let mut node = node_record_for_current_process("scoped-node".into(), None);
    node.scope_id = Some("cluster-a".into());
    client.join(&node).await.unwrap();
    assert_eq!(client.list_nodes().await.unwrap().nodes.len(), 1);

    server.abort();
}

#[tokio::test]
#[allow(unsafe_code)]
async fn list_nodes_omits_stale_entries_when_server_prune_env_set() {
    let _guard = ENV_MUTEX.lock().expect("env lock");
    let prev = std::env::var("VOX_MESH_SERVER_STALE_PRUNE_MS").ok();
    unsafe {
        std::env::set_var("VOX_MESH_SERVER_STALE_PRUNE_MS", "5");
    }

    let state = PopuliTransportState::new();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();

    let app = transport::populi_http_app_with_auth(state, PopuliHttpAuth::Open);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let base = format!("http://{}", bound);
    let client = PopuliHttpClient::new(&base);
    let node = node_record_for_current_process("stale-a".into(), None);
    client.join(&node).await.unwrap();
    assert_eq!(client.list_nodes().await.unwrap().nodes.len(), 1);

    tokio::time::sleep(Duration::from_millis(40)).await;
    let listed = client.list_nodes().await.unwrap();
    assert!(
        listed.nodes.is_empty(),
        "expected stale node hidden after prune window"
    );

    server.abort();
    unsafe {
        match prev {
            Some(v) => std::env::set_var("VOX_MESH_SERVER_STALE_PRUNE_MS", v),
            None => std::env::remove_var("VOX_MESH_SERVER_STALE_PRUNE_MS"),
        }
    }
}

#[tokio::test]
#[allow(unsafe_code)]
async fn a2a_deliver_respects_in_memory_cap() {
    let _guard = ENV_MUTEX.lock().expect("env lock");
    let prev = std::env::var("VOX_MESH_A2A_MAX_MESSAGES").ok();
    unsafe {
        std::env::set_var("VOX_MESH_A2A_MAX_MESSAGES", "3");
    }

    let state = PopuliTransportState::new();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();
    let app = transport::populi_http_app_with_auth(state, PopuliHttpAuth::Open);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let base = format!("http://{}", bound);
    let http = reqwest::Client::new();
    for i in 0..5 {
        let r = http
            .post(format!("{base}/v1/populi/a2a/deliver"))
            .json(&serde_json::json!({
                "sender_agent_id": "s",
                "receiver_agent_id": "r",
                "message_type": "t",
                "payload": format!("{i}")
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), reqwest::StatusCode::OK);
    }
    let inbox = http
        .post(format!("{base}/v1/populi/a2a/inbox"))
        .json(&serde_json::json!({ "receiver_agent_id": "r" }))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = inbox.json().await.unwrap();
    let n = body["messages"].as_array().map_or(0, |a| a.len());
    assert!(n <= 3, "expected cap 3, got {n}: {body:?}");

    server.abort();
    unsafe {
        match prev {
            Some(v) => std::env::set_var("VOX_MESH_A2A_MAX_MESSAGES", v),
            None => std::env::remove_var("VOX_MESH_A2A_MAX_MESSAGES"),
        }
    }
}

#[tokio::test]
async fn oversized_json_body_returns_413() {
    let state = PopuliTransportState::new();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();
    let app = transport::populi_http_app_with_auth(state, PopuliHttpAuth::Open);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let base = format!("http://{}", bound);
    let pad = "x".repeat(600 * 1024);
    let http = reqwest::Client::new();
    let r = http
        .post(format!("{base}/v1/populi/join"))
        .json(&serde_json::json!({
            "id": pad,
            "capabilities": {},
            "version": "test",
            "last_seen_unix_ms": 0
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        r.status(),
        reqwest::StatusCode::PAYLOAD_TOO_LARGE,
        "expected 413 for oversized join body"
    );

    server.abort();
}

#[tokio::test]
#[allow(unsafe_code)]
async fn bootstrap_exchange_works_once() {
    let _guard = ENV_MUTEX.lock().expect("env lock");
    let prev_bootstrap = std::env::var("VOX_MESH_BOOTSTRAP_TOKEN").ok();
    let prev_expires = std::env::var("VOX_MESH_BOOTSTRAP_EXPIRES_UNIX_MS").ok();
    let prev_token = std::env::var("VOX_MESH_TOKEN").ok();
    let prev_scope = std::env::var("VOX_MESH_SCOPE_ID").ok();

    let bootstrap = "bootstrap-unit-test-token";
    // SAFETY: serialized by `ENV_MUTEX`, restored at test end.
    unsafe {
        std::env::set_var("VOX_MESH_BOOTSTRAP_TOKEN", bootstrap);
        std::env::set_var(
            "VOX_MESH_BOOTSTRAP_EXPIRES_UNIX_MS",
            (vox_populi::wall_clock_unix_ms() + 120_000).to_string(),
        );
        std::env::set_var("VOX_MESH_TOKEN", "mesh-unit-token");
        std::env::set_var("VOX_MESH_SCOPE_ID", "scope-unit");
    }

    let state = PopuliTransportState::new_for_serve();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();
    let app = transport::populi_http_app_with_auth(state, PopuliHttpAuth::Open);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let base = format!("http://{}", bound);
    let client = reqwest::Client::new();
    let first = client
        .post(format!("{base}/v1/populi/bootstrap/exchange"))
        .json(&serde_json::json!({ "bootstrap_token": bootstrap }))
        .send()
        .await
        .unwrap();
    assert_eq!(first.status(), reqwest::StatusCode::OK);
    let payload: serde_json::Value = first.json().await.unwrap();
    assert_eq!(payload["mesh_token"], "mesh-unit-token");
    assert_eq!(payload["scope_id"], "scope-unit");

    let second = client
        .post(format!("{base}/v1/populi/bootstrap/exchange"))
        .json(&serde_json::json!({ "bootstrap_token": bootstrap }))
        .send()
        .await
        .unwrap();
    assert_eq!(second.status(), reqwest::StatusCode::GONE);

    server.abort();
    unsafe {
        match prev_bootstrap {
            Some(v) => std::env::set_var("VOX_MESH_BOOTSTRAP_TOKEN", v),
            None => std::env::remove_var("VOX_MESH_BOOTSTRAP_TOKEN"),
        }
        match prev_expires {
            Some(v) => std::env::set_var("VOX_MESH_BOOTSTRAP_EXPIRES_UNIX_MS", v),
            None => std::env::remove_var("VOX_MESH_BOOTSTRAP_EXPIRES_UNIX_MS"),
        }
        match prev_token {
            Some(v) => std::env::set_var("VOX_MESH_TOKEN", v),
            None => std::env::remove_var("VOX_MESH_TOKEN"),
        }
        match prev_scope {
            Some(v) => std::env::set_var("VOX_MESH_SCOPE_ID", v),
            None => std::env::remove_var("VOX_MESH_SCOPE_ID"),
        }
    }
}

#[tokio::test]
async fn quarantine_blocks_claim_until_cleared() {
    let state = PopuliTransportState::new();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();
    let app = transport::populi_http_app_with_auth(state, PopuliHttpAuth::Open);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    let base = format!("http://{}", bound);
    let http = PopuliHttpClient::new(&base);
    let node = node_record_for_current_process("q-worker".into(), None);
    http.join(&node).await.unwrap();
    http.admin_quarantine(&AdminQuarantineRequest {
        node_id: "q-worker".into(),
        quarantined: true,
    })
    .await
    .unwrap();
    http.relay_a2a(&vox_populi::transport::A2ADeliverRequest {
        sender_agent_id: "0".into(),
        receiver_agent_id: "99".into(),
        message_type: vox_populi::transport::A2A_MESSAGE_JOB_SUBMIT.into(),
        payload: "{}".into(),
        idempotency_key: None,
        privacy_class: None,
        payload_blake3_hex: None,
        worker_ed25519_sig_b64: None,
    })
    .await
    .unwrap();
    let r = reqwest::Client::new()
        .post(format!("{base}/v1/populi/a2a/inbox"))
        .json(&serde_json::json!({
            "receiver_agent_id": "99",
            "claimer_node_id": "q-worker",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), reqwest::StatusCode::OK);
    let body: serde_json::Value = r.json().await.unwrap();
    assert_eq!(body["messages"].as_array().unwrap().len(), 0);
    http.admin_quarantine(&AdminQuarantineRequest {
        node_id: "q-worker".into(),
        quarantined: false,
    })
    .await
    .unwrap();
    let r2i = reqwest::Client::new()
        .post(format!("{base}/v1/populi/a2a/inbox"))
        .json(&serde_json::json!({
            "receiver_agent_id": "99",
            "claimer_node_id": "q-worker",
        }))
        .send()
        .await
        .unwrap();
    let body2: serde_json::Value = r2i.json().await.unwrap();
    assert_eq!(body2["messages"].as_array().unwrap().len(), 1);
    server.abort();
}

#[tokio::test]
async fn a2a_lease_renew_requires_holder() {
    let state = PopuliTransportState::new();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();
    let app = transport::populi_http_app_with_auth(state, PopuliHttpAuth::Open);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    let base = format!("http://{}", bound);
    let http = PopuliHttpClient::new(&base);
    let node = node_record_for_current_process("lease-a".into(), None);
    http.join(&node).await.unwrap();
    http.relay_a2a(&vox_populi::transport::A2ADeliverRequest {
        sender_agent_id: "0".into(),
        receiver_agent_id: "7".into(),
        message_type: "x".into(),
        payload: "{}".into(),
        idempotency_key: None,
        privacy_class: None,
        payload_blake3_hex: None,
        worker_ed25519_sig_b64: None,
    })
    .await
    .unwrap();
    let inbox: serde_json::Value = reqwest::Client::new()
        .post(format!("{base}/v1/populi/a2a/inbox"))
        .json(&serde_json::json!({
            "receiver_agent_id": "7",
            "claimer_node_id": "lease-a",
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let mid = inbox["messages"][0]["id"].as_u64().unwrap();
    http.relay_a2a_lease_renew(&vox_populi::transport::A2ALeaseRenewRequest {
        receiver_agent_id: "7".into(),
        message_id: mid,
        claimer_node_id: "lease-a".into(),
    })
    .await
    .unwrap();
    let node_b = node_record_for_current_process("lease-b".into(), None);
    http.join(&node_b).await.unwrap();
    let bad = reqwest::Client::new()
        .post(format!("{base}/v1/populi/a2a/lease-renew"))
        .json(&serde_json::json!({
            "receiver_agent_id": "7",
            "message_id": mid,
            "claimer_node_id": "lease-b",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(bad.status(), reqwest::StatusCode::CONFLICT);
    server.abort();
}

#[tokio::test]
async fn mesh_jwt_hs256_accepts_and_rejects_jti_replay() {
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
    use serde::Serialize;

    #[derive(Serialize)]
    struct Claims<'a> {
        role: &'a str,
        jti: &'a str,
        exp: u64,
    }

    let state = PopuliTransportState::new();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();
    let jwt_rt = transport::PopuliMeshAuthRuntime::with_jwt_hmac_only("jwt-unit-secret-for-test");
    let app =
        transport::populi_http_app_with_auth(state, transport::PopuliHttpAuth::Custom(jwt_rt));
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    let base = format!("http://{}", bound);
    let exp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 3600;
    let token = encode(
        &Header::new(Algorithm::HS256),
        &Claims {
            role: "worker",
            jti: "jti-unit-once",
            exp,
        },
        &EncodingKey::from_secret(b"jwt-unit-secret-for-test"),
    )
    .unwrap();
    let url = format!("{base}/v1/populi/nodes");
    let ok = reqwest::Client::new()
        .get(&url)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {}", token))
        .send()
        .await
        .unwrap();
    assert_eq!(ok.status(), reqwest::StatusCode::OK);
    let replay = reqwest::Client::new()
        .get(&url)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {}", token))
        .send()
        .await
        .unwrap();
    assert_eq!(replay.status(), reqwest::StatusCode::UNAUTHORIZED);
    server.abort();
}

#[tokio::test]
async fn job_result_attestation_requires_full_key_when_fields_present() {
    let state = PopuliTransportState::new();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();
    let app = transport::populi_http_app_with_auth(state, PopuliHttpAuth::Open);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    let base = format!("http://{}", bound);
    let http = PopuliHttpClient::new(&base);
    let err = http
        .relay_a2a(&vox_populi::transport::A2ADeliverRequest {
            sender_agent_id: "1".into(),
            receiver_agent_id: "2".into(),
            message_type: vox_populi::transport::A2A_MESSAGE_JOB_RESULT.into(),
            payload: "{}".into(),
            idempotency_key: None,
            privacy_class: None,
            payload_blake3_hex: Some(
                "0000000000000000000000000000000000000000000000000000000000000000".into(),
            ),
            worker_ed25519_sig_b64: Some(
                "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".into(),
            ),
        })
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("503")
            || err.to_string().to_ascii_lowercase().contains("unavailable"),
        "{err}"
    );
    server.abort();
}

#[tokio::test]
async fn job_result_attestation_accepts_valid_signature() {
    use base64::Engine as _;
    use ed25519_dalek::{Signer, SigningKey};
    use rand::RngCore;
    use rand::rngs::OsRng;

    let mut seed = [0u8; 32];
    OsRng.fill_bytes(&mut seed);
    let sk = SigningKey::from_bytes(&seed);
    let vk = sk.verifying_key();

    let state = PopuliTransportState::new().with_worker_result_verify_key(Some(vk.to_bytes()));
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();
    let app = transport::populi_http_app_with_auth(state, PopuliHttpAuth::Open);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    let base = format!("http://{}", bound);
    let http = PopuliHttpClient::new(&base);
    let payload = r#"{"status":"done"}"#;
    let digest = *blake3::hash(payload.as_bytes()).as_bytes();
    let sig = sk.sign(&digest);
    let digest_hex = data_encoding::HEXLOWER.encode(&digest);
    let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig.to_bytes());
    http.relay_a2a(&vox_populi::transport::A2ADeliverRequest {
        sender_agent_id: "1".into(),
        receiver_agent_id: "2".into(),
        message_type: vox_populi::transport::A2A_MESSAGE_JOB_RESULT.into(),
        payload: payload.into(),
        idempotency_key: None,
        privacy_class: None,
        payload_blake3_hex: Some(digest_hex),
        worker_ed25519_sig_b64: Some(sig_b64),
    })
    .await
    .unwrap();
    server.abort();
}
