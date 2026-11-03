//! Internal types for the QLoRA training loop.
//!
//! `QloraTrainingResume`, `EncodedTrainStep` and `TryEncodeOutcome` moved to
//! `vox-plugin-mens-candle-core` — byte-for-byte identical to the Metal
//! plugin's copies. `MaskedCeForward` stays here: it carries an extra
//! `segments` field (below) for gradient-checkpointing support the Metal
//! plugin does not implement yet. `EncodedTrainStep` is not re-exported at
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
        /// Checkpoint segments when the forward was run with activation
        /// checkpointing. `None` for the eager path. When `Some`, the loss's
        /// autograd tape spans only the final segment + head; the training loop
        /// must run the segmented recompute-backward (threading the boundary
        /// cotangent in reverse) rather than a single `backward_step`.
        segments: Option<Vec<crate::model::CheckpointSegment>>,
    },
}
