//! TRUST-02: web hits never enter the searchable corpus; they reach an injected
//! `WebFindingCapture` sink or nowhere at all (fail-closed).

mod common;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use vox_db::VoxDb;
use vox_db::heuristic_search_plan;
use vox_db::kb_text::FindingOrigin;
use vox_search::capture::{CapturedWebHit, WebFindingCapture};
use vox_search::context::SearchRuntimeContext;
use vox_search::execute_search_plan;
use vox_search::policy::SearchPolicy;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const QUERY: &str = "what is the latest gemini flash model and when was it released";
const HIT_URL: &str = "https://example.com/gemini-3-8-flash";

#[derive(Default)]
struct Recorder(Mutex<Vec<CapturedWebHit>>);

impl WebFindingCapture for Recorder {
    fn capture(&self, hits: Vec<CapturedWebHit>) {
        self.0.lock().unwrap().extend(hits);
    }
}

fn policy(searxng_url: &str, persist: bool) -> SearchPolicy {
    SearchPolicy {
        searxng_url: Some(searxng_url.to_string()),
        enable_wikipedia: false,
        enable_openalex: false,
        enable_arxiv: false,
        tavily_enabled: false,
        duckduckgo_fallback_enabled: false,
        persist_web_hits: persist,
        deep_timeout_ms: 10_000,
        fast_timeout_ms: 10_000,
        ..Default::default()
    }
}

async fn searxng() -> MockServer {
    let s = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{
                "url": HIT_URL,
                "title": "Gemini 3.8 Flash",
                "content": "Gemini 3.8 Flash release notes",
                "engine": "test",
            }],
        })))
        .mount(&s)
        .await;
    s
}

async fn count(db: &VoxDb, table: &str) -> i64 {
    let mut rows = db
        .connection()
        .query(&format!("SELECT COUNT(*) FROM {table}"), ())
        .await
        .unwrap();
    rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap()
}

async fn run(persist: bool, sink: Option<Arc<dyn WebFindingCapture>>) -> (Arc<VoxDb>, MockServer) {
    let mock = searxng().await;
    let db = Arc::new(VoxDb::open_memory().await.unwrap());
    let ctx = SearchRuntimeContext::new(
        PathBuf::from("."),
        Some(db.clone()),
        PathBuf::from("/tmp/vox-web-capture-test/log"),
        PathBuf::from("/tmp/vox-web-capture-test/lt.md"),
    )
    .with_web_capture(sink);
    let plan = heuristic_search_plan(QUERY, false, None);
    execute_search_plan(&ctx, QUERY, &plan, 5, &policy(&mock.uri(), persist), None)
        .await
        .expect("execute_search_plan");
    // Bounded wait for any (wrongly) spawned corpus writes to land.
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    (db, mock)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn capture_no_sink_no_writes() {
    let (db, mock) = run(true, None).await;
    assert_eq!(
        mock.received_requests().await.unwrap().len(),
        1,
        "web leg must have run"
    );
    assert_eq!(count(&db, "search_documents").await, 0);
    assert_eq!(count(&db, "search_document_chunks").await, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn capture_sink_receives_hit_and_corpus_stays_empty() {
    let rec = Arc::new(Recorder::default());
    let (db, _mock) = run(true, Some(rec.clone())).await;
    let got = rec.0.lock().unwrap().clone();
    assert_eq!(got.len(), 1, "{got:?}");
    let h = &got[0];
    assert_eq!(h.url, HIT_URL);
    assert_eq!(h.title, "Gemini 3.8 Flash");
    assert_eq!(h.snippet, "Gemini 3.8 Flash release notes");
    assert_eq!(h.engine, "test");
    assert_eq!(h.query, QUERY);
    assert_eq!(h.origin, FindingOrigin::WebSearch);
    assert_eq!(count(&db, "search_documents").await, 0);
    assert_eq!(count(&db, "search_document_chunks").await, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn capture_flag_off_sink_receives_nothing() {
    let rec = Arc::new(Recorder::default());
    let (_db, _mock) = run(false, Some(rec.clone())).await;
    assert!(rec.0.lock().unwrap().is_empty());
}
