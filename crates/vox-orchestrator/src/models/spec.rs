//! LLM model specifications, capabilities, and routing keys.
//!
//! [`ModelRegistry`](crate::models::ModelRegistry) (in `registry.rs`) uses these types for task-category routing.

use crate::types::TaskCategory;
use crate::usage::LlmUsageKey;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Records which data source last set the pricing fields on a [`ModelSpec`].
///
/// Priority (highest → lowest): `Telemetry` > `LiteLLM` > `OpenRouter` | `AnthropicDirect` > `UserConfig` > `Bootstrap`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PricingSource {
    /// Compile-time bootstrap JSON (may be stale). Used only as a cold-start fallback.
    #[default]
    Bootstrap,
    /// We know this model exists but have no pricing data.
    Unknown,
    /// Fetched from OpenRouter `/api/v1/models` at runtime.
    OpenRouter,
    /// Fetched directly from the Anthropic `/v1/models` API (key-gated).
    /// Model discovery only; pricing is still filled in by the LiteLLM oracle.
    AnthropicDirect,
    /// Supplemented from the LiteLLM `model_prices_and_context_window.json` oracle.
    /// Fills gaps OpenRouter doesn't expose (cache-hit pricing, Anthropic, Google).
    LiteLLM,
    /// User's local `models.toml` override.
    UserConfig,
    /// Calibrated from observed telemetry rollup (highest trust — observed > catalog).
    Telemetry,
}

use super::generated::{ModelTier, StrengthTag};

/// Rich capabilities for a model, imported from DeI and the OpenRouter /models catalog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ModelCapabilities {
    pub supports_json: bool,
    pub supports_vision: bool,
    #[serde(default = "vox_config::serde_defaults::default_true")]
    pub supports_native_tools: bool,
    #[serde(default)]
    pub supports_tool_use: bool,
    #[serde(default)]
    pub supports_reasoning: bool,
    #[serde(default)]
    pub supports_web_search: bool,
    #[serde(default)]
    pub supports_image_generation: bool,
    #[serde(default)]
    pub supports_audio_input: bool,
    #[serde(default)]
    pub supports_audio_output: bool,
    /// Whether the model accepts file attachments (PDFs, documents, etc.) as input.
    #[serde(default)]
    pub supports_file_input: bool,
    /// Whether the model can consume and produce JSON-Lines streams.
    #[serde(default)]
    pub supports_jsonl: bool,
    /// Whether this model is trusted to author Vox language source
    /// (`VoxLang` artifact forms). Registry-authoring flag, NOT an
    /// OpenRouter-advertised routing capability — seeded `true` only for
    /// MENS models, defaults `false` for everything else. Operators can
    /// additionally opt models in via the `[audit.route] vox_capable_models`
    /// allowlist override (see `vox-cli` `audit_route::resolve_vox_capability`).
    #[serde(default)]
    pub writes_vox: bool,
    pub max_context: u64,
    pub tier: ModelTier,
    /// Provider-reported RPM limit (e.g. from OpenRouter `per_request_limits`).
    pub rate_limit_rpm: Option<u32>,
    /// Provider-reported RPD limit (e.g. from OpenRouter `per_request_limits`).
    pub rate_limit_rpd: Option<u32>,
    /// Median response latency in milliseconds from catalog metadata (p50).
    #[serde(default)]
    pub latency_p50_ms: Option<u32>,
    /// Whether the provider applies content moderation to outputs.
    #[serde(default)]
    pub is_moderated: bool,
    /// Provider-reported uptime score 0.0–1.0 (1.0 = fully available).
    #[serde(default)]
    pub uptime_score: Option<f32>,
    /// Unix time the provider published this model (OpenRouter `/models` `created`).
    /// `None` when the source does not report it; never defaulted to 0 or "now",
    /// because recency ordering (`models::family`) must not invent a date.
    #[serde(default)]
    pub released_at: Option<u64>,
    /// Artificial Analysis intelligence index as published in OpenRouter's catalog
    /// (`benchmarks.artificial_analysis.intelligence_index`, ~0–60 today). `None` = unbenchmarked.
    #[serde(default)]
    pub intelligence_index: Option<f32>,
    /// Quality prior stamped by the registry (`ModelRegistry::register` / `apply_routing_reference`).
    /// `None` on a spec that was never registered; scoring then uses the fixed-constant formula.
    #[serde(default)]
    pub quality_prior: Option<QualityPrior>,
    /// OpenRouter lists this model as the current target of a `~vendor/…-latest` alias. An alias
    /// target is never treated as superseded, whatever `models::family::family_key` infers.
    #[serde(default)]
    pub is_alias_target: bool,
    /// Parameter count in billions, when known (e.g. parsed from Ollama's
    /// `/api/tags` `details.parameter_size` field, "8.2B" -> `8.2`). Used only
    /// as an advisory VRAM-fit signal (see `models::vram`); `None` means no
    /// signal, never "assume small" or "assume large".
    #[serde(default)]
    pub param_count_b: Option<f32>,
}

