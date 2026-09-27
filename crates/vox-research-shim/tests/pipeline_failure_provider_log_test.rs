//! Task 9b: a deep run that FAILS still carries its per-provider log, so the
//! chat trace can show which provider failed — the case where the table
//! explains the failure. End-to-end through `run_research`, offline: wiremock
//! providers, Fast lane (no planner LLM), and this crate's tests build without
//! the `runtime` feature, so synthesis fails deterministically with no network.

use vox_research_shim::research::ResearchConfig;
use vox_research_shim::research::orchestrator::pipeline::run_research;
use vox_research_shim::research::types::{ResearchQuery, ResearchRunFailure, ResearchScope};
use vox_search::policy::ResearchLane;
use vox_search::web_dispatcher::ProviderStatus;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn query(text: &str) -> ResearchQuery {
    ResearchQuery {
        query: text.to_string(),
        scope: ResearchScope::Web,
        max_sources: 5,
        persist_to_docs: false,
        verify_claims: false,
        site_scope: None,
        domain_mode: Default::default(),
        waves: 1,
        lane: ResearchLane::Fast,
    }
}

/// Only openalex (→ `openalex`) and optionally wikipedia (→ `wiki`) are live.
fn config(openalex: &MockServer, wiki: Option<&MockServer>) -> ResearchConfig {
    let mut cfg = ResearchConfig::default();
    let p = &mut cfg.search_policy;
    p.fast_timeout_ms = 10_000;
    p.deep_timeout_ms = 10_000;
    p.duckduckgo_fallback_enabled = false;
    p.tavily_enabled = false;
    p.searxng_url = None;
    p.enable_arxiv = false;
    p.enable_openalex = true;
    p.openalex_api_url = Some(openalex.uri());
    match wiki {
        Some(w) => p.wikipedia_api_url = Some(format!("{}/w/api.php", w.uri())),
        None => {
            p.enable_wikipedia = false;
            p.wikipedia_fallback_enabled = false;
        }
    }
    cfg
}

async fn failing_openalex() -> MockServer {
    let s = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/works"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&s)
        .await;
    s
}

fn failure(err: &anyhow::Error) -> &ResearchRunFailure {
    err.downcast_ref::<ResearchRunFailure>()
        .unwrap_or_else(|| panic!("run error must carry the provider log: {err:#}"))
}

fn has_error_row(f: &ResearchRunFailure, provider: &str) -> bool {
    f.providers
        .iter()
        .any(|r| r.provider == provider && matches!(r.status, ProviderStatus::Error { .. }))
}

/// The wrapper must not change what CLI / MCP / chat callers print.
#[test]
fn run_failure_is_transparent_to_display_and_the_source_chain() {
    let make = || {
        anyhow::Error::new(std::io::Error::other("connection refused")).context("synthesis failed")
    };
    let wrapped = anyhow::Error::new(ResearchRunFailure {
        error: make(),
        providers: vec![],
        tavily_credits: None,
        sources: vec![],
    });
    assert_eq!(wrapped.to_string(), make().to_string());
    assert_eq!(format!("{wrapped:#}"), format!("{:#}", make()));
    assert_eq!(wrapped.chain().count(), make().chain().count());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn zero_hits_failure_keeps_the_failing_providers_row() {
    let openalex = failing_openalex().await;
    let err = run_research(
        query("zero hits provider log"),
        None,
        &config(&openalex, None),
    )
    .await
    .expect_err("no provider returned hits");

    // Callers see the same error text as before.
    assert!(
        err.to_string().contains("Zero research hits retrieved"),
        "{err}"
    );
    let f = failure(&err);
    assert!(has_error_row(f, "openalex"), "{:?}", f.providers);
    assert!(f.sources.is_empty());
}

/// Offline ONLY without `runtime`: then `chat_stage` bails before any network.
/// With `runtime` (on in any workspace-wide build — vox-cli and
/// vox-orchestrator-mcp enable it) synthesis runs the real model cascade:
/// `ResearchConfig::llm_endpoint` only PREPENDS a manual candidate
/// (`cascade_with_optional_manual`), the registry primary and the stage
/// cascade (OpenRouter, local Ollama, …) still follow, so no endpoint stub
/// can make it hermetic — it could make a paid call and even succeed.
#[cfg_attr(
    feature = "runtime",
    ignore = "needs a no-LLM build: with `runtime` synthesis would call real model providers"
)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn synthesis_failure_keeps_the_whole_provider_log() {
    let openalex = failing_openalex().await;
    let wiki = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/w/api.php"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "query": {"search": [{"title": "Gemini (language model)", "pageid": 7, "snippet": "Gemini Flash"}]}
        })))
        .mount(&wiki)
        .await;

    let err = run_research(query("gemini flash"), None, &config(&openalex, Some(&wiki)))
        .await
        .expect_err("synthesis has no LLM without the runtime feature");

    assert!(err.to_string().contains("synthesis failed"), "{err}");
    let f = failure(&err);
    assert!(has_error_row(f, "openalex"), "{:?}", f.providers);
    // The kept sources travel too, so the trace header is not "0 sources".
    assert_eq!(f.sources.len(), 1, "{:?}", f.sources);
    assert!(
        f.providers.iter().any(|r| r.provider == "wikipedia"
            && matches!(r.status, ProviderStatus::Ok { hits } if hits > 0)),
        "{:?}",
        f.providers
    );
}
