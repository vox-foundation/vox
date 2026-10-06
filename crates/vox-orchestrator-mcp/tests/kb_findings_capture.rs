//! Web hit -> pending Untrusted finding end to end, plus capture hardening (TRUST-02/04/05).

mod common;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use vox_db::kb_text::{FindingOrigin, FindingStatus};
use vox_db::{ContextTrustTier, KbFindingRow, VoxDb, heuristic_search_plan};
use vox_orchestrator_mcp::ServerState;
use vox_orchestrator_mcp::kb_tools::findings::{
    CaptureInput, McpFindingCapture, capture_finding, web_capture_for_state,
};
use vox_search::context::SearchRuntimeContext;
use vox_search::execute_search_plan;
use vox_search::policy::SearchPolicy;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const QUERY: &str = "what is the latest gemini flash model and when was it released";

fn input(text: &str, url: &str) -> CaptureInput {
    CaptureInput {
        origin: FindingOrigin::WebSearch,
        title: "A title".into(),
        text: text.into(),
        full_text: None,
        source_url: Some(url.into()),
        dedupe_scope: None,
        provenance: serde_json::json!({"engine": "test"}),
    }
}

async fn rows(db: &VoxDb) -> Vec<KbFindingRow> {
    db.kb_findings_list(None, 100).await.unwrap()
}

async fn count(db: &VoxDb, table: &str) -> i64 {
    let mut r = db
        .connection()
        .query(&format!("SELECT COUNT(*) FROM {table}"), ())
        .await
        .unwrap();
    r.next().await.unwrap().unwrap().get::<i64>(0).unwrap()
}

fn snapshot_files(root: &Path) -> Vec<String> {
    match std::fs::read_dir(root.join(".vox/cache/kb-findings")) {
        Ok(d) => d
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect(),
        Err(_) => vec![],
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn capture_web_leg_end_to_end() {
    let mock = MockServer::start().await;
    let hit_url = format!("{}/gemini-3-8-flash", mock.uri());
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{"url": hit_url, "title": "Gemini 3.8 Flash",
                "content": "Gemini 3.8 Flash release notes", "engine": "test"}],
        })))
        .mount(&mock)
        .await;
    let db = Arc::new(VoxDb::open_memory().await.unwrap());
    let tmp = tempfile::tempdir().unwrap();
    let ctx = SearchRuntimeContext::new(
        tmp.path().to_path_buf(),
        Some(db.clone()),
        tmp.path().join("log"),
        tmp.path().join("lt.md"),
    )
    .with_web_capture(Some(Arc::new(McpFindingCapture {
        db: db.clone(),
        repo_root: tmp.path().to_path_buf(),
    })));
    let policy = SearchPolicy {
        searxng_url: Some(mock.uri()),
        enable_wikipedia: false,
        enable_openalex: false,
        enable_arxiv: false,
        tavily_enabled: false,
        duckduckgo_fallback_enabled: false,
        persist_web_hits: true,
        deep_timeout_ms: 10_000,
        fast_timeout_ms: 10_000,
        ..Default::default()
    };
    let plan = heuristic_search_plan(QUERY, false, None);
    execute_search_plan(&ctx, QUERY, &plan, 5, &policy, None)
        .await
        .unwrap();

    let mut found = vec![];
    for _ in 0..50 {
        found = rows(&db).await;
        if !found.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(found.len(), 1, "{found:?}");
    let r = &found[0];
    assert_eq!(r.status, FindingStatus::Pending);
    assert_eq!(r.trust_tier, ContextTrustTier::Untrusted);
    assert_eq!(r.origin, FindingOrigin::WebSearch);
    assert!(!r.summary.is_empty());
    // The loopback mock URL carries an IPv4 literal, which capture redacts by design.
    assert_eq!(r.source_url, Some(vox_redact::redact_owned(&hit_url)));
    assert!(r.fetched_at_ms > 0);
    let hex = r
        .content_hash
        .strip_prefix("blake3:")
        .expect("blake3: prefix");
    assert_eq!(hex.len(), 64);
    assert!(
        tmp.path()
            .join(format!(".vox/cache/kb-findings/{hex}.txt"))
            .exists()
    );
    assert_eq!(count(&db, "search_documents").await, 0);
    assert_eq!(count(&db, "search_document_chunks").await, 0);
}

#[tokio::test]
async fn capture_web_capture_for_state_needs_db() {
    let none = ServerState::new_test().await;
    assert!(web_capture_for_state(&none).is_none());
    let db = Arc::new(VoxDb::open_memory().await.unwrap());
    let some = ServerState::new_test().await.with_db_initialized(db).await;
    assert!(web_capture_for_state(&some).is_some());
}

#[tokio::test]
async fn capture_empty_input_is_rejected_without_writes() {
    let db = VoxDb::open_memory().await.unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let err = capture_finding(&db, tmp.path(), input("  \n\t", "https://e.x/a"))
        .await
        .unwrap_err();
    assert!(err.contains("empty"), "{err}");
    assert!(rows(&db).await.is_empty());
    assert!(!tmp.path().join(".vox").exists());
    assert!(snapshot_files(tmp.path()).is_empty());

    let mut full_only = input("", "https://e.x/a");
    full_only.full_text = Some("Body sentence one. Two.".into());
    let out = capture_finding(&db, tmp.path(), full_only).await.unwrap();
    assert!(out.finding_id.is_some());
    assert_eq!(snapshot_files(tmp.path()).len(), 1);
    let row = &rows(&db).await[0];
    assert!(row.summary.starts_with("Body sentence one."));
    assert_eq!(row.source_url.as_deref(), Some("https://e.x/a"));
}
