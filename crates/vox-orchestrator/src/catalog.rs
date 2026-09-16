use crate::models::{ModelCapabilities, ModelSpec, ProviderType, StrengthTag};

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
            client: vox_http_client::client_builder()
                .timeout(vox_config::timeouts::D_10S)
                .build()
                .unwrap_or_else(|_| vox_http_client::client()),
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
) -> Vec<StrengthTag> {
    crate::models::generated::infer_strengths(id, description, supported_parameters)
}

#[async_trait::async_trait]
impl ModelCatalog for OpenRouterCatalog {
    async fn refresh(&self) -> Result<Vec<ModelSpec>, anyhow::Error> {
        // Unauthenticated request is fine for getting the OpenRouter model list.
        let resp = self
            .client
            .get(vox_config::openrouter_models_list_url())
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
            let cost_input = (m.pricing.prompt.parse::<f64>().unwrap_or(0.0) * 1000.0).max(0.0);
            let cost_output =
                (m.pricing.completion.parse::<f64>().unwrap_or(0.0) * 1000.0).max(0.0);
            let p_zero = m.pricing.prompt == "0"
                || m.pricing.prompt == "0.0"
                || m.pricing.prompt.starts_with("-")
                || m.pricing.prompt.is_empty();
            let c_zero = m.pricing.completion == "0"
                || m.pricing.completion == "0.0"
                || m.pricing.completion.starts_with("-")
                || m.pricing.completion.is_empty();
            let is_free = p_zero && c_zero;

            // True tokenomics tracked separately via cost_per_1k_input and cost_per_1k_output.
            // The cost_per_1k legacy field defaults to output cost for registry sorting.
            let cost_per_1k = cost_output;

            let provider_prefix = m.id.split('/').next().unwrap_or("unknown");

            let architecture = m.architecture.unwrap_or_default();
            let supports_vision = architecture
                .input_modalities
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

            let mut capabilities = ModelCapabilities {
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
            let inferred = crate::models::infer_capabilities(
                &m.supported_parameters,
                &architecture.input_modalities,
                &architecture.output_modalities,
            );
            capabilities.merge_capability_flags(&inferred);
            let strengths =
                infer_strengths(&m.id, m.description.as_deref(), &m.supported_parameters);
            let max_tokens = m
                .top_provider
                .and_then(|tp| tp.max_completion_tokens)
                .filter(|n| *n > 0)
                .unwrap_or(m.context_length);

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
                observed_cost_per_1k: None,
                cache_creation_cost_per_1k: 0.0,
                cache_read_cost_per_1k: 0.0,
                supports_prompt_caching: false, // LiteLLM oracle fills this in
                pricing_source: crate::models::spec::PricingSource::OpenRouter,
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
            strengths.contains(&StrengthTag::Codegen),
            "tools param should yield codegen"
        );
        assert!(
            strengths.contains(&StrengthTag::Logic),
            "tools param should yield logic"
        );
        assert!(
            !strengths.contains(&StrengthTag::Generalist),
            "must not fall through to generalist when tools present"
        );
    }

    #[test]
    fn infer_strengths_provider_family_fills_gap() {
        // deepseek with no special parameters and no name signals — family table alone fills it.
        let strengths = infer_strengths("deepseek/deepseek-r1", None, &[]);
        assert!(
            strengths.contains(&StrengthTag::Codegen),
            "deepseek family → codegen"
        );
        assert!(
            !strengths.contains(&StrengthTag::Generalist),
            "family fill must suppress generalist"
        );
    }

    #[test]
    fn infer_strengths_unknown_provider_uses_name_heuristic() {
        let strengths = infer_strengths("acme/code-assist-7b", None, &["temperature".to_string()]);
        assert!(
            strengths.contains(&StrengthTag::Codegen),
            "name heuristic: 'code' → codegen"
        );
    }

    #[test]
    fn infer_strengths_last_resort_generalist() {
        let strengths = infer_strengths("acme/blob-7b", None, &[]);
        assert_eq!(
            strengths,
            vec![StrengthTag::Generalist],
            "totally unknown model with no signals → generalist only"
        );
    }

    #[test]
    fn infer_strengths_reasoning_param_yields_logic_debugging() {
        let strengths = infer_strengths("x/m", None, &["reasoning".to_string()]);
        assert!(strengths.contains(&StrengthTag::Logic));
        assert!(strengths.contains(&StrengthTag::Debugging));
    }

