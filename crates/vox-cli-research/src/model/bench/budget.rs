//! Spend control for `vox model bench`: a pre-flight estimate that refuses to start a run
//! that could exceed `--max-usd`, and a running guard that stops before the next call would.

use vox_orchestrator::models::{ModelSpec, PricingSource, ProviderType};

/// Per-1k-token prices assumed for a model with no known pricing. Deliberately high so an
/// unpriced model cannot slip past the cap on an optimistic guess.
pub const UNKNOWN_INPUT_PER_1K: f64 = 0.01;
pub const UNKNOWN_OUTPUT_PER_1K: f64 = 0.03;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Price {
    pub input_per_1k: f64,
    pub output_per_1k: f64,
    /// True when the price is the conservative fallback, not catalog pricing.
    pub assumed: bool,
}

impl Price {
    pub const FREE: Price = Price {
        input_per_1k: 0.0,
        output_per_1k: 0.0,
        assumed: false,
    };

    #[must_use]
    pub fn cost(&self, input_tokens: u64, output_tokens: u64) -> f64 {
        input_tokens as f64 / 1000.0 * self.input_per_1k
            + output_tokens as f64 / 1000.0 * self.output_per_1k
    }
}

/// The price to budget with for `spec`.
#[must_use]
pub fn price_for(spec: &ModelSpec) -> Price {
    let local = matches!(
        spec.provider_type,
        ProviderType::Ollama | ProviderType::VoxLocal | ProviderType::Custom(_)
    );
    if local || spec.is_free || spec.id.ends_with(":free") {
        return Price::FREE;
    }
    if spec.cost_per_1k_input > 0.0 || spec.cost_per_1k_output > 0.0 {
        return Price {
            input_per_1k: spec.cost_per_1k_input,
            output_per_1k: spec.cost_per_1k_output.max(spec.cost_per_1k_input),
            assumed: false,
        };
    }
    if spec.cost_per_1k > 0.0 {
        return Price {
            input_per_1k: spec.cost_per_1k,
            output_per_1k: spec.cost_per_1k,
            assumed: false,
        };
    }
    if spec.pricing_source != PricingSource::Unknown
        && spec.pricing_source != PricingSource::Bootstrap
    {
        // Catalog says zero and the source is real pricing: a genuinely free model.
        return Price::FREE;
    }
    Price {
        input_per_1k: UNKNOWN_INPUT_PER_1K,
        output_per_1k: UNKNOWN_OUTPUT_PER_1K,
        assumed: true,
    }
}

/// Rough prompt token count: ~4 characters per token plus per-message framing.
#[must_use]
pub fn estimate_prompt_tokens(prompt_chars: usize) -> u64 {
    (prompt_chars as u64).div_ceil(4) + 32
}

/// Upper-bound cost of one case: the full prompt plus every allowed output token.
#[must_use]
pub fn estimate_case_usd(price: &Price, prompt_chars: usize, max_tokens: u64) -> f64 {
    price.cost(estimate_prompt_tokens(prompt_chars), max_tokens)
}

/// Running spend against a hard cap.
#[derive(Debug, Clone)]
pub struct Budget {
    pub max_usd: f64,
    pub spent_usd: f64,
}

impl Budget {
    #[must_use]
    pub fn new(max_usd: f64) -> Self {
        Self {
            max_usd: max_usd.max(0.0),
            spent_usd: 0.0,
        }
    }

    /// Whether a call estimated at `estimate_usd` fits in what is left.
    #[must_use]
    pub fn can_afford(&self, estimate_usd: f64) -> bool {
        self.spent_usd + estimate_usd <= self.max_usd + 1e-12
    }

    /// Charge a finished call: the provider-reported cost when present, else the token-priced cost.
    pub fn charge(&mut self, reported_usd: Option<f64>, priced_usd: f64) -> f64 {
        let c = reported_usd
            .filter(|c| c.is_finite() && *c >= 0.0)
            .unwrap_or(priced_usd);
        self.spent_usd += c;
        c
    }

