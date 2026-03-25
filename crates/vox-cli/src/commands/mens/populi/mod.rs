//! `vox mens` action surface: clap enum + async dispatch.

mod action;
#[cfg(feature = "gpu")]
mod train_arm;
mod dispatch;

pub use action::{PipelineProgress, PipelineStage, PopuliAction};
#[cfg(feature = "gpu")]
pub use action::{MensTokenizerCli, PopuliTrainBackendCli, TrainingDeploymentTargetCli};
pub use dispatch::run;
