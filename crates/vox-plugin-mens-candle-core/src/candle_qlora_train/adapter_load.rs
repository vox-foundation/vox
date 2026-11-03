//! Warm-start a `QLoraTrainer`'s varmap from a saved adapter checkpoint.
//!
//! Extracted from `candle_qlora_train::mod` — byte-for-byte identical between
//! `vox-plugin-mens-candle-metal` and `vox-plugin-mens-candle-cuda`. `pub` here
//! (the original was a private `fn`) because both plugins' `training_loop::checkpoint`
//! call it across the crate boundary.

use std::path::Path;

use anyhow::{Context, Result};
use qlora_rs::training::QLoraTrainer;

/// Load LoRA adapter weights into a trainer's varmap (warm-start).
pub fn load_adapter_into_trainer(trainer: &mut QLoraTrainer, path: &Path) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("checkpoint adapter not found: {}", path.display());
    }
    trainer
        .load_lora_weights(path)
        .context("warm-start LoRA weights")?;
    crate::train_log::info(&format!(
        "Warm-started LoRA weights from {}",
        path.display()
    ));
    Ok(())
}
