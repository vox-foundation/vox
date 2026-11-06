//! Checkpoint save/load for the CandleModel.
//!
//! # SP3 stub
//!
//! The real implementation calls `QLoraTrainer::save_adapter(path)` from `qlora-rs`.
//! The `QLoraTrainer` is constructed inside `vox-populi`'s `run_candle_qlora_train`
//! and holds the VarMap with trainable LoRA parameters. The trainer is not owned by
//! `CandleModel` in the current extraction because:
//!
//! 1. `CandleModel::load_from_path` is stubbed (see model.rs) — no model is actually
//!    loaded yet from within this plugin.
//! 2. `QLoraTrainer` has a non-trivial lifecycle: it is constructed after
//!    `QLoraConfig::preset_all_bf16` and `trainer.init_optimizer()` which depend on
//!    the full `QLoraTrainingConfig` assembled from `LoraTrainingConfig`.
//!
//! Batch 3/4 plan:
//! - Extend `CandleModel` to hold both `Qwen35Model` and `QLoraTrainer`.
//! - Wire `save` to call `trainer.save_adapter(Path::new(dest))`.
//! - Wire `load_from_path` to reconstruct both from the model path + a sidecar
//!   `adapter.safetensors` for warm-start.
//!
//! Source reference: `crates/vox-populi/src/mens/tensor/candle_qlora_train/checkpoint_mid.rs`
//! and `finalize.rs` for the save patterns; `training_loop/checkpoint.rs` for resume logic.

use crate::model::CandleModel;

/// Save a LoRA adapter checkpoint to `dest`.
pub fn save(_model: &CandleModel, _dest: &str) -> anyhow::Result<()> {
    // Planned batch 3/4: call model.trainer.save_adapter(Path::new(dest))
    anyhow::bail!(
        "vox-plugin-mens-candle-cuda: save_checkpoint not yet wired (SP3 stub). \
         Batch 3/4 will add QLoraTrainer to CandleModel and call save_adapter. \
         See checkpoint.rs for details."
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::{DType, Device, Tensor};
    use qlora_rs::QLoraConfig;
    use qlora_rs::qlora::QuantizedLinear;
    use qlora_rs::quantization::ComputeDType;

    /// A zero-layer model: enough to build a `CandleModel` without weights on disk.
    fn tiny_model() -> CandleModel {
        let dev = Device::Cpu;
        let d = 8usize;
        let mut cfg = QLoraConfig::preset_all_bf16(4, 8);
        cfg.quantization.compute_dtype = ComputeDType::F32;
        cfg.cache_dequantized = false;
        let w = Tensor::randn(0f32, 0.02f32, (d, d), &dev).unwrap();
        let qwen = crate::model::Qwen35Model {
            embed_tokens: Tensor::zeros((4, d), DType::F32, &dev).unwrap(),
            layers: Vec::new(),
            norm: candle_nn::RmsNorm::new(Tensor::ones(d, DType::F32, &dev).unwrap(), 1e-6),
            lm_head: QuantizedLinear::from_weight(&w, None, &cfg, &dev).unwrap(),
        };
        let tokenizer = tokenizers::Tokenizer::new(tokenizers::models::bpe::BPE::default());
        CandleModel {
            engine: std::sync::Mutex::new(crate::inference::InferenceEngine {
                model: crate::inference::InferenceModel::Qwen35(qwen),
                tokenizer,
                device: dev,
                eos_token_ids: Vec::new(),
            }),
            model_path: "unused".to_string(),
            trainer: None,
        }
    }

    #[test]
    fn save_reports_the_sp3_stub_instead_of_pretending_to_write() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("adapter.safetensors");
        let err = save(&tiny_model(), dest.to_str().unwrap()).unwrap_err();
        assert!(err.to_string().contains("not yet wired"), "{err}");
        assert!(!dest.exists(), "a stub save must not leave a file behind");
    }
}
