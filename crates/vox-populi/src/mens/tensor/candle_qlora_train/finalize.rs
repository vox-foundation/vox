//! Final adapter save, MODEL_CARD, adapter meta, telemetry completion.

use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result};
use qlora_rs::training::QLoraTrainer;

use super::{QloraAdapterMetaV2, TrainingDbEvent};
use crate::mens::tensor::{
    backend::TrainingSummary, checkpoint_state::CheckpointState, qlora_preflight::QloraEmbedBundle,
    telemetry, telemetry_schema, train_log, training_config::LoraTrainingConfig,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn finalize_training_run(
    trainer: &mut QLoraTrainer,
    bundle: &QloraEmbedBundle,
    out: &Path,
    config: &LoraTrainingConfig,
    db_tx: &tokio::sync::mpsc::UnboundedSender<TrainingDbEvent>,
    run_id: &str,
    device_label: &str,
    adapter_layer_order: &[String],
    base_key_map: &std::collections::HashMap<String, String>,
    global_step: u32,
    total_tokens: usize,
    total_step_count: u32,
    total_loss_sum: f64,
    run_start_inst: Instant,
) -> Result<TrainingSummary> {
    let final_path = out.join("candle_qlora_adapter.safetensors");
    trainer
        .save_adapter(&final_path)
        .context("save final adapter")?;

    let final_avg_loss = if total_step_count > 0 {
        total_loss_sum / total_step_count as f64
    } else {
        0.0
    };
    let card = crate::mens::tensor::model_card::ModelCard {
        title: format!(
            "Vox LoRA Adapter — {}",
            config.adapter_tag.as_deref().unwrap_or("default")
        ),
        base_model: config.base_model.clone(),
        train_file: config
            .train_file
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        vocab_size: bundle.vocab,
        d_model: bundle.d_model,
        n_layers: bundle.layout.num_hidden_layers,
        n_heads: bundle.layout.num_attention_heads,
        notes: format!(
            "rank={rank} α={alpha} epochs={epochs} steps={global_step} avg_loss={avg:.4}\nDevice: {device}\nCheckpoint: {ckpt}",
            rank = config.rank,
            alpha = config.alpha,
            epochs = config.epochs,
            avg = final_avg_loss,
            device = device_label,
            ckpt = final_path.display(),
        ),
    };
    if let Err(e) = crate::mens::tensor::model_card::write(out, &card) {
        train_log::warn(&format!("MODEL_CARD.md could not be written: {e}"));
    } else {
        train_log::info(&format!(
            "Wrote MODEL_CARD.md to {}",
            out.join("MODEL_CARD.md").display()
        ));
    }

    let meta = QloraAdapterMetaV2 {
        format: QloraAdapterMetaV2::FORMAT.to_string(),
        version: QloraAdapterMetaV2::VERSION,
        embed_key: bundle.embed_key.clone(),
        vocab: bundle.vocab,
        d_model: bundle.d_model,
        rank: config.rank,
        alpha: config.alpha as usize,
        layer_order: adapter_layer_order.to_vec(),
        base_key_map: base_key_map.clone(),
        base_model: config.base_model.clone(),
    };
    std::fs::write(
        out.join("adapter_meta_v2.json"),
        serde_json::to_string_pretty(&meta)?,
    )?;

    CheckpointState::delete(out);

    let _ = db_tx.send(TrainingDbEvent::Complete {
        run_id: run_id.to_string(),
        global_step,
        adapter_path: final_path.display().to_string(),
    });

    telemetry::append(
        out,
        telemetry_schema::events::TRAIN_COMPLETE,
        serde_json::json!({
            "global_step": global_step,
            "final_adapter": final_path.display().to_string(),
            "run_id": run_id,
        }),
    )?;

    train_log::info(&format!(
        "Training complete — {global_step} steps — adapter: {}",
        final_path.display()
    ));

    let wall_secs = run_start_inst.elapsed().as_secs_f64();
    let ms_per_step = if global_step > 0 {
        (wall_secs * 1000.0) / global_step as f64
    } else {
        0.0
    };

    Ok(TrainingSummary {
        wall_secs,
        total_steps: global_step as usize,
        total_tokens,
        ms_per_step,
    })
}
