//! Zenodo scholarly adapter against a local mock HTTP server (`VOX_ZENODO_API_BASE`).

mod common;

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    body::Bytes,
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
};
use serde_json::json;
use tokio::net::TcpListener;
use vox_publisher::publication::PublicationManifest;

use common::wait_for_local_server;
use vox_publisher::scholarly::{self, fetch_scholarly_remote_status_for_adapter};

static ZENODO_ENV_LOCK: Mutex<()> = Mutex::new(());

fn swap_env(key: &str, val: Option<&str>) -> Option<String> {
    let prev = std::env::var(key).ok();
    // SAFETY: `set_var`/`remove_var` are only safe when no other thread reads this env key concurrently.
    // This test holds `ZENODO_ENV_LOCK` for the whole test; restore runs on the same thread in `Drop`.
    unsafe {
        match val {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
    }
    prev
}

struct EnvRestore {
    pairs: Vec<(String, Option<String>)>,
}

impl EnvRestore {
    fn new() -> Self {
        Self { pairs: Vec::new() }
    }

    fn set(&mut self, key: &str, val: &str) {
        let prev = swap_env(key, Some(val));
        self.pairs.push((key.to_string(), prev));
    }

    fn remove(&mut self, key: &str) {
        let prev = swap_env(key, None);
        self.pairs.push((key.to_string(), prev));
    }
}

impl Drop for EnvRestore {
    fn drop(&mut self) {
        for (k, prev) in self.pairs.iter().rev() {
            unsafe {
                match prev {
                    Some(v) => std::env::set_var(k, v),
                    None => std::env::remove_var(k),
                }
            }
        }
    }
}

#[tokio::test]
async fn zenodo_adapter_submit_and_status_use_api_base_override() {
    let _lock = ZENODO_ENV_LOCK.lock().expect("env lock");

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let port = addr.port();
    let base = format!("http://127.0.0.1:{port}/api");
    let bucket = format!("http://127.0.0.1:{port}/api/deposit/depositions/424242/files");

    let app = Router::new()
        .route(
            "/api/deposit/depositions",
            post(move |Json(body): Json<serde_json::Value>| {
                let bucket = bucket.clone();
                async move {
                    assert!(body.get("metadata").is_some());
                    Json(json!({
                        "id": 424242,
                        "state": "draft",
                        "links": { "bucket": bucket }
                    }))
                }
            }),
        )
        .route(
            "/api/deposit/depositions/{id}",
            get(|Path(id): Path<String>| async move {
                assert_eq!(id, "424242");
                Json(json!({ "id": 424242, "state": "done" }))
            }),
        );

    let _guard = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    wait_for_local_server(addr, "zenodo mock").await;

    let mut env = EnvRestore::new();
    env.set("VOX_ZENODO_API_BASE", &base);
    env.set("ZENODO_ACCESS_TOKEN", "mock-token");
    for key in [
        "VOX_SCHOLARLY_DISABLE",
        "VOX_SCHOLARLY_DISABLE_LIVE",
        "VOX_SCHOLARLY_DISABLE_ZENODO",
    ] {
        env.remove(key);
    }

    let manifest = PublicationManifest {
        publication_id: "zenodo-mock-1".into(),
        content_type: "paper".into(),
        source_ref: None,
        title: "Mock Title".into(),
        author: "A U Thor".into(),
        abstract_text: Some("Abstract.".into()),
        body_markdown: "# Hello".into(),
        citations_json: None,
        metadata_json: None,
    };

    let receipt = scholarly::submit_with_adapter(&manifest, "zenodo")
        .await
        .expect("zenodo submit");
    assert_eq!(receipt.adapter, "zenodo");
    assert_eq!(receipt.external_submission_id, "424242");
    assert_eq!(receipt.status, "draft");

    let st = fetch_scholarly_remote_status_for_adapter("zenodo", "424242")
        .await
        .expect("status");
    assert_eq!(st.status, "done");
}

#[tokio::test]
async fn zenodo_create_deposition_retries_on_5xx_then_succeeds() {
    let _lock = ZENODO_ENV_LOCK.lock().expect("env lock");

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let port = addr.port();
    let base = format!("http://127.0.0.1:{port}/api");
    let bucket_ok = format!("http://127.0.0.1:{port}/api/deposit/depositions/9001/files");

    let post_count = Arc::new(AtomicU32::new(0));
    let post_count_cb = post_count.clone();
    let bucket_for_ok = bucket_ok.clone();
    let app = Router::new()
        .route(
            "/api/deposit/depositions",
            post(move |Json(body): Json<serde_json::Value>| {
                let bucket_for_ok = bucket_for_ok.clone();
                async move {
                    assert!(body.get("metadata").is_some());
                    let n = post_count_cb.fetch_add(1, Ordering::SeqCst);
                    if n < 2 {
                        (
                            StatusCode::SERVICE_UNAVAILABLE,
                            Json(json!({ "message": "temporary" })),
                        )
                            .into_response()
                    } else {
                        (
                            StatusCode::OK,
                            Json(json!({
                                "id": 9001,
                                "state": "draft",
                                "links": { "bucket": bucket_for_ok }
                            })),
                        )
                            .into_response()
                    }
                }
            }),
        )
        .route(
            "/api/deposit/depositions/{id}",
            get(|Path(id): Path<String>| async move {
                assert_eq!(id, "9001");
                Json(json!({ "id": 9001, "state": "done" }))
            }),
        );

    let _guard = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    wait_for_local_server(addr, "zenodo mock retries").await;

    let mut env = EnvRestore::new();
    env.set("VOX_ZENODO_API_BASE", &base);
    env.set("ZENODO_ACCESS_TOKEN", "mock-token");
    env.set("VOX_ZENODO_HTTP_MAX_ATTEMPTS", "4");
    for key in [
        "VOX_SCHOLARLY_DISABLE",
        "VOX_SCHOLARLY_DISABLE_LIVE",
        "VOX_SCHOLARLY_DISABLE_ZENODO",
    ] {
        env.remove(key);
    }

    let manifest = PublicationManifest {
        publication_id: "zenodo-retry-1".into(),
        content_type: "paper".into(),
        source_ref: None,
        title: "Retry Title".into(),
        author: "A U Thor".into(),
        abstract_text: Some("Abstract.".into()),
        body_markdown: "# Hello".into(),
        citations_json: None,
        metadata_json: None,
    };

    let receipt = scholarly::submit_with_adapter(&manifest, "zenodo")
        .await
        .expect("zenodo submit after retries");
    assert_eq!(receipt.external_submission_id, "9001");
    assert!(post_count.load(Ordering::SeqCst) >= 3);

    let st = fetch_scholarly_remote_status_for_adapter("zenodo", "9001")
        .await
        .expect("status");
    assert_eq!(st.status, "done");
}

#[tokio::test]
async fn zenodo_attach_body_and_publish_mock() {
    let _lock = ZENODO_ENV_LOCK.lock().expect("env lock");

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let port = addr.port();
    let base = format!("http://127.0.0.1:{port}/api");
    let bucket_for_deposit = format!("http://127.0.0.1:{port}/api/files/bucket-mock");

    let app = Router::new()
        .route(
            "/api/deposit/depositions",
            post(move |Json(body): Json<serde_json::Value>| {
                let bucket = bucket_for_deposit.clone();
                async move {
                    assert!(body.get("metadata").is_some());
                    Json(json!({
                        "id": 701,
                        "state": "draft",
                        "links": { "bucket": bucket }
                    }))
                }
            }),
        )
        .route(
            "/api/files/bucket-mock/body.md",
            put(|body: Bytes| async move {
                assert_eq!(body.as_ref(), b"# Handoff markdown\n");
                StatusCode::OK
            }),
        )
        .route(
            "/api/deposit/depositions/701/actions/publish",
            post(|| async { Json(json!({ "id": 701, "state": "published" })) }),
        )
        .route(
            "/api/deposit/depositions/701",
            get(|| async { Json(json!({ "id": 701, "state": "published" })) }),
        );

    let _guard = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    wait_for_local_server(addr, "zenodo publish mock").await;

    let mut env = EnvRestore::new();
    env.set("VOX_ZENODO_API_BASE", &base);
    env.set("ZENODO_ACCESS_TOKEN", "mock-token");
    env.set("VOX_ZENODO_ATTACH_MANIFEST_BODY", "1");
    env.set("VOX_ZENODO_PUBLISH_DEPOSITION", "1");
    for key in [
        "VOX_SCHOLARLY_DISABLE",
        "VOX_SCHOLARLY_DISABLE_LIVE",
        "VOX_SCHOLARLY_DISABLE_ZENODO",
    ] {
        env.remove(key);
    }

    let manifest = PublicationManifest {
        publication_id: "zenodo-publish-mock-1".into(),
        content_type: "paper".into(),
        source_ref: None,
        title: "Pub Mock".into(),
        author: "A U Thor".into(),
        abstract_text: Some("Abstract.".into()),
        body_markdown: "# Handoff markdown\n".into(),
        citations_json: None,
        metadata_json: None,
    };

    let receipt = scholarly::submit_with_adapter(&manifest, "zenodo")
        .await
        .expect("zenodo submit with attach+publish");
    assert_eq!(receipt.external_submission_id, "701");
    assert_eq!(receipt.status, "published");

    let st = fetch_scholarly_remote_status_for_adapter("zenodo", "701")
        .await
        .expect("status");
    assert_eq!(st.status, "published");
}
