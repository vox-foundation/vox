//! Task 8d: `SearchPolicy::web_research_enabled` is the seam `run_retrieval_bundle`
//! uses to suppress the chat preamble's web leg on a Quick/Deep research turn
//! (its numbered sources are that turn's web evidence — see
//! `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs`). This test
//! exercises the seam directly at the `execute_search_plan` level it gates,
//! with an isolated wiremock SearXNG and no other web provider enabled, so a
//! nonzero/zero request count on the mock is an unambiguous, deterministic
//! signal — not a hope that `heuristic_search_plan` picked a particular corpus.

// Task 9c: no real web host from any test (see tests/common/mod.rs).
mod common;

use std::path::PathBuf;

use vox_db::heuristic_search_plan;
use vox_search::context::SearchRuntimeContext;
use vox_search::execute_search_plan;
use vox_search::policy::SearchPolicy;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn ctx() -> SearchRuntimeContext {
    SearchRuntimeContext::new(
        PathBuf::from("."),
        None,
        PathBuf::from("/tmp/vox-web-leg-toggle-test/log"),
        PathBuf::from("/tmp/vox-web-leg-toggle-test/lt.md"),
    )
}

fn searxng_only_policy(searxng_url: &str) -> SearchPolicy {
    SearchPolicy {
        // Isolate to a single, mocked web provider so a request/no-request
        // observation on it is unambiguous.
        searxng_url: Some(searxng_url.to_string()),
        enable_wikipedia: false,
        enable_openalex: false,
        enable_arxiv: false,
        tavily_enabled: false,
        duckduckgo_fallback_enabled: false,
        // Generous, non-racy deadline — no artificial mock delay is used below.
        deep_timeout_ms: 10_000,
        fast_timeout_ms: 10_000,
        ..Default::default()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn web_research_enabled_false_skips_the_web_leg_entirely() {
    let searxng = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{
                "url": "https://example.com/gemini-3-8-flash",
                "title": "Gemini 3.8 Flash",
                "content": "Gemini 3.8 Flash release notes",
                "engine": "test",
            }],
        })))
        .mount(&searxng)
        .await;

    let query = "what is the latest gemini flash model and when was it released";
    let plan = heuristic_search_plan(query, false, None);
    assert!(
        plan.corpora.contains(&vox_db::SearchCorpus::WebResearch),
        "sanity: this query's heuristic plan must select the web corpus, {plan:?}"
    );

    // Mutation guard: with the gate removed (`web_research_enabled` absent, or
    // ignored by `execute_search_plan`), this assertion is the one that fails —
    // the disabled-policy run would still hit the mock.
    let mut disabled = searxng_only_policy(&searxng.uri());
    disabled.web_research_enabled = false;
    let exec_disabled = execute_search_plan(&ctx(), query, &plan, 5, &disabled, None)
        .await
        .expect("execute_search_plan (disabled)");
    assert_eq!(
        searxng
            .received_requests()
            .await
            .expect("mock requests")
            .len(),
        0,
        "web_research_enabled=false must not call the web provider at all"
    );
    assert!(
        exec_disabled.rrf_fused_lines.is_empty()
            || !exec_disabled
                .rrf_fused_lines
                .iter()
                .any(|l| l.contains("web:")),
        "no web hits should appear in fused output when the web leg is disabled: {:?}",
        exec_disabled.rrf_fused_lines
    );

    // Control: the same plan/query with the leg enabled (the default for every
    // other caller) does reach the mock — proves the disabled case above is a
    // real suppression, not an artifact of the plan/query never selecting web.
    let mut enabled = searxng_only_policy(&searxng.uri());
    enabled.web_research_enabled = true;
    execute_search_plan(&ctx(), query, &plan, 5, &enabled, None)
        .await
        .expect("execute_search_plan (enabled)");
    assert_eq!(
        searxng
            .received_requests()
            .await
            .expect("mock requests")
            .len(),
        1,
        "web_research_enabled=true (unchanged default) must still call the web provider"
    );
}

/// Task 9b r2: `web_research_enabled = false` is honoured by the dispatcher
/// itself — the one path every direct caller shares (the deep research
/// pipeline's `ProviderRegistry`, chat quick research, autonomous research) —
/// not only by `execute_search_plan`. No provider is contacted, and the report
/// says so honestly: every provider row is `Disabled`, none is hidden.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dispatcher_honours_web_research_disabled_with_honest_disabled_rows() {
    use vox_search::policy::ResearchLane;
    use vox_search::web_dispatcher::{ProviderStatus, WebSearchDispatcher};

    let searxng = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{"url": "https://example.org/a", "title": "t", "content": "c", "engine": "e", "score": 1.0}]
        })))
        .mount(&searxng)
        .await;
    let wiki = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/w/api.php"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&wiki)
        .await;

    let mut policy = searxng_only_policy(&searxng.uri());
    policy.enable_wikipedia = true;
    policy.wikipedia_api_url = Some(format!("{}/w/api.php", wiki.uri()));
    policy.web_research_enabled = false;

    let r =
        WebSearchDispatcher::search_with_report("gemini flash", ResearchLane::Deep, &policy).await;

    assert!(r.hits.is_empty());
    assert_eq!(searxng.received_requests().await.unwrap().len(), 0);
    assert_eq!(wiki.received_requests().await.unwrap().len(), 0);
    let rows: Vec<(&str, &ProviderStatus)> = r
        .providers
        .iter()
        .map(|p| (p.provider, &p.status))
        .collect();
    assert_eq!(rows.len(), 5, "every provider is reported: {rows:?}");
    assert!(
        rows.iter().all(|(_, s)| **s == ProviderStatus::Disabled),
        "{rows:?}"
    );
    assert_eq!(r.tavily_credits, None);
}

/// Task 9b follow-up: the kill-switch report lists exactly the providers the
/// normal path reports, from one shared list — a new provider cannot silently
/// drop out of the disabled report.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disabled_report_lists_the_same_providers_as_the_normal_path() {
    use vox_search::policy::ResearchLane;
    use vox_search::web_dispatcher::{WEB_PROVIDERS, WebSearchDispatcher};

    // Normal path, every provider individually off: no network, full report.
    let mut policy = SearchPolicy {
        searxng_url: None,
        enable_wikipedia: false,
        wikipedia_fallback_enabled: false,
        enable_openalex: false,
        enable_arxiv: false,
        tavily_enabled: false,
        duckduckgo_fallback_enabled: false,
        ..Default::default()
    };
    let normal = WebSearchDispatcher::search_with_report("q", ResearchLane::Deep, &policy).await;
    policy.web_research_enabled = false;
    let killed = WebSearchDispatcher::search_with_report("q", ResearchLane::Deep, &policy).await;

    let names = |r: &vox_search::web_dispatcher::SearchReport| {
        r.providers.iter().map(|p| p.provider).collect::<Vec<_>>()
    };
    assert_eq!(names(&normal), WEB_PROVIDERS.to_vec());
    assert_eq!(names(&killed), WEB_PROVIDERS.to_vec());
}