impl ModelCapabilities {
    /// Whether this catalog row advertises the given routing [`Capability`](super::generated::Capability).
    #[must_use]
    pub fn supports(&self, cap: super::generated::Capability) -> bool {
        use super::generated::Capability::*;
        match cap {
            SupportsJson => self.supports_json,
            SupportsVision => self.supports_vision,
            SupportsToolUse => self.supports_tool_use || self.supports_native_tools,
            SupportsReasoning => self.supports_reasoning,
            SupportsWebSearch => self.supports_web_search,
            SupportsImageGeneration => self.supports_image_generation,
            SupportsAudioInput => self.supports_audio_input,
            SupportsAudioOutput => self.supports_audio_output,
        }
    }

    /// OR-merge OpenRouter / contract-inferred capability flags into this struct.
    pub fn merge_capability_flags(&mut self, flags: &super::generated::CapabilityFlags) {
        self.supports_json |= flags.supports_json;
        self.supports_vision |= flags.supports_vision;
        self.supports_tool_use |= flags.supports_tool_use;
        self.supports_reasoning |= flags.supports_reasoning;
        self.supports_web_search |= flags.supports_web_search;
        self.supports_image_generation |= flags.supports_image_generation;
        self.supports_audio_input |= flags.supports_audio_input;
        self.supports_audio_output |= flags.supports_audio_output;
    }
}

/// A model's quality prior (0–1) and where it came from, so every surface can say why a model is
/// rated as it is. Stamped by the registry from its `models::reference::RoutingReference`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualityPrior {
    pub value: f64,
    pub source: QualitySource,
}

/// Provenance of a [`QualityPrior`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum QualitySource {
    /// The model's own Artificial Analysis intelligence index.
    Benchmark { index: f32 },
    /// The index of the newest benchmarked member of its family (`models::family`).
    Inherited { index: f32, from: String },
    /// No benchmark: the context-length and paid/free proxy.
    Estimate,
    /// Vox's own graded benchmark (`vox models bench`): `score` is the mean graded score over
    /// `cases`, blended with `prior` (the external value it was shrunk toward).
    Measured { score: f32, cases: u32, prior: f32 },
}

/// Specification for an LLM model in the registry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelSpec {
    /// Stable model slug used in APIs and config (e.g. `gemini-2.0-flash-lite`).
    pub id: String,
    /// The unique system-wide slug
    #[serde(default)]
    pub canonical_slug: String,
    /// Provider namespace for billing and routing (`google`, `openrouter`, …).
    pub provider: String,
    /// Which API endpoint to use: "google_direct", "openrouter", or "ollama".
    pub provider_type: ProviderType,
    /// Advertised context window / max output budget in tokens.
    pub max_tokens: u64,
    /// Simplified cost metric representing aggregate cost per 1000 tokens.
    pub cost_per_1k: f64,
    #[serde(default)]
    pub cost_per_1k_input: f64,
    #[serde(default)]
    pub cost_per_1k_output: f64,
    /// Optional ground-truth cost observed from provider telemetry (blended).
    #[serde(default)]
    pub observed_cost_per_1k: Option<f64>,
    /// Per-1k-token cost for writing a new prompt-cache prefix (e.g. 1.25× normal on Anthropic).
    /// Zero means caching is not supported or the cost is not known.
    #[serde(default)]
    pub cache_creation_cost_per_1k: f64,
    /// Per-1k-token cost for a prompt-cache hit read (typically ~10% of input cost).
    /// Zero means caching is not supported or the cost is not known.
    #[serde(default)]
    pub cache_read_cost_per_1k: f64,
    /// Whether the provider supports prompt-prefix caching for this model.
    #[serde(default)]
    pub supports_prompt_caching: bool,
    /// Which data source last set the pricing fields on this spec.
    #[serde(default)]
    pub pricing_source: PricingSource,
    /// Whether this model is free (no per-token cost).
    pub is_free: bool,
    /// Tags describing fit (speed, reasoning, codegen) for heuristic routing.
    pub strengths: Vec<StrengthTag>,
    #[serde(default)]
    pub capabilities: ModelCapabilities,
    #[serde(default)]
    pub supported_parameters: Vec<String>,
}

