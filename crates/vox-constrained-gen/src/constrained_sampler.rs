use crate::error::Result;
use crate::sampler_state::SamplerState;

/// A grammar-constrained logit masker.
///
/// Implementors examine the current sampler state and mask out logits for tokens
/// that would violate the grammar, returning updated logits and a new state.
pub trait ConstrainedSampler: Send + Sync {
    /// Mask `logits` so that only grammar-valid continuations remain.
    ///
    /// Returns `(masked_logits, new_state)` on success.  
    /// `masked_logits[i] = f32::NEG_INFINITY` for every token `i` that would
    /// produce an invalid parse prefix.
    fn mask_logits(
        &self,
        logits: &[f32],
        state: &SamplerState,
        token_strings: &[String],
    ) -> Result<(Vec<f32>, SamplerState)>;

    /// Reset to the initial state for a new generation.
    fn initial_state(&self) -> SamplerState;

    /// Human-readable name for logging.
    fn name(&self) -> &'static str;
}
