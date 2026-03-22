//! Native Populi training entrypoints (`vox populi train`).
//!
//! **SSOT:** Canonical CLI entry is `vox populi train`. Burn LoRA lives in `backend_burn_lora`;
//! Candle qlora-style (`f32` adapter on frozen HF embeddings) in `backend_candle_qlora` when built with `candle-qlora`.
//!
//! Dispatch is **contract-first**: [`FineTuneContract`] + [`ExecutionPlanner`] → kernel.

use std::path::Path;

use crate::tensor::backend::TrainingBackend;
use crate::tensor::backend_burn_lora::BurnLoraBackend;
use crate::tensor::backend_candle_qlora::CandleQloraBackend;
use crate::tensor::device::DeviceKind;
use crate::tensor::execution_planner::ExecutionPlanner;
use crate::tensor::finetune_contract::FineTuneContract;
use crate::tensor::preflight_train::preflight_for_contract;
use crate::tensor::train_backend::PopuliTrainBackend;
use crate::tensor::training_config::LoraTrainingConfig;

/// Dispatch by execution kernel after contract validation and preflight.
pub fn run_populi_training(
    backend: PopuliTrainBackend,
    data_dir: &Path,
    output_dir: Option<&Path>,
    config: &LoraTrainingConfig,
    device_kind: DeviceKind,
    system_prompt: &str,
) -> anyhow::Result<()> {
    let contract = FineTuneContract::from_training_config(config, backend);
    let planner = ExecutionPlanner {
        force_kernel: Some(backend),
    };
    let plan = planner.plan(&contract)?;
    preflight_for_contract(plan.kernel, &contract)?;

    let mut cfg = config.clone();
    cfg.finetune_contract_digest = Some(plan.contract_digest.clone());

    match plan.kernel {
        PopuliTrainBackend::BurnLora => {
            BurnLoraBackend.run(data_dir, output_dir, &cfg, device_kind, system_prompt)
        }
        PopuliTrainBackend::CandleQlora => {
            CandleQloraBackend.run(data_dir, output_dir, &cfg, device_kind, system_prompt)
        }
    }
}

/// Back-compat: [`run_populi_training`] with [`PopuliTrainBackend::BurnLora`](crate::tensor::train_backend::PopuliTrainBackend).
pub fn run_lora_training(
    data_dir: &Path,
    output_dir: Option<&Path>,
    config: &LoraTrainingConfig,
    device_kind: DeviceKind,
    system_prompt: &str,
) -> anyhow::Result<()> {
    run_populi_training(
        PopuliTrainBackend::BurnLora,
        data_dir,
        output_dir,
        config,
        device_kind,
        system_prompt,
    )
}