pub use vox_orchestrator_types::ProviderType;

pub use vox_orchestrator_types::ChatRouteBackend as ModelRouteBackend;

/// Resolve the transport/backend lane for a concrete model spec.
#[must_use]
pub fn route_backend_for_model(spec: &ModelSpec) -> ModelRouteBackend {
    match spec.provider_type {
        ProviderType::Ollama => ModelRouteBackend::Ollama,
        ProviderType::VoxLocal => ModelRouteBackend::VoxLocal,
        ProviderType::PopuliMesh => ModelRouteBackend::PopuliMesh,
        ProviderType::GoogleDirect => ModelRouteBackend::GeminiDirect,
        ProviderType::OpenRouter => ModelRouteBackend::OpenRouter,
        ProviderType::Groq
        | ProviderType::Mistral
        | ProviderType::DeepSeek
        | ProviderType::Cerebras
        | ProviderType::SambaNova
        | ProviderType::Anthropic
        | ProviderType::HuggingFaceRouter
        | ProviderType::Custom(_) => {
            // P0 Fix: Map arbitrarily typed third-party providers (even those lacking '/') to
            // OpenRouter or a non-cascading endpoint. CascadeFallback on unknown IDs loops infinitely.
            ModelRouteBackend::OpenRouter
        }
    }
}

impl ModelSpec {
    /// Whether the model supports native web-search grounding.
    ///
    /// This is a convenience forwarder to [`ModelCapabilities::supports_web_search`] so callers
    /// can query the capability directly on the spec without drilling into `.capabilities`.
    #[must_use]
    #[inline]
    pub fn supports_web_search(&self) -> bool {
        self.capabilities.supports_web_search
    }

    /// Whether the model accepts file attachments as context.
    #[must_use]
    #[inline]
    pub fn supports_file_input(&self) -> bool {
        self.capabilities.supports_file_input
    }

    /// Whether the model supports JSON-Lines streaming output.
    #[must_use]
    #[inline]
    pub fn supports_jsonl(&self) -> bool {
        self.capabilities.supports_jsonl
    }

    /// Keys for daily quota rows in `provider_usage` (aligned with `usage` module limits; OpenRouter `:free` aggregate, Ollama `*`).
    #[must_use]
    pub fn llm_usage_key(&self) -> LlmUsageKey {
        match &self.provider_type {
            ProviderType::GoogleDirect => LlmUsageKey {
                provider: "google".to_string(),
                model: self.id.clone(),
            },
            ProviderType::OpenRouter => {
                let model = if self.is_free || self.id.contains(":free") {
                    ":free".to_string()
                } else {
                    self.id.clone()
                };
                LlmUsageKey {
                    provider: "openrouter".to_string(),
                    model,
                }
            }
            ProviderType::Ollama => LlmUsageKey {
                provider: "ollama".to_string(),
                model: "*".to_string(),
            },
            ProviderType::VoxLocal => LlmUsageKey {
                provider: "vox_local".to_string(),
                model: "*".to_string(),
            },
            ProviderType::PopuliMesh => LlmUsageKey {
                provider: "mens".to_string(),
                model: "*".to_string(),
            },
            ProviderType::Groq => LlmUsageKey {
                provider: "groq".to_string(),
                model: self.id.clone(),
            },
            ProviderType::Cerebras => LlmUsageKey {
                provider: "cerebras".to_string(),
                model: self.id.clone(),
            },
            ProviderType::Mistral => LlmUsageKey {
                provider: "mistral".to_string(),
                model: self.id.clone(),
            },
            ProviderType::DeepSeek => LlmUsageKey {
                provider: "deepseek".to_string(),
                model: self.id.clone(),
            },
            ProviderType::SambaNova => LlmUsageKey {
                provider: "sambanova".to_string(),
                model: self.id.clone(),
            },
            ProviderType::Anthropic => LlmUsageKey {
                provider: "anthropic".to_string(),
                model: self.id.clone(),
            },
            ProviderType::HuggingFaceRouter => LlmUsageKey {
                provider: "huggingface".to_string(),
                model: self.id.clone(),
            },
            ProviderType::Custom(_url) => LlmUsageKey {
                provider: "custom".to_string(),
                model: self.id.clone(),
            },
        }
    }
}

