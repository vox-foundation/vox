//! Load Burn checkpoints for HTTP / eval inference.
//!
//! Supports:
//! - **`LoraVoxTransformer`** checkpoints from `vox populi train` (`model_final.bin`, `checkpoint_*.bin`)
//! - **`VoxTransformer`** checkpoints from `vox populi merge-weights` (`model_merged.bin`)

use std::path::Path;

use burn::tensor::backend::Backend;
use burn::tensor::{Int, Tensor};

use super::burn_stack::VoxTransformer;
use super::lora::LoraVoxTransformer;
use super::train::Checkpoint;

/// Architecture + LoRA hyperparameters needed to construct adapters before checkpoint load.
#[derive(Debug, Clone, Copy)]
pub struct BurnInferenceLoadSpec {
    pub vocab_size: usize,
    pub d_model: usize,
    pub n_heads: usize,
    pub n_layers: usize,
    pub rank: usize,
    pub alpha: f32,
}

/// Loaded Burn model for token-in → logits-out inference (`forward` matches training layout).
#[derive(Debug)]
pub enum BurnInferenceModel<B: Backend> {
    Lora(LoraVoxTransformer<B>),
    Merged(VoxTransformer<B>),
}

impl<B: Backend> BurnInferenceModel<B> {
    pub fn forward(&self, x: Tensor<B, 2, Int>) -> Tensor<B, 3> {
        match self {
            Self::Lora(m) => m.forward(x),
            Self::Merged(m) => m.forward(x),
        }
    }
}

/// Try LoRA checkpoint first, then merged `VoxTransformer` (merge-weights output).
pub fn load_burn_inference_model<B: Backend>(
    device: &B::Device,
    path: &Path,
    spec: BurnInferenceLoadSpec,
) -> anyhow::Result<BurnInferenceModel<B>> {
    let lora = LoraVoxTransformer::new(
        device,
        spec.vocab_size,
        spec.d_model,
        spec.n_heads,
        spec.n_layers,
        spec.rank,
        spec.alpha,
    );
    match Checkpoint::load(lora, path) {
        Ok(m) => Ok(BurnInferenceModel::Lora(m)),
        Err(e_lora) => {
            let merged = VoxTransformer::new(
                device,
                spec.vocab_size,
                spec.d_model,
                spec.n_heads,
                spec.n_layers,
            );
            match Checkpoint::load(merged, path) {
                Ok(m) => Ok(BurnInferenceModel::Merged(m)),
                Err(e_merged) => anyhow::bail!(
                    "checkpoint is neither a LoRA adapter nor a merged VoxTransformer (merge-weights output). \
                     LoRA load error: {e_lora}. Merged load error: {e_merged}."
                ),
            }
        }
    }
}

#[cfg(all(test, feature = "gpu"))]
mod tests {
    use super::*;
    use burn::backend::NdArray;
    use tempfile::tempdir;

    type B = NdArray<f32>;

    #[test]
    fn load_recognizes_lora_vs_merged_checkpoints() {
        let dir = tempdir().expect("tempdir");
        let lora_path = dir.path().join("lora.bin");
        let merged_path = dir.path().join("merged.bin");

        let device = <B as Backend>::Device::default();
        let lora: LoraVoxTransformer<B> = LoraVoxTransformer::new(&device, 32, 8, 2, 1, 4, 8.0);
        Checkpoint::save(&lora, &lora_path).expect("save lora");

        let spec = BurnInferenceLoadSpec {
            vocab_size: 32,
            d_model: 8,
            n_heads: 2,
            n_layers: 1,
            rank: 4,
            alpha: 8.0,
        };
        let loaded = load_burn_inference_model::<B>(&device, &lora_path, spec).expect("load lora");
        assert!(matches!(loaded, BurnInferenceModel::Lora(_)));

        let merged: VoxTransformer<B> = lora.merge();
        Checkpoint::save(&merged, &merged_path).expect("save merged");
        let loaded_m =
            load_burn_inference_model::<B>(&device, &merged_path, spec).expect("load merged");
        assert!(matches!(loaded_m, BurnInferenceModel::Merged(_)));
    }
}
