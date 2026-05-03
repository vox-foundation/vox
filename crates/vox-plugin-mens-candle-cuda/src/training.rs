//! Training step helpers: run_train_step and run_eval_step.
//!
//! # SP3 stub
//!
//! The real implementation lives in `vox-populi`'s `candle_qlora_train` module
//! (training_loop/mod.rs, training_loop/forward.rs, training_loop/validation.rs).
//! These functions are deeply tangled with vox-populi types:
//!
//! - `LoraTrainingConfig` (vox-populi's config type)
//! - `QloraEmbedBundle` (preflight result with safetensors paths, vocab, d_model, etc.)
//! - `TrainingPair` from `vox-tensor`
//! - `CheckpointState` from vox-populi
//! - `VoxDB` async channel (`tokio::sync::mpsc::UnboundedSender<TrainingDbEvent>`)
//! - `train_log`, `telemetry`, `vox_clavis` secrets
//!
//! Extracting these cleanly requires either:
//! (A) Copying all the above types into the plugin crate (large scope for SP3), or
//! (B) Accepting a JSON-serialized batch that encodes all per-step inputs, and having
//!     the plugin parse the batch internally (the intended long-term design per the
//!     MlBackend wire format spec in vox-plugin-api/src/extensions/ml_backend.rs).
//!
//! Option B is the right design (already modeled by the `batch_json` argument on
//! `train_step`), but the batch schema needs to be agreed between vox-populi (caller)
//! and the plugin (callee) and is deferred to batch 3/4 when vox-populi is rewired to
//! call through the plugin host.

use crate::model::CandleModel;

/// Run one QLoRA training step.
///
/// `batch_json`: JSON-encoded batch from vox-populi. Schema TBD (batch 3/4).
/// Returns JSON-encoded training stats on success.
pub fn run_train_step(
    _model: &mut CandleModel,
    _batch_json: &str,
) -> anyhow::Result<String> {
    // TODO(batch 3/4): implement by:
    // 1. Parsing batch_json into a BatchInput struct (schema agreed with vox-populi)
    // 2. Calling forward_masked_ce (from training_loop/forward.rs) with the decoded
    //    ids, prefix_len, trunc_offset, sample_weight, token_weights
    // 3. Calling trainer.backward_step / optimizer step
    // 4. Returning JSON-encoded {loss, supervised_tokens, step} stats
    anyhow::bail!(
        "vox-plugin-mens-candle-cuda: run_train_step not yet wired (SP3 stub). \
         Batch 3 will implement the JSON batch protocol. See training.rs for details."
    )
}

/// Run one evaluation step (forward only, no gradient).
///
/// `batch_json`: JSON-encoded batch from vox-populi. Schema TBD (batch 3/4).
/// Returns JSON-encoded eval stats on success.
pub fn run_eval_step(
    _model: &CandleModel,
    _batch_json: &str,
) -> anyhow::Result<String> {
    // TODO(batch 3/4): implement by:
    // 1. Parsing batch_json into a BatchInput struct
    // 2. Calling forward_masked_ce with sample_weight=1.0 (eval mode, no gradient)
    // 3. Returning JSON-encoded {loss, supervised_tokens} stats
    anyhow::bail!(
        "vox-plugin-mens-candle-cuda: run_eval_step not yet wired (SP3 stub). \
         Batch 3 will implement the JSON batch protocol. See training.rs for details."
    )
}
