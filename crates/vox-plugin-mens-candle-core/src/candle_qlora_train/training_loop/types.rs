//! Internal types for the QLoRA training loop shared by both device plugins.
//!
//! `MaskedCeForward` is deliberately NOT here: `vox-plugin-mens-candle-cuda`'s
//! copy carries an extra `segments: Option<Vec<CheckpointSegment>>` field on its
//! `Finite` variant (gradient-checkpointing support Metal does not have yet), so
//! that one type stays per-plugin. These three were byte-identical between both
//! plugins' `types.rs`.

#[derive(Debug, Clone)]
pub struct QloraTrainingResume {
    pub start_epoch: usize,
    pub global_step: u32,
    pub resume_pair_offset: usize,
    pub resume_shuffled_indices: Option<Vec<usize>>,
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
