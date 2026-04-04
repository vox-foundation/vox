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
    /// Per-request rate limits when the provider exposes them (rpm/rpd).
    #[serde(default)]
    per_request_limits: Option<OpenRouterPerRequestLimits>,
    /// Provider latency statistics surfaced by the OpenRouter catalog.
    #[serde(default)]
    latency: Option<OpenRouterLatency>,
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
    /// Whether this provider applies content moderation filtering.
    #[serde(default)]
    is_moderated: bool,
}

/// Rate limits per request window as reported by OpenRouter.
#[derive(serde::Deserialize, Default)]
struct OpenRouterPerRequestLimits {
    /// Requests per minute allowed for this model/provider combination.
    #[serde(default)]
    requests_per_minute: Option<u32>,
    /// Requests per day allowed for this model/provider combination.
    #[serde(default)]
    requests_per_day: Option<u32>,
}

/// Provider latency statistics from the OpenRouter catalog endpoint.
#[derive(serde::Deserialize, Default)]
struct OpenRouterLatency {
    /// Median latency in milliseconds (p50).
    #[serde(default)]
    p50: Option<u32>,
}

/// Derives `strengths` tags for a model using a three-tier strategy:
///
/// 1. **Parameter graph** — `supported_parameters` fields yield precise capability tags.
/// 2. **Provider family** — known prefixes (e.g. `anthropic`, `deepseek`) fill gaps without
///    relying on prose matching against description strings.
/// 3. **Name / description heuristic** — substring matching as a final catch-all.
///
/// The final set is deduplicated; if still empty after all three tiers, `generalist` is inserted
/// only as a last resort.
fn infer_strengths(
    id: &str,
    description: Option<&str>,
    supported_parameters: &[String],
) -> Vec<String> {
    use crate::models::provider_family_strengths;
    use std::collections::BTreeSet;

    let mut strengths: BTreeSet<String> = BTreeSet::new();

    // ── Tier 1: supported_parameters capability graph ──────────────────────────────────────────
    let has_tools = supported_parameters
        .iter()
        .any(|p| p == "tools" || p == "tool_use");
    let has_structured = supported_parameters
        .iter()
        .any(|p| p == "response_format" || p == "structured_outputs");
    let has_reasoning = supported_parameters
        .iter()
        .any(|p| p == "reasoning" || p == "thinking");
    let has_web_search = supported_parameters
        .iter()
        .any(|p| p == "web_search" || p == "search");

    if has_tools || has_structured {
        strengths.insert("codegen".to_string());
        strengths.insert("logic".to_string());
    }
    if has_reasoning {
        strengths.insert("logic".to_string());
        strengths.insert("debugging".to_string());
    }
    if has_web_search {
        strengths.insert("research".to_string());
    }

    // ── Tier 2: provider family table ─────────────────────────────────────────────────────────
    let provider_prefix = id.split('/').next().unwrap_or("");
    let family = provider_family_strengths(provider_prefix);
    for s in family {
        strengths.insert((*s).to_string());
    }

    // ── Tier 3: name / description heuristic (catch-all for unknown providers) ─────────────────
    if strengths.is_empty() {
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
    }

    // Last resort: generalist if all three tiers yielded nothing.
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

            // ── Capabilities: rate limits, latency, moderation, uptime ─────────────────────────
            let (rate_limit_rpm, rate_limit_rpd) = m
                .per_request_limits
                .as_ref()
                .map(|r| (r.requests_per_minute, r.requests_per_day))
                .unwrap_or((None, None));

            let latency_p50_ms = m.latency.as_ref().and_then(|l| l.p50);
            let is_moderated = m
                .top_provider
                .as_ref()
                .map(|tp| tp.is_moderated)
                .unwrap_or(false);

            let capabilities = ModelCapabilities {
                supports_json,
                supports_vision,
                max_context: m.context_length,
                rate_limit_rpm,
                rate_limit_rpd,
                latency_p50_ms,
                is_moderated,
                uptime_score: None, // populated later by catalog_classifier
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_strengths_tools_parameter_wins_over_generalist() {
        let strengths = infer_strengths(
            "unknown/mystery-model",
            None,
            &["tools".to_string(), "temperature".to_string()],
        );
        assert!(
            strengths.contains(&"codegen".to_string()),
            "tools param should yield codegen"
        );
        assert!(
            strengths.contains(&"logic".to_string()),
            "tools param should yield logic"
        );
        assert!(
            !strengths.contains(&"generalist".to_string()),
            "must not fall through to generalist when tools present"
        );
    }

    #[test]
    fn infer_strengths_provider_family_fills_gap() {
        // deepseek with no special parameters and no name signals — family table alone fills it.
        let strengths = infer_strengths("deepseek/deepseek-r1", None, &[]);
        assert!(
            strengths.contains(&"codegen".to_string()),
            "deepseek family → codegen"
        );
        assert!(
            !strengths.contains(&"generalist".to_string()),
            "family fill must suppress generalist"
        );
    }

    #[test]
    fn infer_strengths_unknown_provider_uses_name_heuristic() {
        let strengths =
            infer_strengths("acme/code-assist-7b", None, &["temperature".to_string()]);
        assert!(
            strengths.contains(&"codegen".to_string()),
            "name heuristic: 'code' → codegen"
        );
    }

    #[test]
    fn infer_strengths_last_resort_generalist() {
        let strengths = infer_strengths("acme/blob-7b", None, &[]);
        assert_eq!(
            strengths,
            vec!["generalist"],
            "totally unknown model with no signals → generalist only"
        );
    }

    #[test]
    fn infer_strengths_reasoning_param_yields_logic_debugging() {
        let strengths = infer_strengths("x/m", None, &["reasoning".to_string()]);
        assert!(strengths.contains(&"logic".to_string()));
        assert!(strengths.contains(&"debugging".to_string()));
    }
}
