use vox_db::Codex;

use super::config::ResearchConfig;
use super::super::provider::ProviderRegistry;
use super::super::types::{ResearchHit, ResearchPlan, ResearchQuery};

pub(super) async fn gather_web_hits_for_plan(
    _db: Option<&Codex>,
    _session_id: i64,
    query: &ResearchQuery,
    plan: &ResearchPlan,
    registry: &ProviderRegistry,
    _config: &ResearchConfig,
) -> (
    Vec<ResearchHit>,
    usize,
    usize,
    usize,
) {
    // PHASE_0a_STUB: no real web provider; returns empty results.
    // Phase 5 replaces with real provider search/crawl/extract pipeline.
    // DB writes (create_research_source, ingest_research_document, start_provider_run,
    // finish_provider_run) are also deferred to Phase 1 after vox_db gains those methods.
    let _ = query;
    let _ = plan;
    let _ = registry;
    (Vec::new(), 0, 0, 0)
}
