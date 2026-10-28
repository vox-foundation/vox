//! Mid-epoch checkpoint save + DB notification.
//!
//! Ported verbatim from vox-populi (SP3 sub-batch C).

use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use qlora_rs::training::QLoraTrainer;

use super::TrainingDbEvent;
use crate::{checkpoint_state::CheckpointState, config::LoraTrainingConfig};

/// Wall time after which a mid-epoch checkpoint is saved even when the row
/// interval has not come round (500 rows is ~2.5 h on 0.6B at seq 3072).
pub(super) const CHECKPOINT_WALL_INTERVAL: Duration = Duration::from_secs(20 * 60);

/// A save is due every `every_rows` rows (as before), or once
/// [`CHECKPOINT_WALL_INTERVAL`] has passed since the last save, but the time
/// trigger only fires on an optimizer-step boundary, where no gradient is
/// still accumulating, so the saved adapter is the whole state.
pub(super) fn checkpoint_due(
    every_rows: Option<usize>,
    rows_done: usize,
    since_last_save: Duration,
    optimizer_boundary: bool,
) -> bool {
    let by_rows = every_rows.is_some_and(|every| every > 0 && rows_done.is_multiple_of(every));
    by_rows || (optimizer_boundary && since_last_save >= CHECKPOINT_WALL_INTERVAL)
}

/// Keep only `latest` among the `checkpoint_step_*` adapters in `out`; the
/// resume state names only the newest, so older ones are dead disk.
pub(super) fn prune_older_step_checkpoints(out: &Path, latest: &Path) {
    let Ok(entries) = std::fs::read_dir(out) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("checkpoint_step_")
            && name.ends_with(".safetensors")
            && entry.path() != latest
        {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Saves a step checkpoint and the resume state when one is due, or when
/// `force` (the run cap stopped the run at an optimizer-step boundary).
#[allow(clippy::too_many_arguments)]
pub(super) fn maybe_save_mid_epoch_checkpoint(
    trainer: &mut QLoraTrainer,
    out: &Path,
    config: &LoraTrainingConfig,
    db_tx: &tokio::sync::mpsc::UnboundedSender<TrainingDbEvent>,
    run_id: &str,
    epoch: usize,
    global_step: u32,
    pair_loop_idx: usize,
    shuffled_indices: &[usize],
    last_loss_val: f32,
    run_start_inst: Instant,
    last_save: &mut Instant,
    optimizer_boundary: bool,
    force: bool,
) -> Result<()> {
    if !force
        && !checkpoint_due(
            config.checkpoint_every,
            pair_loop_idx + 1,
            last_save.elapsed(),
            optimizer_boundary,
        )
    {
        return Ok(());
    }
    let ckpt_path = out.join(format!("checkpoint_step_{global_step}.safetensors"));
    trainer
        .save_adapter(&ckpt_path)
        .context("save mid-epoch adapter")?;

    let state = CheckpointState {
        schema: crate::checkpoint_state::CHECKPOINT_SCHEMA.to_string(),
        run_id: run_id.to_string(),
        epoch: epoch as u32,
        global_step,
        pair_offset: pair_loop_idx + 1,
        shuffled_indices: shuffled_indices.to_vec(),
        rng_seed: config.seed,
        adapter_path: ckpt_path.display().to_string(),
        last_loss: last_loss_val,
        wall_seconds_elapsed: run_start_inst.elapsed().as_secs_f64(),
        saved_at_utc: CheckpointState::now_utc(),
        data_fingerprint: config.data_fingerprint.clone(),
        base_model: config.base_model.clone(),
    };
    state.save(out).context("save CheckpointState mid-epoch")?;
    // Only after the state names the new file.
    prune_older_step_checkpoints(out, &ckpt_path);
    *last_save = Instant::now();

    let _ = db_tx.send(TrainingDbEvent::Checkpoint {
        run_id: run_id.to_string(),
        epoch: epoch as u32,
        global_step,
        last_loss: Some(last_loss_val),
        adapter_path: ckpt_path.display().to_string(),
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// 320: the row interval alone (500 rows, ~2.5 h on 0.6B at seq 3072) left
    /// hours of work unrecoverable; wall time also triggers a save, but only on
    /// an optimizer-step boundary so no accumulated gradient is lost.
    #[test]
    fn a_checkpoint_is_due_by_rows_or_by_wall_time_at_a_step_boundary() {
        let (short, long) = (Duration::from_secs(60), CHECKPOINT_WALL_INTERVAL);
        assert!(
            checkpoint_due(Some(500), 500, short, false),
            "row interval as before"
        );
        assert!(!checkpoint_due(Some(500), 499, short, true));
        assert!(
            checkpoint_due(Some(500), 7, long, true),
            "20 min since the last save"
        );
        assert!(
            !checkpoint_due(Some(500), 7, long, false),
            "never mid-accumulation"
        );
        assert!(
            checkpoint_due(None, 7, long, true),
            "time works without --checkpoint-every"
        );
        assert!(!checkpoint_due(Some(0), 7, short, true));
    }

    #[test]
    fn only_the_latest_step_checkpoint_is_kept() {
        let d = tempfile::tempdir().unwrap();
        for f in [
            "checkpoint_step_10.safetensors",
            "checkpoint_step_20.safetensors",
            "checkpoint_epoch_1.safetensors",
            "pause_step_5.safetensors",
        ] {
            std::fs::write(d.path().join(f), "x").unwrap();
        }
        prune_older_step_checkpoints(d.path(), &d.path().join("checkpoint_step_20.safetensors"));
        let left: std::collections::BTreeSet<String> = std::fs::read_dir(d.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            left,
            [
                "checkpoint_epoch_1.safetensors",
                "checkpoint_step_20.safetensors",
                "pause_step_5.safetensors"
            ]
            .map(String::from)
            .into()
        );
    }
}
