//! Environment resolution for **inference providers** (local Mens/Ollama and cloud keys).
//!
//! This module is the **SSOT** for reading env vars used across CLI, MCP, and runtime. Callers that
//! need HTTP probes (health, model lists) use `vox_actor_runtime::inference_env::probe_populi_capabilities`.

use crate::snapshot::SnapshotCache;

/// Where chat / completion traffic is expected to run (desktop daemon vs cloud vs on-device).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InferenceProfile {
    /// Default: local Ollama-compatible HTTP (`OLLAMA_HOST` / `POPULI_URL` / localhost).
    #[default]
    DesktopOllama,
    /// OpenRouter / HF / other OpenAI-compatible cloud endpoints from config.
    CloudOpenAiCompatible,
    /// On-device LiteRT-LM (app-owned runtime).
    MobileLitert,
    /// Apple Core ML (app-owned).
    MobileCoreml,
    /// Ollama or compatible gateway on LAN (explicit base URL).
    LanGateway,
}

impl InferenceProfile {
    /// Whether tooling may probe and call **local** Ollama-compatible HTTP (loopback or `OLLAMA_HOST`).
    #[must_use]
    pub const fn allows_local_ollama_http(self) -> bool {
        matches!(self, Self::DesktopOllama | Self::LanGateway)
    }
}

static INFERENCE_PROFILE_CACHE: SnapshotCache<InferenceProfile> = SnapshotCache::new();

/// Read [`InferenceProfile`] from **`vox_populi::inference_PROFILE`** (case-insensitive).
#[must_use]
pub fn inference_profile_from_env() -> InferenceProfile {
    INFERENCE_PROFILE_CACHE.get_or_init(|| {
        let raw =
            crate::env_parse::resolve_config_str("vox_populi::inference_PROFILE", "desktop_ollama");
        let raw = raw.trim().to_ascii_lowercase();
        match raw.as_str() {
            "cloud_openai_compatible" | "cloud" => InferenceProfile::CloudOpenAiCompatible,
            "mobile_litert" | "litert" => InferenceProfile::MobileLitert,
            "mobile_coreml" | "coreml" => InferenceProfile::MobileCoreml,
            "lan_gateway" | "lan" => InferenceProfile::LanGateway,
            _ => InferenceProfile::DesktopOllama,
        }
    })
}

/// Whether MCP / other HTTP clients may use **local** Ollama (`vox_populi::inference_PROFILE`).
#[must_use]
pub fn inference_profile_allows_local_ollama_http() -> bool {
    inference_profile_from_env().allows_local_ollama_http()
}

/// OpenRouter chat completions endpoint (OpenAI-compatible).
pub const OPENROUTER_CHAT_COMPLETIONS_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
/// OpenRouter models list endpoint used for catalog discovery.
pub const OPENROUTER_MODELS_LIST_URL: &str = "https://openrouter.ai/api/v1/models";
/// OpenRouter embeddings endpoint (OpenAI-compatible).
pub const OPENROUTER_EMBEDDINGS_URL: &str = "https://openrouter.ai/api/v1/embeddings";
/// OpenAI chat completions endpoint.
pub const OPENAI_CHAT_COMPLETIONS_URL: &str = "https://api.openai.com/v1/chat/completions";
/// OpenAI embeddings endpoint.
pub const OPENAI_EMBEDDINGS_URL: &str = "https://api.openai.com/v1/embeddings";
/// Local Ollama/Populi base URL fallback.
pub const LOCAL_OLLAMA_POPULI_BASE_URL_DEFAULT: &str = "http://localhost:11434";

/// OpenRouter API base URL (config-aware: `OPENROUTER_BASE_URL` → config.toml → default).
///
/// Default `https://openrouter.ai/api`; the `/v1/...` suffixes are appended by the
/// endpoint accessors so that defaults match the legacy `OPENROUTER_*_URL` consts byte-for-byte.
static OPENROUTER_BASE_CACHE: SnapshotCache<String> = SnapshotCache::new();

#[must_use]
pub fn openrouter_base_url() -> String {
    OPENROUTER_BASE_CACHE.get_or_init(|| {
        let resolved = crate::env_parse::resolve_config_str(
            "OPENROUTER_BASE_URL",
            "https://openrouter.ai/api",
        );
        sanitize_base_url(
            &resolved,
            "https://openrouter.ai/api",
            "OPENROUTER_BASE_URL",
        )
    })
}

/// Validate a user-supplied base URL: trim trailing slashes and, if the result is
/// empty or has no `scheme://`, fall back to `default` (warning once).
fn sanitize_base_url(resolved: &str, default: &str, key: &str) -> String {
    let trimmed = resolved.trim_end_matches('/');
    if trimmed.is_empty() || !trimmed.contains("://") {
        static WARNED: std::sync::Once = std::sync::Once::new();
        WARNED.call_once(|| {
            tracing::warn!(
                config_key = key,
                value = resolved,
                "ignoring malformed base URL (empty or missing scheme); using default"
            );
        });
        return default.to_string();
    }
    trimmed.to_string()
}

/// OpenAI-compatible API base URL.
///
/// Precedence: `VOX_OPENAI_BASE_URL` → legacy `OPENAI_BASE_URL` → config.toml → default
/// `https://api.openai.com/v1`. Endpoint accessors append the path suffix.
static OPENAI_COMPAT_BASE_CACHE: SnapshotCache<String> = SnapshotCache::new();

