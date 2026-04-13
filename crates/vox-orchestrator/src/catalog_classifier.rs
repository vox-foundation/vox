use crate::models::ModelSpec;
use serde::{Deserialize, Serialize};


#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
struct ClassificationResponse {
    /// True if the provider is highly reliable, maintains uptime > 99%, and supports the model cleanly.
    is_stable: bool,
    /// Number between 0.0 and 1.0 expressing uptime/health.
    uptime_score: f32,
    /// Refined list of strengths based on meta-analysis.
    #[serde(default)]
    refined_strengths: Vec<String>,
}

/// Applies meta-model classification to dynamically tag model strengths and populate uptime_score.
/// In a real implementation, this would use a fast, cheap model (e.g. Haiku or Flash) to review
/// the incoming catalog metadata and refine the `strengths` array, acting as an AI orchestrator.
/// For now, we simulate this layer by enriching known missing data fields with heuristic health checks.
pub async fn classify_models(models: &mut [ModelSpec]) {
    // If the user explicitly disabled the classifier, no-op.
    if vox_clavis::resolve_secret(vox_clavis::SecretId::VoxOpenRouterClassifierEnabled)
        .expose()
        .unwrap_or("1")
        == "0"
    {
        return;
    }

    // Simulate API batch processing: in a real implementation we would send batch requests
    // to `OpenRouter/auto` asking an LLM to evaluate the metadata of `models` and return JSON.
    // Here we inject an uptime score based on the provider string as a stand-in for the classifier.

    for m in models.iter_mut() {
        // Only classify if it doesn't already have an uptime score from the catalog.
        if m.capabilities.uptime_score.is_none() {
            // Apply heuristic meta-tagging for uptime.
            let health = match m.provider.as_str() {
                "openai" | "anthropic" | "google" => 0.99,
                "deepseek" => 0.95,
                "openrouter" => 0.99,
                "groq" => 0.98,
                "together" => 0.97,
                _ => 0.85,
            };
            m.capabilities.uptime_score = Some(health);
        }

        // Meta-classification: if model is extremely large context, tag as 'long-context-analysis'
        if m.max_tokens >= 128_000 && !m.strengths.contains(&"long-context".to_string()) {
            m.strengths.push("long-context".to_string());
        }
    }
}
