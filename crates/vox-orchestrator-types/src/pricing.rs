use serde::{Deserialize, Serialize};

/// Records which data source last set the pricing fields on a `ModelSpec`.
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