#[must_use]
pub fn openai_compatible_base_url() -> String {
    OPENAI_COMPAT_BASE_CACHE.get_or_init(|| {
        let legacy =
            crate::env_parse::resolve_config_str("OPENAI_BASE_URL", "https://api.openai.com/v1");
        let resolved = crate::env_parse::resolve_config_str("VOX_OPENAI_BASE_URL", &legacy);
        sanitize_base_url(
            &resolved,
            "https://api.openai.com/v1",
            "VOX_OPENAI_BASE_URL",
        )
    })
}

/// OpenRouter chat completions endpoint (config-aware). Default equals
/// [`OPENROUTER_CHAT_COMPLETIONS_URL`].
#[must_use]
pub fn openrouter_chat_completions_url() -> String {
    format!("{}/v1/chat/completions", openrouter_base_url())
}

/// OpenRouter models list endpoint (config-aware). Default equals
/// [`OPENROUTER_MODELS_LIST_URL`].
#[must_use]
pub fn openrouter_models_list_url() -> String {
    format!("{}/v1/models", openrouter_base_url())
}

/// OpenRouter embeddings endpoint (config-aware). Default equals
/// [`OPENROUTER_EMBEDDINGS_URL`].
#[must_use]
pub fn openrouter_embeddings_url() -> String {
    format!("{}/v1/embeddings", openrouter_base_url())
}

/// OpenAI chat completions endpoint (config-aware). Default equals
/// [`OPENAI_CHAT_COMPLETIONS_URL`].
#[must_use]
pub fn openai_chat_completions_url() -> String {
    format!("{}/chat/completions", openai_compatible_base_url())
}

/// OpenAI embeddings endpoint (config-aware). Default equals
/// [`OPENAI_EMBEDDINGS_URL`].
#[must_use]
pub fn openai_embeddings_url() -> String {
    format!("{}/embeddings", openai_compatible_base_url())
}

/// Default VoxLocal (`vox mens serve`) base URL — same port as Ollama for co-location.
pub const VOX_LOCAL_ENDPOINT_DEFAULT: &str = "http://127.0.0.1:11434";

/// Alternate loopback port when Ollama already binds `:11434` (macOS Metal how-to).
pub const VOX_LOCAL_ENDPOINT_OLLAMA_CONFLICT_ALT: &str = "http://127.0.0.1:11435";

/// Probe order when **`VOX_LOCAL_ENDPOINT`** is unset (first `vox-ml-cli` health wins).
pub const VOX_LOCAL_ENDPOINT_PROBE_CANDIDATES: &[&str] = &[
    VOX_LOCAL_ENDPOINT_DEFAULT,
    VOX_LOCAL_ENDPOINT_OLLAMA_CONFLICT_ALT,
];

/// Trim trailing slashes from a VoxLocal base URL.
#[must_use]
pub fn normalize_vox_local_base_url(base: &str) -> String {
    base.trim().trim_end_matches('/').to_string()
}

/// Returns VoxLocal base URLs to probe, in order.
///
/// When `explicit` is [`Some`], only that URL is returned (honors `VOX_LOCAL_ENDPOINT`
/// override). When [`None`], returns [`VOX_LOCAL_ENDPOINT_PROBE_CANDIDATES`].
#[must_use]
pub fn vox_local_endpoint_probe_candidates_from(explicit: Option<&str>) -> Vec<String> {
    if let Some(url) = explicit {
        let trimmed = url.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }
        return vec![normalize_vox_local_base_url(trimmed)];
    }
    VOX_LOCAL_ENDPOINT_PROBE_CANDIDATES
        .iter()
        .map(|candidate| normalize_vox_local_base_url(candidate))
        .collect()
}

/// VoxLocal probe targets: explicit **`VOX_LOCAL_ENDPOINT`** or the default candidate list.
#[must_use]
pub fn vox_local_endpoint_probe_candidates() -> Vec<String> {
    vox_local_endpoint_probe_candidates_from(std::env::var("VOX_LOCAL_ENDPOINT").ok().as_deref())
}

/// True when a `/health` or `/ready` body identifies `vox mens serve` (`vox-ml-cli`).
#[must_use]
pub fn vox_local_health_identifies_serve(body: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| {
            v.get("service")
                .and_then(|s| s.as_str())
                .map(|s| s == "vox-ml-cli")
        })
        .unwrap_or(false)
}

/// Local Ollama-compatible API base URL.
///
/// Precedence: **`VOX_POPULI_LOCAL_OLLAMA_URL`** → **`POPULI_URL`** → **`OLLAMA_URL`** → `http://localhost:11434`.
static LOCAL_OLLAMA_BASE_CACHE: SnapshotCache<String> = SnapshotCache::new();

pub fn local_ollama_populi_base_url() -> String {
    LOCAL_OLLAMA_BASE_CACHE.get_or_init(|| {
        if let Some(secret) =
            vox_secrets::resolve_secret(vox_secrets::SecretId::VoxPopuliLocalOllamaUrl)
                .expose()
                .map(std::string::ToString::to_string)
        {
            return secret;
        }
        const UNSET: &str = "\u{0}__vox_unset__";
        let populi = crate::env_parse::resolve_config_str("POPULI_URL", UNSET);
        if populi != UNSET {
            return populi;
        }
        let ollama = crate::env_parse::resolve_config_str("OLLAMA_URL", UNSET);
        if ollama != UNSET {
            return ollama;
        }
        LOCAL_OLLAMA_POPULI_BASE_URL_DEFAULT.to_string()
    })
}

/// Hugging Face Hub / Inference token for router and Hub APIs.
///
/// Precedence: **`HF_TOKEN`** → **`HUGGING_FACE_HUB_TOKEN`**.
pub fn huggingface_hub_token() -> Option<String> {
    vox_secrets::resolve_env_only(vox_secrets::SecretId::HuggingFaceToken)
        .expose()
        .map(std::string::ToString::to_string)
}

