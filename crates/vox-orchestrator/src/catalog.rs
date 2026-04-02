use crate::models::{ModelCapabilities, ModelSpec, ProviderType};
use std::time::Duration;

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
            client: vox_reqwest_defaults::client_builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| vox_reqwest_defaults::client()),
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
    #[serde(default)]
    supported_parameters: Vec<String>,
    #[serde(default)]
    architecture: Option<OpenRouterArchitecture>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    top_provider: Option<OpenRouterTopProvider>,
}

#[derive(serde::Deserialize)]
struct OpenRouterPricing {
    prompt: String,
    completion: String,
}

#[derive(serde::Deserialize, Default)]
struct OpenRouterArchitecture {
    #[serde(default)]
    input_modalities: Vec<String>,
    #[serde(default)]
    output_modalities: Vec<String>,
}

#[derive(serde::Deserialize, Default)]
struct OpenRouterTopProvider {
    #[serde(default)]
    max_completion_tokens: Option<u64>,
}

fn infer_strengths(
    id: &str,
    description: Option<&str>,
    supported_parameters: &[String],
) -> Vec<String> {
    let mut strengths: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut haystack = id.to_ascii_lowercase();
    if let Some(desc) = description {
        haystack.push(' ');
        haystack.push_str(&desc.to_ascii_lowercase());
    }

    if haystack.contains("code")
        || haystack.contains("coder")
        || haystack.contains("program")
        || haystack.contains("software")
    {
        strengths.insert("codegen".to_string());
    }
    if haystack.contains("reason")
        || haystack.contains("logic")
        || haystack.contains("math")
        || haystack.contains("proof")
    {
        strengths.insert("logic".to_string());
    }
    if haystack.contains("debug") || haystack.contains("fix") {
        strengths.insert("debugging".to_string());
    }
    if haystack.contains("research")
        || haystack.contains("analysis")
        || haystack.contains("science")
        || haystack.contains("academic")
    {
        strengths.insert("research".to_string());
    }
    if haystack.contains("review") || haystack.contains("critic") {
        strengths.insert("review".to_string());
    }
    if haystack.contains("parse") || haystack.contains("extract") {
        strengths.insert("parsing".to_string());
    }
    if supported_parameters
        .iter()
        .any(|p| p == "tools" || p == "structured_outputs")
    {
        strengths.insert("codegen".to_string());
        strengths.insert("logic".to_string());
    }
    if strengths.is_empty() {
        strengths.insert("generalist".to_string());
    }
    strengths.into_iter().collect()
}

#[async_trait::async_trait]
impl ModelCatalog for OpenRouterCatalog {
    async fn refresh(&self) -> Result<Vec<ModelSpec>, anyhow::Error> {
        // Unauthenticated request is fine for getting the OpenRouter model list.
        let resp = self
            .client
            .get(vox_config::OPENROUTER_MODELS_LIST_URL)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch models from OpenRouter: HTTP {}",
                resp.status()
            ));
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

            let architecture = m.architecture.unwrap_or_default();
            let supports_vision = architecture
                .input_modalities
                .iter()
                .any(|v| v.eq_ignore_ascii_case("image"))
                || architecture
                    .output_modalities
                    .iter()
                    .any(|v| v.eq_ignore_ascii_case("image"));
            let supports_json = m
                .supported_parameters
                .iter()
                .any(|p| p == "response_format" || p == "structured_outputs");
            let capabilities = ModelCapabilities {
                supports_json,
                supports_vision,
                max_context: m.context_length,
                ..Default::default()
            };
            let strengths =
                infer_strengths(&m.id, m.description.as_deref(), &m.supported_parameters);
            let max_tokens = m
                .top_provider
                .and_then(|tp| tp.max_completion_tokens)
                .filter(|n| *n > 0)
                .unwrap_or_else(|| m.context_length.min(16_384));

            models.push(ModelSpec {
                id: m.id.clone(),
                canonical_slug: m.id.clone(),
                provider: provider_prefix.to_string(),
                provider_type: ProviderType::OpenRouter,
                max_tokens,
                cost_per_1k,
                cost_per_1k_input: cost_input,
                cost_per_1k_output: cost_output,
                is_free,
                strengths,
                capabilities,
                supported_parameters: m.supported_parameters,
            });
        }

        Ok(models)
    }
}
