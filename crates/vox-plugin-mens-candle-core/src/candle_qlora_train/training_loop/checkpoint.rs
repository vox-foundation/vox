//! Checkpoint resume logic.
//!
//! Ported verbatim from vox-populi (SP3 sub-batch C).

use anyhow::Result;
use qlora_rs::training::QLoraTrainer;
use rand::seq::SliceRandom;
use std::path::Path;
use vox_tensor::data::TrainingPair;

use super::types::QloraTrainingResume;
use crate::{checkpoint_state::CheckpointState, config::LoraTrainingConfig, train_log};

pub fn apply_checkpoint_resume(
    trainer: &mut QLoraTrainer,
    config: &LoraTrainingConfig,
    out: &Path,
    pairs_len: usize,
) -> Result<QloraTrainingResume> {
    let mut start_epoch = 1usize;
    let mut global_step = 0u32;
    let mut resume_pair_offset = 0usize;
    let mut resume_shuffled_indices: Option<Vec<usize>> = None;

    let checkpoint_root = config.resume_from.as_deref().unwrap_or(out);
    if !config.force_restart
        && let Some(ckpt) = CheckpointState::load(checkpoint_root)
    {
        if let (Some(ckpt_fp), Some(cur_fp)) = (
            ckpt.data_fingerprint.as_deref(),
            config.data_fingerprint.as_deref(),
        ) && ckpt_fp != cur_fp
        {
            anyhow::bail!(
                "Checkpoint in {} was trained on a different dataset (fingerprint {ckpt_fp}) \
                 than the one loaded for this run (fingerprint {cur_fp}). Resuming would keep \
                 training an adapter that was warmed up on different data. Pass --force-restart \
                 to discard this checkpoint and start fresh, or point --data-dir/--resume-from \
                 at the original dataset.",
                checkpoint_root.display()
            );
        }
        train_log::info(&format!(
            "Checkpoint found in {} — resuming from epoch={} global_step={} pair_offset={}",
            checkpoint_root.display(),
            ckpt.epoch,
            ckpt.global_step,
            ckpt.pair_offset
        ));
        if std::path::Path::new(&ckpt.adapter_path).exists() {
            if let Err(err) = super::super::load_adapter_into_trainer(
                trainer,
                std::path::Path::new(&ckpt.adapter_path),
            ) {
                train_log::warn(&format!(
                    "Resume adapter load failed for {}: {err}",
                    ckpt.adapter_path
                ));
            }
        } else {
            train_log::warn(&format!(
                "Resume checkpoint references missing adapter {}; continuing with fresh adapter weights.",
                ckpt.adapter_path
            ));
        }
        start_epoch = ckpt.epoch as usize;
        global_step = ckpt.global_step;
        resume_pair_offset = ckpt.pair_offset;
        if ckpt.shuffled_indices.is_empty() {
            train_log::warn(
                "Resume checkpoint did not include shuffled_indices (epoch-boundary checkpoint); reshuffling for resume epoch.",
            );
            resume_shuffled_indices = None;
            resume_pair_offset = 0;
        } else {
            let (validated_indices, dropped_bad_indices) =
                sanitize_resume_indices(&ckpt.shuffled_indices, pairs_len);
            if dropped_bad_indices > 0 {
                train_log::warn(&format!(
                    "Resume checkpoint shuffled_indices dropped {} out-of-range/duplicate entries; reshuffling current epoch.",
                    dropped_bad_indices
                ));
                resume_shuffled_indices = None;
                resume_pair_offset = 0;
            } else if validated_indices.len() != pairs_len {
                train_log::warn(&format!(
                    "Resume checkpoint shuffled_indices length {} does not match current dataset size {}; reshuffling current epoch.",
                    validated_indices.len(),
                    pairs_len
                ));
                resume_shuffled_indices = None;
                resume_pair_offset = 0;
            } else {
                resume_shuffled_indices = Some(validated_indices);
            }
        }
    }

    Ok(QloraTrainingResume {
        start_epoch,
        global_step,
        resume_pair_offset,
        resume_shuffled_indices,
    })
}

pub fn build_epoch_shuffled_indices(
    epoch: usize,
    start_epoch: usize,
    pairs: &[TrainingPair],
    resume_shuffled_indices: &Option<Vec<usize>>,
    rng: &mut rand::rngs::StdRng,
    monotonic_difficulty: bool,
) -> Vec<usize> {
    if epoch == start_epoch
        && let Some(idx) = resume_shuffled_indices
        && !idx.is_empty()
    {
        return idx.clone();
    }
    let mut idx: Vec<usize> = (0..pairs.len()).collect();
    // Shuffle first; the curriculum's stable sort then keeps rows random *within*
    // each difficulty level instead of replaying the file (one source block after
    // another) in the same order every epoch.
    idx.shuffle(rng);
    if monotonic_difficulty {
        idx.sort_by_key(|&v| pairs[v].difficulty.unwrap_or(5));
    }
    idx
}