/// OpenRouter API key (`OPENROUTER_API_KEY`).
pub fn openrouter_api_key() -> Option<String> {
    vox_secrets::resolve_secret(vox_secrets::SecretId::OpenRouterApiKey)
        .expose()
        .map(std::string::ToString::to_string)
}

/// True when research should try the OpenRouter **free tier first**
/// (`VOX_RESEARCH_PREFER_FREE_TIER`). This only REORDERS candidates — the free
/// tier is always present as a fallback floor regardless of this flag. Accepts
/// `1`/`true`/`yes`/`on` (case-insensitive, trimmed); unset/other → `false`.
///
/// Non-secret behavioral flag, read from the environment like `VOX_SELECTOR_MODEL`
/// in `vox-actor-runtime::model_resolution`.
#[must_use]
pub fn research_prefer_free_tier() -> bool {
    research_prefer_free_tier_from(
        std::env::var("VOX_RESEARCH_PREFER_FREE_TIER")
            .ok()
            .as_deref(),
    )
}

/// Pure parser for [`research_prefer_free_tier`] — testable without the environment.
#[must_use]
pub(crate) fn research_prefer_free_tier_from(raw: Option<&str>) -> bool {
    matches!(
        raw.map(|v| v.trim().to_ascii_lowercase()).as_deref(),
        Some("1" | "true" | "yes" | "on")
    )
}

/// Preferred Hugging Face **router** model id for chat when policy selects HF (`HF_CHAT_MODEL`).
static HF_CHAT_MODEL_PREF_CACHE: SnapshotCache<Option<String>> = SnapshotCache::new();

pub fn hf_chat_model_preference() -> Option<String> {
    HF_CHAT_MODEL_PREF_CACHE
        .get_or_init(|| crate::secrets::secrets_str(vox_secrets::SecretId::VoxHfChatModel))
}

/// Preferred OpenRouter model id when policy selects OpenRouter (`OPENROUTER_CHAT_MODEL`).
///
/// Falls back to [`crate::bootstrap_inference::OPENROUTER_AUTO`] when unset.
pub fn openrouter_chat_model_preference() -> String {
    crate::routing_migration::trace_openrouter_chat_env_migration_once();
    let preferred = crate::secrets::secrets_str(vox_secrets::SecretId::VoxOpenRouterChatModel)
        .or_else(|| crate::secrets::secrets_str(vox_secrets::SecretId::OpenRouterGeminiModel));
    crate::routing_policy::resolve_openrouter_model(preferred)
}

/// OpenAI-compatible chat completions URL for a **pinned** Hugging Face Inference Endpoint
/// (`HF_DEDICATED_CHAT_URL`), when policy should prefer dedicated over the shared router.
static HF_DEDICATED_CHAT_URL_CACHE: SnapshotCache<Option<String>> = SnapshotCache::new();

pub fn hf_dedicated_chat_completions_url() -> Option<String> {
    HF_DEDICATED_CHAT_URL_CACHE
        .get_or_init(|| crate::secrets::secrets_str(vox_secrets::SecretId::VoxHfDedicatedChatUrl))
}

/// Model id sent in the JSON body for [`hf_dedicated_chat_completions_url`] (`HF_DEDICATED_CHAT_MODEL`).
static HF_DEDICATED_CHAT_MODEL_CACHE: SnapshotCache<Option<String>> = SnapshotCache::new();

pub fn hf_dedicated_chat_model() -> Option<String> {
    HF_DEDICATED_CHAT_MODEL_CACHE
        .get_or_init(|| crate::secrets::secrets_str(vox_secrets::SecretId::VoxHfDedicatedChatModel))
}

/// Canonical HF Inference Providers router chat completions URL (override via secrets `VOX_HF_ROUTER_CHAT_COMPLETIONS_URL`).
static HF_ROUTER_URL_CACHE: SnapshotCache<String> = SnapshotCache::new();

#[must_use]
pub fn hf_router_chat_completions_url() -> String {
    HF_ROUTER_URL_CACHE.get_or_init(|| {
        crate::secrets::secrets_str(vox_secrets::SecretId::VoxHfRouterChatCompletionsUrl)
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "https://router.huggingface.co/v1/chat/completions".to_string())
    })
}

/// Sanitize a string for ChatML formatting by replacing control tokens that could
/// trigger prompt injection (e.g., `<|im_start|>`, `<|im_end|>`).
#[must_use]
pub fn sanitize_chatml(input: &str) -> String {
    input
        .replace("<|im_start|>", "[im_start]")
        .replace("<|im_end|>", "[im_end]")
}

/// Resolve a tuning f32: secret first (request/env via `vox_secrets`), then `~/.vox/config.toml`
/// under the canonical env name, else `None`. Preserves the Optional semantics.
fn tuning_f32(secret: vox_secrets::SecretId, canonical_env: &str) -> Option<f32> {
    if let Some(v) = vox_secrets::resolve_secret(secret)
        .expose()
        .and_then(|s| s.parse::<f32>().ok())
    {
        return Some(v);
    }
    crate::env_parse::resolve_config_opt_f32(canonical_env)
}

/// Resolve a tuning i32: secret first, then `~/.vox/config.toml`, else `None`.
fn tuning_i32(secret: vox_secrets::SecretId, canonical_env: &str) -> Option<i32> {
    if let Some(v) = vox_secrets::resolve_secret(secret)
        .expose()
        .and_then(|s| s.parse::<i32>().ok())
    {
        return Some(v);
    }
    crate::env_parse::resolve_config_opt_i32(canonical_env)
}

