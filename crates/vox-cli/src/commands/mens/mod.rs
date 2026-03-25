//! `vox mens` — the unified AI/ML command surface for Vox.
//!
//! All model training, serving, and corpus management lives here.
//! This is the canonical entry point; the deprecated top-level `vox train` remains
//! for Together / legacy native paths (see registry + `commands::ai::train`).
//!
//! ## Subcommands
//!
//! ```text
//! vox schola train      — Fine-tune: Candle QLoRA (default) or Burn LoRA (`--backend lora` deprecated)
//! vox mens serve      — HTTP inference (build `vox-cli` with `--features execution-api`)
//! vox mens corpus     — Training data pipeline (extract, validate, mix, eval…)
//! vox mens probe      — Detect GPU capabilities and print recommended LoRA training config
//! vox mens status     — Show training run status from the latest telemetry log
//! vox mens eval-local — Evaluate a trained model against the heldout benchmark set
//! Oratio speech-to-text lives at **`vox oratio`** (top-level), not under `mens`.
//! ```

/// Latency and throughput benchmarking for completions.
pub mod bench_completion;
pub(crate) mod eval_gate;
#[cfg(feature = "gpu")]
mod eval_local;
mod eval_local_prompt;
#[cfg(feature = "gpu")]
mod merge_weights;
#[cfg(feature = "gpu")]
pub mod models;
#[cfg(feature = "mens-base")]
mod pipeline;
/// AI-agent planning sessions and task decomposition.
pub mod plan;
#[cfg(feature = "gpu")]
mod probe;
mod status;
mod system_prompt_template;

mod populi;

#[cfg(feature = "gpu")]
pub use populi::{MensTokenizerCli, PopuliTrainBackendCli, TrainingDeploymentTargetCli};
pub use populi::{PipelineProgress, PipelineStage, PopuliAction, run};

#[cfg(all(test, feature = "gpu"))]
mod tests {
    include!("populi/gpu_tests_body.rs");
}
