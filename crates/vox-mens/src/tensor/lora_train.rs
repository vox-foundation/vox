//! Native Mens training entrypoints (`vox schola train`).
//!
//! **SSOT:** Canonical CLI entry is `vox schola train`. The sole active backend is
//! Candle+qlora-rs (`--backend qlora`). Burn LoRA is permanently deprecated.
//!
//! Dispatch is **contract-first**: [`FineTuneContract`] + [`ExecutionPlanner`] → kernel.

use std::path::Path;

use crate::tensor::backend::TrainingBackend;
use crate::tensor::backend_candle_qlora::CandleQloraBackend;
use crate::tensor::device::DeviceKind;
use crate::tensor::execution_planner::ExecutionPlanner;
use crate::tensor::finetune_contract::FineTuneContract;
use crate::tensor::preflight_train::preflight_for_contract;
use crate::tensor::train_backend::PopuliTrainBackend;
use crate::tensor::training_config::LoraTrainingConfig;

/// Dispatch training by execution kernel after contract validation and preflight.
///
/// The only valid backend is [`PopuliTrainBackend::CandleQlora`].
/// Requesting [`PopuliTrainBackend::BurnLora`] returns an instructive error.
pub fn run_mens_training(
    backend: PopuliTrainBackend,
    data_dir: &Path,
    output_dir: Option<&Path>,
    config: &LoraTrainingConfig,
    device_kind: DeviceKind,
    system_prompt: &str,
) -> anyhow::Result<crate::tensor::backend::TrainingSummary> {
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
            anyhow::bail!(
                "Burn LoRA backend is permanently deprecated. \
                 Use `vox schola train --backend qlora --tokenizer hf --model <hf_repo>`. \
                 See docs/src/architecture/mens-training-ssot.md."
            )
        }
        PopuliTrainBackend::CandleQlora => {
            CandleQloraBackend.run(data_dir, output_dir, &cfg, device_kind, system_prompt)
        }
    }
}