static TOGETHER_TUNING_TEMP_CACHE: SnapshotCache<Option<f32>> = SnapshotCache::new();
static TOGETHER_TUNING_TOP_P_CACHE: SnapshotCache<Option<f32>> = SnapshotCache::new();
static GEMINI_TUNING_TEMP_CACHE: SnapshotCache<Option<f32>> = SnapshotCache::new();
static GEMINI_TUNING_TOP_P_CACHE: SnapshotCache<Option<f32>> = SnapshotCache::new();
static OLLAMA_TUNING_TEMP_CACHE: SnapshotCache<Option<f32>> = SnapshotCache::new();
static OLLAMA_TUNING_TOP_P_CACHE: SnapshotCache<Option<f32>> = SnapshotCache::new();
static OPENAI_TUNING_TEMP_CACHE: SnapshotCache<Option<f32>> = SnapshotCache::new();
static OPENAI_TUNING_TOP_P_CACHE: SnapshotCache<Option<f32>> = SnapshotCache::new();
static ANTHROPIC_TUNING_TEMP_CACHE: SnapshotCache<Option<f32>> = SnapshotCache::new();
static ANTHROPIC_TUNING_TOP_P_CACHE: SnapshotCache<Option<f32>> = SnapshotCache::new();
static OLLAMA_TUNING_NUM_CTX_CACHE: SnapshotCache<Option<i32>> = SnapshotCache::new();

/// Temperature for Together AI inference.
pub fn together_tuning_temperature() -> Option<f32> {
    TOGETHER_TUNING_TEMP_CACHE.get_or_init(|| {
        tuning_f32(
            vox_secrets::SecretId::TogetherTuningTemperature,
            "TOGETHER_TUNING_TEMPERATURE",
        )
    })
}

/// Top-P for Together AI inference.
pub fn together_tuning_top_p() -> Option<f32> {
    TOGETHER_TUNING_TOP_P_CACHE.get_or_init(|| {
        tuning_f32(
            vox_secrets::SecretId::TogetherTuningTopP,
            "TOGETHER_TUNING_TOP_P",
        )
    })
}

/// Temperature for Gemini inference.
pub fn gemini_tuning_temperature() -> Option<f32> {
    GEMINI_TUNING_TEMP_CACHE.get_or_init(|| {
        tuning_f32(
            vox_secrets::SecretId::GeminiTuningTemperature,
            "GEMINI_TUNING_TEMPERATURE",
        )
    })
}

/// Top-P for Gemini inference.
pub fn gemini_tuning_top_p() -> Option<f32> {
    GEMINI_TUNING_TOP_P_CACHE.get_or_init(|| {
        tuning_f32(
            vox_secrets::SecretId::GeminiTuningTopP,
            "GEMINI_TUNING_TOP_P",
        )
    })
}

/// Temperature for Ollama inference.
pub fn ollama_tuning_temperature() -> Option<f32> {
    OLLAMA_TUNING_TEMP_CACHE.get_or_init(|| {
        tuning_f32(
            vox_secrets::SecretId::OllamaTuningTemperature,
            "OLLAMA_TUNING_TEMPERATURE",
        )
    })
}

/// Top-P for Ollama inference.
pub fn ollama_tuning_top_p() -> Option<f32> {
    OLLAMA_TUNING_TOP_P_CACHE.get_or_init(|| {
        tuning_f32(
            vox_secrets::SecretId::OllamaTuningTopP,
            "OLLAMA_TUNING_TOP_P",
        )
    })
}

/// Temperature for OpenAI inference.
pub fn openai_tuning_temperature() -> Option<f32> {
    OPENAI_TUNING_TEMP_CACHE.get_or_init(|| {
        tuning_f32(
            vox_secrets::SecretId::OpenaiTuningTemperature,
            "OPENAI_TUNING_TEMPERATURE",
        )
    })
}

/// Top-P for OpenAI inference.
pub fn openai_tuning_top_p() -> Option<f32> {
    OPENAI_TUNING_TOP_P_CACHE.get_or_init(|| {
        tuning_f32(
            vox_secrets::SecretId::OpenaiTuningTopP,
            "OPENAI_TUNING_TOP_P",
        )
    })
}

/// Temperature for Anthropic inference.
pub fn anthropic_tuning_temperature() -> Option<f32> {
    ANTHROPIC_TUNING_TEMP_CACHE.get_or_init(|| {
        tuning_f32(
            vox_secrets::SecretId::AnthropicTuningTemperature,
            "ANTHROPIC_TUNING_TEMPERATURE",
        )
    })
}

/// Top-P for Anthropic inference.
pub fn anthropic_tuning_top_p() -> Option<f32> {
    ANTHROPIC_TUNING_TOP_P_CACHE.get_or_init(|| {
        tuning_f32(
            vox_secrets::SecretId::AnthropicTuningTopP,
            "ANTHROPIC_TUNING_TOP_P",
        )
    })
}

/// Context size for Ollama inference.
pub fn ollama_tuning_num_ctx() -> Option<i32> {
    OLLAMA_TUNING_NUM_CTX_CACHE.get_or_init(|| {
        tuning_i32(
            vox_secrets::SecretId::OllamaTuningNumCtx,
            "OLLAMA_TUNING_NUM_CTX",
        )
    })
}

/// Strict model pin shared by chat and research: `VOX_MODEL_FORCE`, read from the
/// environment first, then `~/.vox/config.toml`. `None` when unset or blank.
#[must_use]
pub fn forced_model() -> Option<String> {
    forced_model_from(&crate::env_parse::resolve_config_str("VOX_MODEL_FORCE", ""))
}

/// Pure half of [`forced_model`]: trims, and treats blank as unset.
#[must_use]
pub fn forced_model_from(raw: &str) -> Option<String> {
    let v = raw.trim();
    (!v.is_empty()).then(|| v.to_string())
}

