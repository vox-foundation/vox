//! `vox populi merge-weights` — fold LoRA adapters into base model.

use anyhow::Result;
use std::path::PathBuf;

// Without `gpu`, the CLI variant is absent from clap dispatch but we keep the module for tests / messages.
#[cfg_attr(not(feature = "gpu"), allow(dead_code))]
pub fn run_merge_weights(
    model: PathBuf,
    output: Option<PathBuf>,
    rank: usize,
    alpha: f32,
) -> Result<()> {
    if !model.exists() {
        anyhow::bail!(
            "Checkpoint not found: {}\nRun `vox populi train` first.",
            model.display()
        );
    }
    if model
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n == "candle_qlora_adapter.safetensors")
    {
        anyhow::bail!(
            "`merge-weights` merges **Burn** LoRA checkpoints (`*.bin` from `--backend lora`).\n\
             Candle QLoRA (`--backend qlora`) writes **LoRA-only** `candle_qlora_adapter.safetensors` + `candle_qlora_adapter_meta.json` (format v2).\n\
             To fold those LoRA deltas into base **f32** safetensors (subset of keys), use **`vox populi merge-qlora`** (`--base-shard`, `--adapter`, `--meta`, `--output`).\n\
             See `docs/src/architecture/populi-training-ssot.md`."
        );
    }

    #[cfg(feature = "gpu")]
    {
        use owo_colors::OwoColorize;
        use vox_populi::tensor::burn_stack::VoxTransformer;
        use vox_populi::tensor::lora::LoraVoxTransformer;
        use vox_populi::tensor::train::Checkpoint;

        let run_dir = model.parent().unwrap_or(std::path::Path::new("."));
        let arch = vox_populi::tensor::manifest::ArchParams::from_manifest(run_dir)?;
        let out_path = output.unwrap_or_else(|| run_dir.join("model_merged.bin"));

        eprintln!("{}", "╔══════════════════════════════════════════╗".cyan());
        eprintln!("{}", "║   Vox Populi: Merge LoRA Weights         ║".cyan());
        eprintln!("{}", "╚══════════════════════════════════════════╝".cyan());
        eprintln!("  Input:  {}", model.display());
        eprintln!("  Output: {}", out_path.display());
        eprintln!(
            "  Config: {}L/{}H/{}D, rank={} alpha={}",
            arch.n_layers, arch.n_heads, arch.d_model, rank, alpha
        );
        eprintln!();

        type B = vox_populi::burn::backend::Wgpu;
        let device = <B as vox_populi::burn::tensor::backend::Backend>::Device::default();

        eprintln!("  Loading LoRA checkpoint...");
        let lora_model: LoraVoxTransformer<B> = LoraVoxTransformer::new(
            &device,
            arch.vocab_size,
            arch.d_model,
            arch.n_heads,
            arch.n_layers,
            rank,
            alpha,
        );
        let lora_model = Checkpoint::load(lora_model, &model)
            .map_err(|e| anyhow::anyhow!("Failed to load LoRA checkpoint: {}", e))?;

        eprintln!("  Merging LoRA adapters into base weights...");
        let merged: VoxTransformer<B> = lora_model.merge();

        eprintln!("  Saving merged checkpoint...");
        Checkpoint::save(&merged, &out_path)
            .map_err(|e| anyhow::anyhow!("Failed to save merged model: {}", e))?;

        let size = std::fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0);
        eprintln!();
        eprintln!("  {} Merge complete!", "✓".green());
        eprintln!(
            "  Merged model: {} ({:.1} MB)",
            out_path.display(),
            size as f64 / 1_048_576.0
        );
        eprintln!(
            "  Serve with: `vox populi serve --model {}` (build `vox-cli` with `--features execution-api`; merged and LoRA `.bin` checkpoints are both supported).",
            out_path.display()
        );
        Ok(())
    }

    #[cfg(not(feature = "gpu"))]
    {
        let _ = (output, rank, alpha);
        anyhow::bail!(
            "`vox populi merge-weights` merges Burn LoRA checkpoints on the GPU stack and requires the **`gpu`** feature.\n\
             Rebuild: `cargo build -p vox-cli --features gpu` (or use the default feature set, which includes `gpu`)."
        );
    }
}

#[cfg(test)]
mod tests {
    use super::run_merge_weights;

    #[test]
    fn merge_weights_rejects_qlora_adapter_filename_with_merge_qlora_hint() {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("candle_qlora_adapter.safetensors");
        std::fs::write(&p, b"x").expect("write");
        let err = run_merge_weights(p, None, 8, 16.0).expect_err("should reject");
        let s = err.to_string();
        assert!(
            s.contains("merge-qlora") || s.contains("merge_qlora"),
            "expected merge-qlora hint, got: {s}"
        );
    }

    #[cfg(not(feature = "gpu"))]
    #[test]
    fn merge_weights_errors_when_gpu_feature_disabled() {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("adapter.bin");
        std::fs::write(&p, b"x").expect("write");
        let err = run_merge_weights(p, None, 8, 16.0).expect_err("gpu required");
        let s = err.to_string();
        assert!(
            s.contains("gpu") || s.contains("GPU"),
            "expected gpu feature hint, got: {s}"
        );
    }
}
