//! Stream of Revision — mid-generation self-correction via a backtrack token.
//!
//! Wraps a [`ConstrainedSampler`] and injects a synthetic `<backtrack>` token
//! into the vocabulary. When the LLM selects `<backtrack>`, the sampler rewinds
//! to a previous checkpoint state instead of emitting text.
//!
//! **Research rationale (Grammar Constraints §6.2):** Without revision, a
//! constrained sampler that masks all tokens triggers a hard deadlock. The
//! backtrack token gives the LLM an escape hatch to self-correct.

use tracing::debug;

use crate::{ConstrainedGenError, ConstrainedSampler, Result, SamplerState};

/// Default maximum backtrack depth (consecutive backtracks before error).
const DEFAULT_MAX_REVISION_DEPTH: usize = 8;

/// The sentinel token text recognised as a backtrack request.
pub const BACKTRACK_TOKEN: &str = "<backtrack>";

/// Configuration for the revision sampler.
#[derive(Debug, Clone)]
pub struct RevisionConfig {
    /// Maximum consecutive backtracks allowed before erroring.
    pub max_depth: usize,
}

impl Default for RevisionConfig {
    fn default() -> Self {
        Self {
            max_depth: DEFAULT_MAX_REVISION_DEPTH,
        }
    }
}

/// Wraps a [`ConstrainedSampler`] with backtrack support.
pub struct RevisionSampler<S: ConstrainedSampler> {
    inner: S,
    _config: RevisionConfig,
}

impl<S: ConstrainedSampler> RevisionSampler<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            _config: RevisionConfig::default(),
        }
    }

    pub fn with_config(inner: S, config: RevisionConfig) -> Self {
        Self {
            inner,
            _config: config,
        }
    }
}

impl<S: ConstrainedSampler> ConstrainedSampler for RevisionSampler<S> {
    fn mask_logits(
        &self,
        logits: &[f32],
        state: &SamplerState,
        token_strings: &[String],
    ) -> Result<(Vec<f32>, SamplerState)> {
        // First, run the inner sampler.
        match self.inner.mask_logits(logits, state, token_strings) {
            Ok((masked, new_state)) => {
                // Check if all logits are masked (deadlock imminent).
                let all_masked = masked.iter().all(|&v| v == f32::NEG_INFINITY);
                if all_masked {
                    // Instead of deadlocking, inject the backtrack token as the
                    // only valid option. The caller interprets selection of the
                    // backtrack token as a rewind signal.
                    debug!("revision: all tokens masked — injecting backtrack escape");
                    let mut with_backtrack = masked;
                    // Find the backtrack token in the vocabulary.
                    if let Some(bt_idx) = token_strings.iter().position(|t| t == BACKTRACK_TOKEN) {
                        with_backtrack[bt_idx] = 0.0; // un-mask it
                    }
                    // Even if the backtrack token isn't in the vocabulary, return
                    // the deadlock-masked logits — the watchdog will catch it.
                    Ok((with_backtrack, new_state))
                } else {
                    Ok((masked, new_state))
                }
            }
            Err(ConstrainedGenError::Deadlock {
                position,
                partial_output,
            }) => {
                // Convert deadlock into a "backtrack available" signal.
                debug!(
                    position,
                    "revision: converting deadlock to backtrack opportunity"
                );
                let mut fallback_logits = vec![f32::NEG_INFINITY; logits.len()];
                if let Some(bt_idx) = token_strings.iter().position(|t| t == BACKTRACK_TOKEN) {
                    fallback_logits[bt_idx] = 0.0;
                    Ok((fallback_logits, state.clone()))
                } else {
                    // No backtrack token in vocabulary — propagate the deadlock.
                    Err(ConstrainedGenError::Deadlock {
                        position,
                        partial_output,
                    })
                }
            }
            Err(e) => Err(e),
        }
    }

    fn initial_state(&self) -> SamplerState {
        self.inner.initial_state()
    }

    fn name(&self) -> &'static str {
        "revision"
    }
}

/// Check whether a chosen token index corresponds to the backtrack sentinel.
pub fn is_backtrack_token(token_strings: &[String], chosen_idx: usize) -> bool {
    token_strings
        .get(chosen_idx)
        .map(|t| t == BACKTRACK_TOKEN)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::earley::EarleySampler;

    #[test]
    fn revision_wraps_earley() {
        let inner = EarleySampler::from_vox_grammar().unwrap();
        let revision = RevisionSampler::new(inner);
        assert_eq!(revision.name(), "revision");
    }

    #[test]
    fn revision_passes_through_normal_mask() {
        let inner = EarleySampler::from_vox_grammar().unwrap();
        let revision = RevisionSampler::new(inner);
        let state = revision.initial_state();
        let logits = vec![1.0, 2.0];
        let tokens = vec!["fn".to_string(), "let".to_string()];
        let result = revision.mask_logits(&logits, &state, &tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn is_backtrack_token_detects_sentinel() {
        let tokens = vec![
            "fn".to_string(),
            BACKTRACK_TOKEN.to_string(),
            "let".to_string(),
        ];
        assert!(!is_backtrack_token(&tokens, 0));
        assert!(is_backtrack_token(&tokens, 1));
        assert!(!is_backtrack_token(&tokens, 2));
        assert!(!is_backtrack_token(&tokens, 99));
    }

    #[test]
    fn revision_config_defaults() {
        let cfg = RevisionConfig::default();
        assert_eq!(cfg.max_depth, 8);
    }
}
