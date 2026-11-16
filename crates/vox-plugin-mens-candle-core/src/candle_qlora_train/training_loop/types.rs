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

pub enum MaskedCeForward<Seg> {
    NoSupervision,
    NonFinite {
        kind: &'static str,
        mask_sum: f32,
    },
    Finite {
        loss: candle_core::Tensor,
        loss_scalar: f32,
        supervised_tokens: u64,
        theoretical_tokens: u64,
        syntax_weight_sum: f32,
        segments: Option<Vec<Seg>>,
    },
}
