//! Bounded JSON shapes for `routing_decisions.reason_json` (orchestrator dequeue path).
//!
//! Kept in `vox-runtime` so embedders share a stable contract without pulling MCP or Ludus.

use serde::Serialize;

/// Upper bound for `reason_json` SQLite text (orchestrator + MCP parity).
pub const ROUTING_REASON_JSON_MAX_BYTES: usize = 4096;

#[must_use]
pub fn unified_routing_rollout_enabled() -> bool {
    vox_config::env_parse::resolve_config_bool("VOX_UNIFIED_ROUTING", false)
}

/// Versioned payload written by [`crate::routing_telemetry::OrchestratorTaskRoutingReasonV1::to_json_bounded`].
#[derive(Debug, Clone, Serialize)]
pub struct OrchestratorTaskRoutingReasonV1 {
    pub schema_version: u32,
    pub task_category: String,
    pub estimated_complexity: u8,
    pub usage_provider: String,
    pub usage_model: String,
    pub registry_hit: bool,
    pub cost_preference: String,
    pub ludus_fallback: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub unified_routing_env: bool,
    pub route_policy_profile: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub policy_denials: Vec<String>,
    pub task_id: u64,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl OrchestratorTaskRoutingReasonV1 {
    #[must_use]
    pub fn new(
        task_category: String,
        estimated_complexity: u8,
        usage_provider: String,
        usage_model: String,
        registry_hit: bool,
        cost_preference: String,
        ludus_fallback: bool,
        unified_routing_env: bool,
        route_policy_profile: String,
        policy_denials: Vec<String>,
        task_id: u64,
    ) -> Self {
        Self {
            schema_version: 1,
            task_category,
            estimated_complexity,
            usage_provider,
            usage_model,
            registry_hit,
            cost_preference,
            ludus_fallback,
            unified_routing_env,
            route_policy_profile,
            policy_denials,
            task_id,
        }
    }

    /// Serialize to JSON, truncating to `max_bytes` (UTF-8 safe: may cut mid-rune at boundary).
    #[must_use]
    pub fn to_json_bounded(&self, max_bytes: usize) -> String {
        let Ok(mut s) = serde_json::to_string(self) else {
            return r#"{"schema_version":1,"error":"serialize"}"#.to_string();
        };
        if s.len() > max_bytes {
            s.truncate(max_bytes);
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_truncation() {
        let mut cat = "x".repeat(5000);
        cat.push_str("category");
        let r = OrchestratorTaskRoutingReasonV1::new(
            cat,
            3,
            "p".into(),
            "m".into(),
            true,
            "Low".into(),
            false,
            false,
            "balanced".to_string(),
            Vec::new(),
            42,
        );
        let j = r.to_json_bounded(200);
        assert!(j.len() <= 200);
    }
}