/// A role a model can be pinned for independently (Task 13). Each role reads
/// its own key first; research roles then fall back to the research group pin;
/// every role finally falls back to the single global pin (`VOX_MODEL_FORCE`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModelRole {
    /// The chat reply model.
    Chat,
    /// Research query planner / subquery decomposition.
    Planner,
    /// Research answer synthesis.
    Synthesis,
    /// Research LLM-as-judge quality score.
    Judge,
    /// Research claim extraction and claim verification.
    Verifier,
}

impl ModelRole {
    pub const ALL: [Self; 5] = [
        Self::Chat,
        Self::Planner,
        Self::Synthesis,
        Self::Judge,
        Self::Verifier,
    ];

    /// The role's own pin key.
    #[must_use]
    pub const fn env_key(self) -> &'static str {
        match self {
            Self::Chat => "VOX_MODEL_FORCE_CHAT",
            Self::Planner => "VOX_MODEL_FORCE_PLANNER",
            Self::Synthesis => "VOX_MODEL_FORCE_SYNTHESIS",
            Self::Judge => "VOX_MODEL_FORCE_JUDGE",
            Self::Verifier => "VOX_MODEL_FORCE_VERIFIER",
        }
    }

    /// The group pin a research role falls back to before the global pin.
    #[must_use]
    pub const fn group_key(self) -> Option<&'static str> {
        match self {
            Self::Chat => None,
            _ => Some("VOX_MODEL_FORCE_RESEARCH"),
        }
    }

    /// Lower-case label used in traces (`chat`, `planner`, …).
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Chat => "chat",
            Self::Planner => "planner",
            Self::Synthesis => "synthesis",
            Self::Judge => "judge",
            Self::Verifier => "verifier",
        }
    }
}

/// Pure half of the role-scoped pin: the role value wins, else the fallback;
/// blanks are unset.
#[must_use]
pub fn resolve_role_pin(role_value: Option<&str>, fallback: Option<&str>) -> Option<String> {
    forced_model_from(role_value.unwrap_or(""))
        .or_else(|| forced_model_from(fallback.unwrap_or("")))
}

/// Strict model pin for `role`: its own key, then (research roles) the
/// `VOX_MODEL_FORCE_RESEARCH` group pin, then the global [`forced_model`].
/// Every key is read env-first, then `~/.vox/config.toml`. Strictness is the
/// caller's: a pinned model that is missing or gated must still error.
#[must_use]
pub fn forced_model_for(role: ModelRole) -> Option<String> {
    let read = |key: &str| crate::env_parse::resolve_config_str(key, "");
    let global = read("VOX_MODEL_FORCE");
    let fallback = match role.group_key() {
        Some(group) => resolve_role_pin(Some(&read(group)), Some(&global)),
        None => forced_model_from(&global),
    };
    resolve_role_pin(Some(&read(role.env_key())), fallback.as_deref())
}

#[cfg(test)]
#[allow(unsafe_code)] // serialized with TEST_ENV_LOCK
mod tests {
    use super::*;
    use crate::toml_config::test_support::{CONFIG_TEST_LOCK as TEST_ENV_LOCK, HomeGuard};

    #[test]
    fn forced_model_from_trims_and_rejects_blank() {
        assert_eq!(
            super::forced_model_from("  google/gemini-3.8-flash \n"),
            Some("google/gemini-3.8-flash".to_string())
        );
        assert_eq!(super::forced_model_from("   "), None);
        assert_eq!(super::forced_model_from(""), None);
    }

    /// Task 13: pure pin precedence — the role value wins, else the fallback;
    /// blanks are unset at every level.
    #[test]
    fn role_pin_falls_back_to_the_global_pin() {
        assert_eq!(
            super::resolve_role_pin(Some("vendor/role-model"), Some("vendor/global-model")),
            Some("vendor/role-model".to_string()),
            "role-specific pin wins"
        );
        assert_eq!(
            super::resolve_role_pin(None, Some("vendor/global-model")),
            Some("vendor/global-model".to_string()),
            "falls back to the global pin"
        );
        assert_eq!(super::resolve_role_pin(None, None), None);
        assert_eq!(
            super::resolve_role_pin(Some("  "), Some("vendor/global-model")),
            Some("vendor/global-model".to_string())
        );
    }