    #[tokio::test]
    async fn test_huggingface_catalog_known_models_modern() {
        let cat = HuggingFaceCatalog::new();
        let specs = cat.refresh().await.expect("refresh should succeed");
        assert!(
            !specs.iter().any(|s| s.id.contains("Qwen2.5")),
            "Qwen 2.5 must not be in default HF catalog"
        );
        assert!(
            specs.iter().any(|s| s.id.contains("Qwen3")),
            "Qwen 3 should be in default HF catalog"
        );
    }

    #[tokio::test]
    async fn test_mens_catalog_extracts_context_length_from_config() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let root = tmp.path();

        // 1. Direct config.json with max_position_embeddings
        let run_1 = root.join("run_direct");
        std::fs::create_dir_all(&run_1).unwrap();
        std::fs::write(
            run_1.join("config.json"),
            r#"{"max_position_embeddings": 40960}"#,
        )
        .unwrap();
        assert_eq!(
            MensCatalog::read_context_length_from_dir(&run_1),
            40960,
            "should extract max_position_embeddings from root config.json"
        );

        // 2. final/config.json with context_length
        let run_2 = root.join("run_final");
        let final_dir = run_2.join("final");
        std::fs::create_dir_all(&final_dir).unwrap();
        std::fs::write(
            final_dir.join("config.json"),
            r#"{"context_length": 128000}"#,
        )
        .unwrap();
        assert_eq!(
            MensCatalog::read_context_length_from_dir(&run_2),
            128000,
            "should extract context_length from final/config.json"
        );

        // 3. adapter_manifest.json referencing base_model dir
        let base_model_dir = root.join("base_model");
        std::fs::create_dir_all(&base_model_dir).unwrap();
        std::fs::write(
            base_model_dir.join("config.json"),
            r#"{"max_position_embeddings": 65536}"#,
        )
        .unwrap();

        let run_3 = root.join("run_adapter");
        std::fs::create_dir_all(&run_3).unwrap();
        std::fs::write(
            run_3.join("adapter_manifest.json"),
            format!(
                r#"{{"format":"vox_mens_adapter","version":3,"base_model":{}}}"#,
                serde_json::to_string(&base_model_dir.to_string_lossy()).unwrap()
            ),
        )
        .unwrap();
        assert_eq!(
            MensCatalog::read_context_length_from_dir(&run_3),
            65536,
            "should extract context length from base_model config via adapter_manifest.json"
        );

        // 4. Fallback default when no config exists
        let run_4 = root.join("run_empty");
        std::fs::create_dir_all(&run_4).unwrap();
        assert_eq!(
            MensCatalog::read_context_length_from_dir(&run_4),
            32768,
            "should fallback to 32768 when no config exists"
        );

        // 5. MensCatalog::refresh discovers run and sets max_tokens dynamically
        let mens_root = root.join("workspace_mens");
        let runs_dir = mens_root.join("mens").join("runs");
        let run_refresh = runs_dir.join("test-model");
        std::fs::create_dir_all(run_refresh.join("final")).unwrap();
        std::fs::write(
            run_refresh.join("final").join("config.json"),
            r#"{"max_position_embeddings": 40960}"#,
        )
        .unwrap();

        let catalog = MensCatalog::new(&mens_root);
        let models = catalog.refresh().await.expect("refresh succeeds");
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "mens/test-model");
        assert_eq!(models[0].max_tokens, 40960);
    }
}
/// Parses Ollama's `details.parameter_size` field (e.g. `"8.2B"`, `"70M"`,
/// `"1.5B"`) into billions of parameters. Returns `None` for anything that
/// doesn't parse — an unknown/future unit suffix, empty string, etc. — so
/// callers never get a silently-wrong number; absence flows through as "no
/// VRAM signal" (`param_count_b: None`), not a guessed default.
fn parse_ollama_parameter_size(raw: &str) -> Option<f32> {
    let s = raw.trim();
    if let Some(digits) = s.strip_suffix('B').or_else(|| s.strip_suffix('b')) {
        return digits.trim().parse::<f32>().ok();
    }
    if let Some(digits) = s.strip_suffix('M').or_else(|| s.strip_suffix('m')) {
        return digits.trim().parse::<f32>().ok().map(|m| m / 1000.0);
    }
    None
}

/// A catalog that pulls available models from local Ollama/Populi.
pub struct OllamaCatalog {
    client: reqwest::Client,
    base_url: String,
}

impl OllamaCatalog {
    pub fn new(base_url: String) -> Self {
        Self {
            client: vox_http_client::client_builder()
                .timeout(vox_config::timeouts::D_5S)
                .build()
                .unwrap_or_else(|_| vox_http_client::client()),
            base_url,
        }
    }

