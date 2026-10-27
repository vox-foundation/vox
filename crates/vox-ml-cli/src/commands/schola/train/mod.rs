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
}
