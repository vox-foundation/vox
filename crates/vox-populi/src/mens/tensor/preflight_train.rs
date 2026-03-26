//! Single **preflight entry** for native training: dispatches HF checks by execution kernel.

use std::fs;
use std::path::Path;

use super::execution_planner::preflight_model_bundle;
use super::finetune_contract::FineTuneContract;
use super::train_backend::PopuliTrainBackend;

/// Machine-readable preflight record aligned with `contracts/mens/training-preflight.schema.json`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TrainingPreflightRecord {
    pub schema_version: String,
    pub contract_digest: String,
    /// Stable kernel label (`qlora` / `lora`); matches [`PopuliTrainBackend`] display.
    pub execution_kernel: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

impl TrainingPreflightRecord {
    #[must_use]
    pub fn new(
        contract_digest: impl Into<String>,
        kernel: PopuliTrainBackend,
        notes: Vec<String>,
    ) -> Self {
        Self {
            schema_version: "vox.mens.preflight.v0".into(),
            contract_digest: contract_digest.into(),
            execution_kernel: kernel.to_string(),
            notes,
        }
    }
}

/// Write [`TrainingPreflightRecord`] as JSON next to run artifacts (`training-preflight.json`).
pub fn write_training_preflight_json(
    path: &Path,
    record: &TrainingPreflightRecord,
) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(record)?;
    fs::write(path, text)?;
    Ok(())
}

/// Run model/tokenizer preflight for the resolved kernel (Candle qlora bundle or Burn HF paths).
pub fn preflight_for_contract(
    kernel: PopuliTrainBackend,
    contract: &FineTuneContract,
) -> anyhow::Result<()> {
    preflight_model_bundle(kernel, contract)
}

#[cfg(all(test, feature = "mens-train"))]
mod tests {
    use super::*;

    #[test]
    fn preflight_record_serializes_schema_version_and_kernel() {
        let r = TrainingPreflightRecord::new(
            "digest-test",
            PopuliTrainBackend::CandleQlora,
            vec!["ok".into()],
        );
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["schema_version"], "vox.mens.preflight.v0");
        assert_eq!(v["contract_digest"], "digest-test");
        assert_eq!(v["execution_kernel"], "qlora");
        assert_eq!(v["notes"], serde_json::json!(["ok"]));
    }
}
