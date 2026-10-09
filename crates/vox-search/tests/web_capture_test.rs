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
    assert_eq!(db.search_corpus_row_counts().await.unwrap(), (0, 0));
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
    assert_eq!(db.search_corpus_row_counts().await.unwrap(), (0, 0));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn capture_flag_off_sink_receives_nothing() {
    let rec = Arc::new(Recorder::default());
    let (_db, _mock) = run(false, Some(rec.clone())).await;
    assert!(rec.0.lock().unwrap().is_empty());
}

/// CRAG branch: a Tavily hit reaches the sink as `CragTavily` with full text and writes no
/// corpus row, and no `tavily:<url>` durable artifact is advertised for a resource that no
/// longer exists.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn capture_crag_tavily_reaches_sink_not_corpus() {
    use vox_search::bundle::{RetrievalTriggerMode, run_search_with_verification};
    use vox_search::tavily_budget::TavilySessionBudget;

    let tavily = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "query": "q", "answer": null, "images": [], "response_time": 0.1,
            "results": [{"title": "Tavily page", "url": "https://example.test/t",
                "content": "Tavily full content body.", "score": 0.9}],
        })))
        .mount(&tavily)
        .await;
    // SAFETY: nextest runs each test in its own process; set before any secret is resolved.
    #[allow(unsafe_code)]
    unsafe {
        std::env::set_var("TAVILY_API_KEY", "tvly-test");
    }
    vox_config::snapshot::bump(&["TAVILY_API_KEY"]);

    let tmp = tempfile::tempdir().unwrap();
    let db = Arc::new(VoxDb::open_memory().await.unwrap());
    let rec = Arc::new(Recorder::default());
    let ctx = SearchRuntimeContext::new(
        tmp.path().to_path_buf(),
        Some(db.clone()),
        tmp.path().join("log"),
        tmp.path().join("lt.md"),
    )
    .with_web_capture(Some(rec.clone()));
    let policy = SearchPolicy {
        web_research_enabled: false,
        tavily_enabled: true,
        tavily_api_url: Some(tavily.uri()),
        tavily_fire_on_empty: true,
        searxng_url: None,
        enable_wikipedia: false,
        enable_openalex: false,
        enable_arxiv: false,
        persist_web_hits: true,
        ..SearchPolicy::default()
    };
    let budget = TavilySessionBudget::new(100);
    let (exec, _diag, _plan) = run_search_with_verification(
        &ctx,
        "zzqx unmatched crag probe",
        RetrievalTriggerMode::ExplicitToolQuery,
        5,
        &policy,
        None,
        Some(&budget),
    )
    .await
    .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let got = rec.0.lock().unwrap().clone();
    let crag: Vec<_> = got
        .iter()
        .filter(|h| h.origin == FindingOrigin::CragTavily)
        .collect();
    assert_eq!(crag.len(), 1, "{got:?}");
    assert_eq!(crag[0].url, "https://example.test/t");
    assert_eq!(
        crag[0].full_text.as_deref(),
        Some("Tavily full content body.")
    );
    assert!(exec.durable_artifacts.is_empty());
    assert_eq!(db.search_corpus_row_counts().await.unwrap(), (0, 0));
    // TRUST-01: an admitted Tavily line is labelled and lowers the envelope minimum.
    assert!(
        exec.web_lines
            .iter()
            .any(|l| l.starts_with("[tier:untrusted] [crag_tavily:https://example.test/t]")),
        "{:?}",
        exec.web_lines
    );
    assert_eq!(
        exec.min_trust_tier,
        Some(vox_db::ContextTrustTier::Untrusted)
    );
}

/// CR-01 / WR-08: under `AutoChatPreamble` the Tavily leg does not fire at all, even when the
/// caller passes a plain policy (as `goal.rs` does): zero requests, zero credits, no capture.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn crag_tavily_never_fires_for_auto_preamble_without_the_flag() {
    use vox_search::bundle::{RetrievalTriggerMode, run_search_with_verification};
    use vox_search::tavily_budget::TavilySessionBudget;

    let tavily = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "query": "q", "answer": null, "images": [], "response_time": 0.1,
            "results": [{"title": "Tavily page", "url": "https://example.test/t",
                "content": "Tavily full content body.", "score": 0.9}],
        })))
        .mount(&tavily)
        .await;
    // SAFETY: set before any secret is resolved; the value is a fake key against loopback.
    #[allow(unsafe_code)]
    unsafe {
        std::env::set_var("TAVILY_API_KEY", "tvly-test");
    }
    vox_config::snapshot::bump(&["TAVILY_API_KEY"]);

    let tmp = tempfile::tempdir().unwrap();
    let db = Arc::new(VoxDb::open_memory().await.unwrap());
    let rec = Arc::new(Recorder::default());
    let ctx = SearchRuntimeContext::new(
        tmp.path().to_path_buf(),
        Some(db),
        tmp.path().join("log"),
        tmp.path().join("lt.md"),
    )
    .with_web_capture(Some(rec.clone()));
    let policy = SearchPolicy {
        web_research_enabled: false,
        tavily_enabled: true,
        tavily_api_url: Some(tavily.uri()),
        tavily_fire_on_empty: true,
        prefer_rrf_merge: true,
        searxng_url: None,
        enable_wikipedia: false,
        enable_openalex: false,
        enable_arxiv: false,
        persist_web_hits: true,
        exclude_untrusted_hits: false,
        ..SearchPolicy::default()
    };
    let budget = TavilySessionBudget::new(100);
    let (exec, diag, _plan) = run_search_with_verification(
        &ctx,
        "zzqx unmatched crag probe",
        RetrievalTriggerMode::AutoChatPreamble,
        5,
        &policy,
        None,
        Some(&budget),
    )
    .await
    .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    assert!(tavily.received_requests().await.unwrap().is_empty());
    assert!(!exec.web_lines.iter().any(|l| l.contains("crag_tavily")));
    assert!(
        !exec
            .rrf_fused_lines
            .iter()
            .any(|l| l.contains("crag_tavily"))
    );
    assert!(
        !rec.0
            .lock()
            .unwrap()
            .iter()
            .any(|h| h.origin == FindingOrigin::CragTavily)
    );
    assert_eq!(budget.remaining(), 100);
    assert!(
        diag.notes
            .iter()
            .any(|n| n == "tavily_skipped=untrusted_excluded"),
        "{:?}",
        diag.notes
    );
}
