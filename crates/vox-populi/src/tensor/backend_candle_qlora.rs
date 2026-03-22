//! Candle **QLoRA** trainer via **qlora-rs**: **NF4-quantized LM head** + trainable LoRA; context
//! embeddings stay **mmap `f32`** (`index_select` from HF safetensors). Device: CUDA / Metal when
//! built with `candle-qlora-cuda` / `candle-qlora-metal`, else CPU (`VOX_CANDLE_DEVICE=cpu` to force).

use std::path::Path;

use crate::tensor::backend::TrainingBackend;
use crate::tensor::device::DeviceKind;
use crate::tensor::training_config::LoraTrainingConfig;

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
    ) -> anyhow::Result<()> {
        tracing::debug!(backend = "candle_qlora", "Candle qlora backend run");
        #[cfg(feature = "candle-qlora")]
        {
            super::candle_qlora_train::run_candle_qlora_train(
                data_dir,
                output_dir,
                config,
                device_kind,
                system_prompt,
            )
        }
        #[cfg(not(feature = "candle-qlora"))]
        {
            let _ = (data_dir, output_dir, config, device_kind, system_prompt);
            anyhow::bail!(
                "`vox-populi` was built without `candle-qlora`. Use `features = [\"train\"]` (includes Candle) or add `candle-qlora` explicitly."
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::tensor::backend::TrainingBackend;
    use crate::tensor::device::DeviceKind;
    use crate::tensor::training_config::LoraTrainingConfig;

    use super::CandleQloraBackend;

    #[cfg(feature = "candle-qlora")]
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
