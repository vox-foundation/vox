//! Flagship detection from price, so the Efficient lane never needs a hand-kept list of
//! "expensive models" that goes stale each release. At time of writing the ≥ $20/M
//! completion band is exactly Opus/Fable/GPT-6-Astra-class.
use crate::models::ModelTier;

/// USD per 1,000 output tokens at or above which a model is a flagship (Elite): `0.020` = $20 per 1M.
pub const ELITE_MIN_OUTPUT_USD_PER_1K: f64 = 0.020;
/// USD per 1,000 output tokens at or above which a model is Pro (below it: Fast): `0.004` = $4 per 1M.
pub const PRO_MIN_OUTPUT_USD_PER_1K: f64 = 0.004;

/// Tier from price alone. `Unknown` when the price is not known (zero or NaN on a non-free model).
// ponytail: fixed bands; move to model-routing.v1.yaml when the council wants to tune them.
#[must_use]
pub fn derive_tier(is_free: bool, cost_per_1k_output: f64) -> ModelTier {
    if is_free {
        return ModelTier::Free;
    }
    if !cost_per_1k_output.is_finite() || cost_per_1k_output <= 0.0 {
        return ModelTier::Unknown;
    }
    if cost_per_1k_output >= ELITE_MIN_OUTPUT_USD_PER_1K {
        ModelTier::Elite
    } else if cost_per_1k_output >= PRO_MIN_OUTPUT_USD_PER_1K {
        ModelTier::Pro
    } else {
        ModelTier::Fast
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ModelTier;

    #[test]
    fn free_models_are_free_tier() {
        assert_eq!(derive_tier(true, 0.0), ModelTier::Free);
    }

    #[test]
    fn output_price_bands_map_to_tiers() {
        assert_eq!(derive_tier(false, 0.050), ModelTier::Elite, "$50/M");
        assert_eq!(
            derive_tier(false, 0.020),
            ModelTier::Elite,
            "$20/M is the boundary"
        );
        assert_eq!(derive_tier(false, 0.0199), ModelTier::Pro);
        assert_eq!(
            derive_tier(false, 0.004),
            ModelTier::Pro,
            "$4/M is the boundary"
        );
        assert_eq!(derive_tier(false, 0.0039), ModelTier::Fast);
        assert_eq!(derive_tier(false, 0.00012), ModelTier::Fast);
    }

    #[test]
    fn unknown_pricing_is_not_guessed() {
        // A non-free model with a 0.0 placeholder price (e.g. Anthropic-direct before the
        // LiteLLM oracle fills it in) must not be called Fast.
        assert_eq!(derive_tier(false, 0.0), ModelTier::Unknown);
        assert_eq!(derive_tier(false, f64::NAN), ModelTier::Unknown);
    }
}