    /// Queries Ollama's `/api/show` endpoint for a specific model's real
    /// context length. `/api/tags` (used by `refresh`) does not return this,
    /// so we need a follow-up call per model. Returns `None` (rather than an
    /// error) on any failure — timeout, non-2xx, missing/unparseable field —
    /// so a single bad model never fails the whole discovery pass; the
    /// caller falls back to the 4096 default in that case.
    async fn fetch_context_length(&self, model_name: &str) -> Option<u64> {
        let url = format!("{}/api/show", self.base_url.trim_end_matches('/'));
        let res = self
            .client
            .post(&url)
            .timeout(vox_config::timeouts::D_5S)
            .json(&serde_json::json!({ "name": model_name }))
            .send()
            .await
            .ok()?;
        if !res.status().is_success() {
            return None;
        }
        let body: serde_json::Value = res.json().await.ok()?;
        // Real shape observed from a running Ollama instance:
        // { ..., "model_info": { "<family>.context_length": 40960, ... }, ... }
        // The key name is family-prefixed (e.g. "qwen3.context_length"), so
        // scan model_info's entries for the first key ending in
        // ".context_length" rather than hardcoding a family name.
        let model_info = body.get("model_info")?.as_object()?;
        model_info
            .iter()
            .find(|(k, _)| k.ends_with(".context_length"))
            .and_then(|(_, v)| v.as_u64())
    }
}

#[async_trait::async_trait]
impl ModelCatalog for OllamaCatalog {
    async fn refresh(&self) -> Result<Vec<ModelSpec>, anyhow::Error> {
        let url = format!("{}/api/tags", self.base_url.trim_end_matches('/'));
        let res = self.client.get(&url).send().await?;
        if !res.status().is_success() {
            return Err(anyhow::anyhow!(
                "Ollama catalog refresh failed: {}",
                res.status()
            ));
        }

        #[derive(serde::Deserialize)]
        struct OllamaTagsResponse {
            models: Vec<OllamaModelData>,
        }
        #[derive(serde::Deserialize)]
        struct OllamaModelData {
            name: String,
            details: Option<OllamaModelDetails>,
        }
        #[derive(serde::Deserialize)]
        struct OllamaModelDetails {
            parameter_size: Option<String>,
        }

        let resp: OllamaTagsResponse = res.json().await?;
        let mut specs = Vec::new();
        for m in resp.models {
            let max_tokens = self.fetch_context_length(&m.name).await.unwrap_or(4096);
            let param_count_b = m
                .details
                .as_ref()
                .and_then(|d| d.parameter_size.as_deref())
                .and_then(parse_ollama_parameter_size);
            specs.push(ModelSpec {
                id: m.name.clone(),
                canonical_slug: format!("ollama/{}", m.name),
                provider: "ollama".to_string(),
                provider_type: ProviderType::Ollama,
                max_tokens,
                cost_per_1k: 0.0,
                cost_per_1k_input: 0.0,
                cost_per_1k_output: 0.0,
                is_free: true,
                strengths: vec![StrengthTag::Generalist],
                capabilities: ModelCapabilities {
                    tier: crate::models::ModelTier::Local,
                    param_count_b,
                    ..Default::default()
                },
                supported_parameters: vec![],
                observed_cost_per_1k: None,
                cache_creation_cost_per_1k: 0.0,
                cache_read_cost_per_1k: 0.0,
                supports_prompt_caching: false,
                pricing_source: crate::models::spec::PricingSource::Bootstrap,
            });
        }
        Ok(specs)
    }
}

/// A catalog for Hugging Face Inference Providers.
pub struct HuggingFaceCatalog {
    #[allow(dead_code)]
    client: reqwest::Client,
}

impl Default for HuggingFaceCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl HuggingFaceCatalog {
    pub fn new() -> Self {
        Self {
            client: vox_http_client::client_builder()
                .timeout(vox_config::timeouts::D_10S)
                .build()
                .unwrap_or_else(|_| vox_http_client::client()),
        }
    }
}

