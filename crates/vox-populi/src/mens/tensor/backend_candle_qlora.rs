//! Candle **QLoRA** trainer via **qlora-rs**: **NF4-quantized LM head** + trainable LoRA; context
//! embeddings stay **mmap `f32`** (`index_select` from HF safetensors). Device: CUDA / Metal when
//! built with `candle-qlora-cuda` / `candle-qlora-metal`, else CPU (`VOX_CANDLE_DEVICE=cpu` to force).

use std::path::Path;

use crate::mens::tensor::backend::TrainingBackend;
use crate::mens::tensor::device::DeviceKind;
use crate::mens::tensor::training_config::{LoraTrainingConfig, OptimizerExperimentMode};

/// Candle + HF tokenizer path (`--backend qlora`).
#[derive(Debug, Clone, Copy, Default)]
pub struct CandleQloraBackend;

impl TrainingBackend for CandleQloraBackend {
    fn run(
        &self,
        data_dir: &Path,
        output_dir: Option<&Path>,
        config: &LoraTrainingConfig,
        device_kind: DeviceKind,
        system_prompt: &str,
    ) -> anyhow::Result<crate::mens::tensor::backend::TrainingSummary> {
        tracing::debug!(backend = "candle_qlora", "Candle qlora backend run");
        if config.optimizer_experiment_mode != OptimizerExperimentMode::Off {
            let optimizer_resolved =
                vox_clavis::resolve_secret(vox_clavis::SecretId::VoxMensExperimentalOptimizer);
            let enabled = optimizer_resolved
                .expose()
                .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
            if !enabled {
                anyhow::bail!(
                    "optimizer experiment mode `{}` requires VOX_MENS_EXPERIMENTAL_OPTIMIZER=1",
                    match config.optimizer_experiment_mode {
                        OptimizerExperimentMode::Off => "off",
                        OptimizerExperimentMode::MuonClipLike => "muon_clip_like",
                    }
                );
            }
            tracing::warn!(
                mode = ?config.optimizer_experiment_mode,
                "experimental optimizer lane enabled"
            );
        }
        #[cfg(feature = "mens-candle-qlora")]
        {
            super::candle_qlora_train::run_candle_qlora_train(
                data_dir,
                output_dir,
                config,
                device_kind,
                system_prompt,
            )
        }
        #[cfg(not(feature = "mens-candle-qlora"))]
        {
            let _ = (data_dir, output_dir, config, device_kind, system_prompt);
            anyhow::bail!(
                "`vox-populi` was built without Candle QLoRA. Enable `mens-candle-qlora` (e.g. `mens-train` on `vox-populi` / `gpu` on `vox-cli`)."
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::mens::tensor::backend::TrainingBackend;
    use crate::mens::tensor::device::DeviceKind;
    use crate::mens::tensor::training_config::{LoraTrainingConfig, OptimizerExperimentMode};

    use super::CandleQloraBackend;

    #[cfg(feature = "mens-candle-qlora")]
    #[test]
    fn qlora_bails_without_hf_setup() {
        let err = CandleQloraBackend
            .run(
                Path::new("."),
                None,
                &LoraTrainingConfig::default(),
                DeviceKind::Cpu,
                "",
            )
            .unwrap_err();
        let s = err.to_string().to_lowercase();
        assert!(
            s.contains("tokenizer") || s.contains("hf") || s.contains("qlora"),
            "{s}"
        );
    }

    #[test]
    fn experimental_optimizer_requires_env_guard() {
        let cfg = LoraTrainingConfig {
            optimizer_experiment_mode: OptimizerExperimentMode::MuonClipLike,
            ..Default::default()
        };
        let err = CandleQloraBackend
            .run(Path::new("."), None, &cfg, DeviceKind::Cpu, "")
            .expect_err("guard should fail before training starts");
        assert!(err.to_string().contains("VOX_MENS_EXPERIMENTAL_OPTIMIZER"));
    }
}
