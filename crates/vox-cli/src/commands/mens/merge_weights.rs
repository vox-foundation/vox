//! `vox mens merge-weights` — merge a **Burn** LoRA `*.bin` checkpoint into dense `model_merged.bin`.

use std::path::PathBuf;

use anyhow::Context;
use vox_populi::mens::burn::backend::NdArray;
use vox_populi::mens::burn::tensor::backend::Backend;
use vox_populi::mens::tensor::LoraVoxTransformer;
use vox_populi::mens::tensor::manifest;
use vox_tensor::train::Checkpoint;

/// Merge Burn [`LoraVoxTransformer`] weights into a plain [`VoxTransformer`] and save as `model_merged.bin`.
///
/// `rank` and `alpha` are reserved for future CLI overrides; architecture and LoRA hyperparameters are
/// taken from `training_manifest.json` in the checkpoint’s parent directory (same layout as `vox mens train --backend lora`).
#[allow(unused_variables)]
pub fn run_merge_weights(
    checkpoint: PathBuf,
    output: Option<PathBuf>,
    rank: usize,
    alpha: f32,
) -> anyhow::Result<()> {
    let name = checkpoint
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let ext = checkpoint
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if ext.eq_ignore_ascii_case("safetensors")
        && (name.contains("candle_qlora")
            || name.contains("qlora_adapter")
            || name.eq_ignore_ascii_case("adapter.safetensors"))
    {
        anyhow::bail!(
            "merge-weights is for **Burn** LoRA checkpoints (`*.bin`), not Candle QLoRA adapter safetensors.\n\
             This path looks like a Candle QLoRA artifact (`{name}`). Use **`vox mens merge-qlora`** \
             (or **`vox schola merge-qlora`**) with `--base-shard`, `--adapter`, `--meta`, and `--output`.\n\
             See `docs/src/architecture/mens-training-ssot.md`."
        );
    }

    if !checkpoint.is_file() {
        anyhow::bail!(
            "Checkpoint not found at {}.\n\
             Pass a Burn LoRA `*.bin` from `vox mens train --backend lora` (with `training_manifest.json` in the same run directory).",
            checkpoint.display()
        );
    }

    let run_dir = checkpoint
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let m = manifest::load_manifest(&run_dir)
        .context("read training manifest")?
        .with_context(|| {
            format!(
                "missing {} — merge-weights needs vocab/architecture from the Burn training run",
                run_dir.join("training_manifest.json").display()
            )
        })?;

    type B = NdArray<f32>;
    let device = <B as Backend>::Device::default();
    let fresh = LoraVoxTransformer::<B>::new(
        &device,
        m.vocab_size,
        m.d_model,
        m.n_heads,
        m.n_layers,
        m.rank,
        m.alpha,
    );
    let loaded: LoraVoxTransformer<B> = Checkpoint::load(fresh, &checkpoint)
        .map_err(|e| anyhow::anyhow!("Checkpoint load: {e}"))?;
    let merged = loaded.merge();
    let out = output.unwrap_or_else(|| run_dir.join("model_merged.bin"));
    Checkpoint::save(&merged, &out).map_err(|e| anyhow::anyhow!("save merged checkpoint: {e}"))?;
    eprintln!("Wrote merged Burn weights to {}", out.display());
    Ok(())
}
