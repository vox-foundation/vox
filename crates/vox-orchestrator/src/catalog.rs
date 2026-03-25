use std::time::Duration;
use crate::models::{ModelCapabilities, ModelSpec, ProviderType};

#[async_trait::async_trait]
pub trait ModelCatalog: Send + Sync {
    /// Discovers and returns the list of models this catalog supports.
    async fn refresh(&self) -> Result<Vec<ModelSpec>, anyhow::Error>;
}

/// A dynamic catalog that pulls available models from OpenRouter's API.
pub struct OpenRouterCatalog {
    client: reqwest::Client,
}

impl OpenRouterCatalog {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }
}

impl Default for OpenRouterCatalog {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(serde::Deserialize)]
struct OpenRouterModelsResponse {
    data: Vec<OpenRouterModelData>,
}

#[derive(serde::Deserialize)]
struct OpenRouterModelData {
    id: String,
    pricing: OpenRouterPricing,
    context_length: u64,
}

#[derive(serde::Deserialize)]
struct OpenRouterPricing {
    prompt: String,
    completion: String,
}

#[async_trait::async_trait]
impl ModelCatalog for OpenRouterCatalog {
    async fn refresh(&self) -> Result<Vec<ModelSpec>, anyhow::Error> {
        // Unauthenticated request is fine for getting the OpenRouter model list.
        let resp = self.client.get("https://openrouter.ai/api/v1/models").send().await?;
        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Failed to fetch models from OpenRouter: HTTP {}", resp.status()));
        }

        let body: OpenRouterModelsResponse = resp.json().await?;
        let mut models = Vec::new();

        for m in body.data {
            let cost_input = m.pricing.prompt.parse::<f64>().unwrap_or(0.0) * 1000.0;
            let cost_output = m.pricing.completion.parse::<f64>().unwrap_or(0.0) * 1000.0;
            let is_free = cost_input == 0.0 && cost_output == 0.0;
            // Simplify overall cost per 1k as the average.
            let cost_per_1k = (cost_input + cost_output) / 2.0;

            let provider_prefix = m.id.split('/').next().unwrap_or("unknown");
            
            let mut capabilities = ModelCapabilities::default();
            capabilities.max_context = m.context_length;
            
            models.push(ModelSpec {
                id: m.id.clone(),
                canonical_slug: m.id.clone(),
                provider: provider_prefix.to_string(),
                provider_type: ProviderType::OpenRouter,
                max_tokens: m.context_length,
                cost_per_1k,
                cost_per_1k_input: cost_input,
                cost_per_1k_output: cost_output,
                is_free,
                strengths: vec![], // Unknown from generic API hit, will be enriched by heuristic maps
                capabilities,
                supported_parameters: vec![],
            });
        }

        Ok(models)
    }
}
