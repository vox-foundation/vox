//! OpenReview scholarly adapter against a local mock HTTP server (`VOX_OPENREVIEW_API_BASE` / `OPENREVIEW_API_BASE`).
#![allow(clippy::await_holding_lock)] // Tests serialize env + mock via std mutex; guard held for whole async body.

mod common;

use std::sync::Mutex;

use common::wait_for_local_server;

use axum::{
    Json, Router,
    extract::Query,
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::json;
use tokio::net::TcpListener;
use vox_publisher::publication::PublicationManifest;
use vox_publisher::scholarly::{self, fetch_scholarly_remote_status_for_adapter};

static OPENREVIEW_ENV_LOCK: Mutex<()> = Mutex::new(());

fn swap_env(key: &str, val: Option<&str>) -> Option<String> {
    let prev = std::env::var(key).ok();
    // SAFETY: env mutation isolated while holding `OPENREVIEW_ENV_LOCK` for the full test.
    unsafe {
        match val {
            Some(v) => unsafe { std::env::set_var(key, v) },
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
                    Some(v) => unsafe { std::env::set_var(k, v) },
                    None => std::env::remove_var(k),
                }
            }
        }
    }
}

#[derive(Deserialize)]
struct NotesQuery {
    id: String,
}

#[tokio::test]
async fn openreview_adapter_submit_and_status_use_api_base_override() {
    let _lock = OPENREVIEW_ENV_LOCK.lock().expect("env lock");

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base = format!("http://{}", addr);

    let app = Router::new()
        .route(
            "/notes/edits",
            post(|Json(body): Json<serde_json::Value>| async move {
                assert!(body.get("invitation").is_some());
                assert!(body.get("content").is_some());
                Json(json!({ "note": { "id": "or-mock-note-7" } }))
            }),
        )
        .route(
            "/notes",
            get(|Query(q): Query<NotesQuery>| async move {
                assert_eq!(q.id, "or-mock-note-7");
                Json(json!({ "notes": [{ "id": "or-mock-note-7", "state": "active" }] }))
            }),
        );

    let _guard = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    wait_for_local_server(addr, "openreview mock").await;

    let mut env = EnvRestore::new();
    env.set("VOX_OPENREVIEW_API_BASE", &base);
    env.set("OPENREVIEW_ACCESS_TOKEN", "mock-bearer");
    for key in [
        "VOX_SCHOLARLY_DISABLE",
        "VOX_SCHOLARLY_DISABLE_LIVE",
        "VOX_SCHOLARLY_DISABLE_OPENREVIEW",
    ] {
        env.remove(key);
    }

    let or_meta = serde_json::json!({
        "openreview": {
            "invitation": "TestVenue/-/Submission",
            "signature": "~Test_Signature1",
            "readers": ["everyone"]
        }
    });
    let manifest = PublicationManifest {
        publication_id: "or-mock-1".into(),
        content_type: "paper".into(),
        source_ref: None,
        title: "OR Mock Title".into(),
        author: "A U Thor".into(),
        abstract_text: Some("Abstract.".into()),
        body_markdown: "# Body".into(),
        citations_json: None,
        metadata_json: Some(or_meta.to_string()),
    };

    let receipt = scholarly::submit_with_adapter(&manifest, "openreview")
        .await
        .expect("openreview submit");
    assert_eq!(receipt.adapter, "openreview");
    assert_eq!(receipt.external_submission_id, "or-mock-note-7");

    let st = fetch_scholarly_remote_status_for_adapter("openreview", "or-mock-note-7")
        .await
        .expect("status");
    assert_eq!(st.status, "active");
}