fn premium_alias_toml_default() -> HashMap<String, String> {
    let m = HashMap::new();
    let _ = std::hint::black_box(m.capacity());
    m
}

/// Configuration wrapper for models.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelConfig {
    /// All models available to the orchestrator for this deployment.
    pub models: Vec<ModelSpec>,
    /// Optional premium model id per task bucket (`codegen`, `testing`, …). Empty = use ranked paid models.
    #[serde(default = "premium_alias_toml_default")]
    pub premium_alias: HashMap<String, String>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        let local_model = vox_secrets::resolve_secret(vox_secrets::SecretId::PopuliModel)
            .expose()
            .filter(|s: &&str| !s.trim().is_empty())
            .unwrap_or("default-model")
            .to_string();

        let bootstrap_json =
            include_str!("../../../../contracts/orchestration/model-catalog.bootstrap.v1.json");
        let mut models: Vec<ModelSpec> =
            serde_json::from_str(bootstrap_json).expect("Invalid bootstrap catalog");

        for m in &mut models {
            if m.id == "llama3:latest" && m.provider == "ollama" {
                m.id = local_model.clone();
                m.canonical_slug = format!("local/{}", local_model);
            }
            if m.capabilities.max_context == 0 {
                m.capabilities.max_context = m.max_tokens;
            }
        }

        Self {
            models,
            premium_alias: vox_config::load_model_routing_config().premium_alias,
        }
    }
}

use super::routing_table::route_for_category;

/// Maps [`TaskCategory`] to a `premium_alias` / routing strength key.
#[must_use]
pub fn task_category_premium_key(task_type: TaskCategory) -> &'static str {
    route_for_category(task_type).premium_alias_key
}

pub fn task_category_strength(task_type: TaskCategory) -> StrengthTag {
    route_for_category(task_type).strength_tag
}

/// Live operational health status of a model (extracted from `t3router` `getModelStatuses`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModelOperationalStatus {
    #[default]
    Operational,
    Degraded,
    Outage,
}

impl ModelOperationalStatus {
    /// Parse a T3 Chat / provider status indicator string (`"operational"`, `"degraded"`, `"outage"`, etc.).
    #[must_use]
    pub fn parse_t3_indicator(indicator: &str) -> Self {
        match indicator.trim().to_ascii_lowercase().as_str() {
            "operational" | "ok" | "healthy" | "up" => Self::Operational,
            "degraded" | "degraded_performance" | "partial_outage" | "slow" => Self::Degraded,
            "outage" | "major_outage" | "down" | "offline" | "disabled" => Self::Outage,
            _ => Self::Operational,
        }
    }

    #[must_use]
    pub fn is_operational(self) -> bool {
        matches!(self, Self::Operational)
    }

    #[must_use]
    pub fn is_routable(self) -> bool {
        !matches!(self, Self::Outage)
    }
}

/// Normalized reasoning effort level across T3 Chat, OpenRouter, and OpenAI wire protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    Low,
    Medium,
    High,
}