    /// Hold `estimate_usd` for an in-flight call so concurrent calls cannot jointly overshoot.
    /// Returns false (and holds nothing) when it does not fit.
    pub fn reserve(&mut self, estimate_usd: f64) -> bool {
        if !self.can_afford(estimate_usd) {
            return false;
        }
        self.spent_usd += estimate_usd;
        true
    }

    /// Replace a reservation with the call's real cost (see [`Self::charge`]); returns that cost.
    pub fn settle(&mut self, reserved_usd: f64, reported_usd: Option<f64>, priced_usd: f64) -> f64 {
        self.spent_usd -= reserved_usd;
        self.charge(reported_usd, priced_usd)
    }

    #[must_use]
    pub fn remaining(&self) -> f64 {
        (self.max_usd - self.spent_usd).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_orchestrator::models::ModelCapabilities;

    fn spec(id: &str, pt: ProviderType, cin: f64, cout: f64, src: PricingSource) -> ModelSpec {
        ModelSpec {
            id: id.into(),
            canonical_slug: id.into(),
            provider: "t".into(),
            provider_type: pt,
            max_tokens: 4096,
            cost_per_1k: 0.0,
            cost_per_1k_input: cin,
            cost_per_1k_output: cout,
            is_free: false,
            observed_cost_per_1k: None,
            strengths: vec![],
            capabilities: ModelCapabilities::default(),
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source: src,
            supported_parameters: vec![],
        }
    }

    #[test]
    fn local_and_free_cost_nothing_unknown_is_priced_conservatively() {
        assert_eq!(
            price_for(&spec(
                "q",
                ProviderType::Ollama,
                0.0,
                0.0,
                PricingSource::Unknown
            )),
            Price::FREE
        );
        assert_eq!(
            price_for(&spec(
                "a/b:free",
                ProviderType::OpenRouter,
                0.0,
                0.0,
                PricingSource::Unknown
            )),
            Price::FREE
        );
        assert_eq!(
            price_for(&spec(
                "a/b",
                ProviderType::OpenRouter,
                0.0,
                0.0,
                PricingSource::OpenRouter
            )),
            Price::FREE
        );
        let unknown = price_for(&spec(
            "a/b",
            ProviderType::OpenRouter,
            0.0,
            0.0,
            PricingSource::Unknown,
        ));
        assert!(unknown.assumed);
        assert_eq!(unknown.output_per_1k, UNKNOWN_OUTPUT_PER_1K);
        let priced = price_for(&spec(
            "a/b",
            ProviderType::OpenRouter,
            0.001,
            0.004,
            PricingSource::OpenRouter,
        ));
        assert!(!priced.assumed);
        assert!((priced.cost(1000, 1000) - 0.005).abs() < 1e-12);
    }

    #[test]
    fn estimate_is_an_upper_bound_on_output() {
        let p = Price {
            input_per_1k: 0.001,
            output_per_1k: 0.002,
            assumed: false,
        };
        // 400 chars -> 100 + 32 prompt tokens; 500 output tokens.
        let est = estimate_case_usd(&p, 400, 500);
        assert!((est - (0.132 * 0.001 + 0.5 * 0.002)).abs() < 1e-12);
    }

    #[test]
    fn budget_refuses_the_call_that_would_cross_the_cap_and_prefers_reported_cost() {
        let mut b = Budget::new(0.10);
        assert!(b.can_afford(0.10));
        assert_eq!(b.charge(Some(0.04), 0.09), 0.04, "reported cost wins");
        assert_eq!(b.charge(None, 0.05), 0.05, "priced cost when unreported");
        assert!(b.can_afford(0.01));
        assert!(!b.can_afford(0.011));
        assert!((b.remaining() - 0.01).abs() < 1e-12);
    }

    #[test]
    fn reservations_block_concurrent_overshoot_and_settle_to_actual() {
        let mut b = Budget::new(0.10);
        assert!(b.reserve(0.06));
        assert!(
            !b.reserve(0.06),
            "second in-flight call would cross the cap"
        );
        assert_eq!(b.settle(0.06, Some(0.01), 0.05), 0.01);
        assert!(b.reserve(0.06), "settling freed the unused reservation");
        assert!((b.spent_usd - 0.07).abs() < 1e-12);
    }
}
