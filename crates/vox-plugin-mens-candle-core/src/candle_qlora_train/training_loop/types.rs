//! Internal types for the QLoRA training loop shared by both device plugins.
//!
//! `QloraTrainingResume`, `EncodedTrainStep`, `TryEncodeOutcome`, and
//! `MaskedCeForward<Seg>` live here so neither plugin duplicates `forward.rs`
//! or the masked cross-entropy result enum.

#[derive(Debug, Clone)]
pub struct QloraTrainingResume {
    pub start_epoch: usize,
    pub global_step: u32,
    pub resume_pair_offset: usize,
    pub resume_shuffled_indices: Option<Vec<usize>>,
    pub total_tokens: usize,
    pub micro_steps_executed: u32,
}

pub struct EncodedTrainStep {
    pub raw_token_len: usize,
    pub ids: Vec<u32>,
    pub prefix_len: usize,
    pub trunc_offset: usize,
    pub sample_weight: f64,
    pub token_weights: Option<Vec<f32>>,
}

pub enum TryEncodeOutcome {
    Encoded(EncodedTrainStep),
    SkipCurriculum,
    SkipShortSeq,
}

/// Result of chunked `lm_head` + cross-entropy forward/backward under detached boundaries.
///
/// Computing and backpropagating `lm_head` one 256-row chunk at a time avoids
/// retaining `[seq_len, vocab]` logits/autograd buffers for `248,320`-vocab models.
pub struct DetachedHeadBackward {
    /// Accumulated gradients for trainable `lm_head` Vars (or empty `GradStore` when `lm_head` is frozen).
    pub head_grads: Option<candle_core::backprop::GradStore>,
    /// Cotangent `dL/d(final_hidden)` of shape `[batch, seq_len, d_model]`, already scaled by
    /// `effective_sample_weight / (mask_sum * grad_accum_steps)`.
    pub d_final_hidden: candle_core::Tensor,
}

pub enum MaskedCeForward<Seg> {
    NoSupervision,
    NonFinite {
        kind: &'static str,
        mask_sum: f32,
    },
    Finite {
        loss: Option<candle_core::Tensor>,
        loss_scalar: f32,
        supervised_tokens: u64,
        theoretical_tokens: u64,
        syntax_weight_sum: f32,
        segments: Option<Vec<Seg>>,
        detached_head: Option<DetachedHeadBackward>,
    },
}
