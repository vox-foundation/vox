//! Background thread that persists **Mens training / run telemetry** to VoxDB (`training_run_*`, training events).
//!
//! This is **local operator + research diagnostics** (checkpoint progress, run lifecycle), not end-user product
//! usage analytics or remote opt-out “usage telemetry.” SSOT: `docs/src/architecture/telemetry-trust-ssot.md`.

use super::TrainingDbEvent;

/// Spawn the dedicated DB writer thread. Always returns the send half (disk checkpoints are unaffected if spawn fails).
pub(super) fn spawn_training_db_writer(
    run_id: String,
) -> tokio::sync::mpsc::UnboundedSender<TrainingDbEvent> {
    let (db_tx, mut db_rx) = tokio::sync::mpsc::unbounded_channel::<TrainingDbEvent>();

    let db_run_id = run_id.clone();
    let spawn_result = std::thread::Builder::new()
        .name("vox-mens-voxdb".into())
        .spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::warn!(
                        run_id = %db_run_id,
                        error = %e,
                        "VoxDB unavailable — training telemetry will not be persisted (failed to start DB writer runtime)"
                    );
                    return;
                }
            };
            rt.block_on(async move {
                let db = match vox_db::VoxDb::connect_default_with_training_fallback().await {
                    Ok(d) => {
                        tracing::info!(
                            target: "vox_db::training_telemetry",
                            run_id = %db_run_id,
                            "VoxDB training telemetry writer connected"
                        );
                        d
                    }
                    Err(err) => {
                        tracing::warn!(
                            run_id = %db_run_id,
                            error = %err,
                            error_debug = ?err,
                            "VoxDB unavailable — training telemetry will not be persisted (open failed after primary + sidecar fallback); disk checkpoints are unaffected. \
                             For a legacy main database run `vox codex export-legacy` then import into a fresh DB, or remove a corrupted `vox_training_telemetry.db` under your Vox data dir."
                        );
                        return;
                    }
                };
                while let Some(evt) = db_rx.recv().await {
                    match evt {
                        TrainingDbEvent::Start {
                            run_id,
                            adapter_tag,
                            model_name,
                            output_dir,
                            data_dir,
                            planned_steps,
                        } => {
                            let params = vox_db::training_run::TrainingRunStartParams {
                                run_id: run_id.clone(),
                                adapter_tag,
                                model_name,
                                output_dir,
                                data_dir,
                                planned_steps,
                            };
                            if let Err(e) = db.record_training_run_start(&params).await {
                                tracing::warn!(
                                    run_id = %run_id,
                                    error = %e,
                                    "VoxDB record_training_run_start failed"
                                );
                            }
                            if let Err(e) = db
                                .record_training_event(
                                    &run_id,
                                    "train_start",
                                    serde_json::json!({"run_id": run_id}),
                                )
                                .await
                            {
                                tracing::warn!(
                                    run_id = %run_id,
                                    error = %e,
                                    "VoxDB record_training_event(train_start) failed"
                                );
                            }
                        }
                        TrainingDbEvent::Checkpoint {
                            run_id,
                            epoch,
                            global_step,
                            last_loss,
                            adapter_path,
                        } => {
                            let _ = db
                                .update_training_checkpoint(
                                    &run_id,
                                    epoch,
                                    global_step,
                                    last_loss,
                                    Some(&adapter_path),
                                )
                                .await;
                            let _ = db
                                .record_training_checkpoint(
                                    &run_id,
                                    epoch,
                                    global_step,
                                    &adapter_path,
                                )
                                .await;
                        }
                        TrainingDbEvent::EpochSummary {
                            run_id,
                            epoch,
                            global_step,
                            avg_loss,
                            avg_val_loss,
                            val_steps,
                        } => {
                            let _ = db
                                .record_training_event(
                                    &run_id,
                                    "epoch_summary",
                                    serde_json::json!({
                                        "epoch": epoch,
                                        "global_step": global_step,
                                        "avg_loss": avg_loss,
                                        "avg_val_loss": avg_val_loss,
                                        "val_steps": val_steps
                                    }),
                                )
                                .await;
                        }
                        TrainingDbEvent::Complete {
                            run_id,
                            global_step,
                            adapter_path,
                        } => {
                            let _ = db
                                .mark_training_complete(&run_id, global_step, Some(&adapter_path))
                                .await;
                            let _ = db
                                .record_training_event(
                                    &run_id,
                                    "train_complete",
                                    serde_json::json!({"global_step": global_step}),
                                )
                                .await;
                        }
                        TrainingDbEvent::Failed {
                            run_id,
                            global_step,
                        } => {
                            let _ = db.mark_training_failed(&run_id, global_step).await;
                            let _ = db
                                .record_training_event(
                                    &run_id,
                                    "train_failed",
                                    serde_json::json!({"global_step": global_step}),
                                )
                                .await;
                        }
                        TrainingDbEvent::GrpoStep {
                            run_id,
                            step,
                            mean_reward,
                            policy_loss,
                            clip_fraction,
                            parse_rate,
                        } => {
                            let _ = db.insert_grpo_step(
                                &run_id,
                                step,
                                mean_reward,
                                policy_loss,
                                clip_fraction,
                                parse_rate,
                            ).await;
                        }
                    }
                }
            });
        });

    if let Err(e) = spawn_result {
        tracing::warn!(
            run_id = %run_id,
            error = %e,
            "VoxDB unavailable — training telemetry will not be persisted (DB writer thread spawn failed)"
        );
    }

    db_tx
}