#[async_trait::async_trait]
impl ModelCatalog for HuggingFaceCatalog {
    async fn refresh(&self) -> Result<Vec<ModelSpec>, anyhow::Error> {
        // This is a placeholder for the actual HF Inference Providers discovery.
        // For now, we return a few high-quality known defaults if no dedicated discovery endpoint is used.
        let known_models = vec![
            "Qwen/Qwen3-32B-Instruct",
            "deepseek-ai/DeepSeek-R1",
            "meta-llama/Llama-3.3-70B-Instruct",
        ];

        let mut specs = Vec::new();
        for m in known_models {
            specs.push(ModelSpec {
                id: m.to_string(),
                canonical_slug: format!("hf/{}", m),
                provider: "hf_router".to_string(),
                provider_type: ProviderType::HuggingFaceRouter,
                max_tokens: 32768,
                cost_per_1k: 0.0, // Often free/included in token
                cost_per_1k_input: 0.0,
                cost_per_1k_output: 0.0,
                is_free: true,
                strengths: vec![StrengthTag::Generalist, StrengthTag::Codegen],
                capabilities: ModelCapabilities {
                    tier: crate::models::ModelTier::Pro,
                    ..Default::default()
                },
                supported_parameters: vec![],
                observed_cost_per_1k: None,
                cache_creation_cost_per_1k: 0.0,
                cache_read_cost_per_1k: 0.0,
                supports_prompt_caching: false,
                pricing_source: crate::models::spec::PricingSource::Bootstrap,
            });
        }
        Ok(specs)
    }
}

/// A catalog for remote Populi mesh nodes.
pub struct PopuliMeshCatalog {
    #[allow(dead_code)]
    client: reqwest::Client,
}

impl Default for PopuliMeshCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl PopuliMeshCatalog {
    pub fn new() -> Self {
        Self {
            client: vox_http_client::client_builder()
                .timeout(vox_config::timeouts::D_5S)
                .build()
                .unwrap_or_else(|_| vox_http_client::client()),
        }
    }
}

#[async_trait::async_trait]
impl ModelCatalog for PopuliMeshCatalog {
    async fn refresh(&self) -> Result<Vec<ModelSpec>, anyhow::Error> {
        discover_populi_mesh_models().await
    }
}

/// Live Populi mesh model discovery via federation directory (mirrors registry refresh).
pub async fn discover_populi_mesh_models() -> Result<Vec<ModelSpec>, anyhow::Error> {
    #[cfg(feature = "populi-transport")]
    {
        let mut control_url_opt =
            vox_secrets::resolve_secret(vox_secrets::SecretId::VoxOrchestratorMeshControlUrl)
                .expose()
                .map(|s| s.to_string());
        if control_url_opt.is_none() {
            control_url_opt =
                vox_secrets::resolve_secret(vox_secrets::SecretId::VoxMeshControlAddr)
                    .expose()
                    .map(|s| s.to_string());
        }
        let Some(control_url) = control_url_opt else {
            return Ok(vec![]);
        };
        let client =
            vox_populi::http_client::PopuliHttpClient::new(control_url.trim()).with_env_token();
        let dir = client.federation_directory().await?;
        let mut specs = Vec::new();
        for peer in dir.entries {
            for kind in peer.task_kinds {
                let kind_str = serde_json::to_value(&kind)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_string))
                    .unwrap_or_else(|| "general".to_string());
                specs.push(ModelSpec {
                    id: format!("mesh/{}/{}", peer.scope_id, kind_str),
                    canonical_slug: format!("mesh/{}/{}", peer.scope_id, kind_str),
                    provider: "mens".to_string(),
                    provider_type: ProviderType::PopuliMesh,
                    max_tokens: 128_000,
                    cost_per_1k: 0.0,
                    cost_per_1k_input: 0.0,
                    cost_per_1k_output: 0.0,
                    is_free: true,
                    strengths: vec![
                        kind_str
                            .parse::<StrengthTag>()
                            .unwrap_or(StrengthTag::Unknown),
                        StrengthTag::Generalist,
                    ],
                    capabilities: ModelCapabilities {
                        tier: crate::models::ModelTier::Local,
                        writes_vox: true,
                        ..Default::default()
                    },
                    supported_parameters: vec![],
                    observed_cost_per_1k: None,
                    cache_creation_cost_per_1k: 0.0,
                    cache_read_cost_per_1k: 0.0,
                    supports_prompt_caching: false,
                    pricing_source: crate::models::spec::PricingSource::Bootstrap,
                });
            }
        }
        return Ok(specs);
    }
    #[cfg(not(feature = "populi-transport"))]
    {
        let _ = ();
        Ok(vec![])
    }
}

/// A catalog for local MENS checkpoints.
pub struct MensCatalog {
    root: std::path::PathBuf,
}