impl ReasoningEffort {
    #[must_use]
    pub fn as_wire_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    #[must_use]
    pub fn from_wire_str(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "low" | "minimal" => Some(Self::Low),
            "medium" | "med" => Some(Self::Medium),
            "high" | "max" => Some(Self::High),
            _ => None,
        }
    }
}

/// Billing and credential tier provenance for a model turn (`t3router` + `thom-chat` synthesis).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostProvenanceTier {
    /// Local execution (`VoxLocal`, `Ollama`, `PopuliMesh`) — zero external spend.
    LocalZeroCost,
    /// Cloud free tier (`:free` model suffix or `ModelSpec::is_free`).
    FreeTier,
    /// User-supplied Bring-Your-Own-Key (`BYOK`).
    Byok,
    /// Managed platform / subscription credit pool (`t3.chat` / shared key).
    PlatformCredits,
}

/// Deterministic per-turn cost and credit-delta receipt (`VOX-ROUTER-T3-03`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurnCostDeltaReceipt {
    pub model_id: String,
    pub tier: CostProvenanceTier,
    pub cost_usd_delta: f64,
    pub credits_deducted: Option<f64>,
    pub credits_remaining: Option<f64>,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub cache_read_tokens: u32,
}

impl TurnCostDeltaReceipt {
    /// Classify the billing tier for `spec` given whether a user-supplied BYOK credential was used.
    #[must_use]
    pub fn classify_tier(
        spec: Option<&ModelSpec>,
        model_id: &str,
        has_user_byok: bool,
    ) -> CostProvenanceTier {
        if let Some(s) = spec {
            if matches!(
                s.provider_type,
                ProviderType::Ollama | ProviderType::VoxLocal | ProviderType::PopuliMesh
            ) {
                return CostProvenanceTier::LocalZeroCost;
            }
            if has_user_byok {
                return CostProvenanceTier::Byok;
            }
            if s.is_free || s.id.ends_with(":free") {
                return CostProvenanceTier::FreeTier;
            }
            return CostProvenanceTier::PlatformCredits;
        }
        if has_user_byok {
            CostProvenanceTier::Byok
        } else if model_id.ends_with(":free") {
            CostProvenanceTier::FreeTier
        } else {
            CostProvenanceTier::PlatformCredits
        }
    }
}

/// Event kind for server-resumable turn stream buffering (`VOX-HARNESS-T3-01`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TurnStreamEventKind {
    TextDelta,
    ReasoningDelta,
    ToolCallStart { id: String, name: String },
    ToolCallArgsDelta { id: String },
    Finish { reason: String },
    Cancelled,
}

/// A sequenced turn stream chunk buffered on the server so clients can reconnect or reload
/// and replay missed events via `after_sequence` (`t3code` + `thom-chat` synthesis).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SequencedTurnChunk {
    pub sequence: u64,
    pub kind: TurnStreamEventKind,
    pub payload: String,
}

#[derive(Debug, Default)]
struct ActiveTurnStream {
    run_id: String,
    generating: bool,
    cancelled: bool,
    chunks: Vec<SequencedTurnChunk>,
    subscribers: Vec<std::sync::mpsc::Sender<SequencedTurnChunk>>,
}

impl Clone for ActiveTurnStream {
    fn clone(&self) -> Self {
        Self {
            run_id: self.run_id.clone(),
            generating: self.generating,
            cancelled: self.cancelled,
            chunks: self.chunks.clone(),
            subscribers: Vec::new(),
        }
    }
}

/// Server-resumable turn stream buffer and cancellation registry (`VOX-HARNESS-T3-01` & `T3-STREAM-GUEST-01`).
///
/// Uses a per-turn `Arc<std::sync::Mutex<ActiveTurnStream>>` (`NotT3Chat` `SemaphoreSlim(1)` pattern)
/// so `subscribe_or_replay` atomically snapshots buffered chunks and registers a live subscriber channel
/// before releasing the lock—guaranteeing zero dropped or duplicated tokens on mid-stream multi-tab join.
#[derive(Debug, Clone, Default)]
pub struct ResumableTurnStreamRegistry {
    streams: HashMap<String, std::sync::Arc<std::sync::Mutex<ActiveTurnStream>>>,
}

