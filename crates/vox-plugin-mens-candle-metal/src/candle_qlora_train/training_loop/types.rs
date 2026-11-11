//! Internal types for the QLoRA training loop.
//!
//! `QloraTrainingResume`, `EncodedTrainStep` and `TryEncodeOutcome` moved to
//! `vox-plugin-mens-candle-core` — byte-for-byte identical to the CUDA
//! plugin's copies.
pub use vox_plugin_mens_candle_core::candle_qlora_train::training_loop::types::{
    QloraTrainingResume, TryEncodeOutcome,
};

pub enum MaskedCeForward {
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
        segments: Option<Vec<crate::model::CheckpointSegment>>,
    },
}
