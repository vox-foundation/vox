//! [`TrainingBackend`] — Burn LoRA vs Candle + **qlora-rs** QLoRA (`--backend lora` / `qlora`).
//!
//! Implementations: `backend_burn_lora`, `backend_candle_qlora`. Dispatch:
//! `lora_train::run_populi_training`.

use std::path::Path;

use crate::tensor::device::DeviceKind;
use crate::tensor::training_config::LoraTrainingConfig;

/// One Populi training implementation (Burn LoRA today; Candle NF4 QLoRA when implemented).
pub trait TrainingBackend {
    fn run(
        &self,
        data_dir: &Path,
        output_dir: Option<&Path>,
        config: &LoraTrainingConfig,
        device_kind: DeviceKind,
        system_prompt: &str,
    ) -> anyhow::Result<()>;
}