impl ResumableTurnStreamRegistry {
    /// Start a new background turn stream for `thread_id`.
    pub fn start_turn(&mut self, thread_id: impl Into<String>, run_id: impl Into<String>) {
        self.streams.insert(
            thread_id.into(),
            std::sync::Arc::new(std::sync::Mutex::new(ActiveTurnStream {
                run_id: run_id.into(),
                generating: true,
                cancelled: false,
                chunks: Vec::new(),
                subscribers: Vec::new(),
            })),
        );
    }

    /// Append a chunk to `thread_id`'s active stream, broadcast to all live subscribers under the per-turn lock,
    /// and return its 1-based sequence number.
    pub fn append_chunk(
        &mut self,
        thread_id: &str,
        kind: TurnStreamEventKind,
        payload: impl Into<String>,
    ) -> u64 {
        let slot = self
            .streams
            .entry(thread_id.to_string())
            .or_insert_with(|| {
                std::sync::Arc::new(std::sync::Mutex::new(ActiveTurnStream::default()))
            })
            .clone();
        let mut entry = slot.lock().unwrap_or_else(|e| e.into_inner());
        let next_seq = (entry.chunks.len() as u64) + 1;
        if matches!(
            kind,
            TurnStreamEventKind::Finish { .. } | TurnStreamEventKind::Cancelled
        ) {
            entry.generating = false;
        }
        let chunk = SequencedTurnChunk {
            sequence: next_seq,
            kind,
            payload: payload.into(),
        };
        entry.chunks.push(chunk.clone());
        entry
            .subscribers
            .retain(|tx| tx.send(chunk.clone()).is_ok());
        next_seq
    }

    /// Atomically replay all buffered chunks strictly after `after_sequence` and register a live
    /// receiver channel under the same per-turn `Mutex` (`NotT3Chat` `SemaphoreSlim(1)` pattern).
    pub fn subscribe_or_replay(
        &self,
        thread_id: &str,
        after_sequence: u64,
    ) -> Option<(
        Vec<SequencedTurnChunk>,
        std::sync::mpsc::Receiver<SequencedTurnChunk>,
    )> {
        let slot = self.streams.get(thread_id)?;
        let mut entry = slot.lock().unwrap_or_else(|e| e.into_inner());
        let replayed: Vec<SequencedTurnChunk> = entry
            .chunks
            .iter()
            .filter(|c| c.sequence > after_sequence)
            .cloned()
            .collect();
        let (tx, rx) = std::sync::mpsc::channel();
        if entry.generating {
            entry.subscribers.push(tx);
        }
        Some((replayed, rx))
    }

