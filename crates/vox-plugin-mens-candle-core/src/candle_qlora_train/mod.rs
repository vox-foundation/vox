//! Shared (device-agnostic) pieces of the QLoRA training loop, previously
//! duplicated byte-for-byte between `vox-plugin-mens-candle-metal` and
//! `vox-plugin-mens-candle-cuda`'s own `candle_qlora_train` modules.
//!
//! The bulk of `candle_qlora_train` (the actual training driver, GPU-specific
//! device selection, checkpoint pruning, gradient-checkpointed backward) stays
//! in each plugin crate — it is genuinely divergent (real CUDA/Metal behavior
//! differences), not accidental duplication. Only the pieces that were
//! byte-identical between the two plugins live here.

pub mod adapter_load;
pub mod ce_mask_align;
pub mod db_event;
pub mod db_thread;
pub mod epoch_boundary;
pub mod oom;
pub mod validation;

// Re-exported at this level (not just `adapter_load::load_adapter_into_trainer`)
// so `super::super::load_adapter_into_trainer` from `training_loop::checkpoint`
// resolves exactly as it did when both lived in one plugin's `mod.rs`.
pub use adapter_load::load_adapter_into_trainer;

pub mod training_loop {
    pub mod checkpoint;
    pub mod curriculum;
    pub mod encoding;
    pub mod logic;
    pub mod telem_helpers;
    pub mod types;
}
