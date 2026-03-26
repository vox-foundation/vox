//! Final adapter save, MODEL_CARD, adapter meta, telemetry completion.

use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result};
use qlora_rs::training::QLoraTrainer;

use super::{QloraAdapterMetaV2, TrainingDbEvent, TrainingLoopStats};
use crate::mens::tensor::{
    adapter_schema_v3::{AdapterProvenanceFields, PopuliAdapterManifestV3},
    backend::TrainingSummary, checkpoint_state::CheckpointState, manifest,
    finetune_contract::{AdapterMethod, BaseQuantMode},
    qlora_preflight::QloraEmbedBundle, telemetry, telemetry_schema, train_log,
    training_config::LoraTrainingConfig,
};

fn adapter_provenance_from_config(config: &LoraTrainingConfig) -> Option<AdapterProvenanceFields> {
    let has_lineage = config.base_model_family.is_some()
        || config.upstream_model_id.is_some()
        || config.license_class.is_some()
        || config.attribution_required;
    if !has_lineage {
        return None;
    }
    Some(AdapterProvenanceFields {
        base_family: config.base_model_family.clone(),
        upstream_model_id: config.upstream_model_id.clone(),
        license_class: config.license_class.clone(),
        attribution_required: config.attribution_required,
    })
}

fn build_adapter_manifest_v3(
    vocab: usize,
    d_model: usize,
    rank: usize,
    alpha: usize,
    config: &LoraTrainingConfig,
    adapter_layer_order: &[String],
    base_key_map: &std::collections::HashMap<String, String>,
) -> PopuliAdapterManifestV3 {
    PopuliAdapterManifestV3::new(
        AdapterMethod::Qlora,
        BaseQuantMode::Nf4,
        config.qlora_double_quant,
        base_key_map.clone(),
        adapter_layer_order.to_vec(),
        vocab,
        d_model,
        rank,
        alpha,
        adapter_provenance_from_config(config),
    )
}

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
    optimizer_step_count: u32,
    total_tokens: usize,
    total_step_count: u32,
    total_loss_sum: f64,
    stats: TrainingLoopStats,
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
    let adapter_manifest_v3 = build_adapter_manifest_v3(
        bundle.vocab,
        bundle.d_model,
        config.rank,
        config.alpha as usize,
        config,
        adapter_layer_order,
        base_key_map,
    );
    std::fs::write(
        out.join("populi_adapter_manifest_v3.json"),
        serde_json::to_string_pretty(&adapter_manifest_v3)?,
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
            "optimizer_step": optimizer_step_count,
            "skip_no_supervised_positions": stats.skip_no_supervised_positions,
            "skip_short_seq": stats.skip_short_seq,
            "skip_curriculum": stats.skip_curriculum,
            "final_adapter": final_path.display().to_string(),
            "run_id": run_id,
        }),
    )?;

    let total_seen =
        stats.skip_no_supervised_positions + stats.skip_short_seq + total_step_count as u64;
    let no_supervised_skip_rate = if total_seen > 0 {
        stats.skip_no_supervised_positions as f64 / total_seen as f64
    } else {
        0.0
    };
    std::fs::write(
        out.join("training_skip_stats.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "schema": "vox_mens_training_skip_stats_v1",
            "optimizer_steps_executed": optimizer_step_count,
            "micro_steps_executed": total_step_count,
            "skip_no_supervised_positions": stats.skip_no_supervised_positions,
            "skip_short_seq": stats.skip_short_seq,
            "skip_curriculum": stats.skip_curriculum,
            "no_supervised_skip_rate": no_supervised_skip_rate
        }))?,
    )?;
    manifest::finalize_candle_qlora_training_manifest(
        out,
        optimizer_step_count as u64,
        0,
        0,
        stats.skip_short_seq,
        true,
    )?;

    train_log::info(&format!(
        "Training complete — micro_steps={} optimizer_steps={} no_supervised_skip_rate={:.2}% — adapter: {}",
        global_step,
        optimizer_step_count,
        no_supervised_skip_rate * 100.0,
        final_path.display(),
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::build_adapter_manifest_v3;
    use crate::mens::tensor::training_config::LoraTrainingConfig;

    #[test]
    fn adapter_manifest_v3_carries_lineage_from_training_config() {
        let mut cfg = LoraTrainingConfig::default();
        cfg.base_model_family = Some("kimi-k2.5".into());
        cfg.upstream_model_id = Some("moonshotai/Kimi-K2.5".into());
        cfg.license_class = Some("modified-mit".into());
        cfg.attribution_required = true;

        let mut key_map = HashMap::new();
        key_map.insert("lm_head".into(), "wte.weight".into());
        let manifest = build_adapter_manifest_v3(
            32000,
            4096,
            16,
            32,
            &cfg,
            &["lm_head".into()],
            &key_map,
        );
        let prov = manifest.provenance.expect("expected provenance");
        assert_eq!(prov.base_family.as_deref(), Some("kimi-k2.5"));
        assert_eq!(
            prov.upstream_model_id.as_deref(),
            Some("moonshotai/Kimi-K2.5")
        );
        assert_eq!(prov.license_class.as_deref(), Some("modified-mit"));
        assert!(prov.attribution_required);
    }
}
