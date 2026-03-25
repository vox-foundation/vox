//! [`TrainingBackend`] — Burn LoRA vs Candle + **qlora-rs** QLoRA (`--backend lora` / `qlora`).
//!
//! Implementations: `backend_burn_lora`, `backend_candle_qlora`. Dispatch:
//! `lora_train::run_mens_training`.

use std::path::Path;

use crate::mens::tensor::device::DeviceKind;
use crate::mens::tensor::training_config::LoraTrainingConfig;

/// Summary of a completed training run.
#[derive(Debug)]
pub struct TrainingSummary {
    pub wall_secs: f64,
    pub total_steps: usize,
    pub total_tokens: usize,
    pub ms_per_step: f64,
}

/// One Mens training implementation (Burn LoRA today; Candle NF4 QLoRA when implemented).
pub trait TrainingBackend {
    fn run(
        &self,
        data_dir: &Path,
        output_dir: Option<&Path>,
        config: &LoraTrainingConfig,
        device_kind: DeviceKind,
        system_prompt: &str,
    ) -> anyhow::Result<TrainingSummary>;
}
