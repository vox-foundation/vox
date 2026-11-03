//! Checkpoint save/load for the CandleModel.
//!
//! `save` delegates to `QLoraTrainer::save_adapter` from `qlora-rs`, so it only applies
//! to a `CandleModel` that carries a trainer; `load_from_path` does not attach one. The
//! full training path constructs and owns its own `QLoraTrainer` inside
//! `candle_qlora_train`, which saves adapters in `checkpoint_mid.rs` / `finalize.rs` and
//! resumes from them in `training_loop/checkpoint.rs`. There is no load counterpart here.

use crate::model::CandleModel;

/// Save a LoRA adapter checkpoint to `dest`.
pub fn save(model: &CandleModel, dest: &str) -> anyhow::Result<()> {
    if let Some(trainer) = &model.trainer {
        trainer.save_adapter(std::path::Path::new(dest))?;
        Ok(())
    } else {
        anyhow::bail!("Cannot save checkpoint: model has no active trainer.")
    }
}