    /// Replay all buffered chunks for `thread_id` strictly after `after_sequence` (`sequence > after_sequence`).
    #[must_use]
    pub fn replay_after(&self, thread_id: &str, after_sequence: u64) -> Vec<SequencedTurnChunk> {
        self.streams
            .get(thread_id)
            .map(|slot| {
                let s = slot.lock().unwrap_or_else(|e| e.into_inner());
                s.chunks
                    .iter()
                    .filter(|c| c.sequence > after_sequence)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Reconstruct the accumulated assistant text and reasoning up to the latest sequence.
    #[must_use]
    pub fn accumulated_snapshot(&self, thread_id: &str) -> (String, String, u64) {
        let Some(slot) = self.streams.get(thread_id) else {
            return (String::new(), String::new(), 0);
        };
        let s = slot.lock().unwrap_or_else(|e| e.into_inner());
        let mut text = String::new();
        let mut reasoning = String::new();
        let mut last_seq = 0;
        for c in &s.chunks {
            last_seq = c.sequence;
            match c.kind {
                TurnStreamEventKind::TextDelta => text.push_str(&c.payload),
                TurnStreamEventKind::ReasoningDelta => reasoning.push_str(&c.payload),
                _ => {}
            }
        }
        (text, reasoning, last_seq)
    }

    /// Cancel an in-flight generation for `thread_id` (`thom-chat` `cancel-generation` pattern).
    pub fn cancel_turn(&mut self, thread_id: &str) -> bool {
        let Some(slot) = self.streams.get(thread_id) else {
            return false;
        };
        let mut s = slot.lock().unwrap_or_else(|e| e.into_inner());
        if !s.generating {
            return false;
        }
        s.generating = false;
        s.cancelled = true;
        let next_seq = (s.chunks.len() as u64) + 1;
        let chunk = SequencedTurnChunk {
            sequence: next_seq,
            kind: TurnStreamEventKind::Cancelled,
            payload: "cancelled_by_user".to_string(),
        };
        s.chunks.push(chunk.clone());
        s.subscribers.retain(|tx| tx.send(chunk.clone()).is_ok());
        true
    }

    #[must_use]
    pub fn is_generating(&self, thread_id: &str) -> bool {
        self.streams
            .get(thread_id)
            .is_some_and(|slot| slot.lock().unwrap_or_else(|e| e.into_inner()).generating)
    }

    #[must_use]
    pub fn is_cancelled(&self, thread_id: &str) -> bool {
        self.streams
            .get(thread_id)
            .is_some_and(|slot| slot.lock().unwrap_or_else(|e| e.into_inner()).cancelled)
    }

    #[must_use]
    pub fn run_id(&self, thread_id: &str) -> Option<String> {
        self.streams.get(thread_id).map(|slot| {
            slot.lock()
                .unwrap_or_else(|e| e.into_inner())
                .run_id
                .clone()
        })
    }
}

/// Session privacy and persistence posture (`helioschat` `temporary` + `thom-chat` `privacy_mode`, `VOX-HARNESS-T3-03`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SessionPrivacyPosture {
    /// Standard persistent session with database and memory sync enabled.
    #[default]
    Standard,
    /// Temporary / incognito session: zero database writes and zero long-term memory persistence.
    Ephemeral,
    /// Local-only privacy session: persists locally only and enforces `CandidateScope::LocalOnly`.
    LocalPrivate,
}

impl SessionPrivacyPosture {
    #[must_use]
    pub fn should_persist_to_db(self) -> bool {
        !matches!(self, Self::Ephemeral)
    }

    #[must_use]
    pub fn should_persist_memories(self) -> bool {
        matches!(self, Self::Standard)
    }

    #[must_use]
    pub fn requires_local_only_models(self) -> bool {
        matches!(self, Self::LocalPrivate)
    }
}

/// Receipt produced by pre-send spoke-aware prompt enhancement (`VOX-HARNESS-T3-03`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnhancedPromptReceipt {
    pub original_prompt: String,
    pub enhanced_prompt: String,
    pub preserved_rule_tags: Vec<String>,
    pub target_spoke: String,
}

#[cfg(test)]
mod writes_vox_tests {
    use super::ModelCapabilities;

    #[test]
    fn default_writes_vox_is_false() {
        assert!(!ModelCapabilities::default().writes_vox);
    }

    #[test]
    fn missing_key_deserializes_to_false() {
        // A catalog/bootstrap JSON that omits `writes_vox` must remain
        // back-compat: the bare `#[serde(default)]` yields bool's Default (false).
        let json = r#"{
            "supports_json": true,
            "supports_vision": false,
            "max_context": 8192,
            "tier": "local"
        }"#;
        let caps: ModelCapabilities = serde_json::from_str(json).expect("deserialize");
        assert!(!caps.writes_vox);
    }

    #[test]
    fn explicit_true_deserializes_to_true() {
        let json = r#"{
            "supports_json": true,
            "supports_vision": false,
            "writes_vox": true,
            "max_context": 8192,
            "tier": "local"
        }"#;
        let caps: ModelCapabilities = serde_json::from_str(json).expect("deserialize");
        assert!(caps.writes_vox);
    }

    #[test]
    fn mens_style_seed_reports_writes_vox_true() {
        // Mirrors the MENS seed sites (catalog.rs / registry.rs): a `Local`-tier
        // capability with `writes_vox: true` plus `..Default::default()`.
        let caps = ModelCapabilities {
            tier: super::ModelTier::Local,
            writes_vox: true,
            ..Default::default()
        };
        assert!(caps.writes_vox);
    }
}