    /// Task 13: every research role falls back role → `VOX_MODEL_FORCE_RESEARCH`
    /// → `VOX_MODEL_FORCE`; chat falls back straight to `VOX_MODEL_FORCE`; each
    /// key is read env-first, then `~/.vox/config.toml` (a temp `VOX_HOME`/`HOME`
    /// here — never the user's real config).
    #[test]
    fn forced_model_for_walks_role_then_group_then_global_through_env_and_toml() {
        let _g = TEST_ENV_LOCK.lock().expect("env lock");
        let home = HomeGuard::new();
        let prev_vox_home = std::env::var("VOX_HOME").ok();
        let all_keys = [
            "VOX_MODEL_FORCE",
            "VOX_MODEL_FORCE_RESEARCH",
            "VOX_MODEL_FORCE_CHAT",
            "VOX_MODEL_FORCE_PLANNER",
            "VOX_MODEL_FORCE_SYNTHESIS",
            "VOX_MODEL_FORCE_JUDGE",
            "VOX_MODEL_FORCE_VERIFIER",
        ];
        let prev_env: Vec<_> = all_keys
            .iter()
            .map(|k| (*k, std::env::var(k).ok()))
            .collect();
        unsafe {
            std::env::set_var("VOX_HOME", home.home());
            for k in all_keys {
                std::env::remove_var(k);
            }
        }
        // Drop any cached real-config values; from here reads/writes hit the temp home.
        crate::toml_config::reload_user_config();
        assert!(crate::paths::dot_vox_user_dir().starts_with(home.home()));

        assert_eq!(forced_model_for(ModelRole::Judge), None);

        unsafe { std::env::set_var("VOX_MODEL_FORCE", "vendor/global") };
        crate::snapshot::bump(&["VOX_MODEL_FORCE"]);
        for role in ModelRole::ALL {
            assert_eq!(
                forced_model_for(role).as_deref(),
                Some("vendor/global"),
                "{role:?}"
            );
        }

        // Group pin from the config file covers the research roles, not chat.
        crate::toml_config::set_user_config_value("VOX_MODEL_FORCE_RESEARCH", "vendor/research")
            .expect("temp config write");
        for role in [
            ModelRole::Planner,
            ModelRole::Synthesis,
            ModelRole::Judge,
            ModelRole::Verifier,
        ] {
            assert_eq!(
                forced_model_for(role).as_deref(),
                Some("vendor/research"),
                "{role:?}"
            );
        }
        assert_eq!(
            forced_model_for(ModelRole::Chat).as_deref(),
            Some("vendor/global")
        );

        // Each role key wins for its own role only.
        unsafe {
            std::env::set_var("VOX_MODEL_FORCE_JUDGE", "vendor/judge");
            std::env::set_var("VOX_MODEL_FORCE_CHAT", "~vendor/chat-latest");
        }
        crate::snapshot::bump(&["VOX_MODEL_FORCE_JUDGE", "VOX_MODEL_FORCE_CHAT"]);
        assert_eq!(
            forced_model_for(ModelRole::Judge).as_deref(),
            Some("vendor/judge")
        );
        assert_eq!(
            forced_model_for(ModelRole::Planner).as_deref(),
            Some("vendor/research")
        );
        assert_eq!(
            forced_model_for(ModelRole::Chat).as_deref(),
            Some("~vendor/chat-latest")
        );
        // The unscoped reader keeps meaning exactly `VOX_MODEL_FORCE`.
        assert_eq!(forced_model().as_deref(), Some("vendor/global"));

        let _ = crate::toml_config::unset_user_config_value("VOX_MODEL_FORCE_RESEARCH");
        unsafe {
            for (k, v) in &prev_env {
                match v {
                    Some(v) => std::env::set_var(k, v),
                    None => std::env::remove_var(k),
                }
            }
            match &prev_vox_home {
                Some(v) => std::env::set_var("VOX_HOME", v),
                None => std::env::remove_var("VOX_HOME"),
            }
        }
        drop(home);
        crate::toml_config::reload_user_config();
    }

    #[test]
    fn local_base_prefers_populi_then_ollama() {
        let _g = TEST_ENV_LOCK.lock().expect("env lock");
        let _home = HomeGuard::new();
        unsafe {
            std::env::remove_var("POPULI_URL");
            std::env::remove_var("OLLAMA_URL");
        }
        crate::snapshot::bump(&["POPULI_URL", "OLLAMA_URL"]);
        assert_eq!(
            local_ollama_populi_base_url(),
            LOCAL_OLLAMA_POPULI_BASE_URL_DEFAULT
        );

        unsafe {
            std::env::set_var("OLLAMA_URL", "http://localhost:9999");
        }
        crate::snapshot::bump(&["OLLAMA_URL"]);
        assert_eq!(local_ollama_populi_base_url(), "http://localhost:9999");

        unsafe {
            std::env::set_var("POPULI_URL", LOCAL_OLLAMA_POPULI_BASE_URL_DEFAULT);
        }
        crate::snapshot::bump(&["POPULI_URL"]);
        assert_eq!(
            local_ollama_populi_base_url(),
            LOCAL_OLLAMA_POPULI_BASE_URL_DEFAULT
        );

        unsafe {
            std::env::remove_var("POPULI_URL");
            std::env::remove_var("OLLAMA_URL");
        }
        crate::snapshot::bump(&["POPULI_URL", "OLLAMA_URL"]);
    }

    #[test]
    fn endpoint_defaults_match_legacy_consts() {
        let _g = TEST_ENV_LOCK.lock().expect("env lock");
        let _home = HomeGuard::new();
        for key in [
            "OPENROUTER_BASE_URL",
            "VOX_OPENAI_BASE_URL",
            "OPENAI_BASE_URL",
        ] {
            unsafe {
                std::env::remove_var(key);
            }
            let _ = crate::toml_config::unset_user_config_value(key);
        }
        crate::snapshot::bump(&[]);
        assert_eq!(
            openrouter_chat_completions_url(),
            OPENROUTER_CHAT_COMPLETIONS_URL
        );
        assert_eq!(openrouter_models_list_url(), OPENROUTER_MODELS_LIST_URL);
        assert_eq!(openrouter_embeddings_url(), OPENROUTER_EMBEDDINGS_URL);
        assert_eq!(openai_chat_completions_url(), OPENAI_CHAT_COMPLETIONS_URL);
        assert_eq!(openai_embeddings_url(), OPENAI_EMBEDDINGS_URL);
    }

    #[test]
    fn endpoint_honors_config_toml_base_url() {
        let _g = TEST_ENV_LOCK.lock().expect("env lock");
        let _home = HomeGuard::new();
        unsafe {
            std::env::remove_var("OPENROUTER_BASE_URL");
        }
        crate::toml_config::set_user_config_value(
            "OPENROUTER_BASE_URL",
            "https://proxy.example/api",
        )
        .expect("set");
        crate::snapshot::bump(&["OPENROUTER_BASE_URL"]);
        assert_eq!(
            openrouter_chat_completions_url(),
            "https://proxy.example/api/v1/chat/completions"
        );
        let _ = crate::toml_config::unset_user_config_value("OPENROUTER_BASE_URL");
        crate::snapshot::bump(&["OPENROUTER_BASE_URL"]);
    }