fn parse_context_length_from_config(path: &std::path::Path) -> Option<u32> {
    if !path.is_file() {
        return None;
    }
    let content = std::fs::read_to_string(path).ok()?;
    let val: serde_json::Value = serde_json::from_str(&content).ok()?;
    if let Some(n) = val
        .get("max_position_embeddings")
        .or_else(|| val.get("context_length"))
        .or_else(|| val.get("seq_length"))
        .or_else(|| val.get("model_max_length"))
        .or_else(|| val.get("n_positions"))
        .or_else(|| {
            val.get("model_config").and_then(|m| {
                m.get("max_position_embeddings")
                    .or_else(|| m.get("context_length"))
                    .or_else(|| m.get("seq_length"))
                    .or_else(|| m.get("model_max_length"))
                    .or_else(|| m.get("n_positions"))
            })
        })
    {
        if let Some(v) = n.as_u64() {
            if v > 0 {
                if let Ok(val) = u32::try_from(v) {
                    return Some(val);
                }
            }
        }
        if let Some(s) = n.as_str() {
            if let Ok(v) = s.parse::<u32>() {
                if v > 0 {
                    return Some(v);
                }
            }
        }
    }
    None
}

fn resolve_base_model_context_length(
    dir: &std::path::Path,
    base_model_val: &serde_json::Value,
) -> Option<u32> {
    if let Some(base_str) = base_model_val.as_str() {
        // First check if base_str resolves relative to the run directory
        let rel = dir.join(base_str);
        if rel.is_file() {
            if let Some(len) = parse_context_length_from_config(&rel) {
                return Some(len);
            }
        } else if rel.is_dir() {
            if let Some(len) = parse_context_length_from_config(&rel.join("config.json")) {
                return Some(len);
            }
        }

        // Then check if base_str is an absolute path
        let p = std::path::Path::new(base_str);
        if p.is_absolute() {
            if p.is_file() {
                if let Some(len) = parse_context_length_from_config(p) {
                    return Some(len);
                }
            } else if p.is_dir() {
                if let Some(len) = parse_context_length_from_config(&p.join("config.json")) {
                    return Some(len);
                }
            }
        }

        // Check local Hugging Face hub cache if base is a repo id like "Qwen/Qwen3-0.6B" or pinned "Qwen/Qwen3-8B@<sha>"
        let repo_id = base_str.split('@').next().unwrap_or(base_str);
        let revision_sha = base_str.split('@').nth(1);

        let hub_base = std::env::var("HF_HUB_CACHE")
            .ok()
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var("HF_HOME")
                    .ok()
                    .map(|h| std::path::PathBuf::from(h).join("hub"))
            })
            .or_else(|| {
                dirs::home_dir().map(|home| home.join(".cache").join("huggingface").join("hub"))
            });

        if let Some(hub_root) = hub_base {
            let snapshots_dir = hub_root
                .join(format!("models--{}", repo_id.replace('/', "--")))
                .join("snapshots");
            if snapshots_dir.is_dir() {
                // If a specific revision SHA was provided, check snapshots/<revision_sha>/config.json first
                if let Some(sha) = revision_sha {
                    let rev_dir = snapshots_dir.join(sha);
                    if rev_dir.is_dir() {
                        if let Some(len) =
                            parse_context_length_from_config(&rev_dir.join("config.json"))
                        {
                            return Some(len);
                        }
                    }
                }
                if let Ok(entries) = std::fs::read_dir(&snapshots_dir) {
                    for entry in entries.flatten() {
                        let ep = entry.path();
                        if ep.is_dir() {
                            if let Some(len) =
                                parse_context_length_from_config(&ep.join("config.json"))
                            {
                                return Some(len);
                            }
                        }
                    }
                }
            }
        }
    } else if let Some(base_obj) = base_model_val.as_object() {
        if let Some(n) = base_obj
            .get("max_position_embeddings")
            .or_else(|| base_obj.get("context_length"))
            .or_else(|| base_obj.get("seq_length"))
            .or_else(|| base_obj.get("model_max_length"))
            .or_else(|| base_obj.get("n_positions"))
        {
            if let Some(v) = n.as_u64() {
                if v > 0 {
                    if let Ok(val) = u32::try_from(v) {
                        return Some(val);
                    }
                }
            }
            if let Some(s) = n.as_str() {
                if let Ok(v) = s.parse::<u32>() {
                    if v > 0 {
                        return Some(v);
                    }
                }
            }
        }
    }
    None
}

