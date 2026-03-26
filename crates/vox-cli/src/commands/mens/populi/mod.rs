//! `vox mens` action surface: clap enum + async dispatch.

mod action;
mod dispatch;
mod mens_tail_subcommands;
#[cfg(feature = "gpu")]
mod train_arm;

#[cfg(feature = "gpu")]
pub use action::{
    MensTokenizerCli, OptimizerExperimentModeCli, PopuliTrainBackendCli,
    TrainingDeploymentTargetCli,
};
pub use action::{PipelineProgress, PipelineStage, PopuliAction};
pub use dispatch::run;
