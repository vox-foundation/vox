//! Internal types for the QLoRA training loop (CUDA plugin).
//!
//! `QloraTrainingResume`, `EncodedTrainStep`, `TryEncodeOutcome`, and the
//! generic `MaskedCeForward` enum live in `vox-plugin-mens-candle-core`; this
//! alias specializes it with CUDA's `CheckpointSegment` type.
pub use vox_plugin_mens_candle_core::candle_qlora_train::training_loop::types::{
    QloraTrainingResume, TryEncodeOutcome,
};

pub type MaskedCeForward =
    vox_plugin_mens_candle_core::candle_qlora_train::training_loop::types::MaskedCeForward<
        crate::model::CheckpointSegment,
    >;
