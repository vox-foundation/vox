//! `vox mens train` — native LoRA training worker (implementation under `commands::schola::train`).

#[cfg(feature = "gpu")]
mod gpu;
#[cfg(feature = "gpu")]
mod plan_only;
mod run_train;
mod spawn;

pub(crate) use run_train::force_train_env;
pub use run_train::run_train;
pub use spawn::spawn_train_with_log;

/// Per-invocation switches for `vox mens train` that are not training knobs.
#[derive(Debug, Clone, Copy, Default)]
pub struct TrainRunOpts {
    /// Resolve and print the run, then return before the base-model download
    /// (no weights loaded, nothing written to the output directory).
    pub plan_only: bool,
    /// Render the plan as one JSON object on stdout instead of text.
    pub json: bool,
    /// `--max-runtime-secs`: local run cap, forwarded to the trainer config.
    pub max_runtime_secs: Option<u64>,
    /// `--lora-exclude-lm-head`: keep `lm_head` frozen outside LoRA adapters.
    pub lora_exclude_lm_head: bool,
    /// `--use-rslora`: enable `alpha / sqrt(rank)` rsLoRA scaling.
    pub use_rslora: bool,
    /// `--token-normalized-grad-accum`: scale window loss by valid supervised tokens.
    pub token_normalized_grad_accum: bool,
    /// `--sequence-packing`: pack sequences with block-diagonal causal masks & RoPE reset.
    pub sequence_packing: bool,
}
