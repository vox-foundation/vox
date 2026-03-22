//! Single **preflight entry** for native training: dispatches HF checks by execution kernel.

use super::execution_planner::preflight_model_bundle;
use super::finetune_contract::FineTuneContract;
use super::train_backend::PopuliTrainBackend;

/// Run model/tokenizer preflight for the resolved kernel (Candle qlora bundle or Burn HF paths).
pub fn preflight_for_contract(
    kernel: PopuliTrainBackend,
    contract: &FineTuneContract,
) -> anyhow::Result<()> {
    preflight_model_bundle(kernel, contract)
}
