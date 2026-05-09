//! Phase 0a — the orphan tree compiles, run_research is callable, and the
//! full pipeline returns a coherent (empty) ResearchResult when called with
//! all stubs.

use vox_orchestrator::dei_shim::research::{
    run_research, ResearchConfig,
};
use vox_orchestrator::dei_shim::research::types::{ResearchQuery, ResearchScope};

#[tokio::test]
async fn run_research_with_stubs_returns_empty_result() {
    let query = ResearchQuery {
        query: "smoke test".into(),
        scope: ResearchScope::Both,
        max_sources: 3,
        persist_to_docs: false,
        verify_claims: false,
        site_scope: None,
    };
    let config = ResearchConfig::default();

    // No Codex handle → no DB writes; pure in-memory exercise.
    let result = run_research(query, None, &config).await.expect("succeeds");

    // Phase 0a expectations:
    //   - answer is non-fatal default (likely empty or a fallback string)
    //   - sources is empty (no real provider)
    //   - citations is empty (no sources)
    //   - claim_verdicts is empty (verifier stub returns Vec::new())
    //   - routing_tier is RoutingTier::Direct (gate stub: 0 citations → score 0)
    assert!(result.sources.is_empty());
    assert!(result.citations.is_empty());
    assert!(result.research_metadata.claim_verdicts.is_empty());
    assert!(matches!(
        result.research_metadata.routing_tier,
        vox_orchestrator::dei_shim::research::types::RoutingTier::Direct
    ));
}
