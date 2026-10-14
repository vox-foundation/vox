//! CR-01 / WR-08 through the real MCP retrieval bundle: with Tavily configured, the unprompted
//! chat preamble never fires a Tavily request and never carries a Tavily line; an explicit query
//! does, labelled `[tier:untrusted]`, and the envelope minimum drops to Untrusted.
//!
//! One test fn on purpose: it mutates process env, so it must not race a sibling in this binary.

mod common;

use std::sync::Arc;

use vox_db::{ContextTrustTier, VoxDb};
use vox_orchestrator_mcp::ServerState;
use vox_orchestrator_mcp::memory_tools::{RetrievalTriggerMode, run_retrieval_bundle};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// Nonsense tokens: no memory, KB or repo-inventory hit may suppress fire-on-empty.
const QUERY: &str = "zzqxvwk plorbnax qjzxy";
const MARKER: &str = "tavilymarkerbody";

#[tokio::test(flavor = "multi_thread")]
async fn kb_tavily_preamble_never_fires_and_explicit_is_labelled_untrusted() {
    let tavily = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "query": "q", "answer": null, "images": [], "response_time": 0.1,
            "results": [{"title": "Tavily page", "url": "https://example.test/t",
                "content": MARKER, "score": 0.9}],
        })))
        .mount(&tavily)
        .await;
    // SAFETY: single test fn in its own binary; set before any secret is resolved. The key is a
    // fake value aimed at a loopback mock.
    #[allow(unsafe_code)]
    unsafe {
        std::env::set_var("TAVILY_API_KEY", "tvly-test");
        std::env::set_var("VOX_SEARCH_TAVILY_URL", tavily.uri());
        std::env::set_var("VOX_SEARCH_TAVILY_ON_EMPTY", "1");
        std::env::set_var("VOX_SEARCH_PREFER_RRF", "1");
    }
    vox_config::snapshot::bump(&[
        "TAVILY_API_KEY",
        "VOX_SEARCH_TAVILY_URL",
        "VOX_SEARCH_TAVILY_ON_EMPTY",
        "VOX_SEARCH_PREFER_RRF",
    ]);

    let db = Arc::new(VoxDb::open_memory().await.unwrap());
    let mut state = ServerState::new_test().await.with_db_initialized(db).await;
    // Empty memory dir so no memory hit suppresses fire-on-empty.
    let mem = tempfile::tempdir().unwrap();
    state.orchestrator_config.memory =
        vox_orchestrator::MemoryConfig::for_account("kb-tavily", mem.path());

    // 1. Unprompted preamble: no request, no Tavily text anywhere, envelope not lowered by it.
    let pre = run_retrieval_bundle(
        &state,
        QUERY,
        RetrievalTriggerMode::AutoChatPreamble,
        20,
        None,
        true,
    )
    .await
    .expect("preamble bundle");
    assert!(
        tavily.received_requests().await.unwrap().is_empty(),
        "Tavily fired for the auto preamble"
    );
    for l in pre
        .memory_lines
        .iter()
        .chain(&pre.knowledge_lines)
        .chain(&pre.chunk_lines)
        .chain(&pre.repo_lines)
        .chain(&pre.rrf_fused_lines)
    {
        assert!(
            !l.contains("crag_tavily") && !l.contains(MARKER),
            "Tavily text reached the preamble: {l}"
        );
    }
    // Nothing was kept, so there is no tier, which the envelope reads as Untrusted.
    assert_eq!(pre.evidence.min_trust_tier, None);
    assert_eq!(
        pre.evidence
            .to_context_envelope("repo", None)
            .trust
            .trust_tier,
        ContextTrustTier::Untrusted
    );

    // 2. Explicit query: Tavily fires, line is labelled, envelope minimum is Untrusted.
    let ex = run_retrieval_bundle(
        &state,
        QUERY,
        RetrievalTriggerMode::ExplicitToolQuery,
        20,
        None,
        true,
    )
    .await
    .expect("explicit bundle");
    assert!(
        !tavily.received_requests().await.unwrap().is_empty(),
        "explicit query should reach Tavily"
    );
    assert!(
        ex.rrf_fused_lines
            .iter()
            .any(|l| l.starts_with("[tier:untrusted] [crag_tavily:")),
        "{:?}",
        ex.rrf_fused_lines
    );
    assert_eq!(
        ex.evidence.min_trust_tier,
        Some(ContextTrustTier::Untrusted)
    );
}