    #[test]
    fn trailing_slash_base_yields_single_slash_join() {
        let _g = TEST_ENV_LOCK.lock().expect("env lock");
        let _home = HomeGuard::new();
        unsafe {
            std::env::remove_var("OPENROUTER_BASE_URL");
            std::env::remove_var("VOX_OPENAI_BASE_URL");
            std::env::remove_var("OPENAI_BASE_URL");
        }
        crate::toml_config::set_user_config_value(
            "OPENROUTER_BASE_URL",
            "https://proxy.example/api/",
        )
        .expect("set");
        crate::toml_config::set_user_config_value(
            "VOX_OPENAI_BASE_URL",
            "https://proxy.example/openai/v1/",
        )
        .expect("set");
        crate::snapshot::bump(&["OPENROUTER_BASE_URL", "VOX_OPENAI_BASE_URL"]);
        assert_eq!(openrouter_base_url(), "https://proxy.example/api");
        assert_eq!(
            openrouter_chat_completions_url(),
            "https://proxy.example/api/v1/chat/completions"
        );
        assert_eq!(
            openai_compatible_base_url(),
            "https://proxy.example/openai/v1"
        );
        assert_eq!(
            openai_chat_completions_url(),
            "https://proxy.example/openai/v1/chat/completions"
        );
        let _ = crate::toml_config::unset_user_config_value("OPENROUTER_BASE_URL");
        let _ = crate::toml_config::unset_user_config_value("VOX_OPENAI_BASE_URL");
        crate::snapshot::bump(&["OPENROUTER_BASE_URL", "VOX_OPENAI_BASE_URL"]);
    }

    #[test]
    fn local_base_honors_config_toml_when_env_absent() {
        let _g = TEST_ENV_LOCK.lock().expect("env lock");
        let _home = HomeGuard::new();
        unsafe {
            std::env::remove_var("POPULI_URL");
            std::env::remove_var("OLLAMA_URL");
            std::env::remove_var("VOX_POPULI_LOCAL_OLLAMA_URL");
        }
        let _ = crate::toml_config::unset_user_config_value("POPULI_URL");
        crate::toml_config::set_user_config_value("OLLAMA_URL", "http://cfg-host:1234")
            .expect("set");
        crate::snapshot::bump(&["OLLAMA_URL", "POPULI_URL"]);
        assert_eq!(local_ollama_populi_base_url(), "http://cfg-host:1234");
        let _ = crate::toml_config::unset_user_config_value("OLLAMA_URL");
        crate::snapshot::bump(&["OLLAMA_URL"]);
    }

    #[test]
    fn degenerate_base_url_falls_back_to_default() {
        let _g = TEST_ENV_LOCK.lock().expect("env lock");
        let _home = HomeGuard::new();
        unsafe {
            std::env::remove_var("OPENROUTER_BASE_URL");
        }
        crate::toml_config::set_user_config_value("OPENROUTER_BASE_URL", "").expect("set");
        crate::snapshot::bump(&["OPENROUTER_BASE_URL"]);
        assert_eq!(openrouter_base_url(), "https://openrouter.ai/api");

        crate::toml_config::set_user_config_value("OPENROUTER_BASE_URL", "proxy/no-scheme")
            .expect("set");
        crate::snapshot::bump(&["OPENROUTER_BASE_URL"]);
        assert_eq!(openrouter_base_url(), "https://openrouter.ai/api");

        crate::toml_config::set_user_config_value("OPENROUTER_BASE_URL", "https://proxy/api")
            .expect("set");
        crate::snapshot::bump(&["OPENROUTER_BASE_URL"]);
        assert_eq!(openrouter_base_url(), "https://proxy/api");

        let _ = crate::toml_config::unset_user_config_value("OPENROUTER_BASE_URL");
        crate::snapshot::bump(&["OPENROUTER_BASE_URL"]);
    }

    #[test]
    fn tuning_honors_config_toml_when_secret_absent() {
        let _g = TEST_ENV_LOCK.lock().expect("env lock");
        let _home = HomeGuard::new();
        unsafe {
            std::env::remove_var("OLLAMA_TUNING_NUM_CTX");
            std::env::remove_var("OLLAMA_TUNING_TEMPERATURE");
        }
        let _ = crate::toml_config::unset_user_config_value("OLLAMA_TUNING_NUM_CTX");
        let _ = crate::toml_config::unset_user_config_value("OLLAMA_TUNING_TEMPERATURE");
        crate::snapshot::bump(&["OLLAMA_TUNING_NUM_CTX", "OLLAMA_TUNING_TEMPERATURE"]);

        assert_eq!(ollama_tuning_num_ctx(), None);
        assert_eq!(ollama_tuning_temperature(), None);

        crate::toml_config::set_user_config_value("OLLAMA_TUNING_NUM_CTX", "8192").expect("set");
        crate::toml_config::set_user_config_value("OLLAMA_TUNING_TEMPERATURE", "0.3").expect("set");
        crate::snapshot::bump(&["OLLAMA_TUNING_NUM_CTX", "OLLAMA_TUNING_TEMPERATURE"]);
        assert_eq!(ollama_tuning_num_ctx(), Some(8192));
        assert_eq!(ollama_tuning_temperature(), Some(0.3));

        let _ = crate::toml_config::unset_user_config_value("OLLAMA_TUNING_NUM_CTX");
        let _ = crate::toml_config::unset_user_config_value("OLLAMA_TUNING_TEMPERATURE");
        crate::snapshot::bump(&["OLLAMA_TUNING_NUM_CTX", "OLLAMA_TUNING_TEMPERATURE"]);
    }

