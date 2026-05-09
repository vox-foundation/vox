//! Query planner. Phase 0a STUB — returns the input query as a single subquery.
//!
//! Phase 1 wires this to a SciClaims-style local Mens model via
//! [`vox-actor-runtime`]; Phase 2 adds prereg enforcement so a campaign
//! without a signed prereg cannot reach this stage.

use anyhow::Result;
use serde_json::Value;

use super::types::{ResearchPlan, ResearchQuery};

/// Decompose a research query into a plan with at least one subquery.
///
/// **PHASE_0a_STUB**: returns a plan with the original query as the only subquery.
pub async fn decompose_query_with_config(
    query: &ResearchQuery,
    _endpoint: Option<&str>,
    _api_key: Option<&str>,
    _model: Option<&str>,
    _temperature: Option<f32>,
    _max_subqueries: Option<usize>,
) -> Result<ResearchPlan> {
    // PHASE_0a_STUB: passthrough. Phase 1 invokes Mens for real decomposition.
    Ok(ResearchPlan {
        original_query: query.query.clone(),
        subqueries: vec![query.query.clone()],
        scope: query.scope.clone(),
        max_sources_per_subquery: query.max_sources,
    })
}

/// Serialize a plan to a JSON value for telemetry persistence.
#[must_use]
pub fn plan_to_json(plan: &ResearchPlan) -> Value {
    serde_json::to_value(plan).unwrap_or_else(|_| Value::Null)
}