pub fn sanitize_resume_indices(indices: &[usize], pair_count: usize) -> (Vec<usize>, usize) {
    if indices.is_empty() {
        return (Vec::new(), 0);
    }
    let mut seen = vec![false; pair_count];
    let mut out = Vec::with_capacity(indices.len());
    let mut dropped = 0usize;
    for &idx in indices {
        if idx >= pair_count || seen[idx] {
            dropped += 1;
            continue;
        }
        seen[idx] = true;
        out.push(idx);
    }
    (out, dropped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn curriculum_order_is_sorted_by_difficulty_but_shuffled_within_levels() {
        let pairs: Vec<TrainingPair> = (0..40)
            .map(|i| TrainingPair {
                difficulty: Some(if i < 20 { 2 } else { 7 }),
                ..Default::default()
            })
            .collect();
        let mut rng = rand::rngs::StdRng::seed_from_u64(3);
        let idx = build_epoch_shuffled_indices(1, 0, &pairs, &None, &mut rng, true);
        let diffs: Vec<u8> = idx.iter().map(|&i| pairs[i].difficulty.unwrap()).collect();
        assert!(
            diffs.windows(2).all(|w| w[0] <= w[1]),
            "not monotonic: {diffs:?}"
        );
        assert_ne!(
            &idx[..20],
            &(0..20).collect::<Vec<_>>()[..],
            "easy bucket kept file order"
        );
    }

    fn cpu_trainer() -> QLoraTrainer {
        QLoraTrainer::new(
            qlora_rs::training::QLoraTrainingConfig::default(),
            candle_core::Device::Cpu,
        )
    }

    fn write_checkpoint(dir: &std::path::Path, data_fingerprint: Option<&str>) {
        let ckpt = CheckpointState {
            schema: crate::checkpoint_state::CHECKPOINT_SCHEMA.to_string(),
            run_id: "r".to_string(),
            epoch: 1,
            global_step: 5,
            pair_offset: 2,
            shuffled_indices: vec![0, 1, 2],
            rng_seed: 0,
            adapter_path: String::new(), // missing — resume warns and continues with fresh weights
            last_loss: 0.0,
            wall_seconds_elapsed: 0.0,
            saved_at_utc: String::new(),
            data_fingerprint: data_fingerprint.map(str::to_string),
        };
        ckpt.save(dir).unwrap();
    }

    /// The bug this guards against: a checkpoint trained on one dataset was
    /// silently resumed against a different one (observed: a 6,754-row mix's
    /// checkpoint continuing against an 883-row corpus, reshuffled without
    /// complaint because the row-count/index-validity checks below don't ask
    /// "is this even the same data").
    #[test]
    fn refuses_to_resume_when_data_fingerprint_differs() {
        let dir = std::env::temp_dir().join("vox_ckpt_resume_mismatch_test");
        std::fs::create_dir_all(&dir).unwrap();
        write_checkpoint(&dir, Some("old-data-hash"));
        let mut config = LoraTrainingConfig {
            data_fingerprint: Some("new-data-hash".to_string()),
            ..Default::default()
        };
        let mut trainer = cpu_trainer();
        let err = apply_checkpoint_resume(&mut trainer, &config, &dir, 3)
            .expect_err("mismatched fingerprint must refuse to resume");
        assert!(err.to_string().contains("--force-restart"));

        // force_restart bypasses the checkpoint entirely (existing behavior) —
        // the mismatch must not block a run that opts out of resuming.
        config.force_restart = true;
        let resume = apply_checkpoint_resume(&mut trainer, &config, &dir, 3)
            .expect("force_restart must skip the mismatch check");
        assert_eq!(resume.start_epoch, 1);
        assert_eq!(resume.global_step, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resumes_normally_when_data_fingerprint_matches_or_is_unknown() {
        let dir = std::env::temp_dir().join("vox_ckpt_resume_match_test");
        std::fs::create_dir_all(&dir).unwrap();

        write_checkpoint(&dir, Some("same-hash"));
        let config = LoraTrainingConfig {
            data_fingerprint: Some("same-hash".to_string()),
            ..Default::default()
        };
        let mut trainer = cpu_trainer();
        let resume = apply_checkpoint_resume(&mut trainer, &config, &dir, 3)
            .expect("matching fingerprint must resume");
        assert_eq!(resume.global_step, 5);

        // Either side missing a fingerprint (old checkpoint, or fingerprinting
        // failed) must never false-positive-block a resume.
        write_checkpoint(&dir, None);
        let config_no_fp = LoraTrainingConfig {
            data_fingerprint: Some("whatever".to_string()),
            ..Default::default()
        };
        let mut trainer2 = cpu_trainer();
        apply_checkpoint_resume(&mut trainer2, &config_no_fp, &dir, 3)
            .expect("checkpoint with no fingerprint must not block resume");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
