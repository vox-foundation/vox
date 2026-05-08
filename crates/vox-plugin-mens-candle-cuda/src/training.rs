//! Training-loop entry points.
//!
//! SP3 sub-batch C: `run_full_training` is now wired through to
//! `candle_qlora_train::run_candle_qlora_train` via the `TrainRequest` JSON envelope.
//! The coarse-grained step wrappers (`run_train_step`, `run_eval_step`) remain stubs
//! since the plugin-host streaming protocol is deferred to SP3-D.

use std::io;

use crate::model::CandleModel;

/// Run one QLoRA training step (streaming protocol — SP3-D).
pub fn run_train_step(
    _model: &mut CandleModel,
    _batch_json: &str,
) -> anyhow::Result<String> {
    anyhow::bail!(
        "vox-plugin-mens-candle-cuda: run_train_step not yet wired (SP3-D). \
         Use run_full_training for a complete session."
    )
}

/// Run one evaluation step — SP3-D.
pub fn run_eval_step(
    _model: &CandleModel,
    _batch_json: &str,
) -> anyhow::Result<String> {
    anyhow::bail!(
        "vox-plugin-mens-candle-cuda: run_eval_step not yet wired (SP3-D). \
         Use run_full_training for a complete session."
    )
}

/// Run a complete QLoRA training session.
///
/// `config_json` is a JSON-encoded [`crate::candle_qlora_train::TrainRequest`].
/// Returns a JSON-encoded [`crate::training_summary::TrainingSummary`] on success.
pub fn run_full_training(config_json: &str) -> io::Result<String> {
    let req: crate::candle_qlora_train::TrainRequest = serde_json::from_str(config_json)
        .map_err(|e| io::Error::other(format!("invalid TrainRequest json: {e}")))?;

    let data_dir_buf;
    let data_dir = match &req.data_dir {
        Some(p) => {
            data_dir_buf = std::path::PathBuf::from(p);
            data_dir_buf.as_path()
        }
        None => {
            data_dir_buf = std::path::PathBuf::from(".");
            data_dir_buf.as_path()
        }
    };

    let output_dir_buf;
    let output_dir: Option<&std::path::Path> = match &req.output_dir {
        Some(p) => {
            output_dir_buf = std::path::PathBuf::from(p);
            Some(output_dir_buf.as_path())
        }
        None => None,
    };

    let summary = crate::candle_qlora_train::run_candle_qlora_train(
        data_dir,
        output_dir,
        &req.config,
        req.device_kind,
        &req.system_prompt,
    )
    .map_err(|e| io::Error::other(format!("training failed: {e}")))?;

    serde_json::to_string(&summary)
        .map_err(|e| io::Error::other(format!("serialize TrainingSummary: {e}")))
}
