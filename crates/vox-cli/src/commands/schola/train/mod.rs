//! `vox schola train` — native LoRA training worker.

#[cfg(feature = "gpu")]
mod gpu;
mod run_train;
mod spawn;

pub use run_train::run_train;
pub use spawn::spawn_train_with_log;