impl MensCatalog {
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Discovers and returns the context length (in tokens) for a MENS checkpoint run directory.
    /// Checks `config.json` and `final/config.json`, then `adapter_manifest.json` for base model
    /// config, falling back to 32768.
    pub fn read_context_length_from_dir(dir: &std::path::Path) -> u32 {
        let discovered = (|| {
            if let Some(len) = parse_context_length_from_config(&dir.join("config.json")) {
                return len;
            }
            if let Some(len) =
                parse_context_length_from_config(&dir.join("final").join("config.json"))
            {
                return len;
            }

            let manifest_candidates = [
                dir.join("adapter_manifest.json"),
                dir.join("final").join("adapter_manifest.json"),
            ];
            for manifest_path in &manifest_candidates {
                if manifest_path.is_file() {
                    if let Ok(content) = std::fs::read_to_string(manifest_path) {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                            if let Some(base_val) = val.get("base_model") {
                                if let Some(len) = resolve_base_model_context_length(dir, base_val)
                                {
                                    return len;
                                }
                            }
                            if let Some(upstream_val) = val
                                .get("provenance")
                                .and_then(|p| p.get("upstream_model_id"))
                            {
                                if let Some(len) =
                                    resolve_base_model_context_length(dir, upstream_val)
                                {
                                    return len;
                                }
                            }
                        }
                    }
                }
            }

            32768
        })();

        if discovered == 0 { 32768 } else { discovered }
    }
}

#[async_trait::async_trait]
impl ModelCatalog for MensCatalog {
    async fn refresh(&self) -> Result<Vec<ModelSpec>, anyhow::Error> {
        let mut specs = Vec::new();
        let runs_dir = self.root.join("mens").join("runs");
        if !runs_dir.is_dir() {
            return Ok(specs);
        }

        let entries = std::fs::read_dir(&runs_dir)?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                if name.starts_with('.') {
                    continue;
                }

                // Look for 'final', 'checkpoint-*' subdirs, or direct 'candle_qlora_adapter.safetensors' or 'adapter_manifest.json'
                let has_checkpoint = std::fs::read_dir(&path)?.flatten().any(|e| {
                    e.file_name()
                        .to_str()
                        .map(|s| {
                            s == "final"
                                || s.starts_with("checkpoint-")
                                || s == "candle_qlora_adapter.safetensors"
                                || s == "adapter_manifest.json"
                        })
                        .unwrap_or(false)
                });

                if has_checkpoint {
                    let ctx_len = Self::read_context_length_from_dir(&path) as u64;
                    specs.push(ModelSpec {
                        id: format!("mens/{}", name),
                        canonical_slug: format!("mens/{}", name),
                        provider: "voxlocal".to_string(),
                        provider_type: ProviderType::VoxLocal,
                        max_tokens: ctx_len,
                        cost_per_1k: 0.0,
                        cost_per_1k_input: 0.0,
                        cost_per_1k_output: 0.0,
                        is_free: true,
                        strengths: vec![StrengthTag::Generalist, StrengthTag::Codegen],
                        capabilities: ModelCapabilities {
                            tier: crate::models::ModelTier::Local,
                            writes_vox: true,
                            ..Default::default()
                        },
                        supported_parameters: vec![],
                        observed_cost_per_1k: None,
                        cache_creation_cost_per_1k: 0.0,
                        cache_read_cost_per_1k: 0.0,
                        supports_prompt_caching: false,
                        pricing_source: crate::models::spec::PricingSource::Bootstrap,
                    });
                }
            }
        }
        Ok(specs)
    }
}

/// A catalog that pulls available models directly from Anthropic's API.
pub struct AnthropicDirectCatalog {
    client: reqwest::Client,
}

impl Default for AnthropicDirectCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl AnthropicDirectCatalog {
    pub fn new() -> Self {
        Self {
            client: vox_http_client::client_builder()
                .timeout(vox_config::timeouts::D_10S)
                .build()
                .unwrap_or_else(|_| vox_http_client::client()),
        }
    }
}

#[async_trait::async_trait]
impl ModelCatalog for AnthropicDirectCatalog {
    async fn refresh(&self) -> Result<Vec<ModelSpec>, anyhow::Error> {
        let api_key = vox_secrets::resolve_secret(vox_secrets::SecretId::AnthropicApiKey)
            .expose()
            .map(|s| s.to_string());
        let Some(key) = api_key else {
            return Ok(vec![]); // Skip if no key
        };

        let res = self
            .client
            .get("https://api.anthropic.com/v1/models")
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(anyhow::anyhow!(
                "Anthropic catalog refresh failed: {}",
                res.status()
            ));
        }

        #[derive(serde::Deserialize)]
        struct AnthropicModelsResponse {
            data: Vec<AnthropicModelData>,
        }
        #[derive(serde::Deserialize)]
        struct AnthropicModelData {
            id: String,
            display_name: String,
        }

