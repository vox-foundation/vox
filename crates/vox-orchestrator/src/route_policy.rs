//! Routing capability policy alignment with `VOX_ROUTE_*` and `contracts/orchestration/model-routing.v1.yaml`.
//!
//! Shared by MCP model resolution and CLI/model explain surfaces.

use crate::models::{ModelSpec, ProviderType};
use vox_runtime::route_capability_policy::{
    RouteCapabilityPolicySnapshot, exclusion_reason_for_llm_lane,
};

#[inline]
fn is_local_http_provider(provider_type: &ProviderType) -> bool {
    matches!(
        provider_type,
        ProviderType::Ollama | ProviderType::PopuliMesh | ProviderType::VoxLocal
    )
}

/// Returns a static denial code when [`ModelSpec`] cannot be used under the current route policy.
#[must_use]
pub fn route_policy_exclusion_reason(model: &ModelSpec) -> Option<&'static str> {
    let snap = RouteCapabilityPolicySnapshot::from_env();
    exclusion_reason_for_llm_lane(is_local_http_provider(&model.provider_type), &snap)
}

#[must_use]
pub fn route_policy_allows_model(model: &ModelSpec) -> bool {
    route_policy_exclusion_reason(model).is_none()
}
