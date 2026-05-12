use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRoutingConfig {
    #[serde(default)]
    pub latency_bands: LatencyBands,
    #[serde(default)]
    pub exploration: ExplorationConfig,
    #[serde(default)]
    pub safety: SafetyConfig,
    #[serde(default)]
    pub premium_alias: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyBands {
    pub excellent_ms: f64,
    pub poor_ms: f64,
}

impl Default for LatencyBands {
    fn default() -> Self {
        Self {
            excellent_ms: 500.0,
            poor_ms: 8000.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorationConfig {
    pub budget_usd_per_day: f64,
}

impl Default for ExplorationConfig {
    fn default() -> Self {
        Self {
            budget_usd_per_day: 50.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    pub max_cost_usd_per_request: f64,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            max_cost_usd_per_request: 5.0,
        }
    }
}

// Load the embedded YAML
pub fn load_model_routing_config() -> ModelRoutingConfig {
    let yaml = include_str!("../../../contracts/orchestration/model-routing.v1.yaml");
    serde_yaml::from_str(yaml).unwrap_or_else(|e| {
        tracing::error!("Failed to parse model-routing.v1.yaml: {}", e);
        // Return default values as fallback
        ModelRoutingConfig {
            latency_bands: LatencyBands::default(),
            exploration: ExplorationConfig::default(),
            safety: SafetyConfig::default(),
            premium_alias: HashMap::new(),
        }
    })
}