        let resp: AnthropicModelsResponse = res.json().await?;
        let mut specs = Vec::new();
        for m in resp.data {
            // Pricing is intentionally left at 0.0 here; the LiteLLMCatalog oracle (fetched
            // immediately after in the refresh pipeline) supplies accurate prices. Hardcoding
            // values caused silent drift as Anthropic updated pricing.
            //
            // IMPORTANT: Until LiteLLM fills in real prices, we must not emit a non-free model
            // with 0.0 costs — that would make it rank as the "cheapest" candidate and distort
            // routing. Mark pricing_unknown=true so the spec is treated as a placeholder and
            // excluded from cost-ranked routing until the next LiteLLM patch arrives.
            let (c_in, c_out) = (0.0_f64, 0.0_f64);

            // Classify tier by model name since we no longer hardcode prices here.
            let is_opus = m.id.contains("opus");
            let tier = if is_opus {
                crate::models::ModelTier::Elite
            } else {
                crate::models::ModelTier::Pro
            };
            specs.push(ModelSpec {
                id: m.id.clone(),
                canonical_slug: format!("anthropic/{}", m.id),
                provider: "anthropic".to_string(),
                provider_type: ProviderType::Anthropic,
                max_tokens: 200_000,
                cost_per_1k: c_out,
                cost_per_1k_input: c_in,
                cost_per_1k_output: c_out,
                // Treat as unknown until LiteLLM supplies real pricing; this prevents
                // zero-priced non-free models from topping economy routing before prices arrive.
                is_free: false,
                strengths: infer_strengths(&m.id, Some(&m.display_name), &[]),
                capabilities: ModelCapabilities {
                    supports_native_tools: true,
                    supports_tool_use: true,
                    supports_reasoning: true,
                    tier,
                    ..Default::default()
                },
                supported_parameters: vec![
                    "tools".to_string(),
                    "tool_choice".to_string(),
                    "thinking".to_string(),
                ],
                observed_cost_per_1k: None,
                cache_creation_cost_per_1k: 0.0,
                cache_read_cost_per_1k: 0.0,
                supports_prompt_caching: false, // LiteLLM will fill this in
                pricing_source: crate::models::spec::PricingSource::Unknown,
            });
        }
        Ok(specs)
    }
}

/// A catalog that pulls available models directly from Google's Generative Language API.
pub struct GoogleDirectCatalog {
    client: reqwest::Client,
}

impl Default for GoogleDirectCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl GoogleDirectCatalog {
    pub fn new() -> Self {
        Self {
            client: vox_http_client::client_builder()
                .timeout(vox_config::timeouts::D_10S)
                .build()
                .unwrap_or_else(|_| vox_http_client::client()),
        }
    }
}

#[async_trait::async_trait]
impl ModelCatalog for GoogleDirectCatalog {
    async fn refresh(&self) -> Result<Vec<ModelSpec>, anyhow::Error> {
        let api_key = vox_secrets::resolve_secret(vox_secrets::SecretId::GeminiApiKey)
            .expose()
            .map(|s| s.to_string());
        let Some(key) = api_key else {
            return Ok(vec![]); // Skip if no key
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models?key={}",
            key
        );
        let res = self.client.get(&url).send().await?;

        if !res.status().is_success() {
            return Err(anyhow::anyhow!(
                "Google catalog refresh failed: {}",
                res.status()
            ));
        }

        #[derive(serde::Deserialize)]
        struct GoogleModelsResponse {
            models: Vec<GoogleModelData>,
        }
        #[derive(serde::Deserialize)]
        struct GoogleModelData {
            name: String,
            description: String,
            #[serde(rename = "inputTokenLimit")]
            input_token_limit: u64,
            #[serde(rename = "outputTokenLimit")]
            output_token_limit: u64,
            #[serde(rename = "supportedGenerationMethods")]
            supported_methods: Vec<String>,
        }

        let resp: GoogleModelsResponse = res.json().await?;
        let mut specs = Vec::new();
        for m in resp.models {
            if !m.supported_methods.iter().any(|s| s == "generateContent") {
                continue;
            }

            let id = m.name.trim_start_matches("models/").to_string();

            // Pricing logic for Google is complex (free tiers vs paid), so we default to 0.0
            // and let the observed cost accounting (FIX-75) calibrate it.
            specs.push(ModelSpec {
                id: id.clone(),
                canonical_slug: format!("google/{}", id),
                provider: "google".to_string(),
                provider_type: ProviderType::GoogleDirect,
                max_tokens: m.output_token_limit,
                cost_per_1k: 0.0,
                cost_per_1k_input: 0.0,
                cost_per_1k_output: 0.0,
                is_free: true,
                strengths: infer_strengths(&id, Some(&m.description), &[]),
                capabilities: ModelCapabilities {
                    max_context: m.input_token_limit,
                    ..Default::default()
                },
                supported_parameters: vec![],
                observed_cost_per_1k: None,
                cache_creation_cost_per_1k: 0.0,
                cache_read_cost_per_1k: 0.0,
                supports_prompt_caching: false,
                pricing_source: crate::models::spec::PricingSource::Bootstrap,
            });
        }
        Ok(specs)
    }
}

