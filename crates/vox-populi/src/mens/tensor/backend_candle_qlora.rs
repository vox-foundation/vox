//! Candle **QLoRA** trainer via **qlora-rs**: **NF4-quantized LM head** + trainable LoRA; context
//! embeddings stay **mmap `f32`** (`index_select` from HF safetensors). Device: CUDA / Metal when
//! built with `candle-qlora-cuda` / `candle-qlora-metal`, else CPU (`VOX_CANDLE_DEVICE=cpu` to force).

use std::path::Path;

use crate::mens::tensor::backend::TrainingBackend;
use crate::mens::tensor::device::DeviceKind;
use crate::mens::tensor::training_config::LoraTrainingConfig;

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
    use crate::mens::tensor::training_config::LoraTrainingConfig;

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
}
