//! Internal types for the QLoRA training loop.
//!
//! `QloraTrainingResume`, `EncodedTrainStep` and `TryEncodeOutcome` moved to
//! `vox-plugin-mens-candle-core` — byte-for-byte identical to the CUDA
//! plugin's copies. `MaskedCeForward` stays here: its `ckpt` field names
//! this crate's `model::CheckpointSegment`. `EncodedTrainStep` is not re-exported at
//! this path: nothing in this crate names it directly (only via the
//! `TryEncodeOutcome::Encoded(_)` pattern), and `training_loop` is a private
//! module, so re-exporting it here would just be dead code under this
//! crate's `-D warnings` gate. Reach it via
//! `vox_plugin_mens_candle_core::candle_qlora_train::training_loop::types::EncodedTrainStep`
//! if a future caller needs to name it.
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
        /// `Some` when the forward ran checkpointed: `loss` is then a plain
        /// scalar and the loop must run the segmented backward.
        ckpt: Option<CheckpointedCe>,
    },
}

/// What a checkpointed forward leaves for its backward: the head is re-run in
/// row chunks there, so no `[seq, vocab]` tensor outlives one chunk (S2).
pub struct CheckpointedCe {
    pub segments: Vec<crate::model::CheckpointSegment>,
    /// Last segment's output; its tape reaches back to `segments.last().input`.
    pub hidden: candle_core::Tensor,
    /// Per-position target ids, `[seq]`.
    pub targets: candle_core::Tensor,
    /// Per-position CE weights (0 = unsupervised), and their sum.
    pub mask: Vec<f32>,
    pub mask_sum: f32,
    /// The row's sample weight (the backward loss is `weight * CE`).
    pub weight: f32,
    pub max_target: usize,
}