// ── LiteLLM pricing oracle ────────────────────────────────────────────────────────────────────

/// Resolved pricing entry from the LiteLLM `model_prices_and_context_window.json` oracle.
///
/// All costs are **per 1 000 tokens** (converted from LiteLLM's per-token representation).
/// Fields are `None` when the upstream JSON does not include them for that model.
#[derive(Debug, Clone, Default)]
pub struct LiteLLMPricingEntry {
    pub cost_per_1k_input: Option<f64>,
    pub cost_per_1k_output: Option<f64>,
    /// Cost to *write* a new prompt-cache prefix (e.g. 1.25× on Anthropic). Per 1k tokens.
    pub cache_creation_cost_per_1k: Option<f64>,
    /// Cost for a prompt-cache *hit* read (e.g. ~0.10× on Anthropic). Per 1k tokens.
    pub cache_read_cost_per_1k: Option<f64>,
    pub supports_prompt_caching: Option<bool>,
    /// Provider string as given by LiteLLM (e.g. `"deepseek"`, `"anthropic"`).
    pub litellm_provider: Option<String>,
}

/// Raw entry from the LiteLLM JSON file (per-token costs; deserialized before conversion).
#[derive(serde::Deserialize)]
struct LiteLLMRawEntry {
    #[serde(default)]
    input_cost_per_token: Option<f64>,
    #[serde(default)]
    output_cost_per_token: Option<f64>,
    #[serde(default)]
    cache_creation_input_token_cost: Option<f64>,
    #[serde(default)]
    cache_read_input_token_cost: Option<f64>,
    #[serde(default)]
    supports_prompt_caching: Option<bool>,
    #[serde(default)]
    litellm_provider: Option<String>,
    /// Absorb all other fields without failing deserialization.
    #[serde(flatten)]
    _rest: serde_json::Map<String, serde_json::Value>,
}

impl LiteLLMRawEntry {
    fn into_pricing_entry(self) -> LiteLLMPricingEntry {
        const K: f64 = 1_000.0;
        LiteLLMPricingEntry {
            cost_per_1k_input: self.input_cost_per_token.map(|v| v * K),
            cost_per_1k_output: self.output_cost_per_token.map(|v| v * K),
            cache_creation_cost_per_1k: self.cache_creation_input_token_cost.map(|v| v * K),
            cache_read_cost_per_1k: self.cache_read_input_token_cost.map(|v| v * K),
            supports_prompt_caching: self.supports_prompt_caching,
            litellm_provider: self.litellm_provider,
        }
    }
}

/// Fetches the LiteLLM `model_prices_and_context_window.json` pricing oracle.
///
/// Returns a map of model-ID → [`LiteLLMPricingEntry`] that callers (registry refresh loop)
/// use to patch `cost_per_1k_input`, `cost_per_1k_output`, cache costs, and
/// `supports_prompt_caching` on existing [`ModelSpec`] entries.
///
/// This catalog does **not** create new model entries; discovery is OpenRouter's responsibility.
pub struct LiteLLMCatalog {
    client: reqwest::Client,
}

const LITELLM_PRICES_URL: &str =
    "https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json";

impl LiteLLMCatalog {
    pub fn new() -> Self {
        Self {
            client: vox_http_client::client_builder()
                .timeout(vox_config::timeouts::D_20S)
                .build()
                .unwrap_or_else(|_| vox_http_client::client()),
        }
    }

    /// Fetch the oracle and return a map of model-ID → pricing entry.
    ///
    /// Failures are returned as `Err` so callers can log and fall back gracefully —
    /// the registry keeps whatever pricing it already has.
    pub async fn fetch(
        &self,
    ) -> Result<std::collections::HashMap<String, LiteLLMPricingEntry>, anyhow::Error> {
        let resp = self.client.get(LITELLM_PRICES_URL).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(
                "LiteLLM pricing fetch failed: HTTP {}",
                resp.status()
            ));
        }

        // The JSON is a flat object: { "model-id": { ...fields... }, ... }
        let raw: std::collections::HashMap<String, LiteLLMRawEntry> = resp.json().await?;
        let entries = raw
            .into_iter()
            .map(|(id, entry)| (id, entry.into_pricing_entry()))
            .collect();

        Ok(entries)
    }
}

impl Default for LiteLLMCatalog {
    fn default() -> Self {
        Self::new()
    }
}