    // --- Cache invalidation tests (Phase 5.2 TDD) ---

    #[test]
    fn openrouter_base_url_re_reads_after_bump() {
        let _g = TEST_ENV_LOCK.lock().expect("env lock");
        let _home = HomeGuard::new();
        let _ = crate::toml_config::unset_user_config_value("OPENROUTER_BASE_URL");
        // Phase 1: set env to "first", bump, read → get "first".
        unsafe {
            std::env::set_var("OPENROUTER_BASE_URL", "https://first.example/api");
        }
        crate::snapshot::bump(&["OPENROUTER_BASE_URL"]);
        let v1 = openrouter_base_url();
        assert_eq!(v1, "https://first.example/api");

        // Phase 2: change env to "second", bump, read → get "second" (cache invalidated).
        unsafe {
            std::env::set_var("OPENROUTER_BASE_URL", "https://second.example/api");
        }
        crate::snapshot::bump(&["OPENROUTER_BASE_URL"]);
        let v2 = openrouter_base_url();
        assert_eq!(v2, "https://second.example/api", "must re-read after bump");

        unsafe {
            std::env::remove_var("OPENROUTER_BASE_URL");
        }
        crate::snapshot::bump(&["OPENROUTER_BASE_URL"]);
    }

    #[test]
    fn openai_base_url_re_reads_after_bump() {
        let _g = TEST_ENV_LOCK.lock().expect("env lock");
        let _home = HomeGuard::new();
        let _ = crate::toml_config::unset_user_config_value("VOX_OPENAI_BASE_URL");
        let _ = crate::toml_config::unset_user_config_value("OPENAI_BASE_URL");
        unsafe {
            std::env::set_var("VOX_OPENAI_BASE_URL", "https://openai-first.example/v1");
        }
        crate::snapshot::bump(&["VOX_OPENAI_BASE_URL"]);
        let v1 = openai_compatible_base_url();
        assert_eq!(v1, "https://openai-first.example/v1");

        unsafe {
            std::env::set_var("VOX_OPENAI_BASE_URL", "https://openai-second.example/v1");
        }
        crate::snapshot::bump(&["VOX_OPENAI_BASE_URL"]);
        let v2 = openai_compatible_base_url();
        assert_eq!(v2, "https://openai-second.example/v1", "re-read after bump");

        unsafe {
            std::env::remove_var("VOX_OPENAI_BASE_URL");
        }
        crate::snapshot::bump(&["VOX_OPENAI_BASE_URL"]);
    }

    #[test]
    fn tuning_re_reads_after_bump() {
        let _g = TEST_ENV_LOCK.lock().expect("env lock");
        let _home = HomeGuard::new();
        let _ = crate::toml_config::unset_user_config_value("GEMINI_TUNING_TEMPERATURE");
        unsafe {
            std::env::remove_var("GEMINI_TUNING_TEMPERATURE");
        }
        // Phase 1: no config set → None.
        crate::snapshot::bump(&["GEMINI_TUNING_TEMPERATURE"]);
        assert_eq!(gemini_tuning_temperature(), None);

        // Phase 2: set config, bump → Some(0.7).
        crate::toml_config::set_user_config_value("GEMINI_TUNING_TEMPERATURE", "0.7").expect("set");
        crate::snapshot::bump(&["GEMINI_TUNING_TEMPERATURE"]);
        assert_eq!(gemini_tuning_temperature(), Some(0.7), "re-read after bump");

        let _ = crate::toml_config::unset_user_config_value("GEMINI_TUNING_TEMPERATURE");
        crate::snapshot::bump(&["GEMINI_TUNING_TEMPERATURE"]);
    }

    #[test]
    fn research_prefer_free_tier_parses_truthy() {
        use super::research_prefer_free_tier_from;
        assert!(research_prefer_free_tier_from(Some("1")));
        assert!(research_prefer_free_tier_from(Some("true")));
        assert!(research_prefer_free_tier_from(Some("TRUE")));
        assert!(research_prefer_free_tier_from(Some("  yes ")));
        assert!(research_prefer_free_tier_from(Some("on")));
        assert!(!research_prefer_free_tier_from(Some("0")));
        assert!(!research_prefer_free_tier_from(Some("false")));
        assert!(!research_prefer_free_tier_from(Some("")));
        assert!(!research_prefer_free_tier_from(None));
    }

    #[test]
    fn vox_local_probe_candidates_default_when_env_unset() {
        assert_eq!(
            vox_local_endpoint_probe_candidates_from(None),
            vec![
                "http://127.0.0.1:11434".to_string(),
                "http://127.0.0.1:11435".to_string(),
            ]
        );
    }

    #[test]
    fn vox_local_probe_candidates_honors_explicit_override() {
        assert_eq!(
            vox_local_endpoint_probe_candidates_from(Some("http://127.0.0.1:17863/")),
            vec!["http://127.0.0.1:17863".to_string()]
        );
        assert!(vox_local_endpoint_probe_candidates_from(Some("  ")).is_empty());
    }

    #[test]
    fn vox_local_health_identifies_serve_requires_ml_cli_service() {
        assert!(vox_local_health_identifies_serve(
            r#"{"status":"ok","service":"vox-ml-cli"}"#
        ));
        assert!(!vox_local_health_identifies_serve(r#"{"status":"ok"}"#));
        assert!(!vox_local_health_identifies_serve("Ollama is running"));
    }
}
