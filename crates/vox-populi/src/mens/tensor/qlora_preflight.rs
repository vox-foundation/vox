//! HF file checks before native Candle QLoRA (`vox schola train --backend qlora`).
//!
//! **Tokenizer contract:** `--backend qlora` requires [`MensTokenizerMode::Hf`] and a real
//! Hugging Face `tokenizer.json` (download via `--model <repo>` or explicit path). The Burn LoRA
//! path (`--backend lora`) uses the Vox tokenizer stack — do not mix modes; CLI enforces this before
//! training dispatches.
//!
//! **Weights:** requires HF `config.json` plus safetensors shards listing a supported **embedding**
//! matrix (`wte.weight` or `model.embed_tokens.weight`) for vocab / `d_model` discovery passed into
//! [`super::candle_qlora_train`]. Full training graph details: see module docs on
//! `candle_qlora_train.rs` and mdBook [`mens-training-ssot.md`](../../../../docs/src/architecture/mens-training-ssot.md).

use std::path::PathBuf;

use anyhow::Context;
use safetensors::SafeTensors;

use super::hf_load::HfTransformerLayout;
use super::operator_messages::{
    self, QLORA_NEEDS_HF_WEIGHTS, QLORA_NEEDS_TOKENIZER_PATH, QLORA_REQUIRES_HF_TOKENIZER,
};
use super::training_config::{LoraTrainingConfig, MensTokenizerMode};

/// Prefer `wte` (GPT-2) before `embed_tokens` (Llama/Qwen) when both exist anywhere in the shard list.
const EMBED_KEYS: &[&str] = &["wte.weight", "model.embed_tokens.weight"];

/// Scan all shards: first **valid** rank-2 table in key order (`wte` then `embed_tokens`).
/// If a preferred key exists but is not rank-2, fail (do not fall back silently).
fn resolve_embedding_table(
    weight_paths: &[std::path::PathBuf],
) -> anyhow::Result<(String, usize, usize)> {
    for &key in EMBED_KEYS {
        let mut bad_for_key: Vec<String> = Vec::new();
        for wp in weight_paths {
            let bytes =
                std::fs::read(wp).with_context(|| format!("read weight shard {}", wp.display()))?;
            let st = SafeTensors::deserialize(&bytes)
                .with_context(|| format!("parse {}", wp.display()))?;
            let Ok(t) = st.tensor(key) else {
                continue;
            };
            let shape = t.shape();
            if shape.len() == 2 {
                return Ok((key.to_string(), shape[0], shape[1]));
            }
            bad_for_key.push(format!("{} shape {:?}", wp.display(), shape));
        }
        if !bad_for_key.is_empty() {
            anyhow::bail!(
                "Embedding tensor `{key}` is present but not a 2D matrix [vocab_size, hidden_size]: {}. \
                 Next: use HF-compatible `wte.weight` or `model.embed_tokens.weight` checkpoints only.",
                bad_for_key.join("; ")
            );
        }
    }

    let mut keys_preview = Vec::new();
    if let Some(wp) = weight_paths.first()
        && let Ok(bytes) = std::fs::read(wp)
        && let Ok(st) = SafeTensors::deserialize(&bytes)
    {
        keys_preview = st
            .tensors()
            .iter()
            .map(|(k, _)| k.clone())
            .take(24)
            .collect();
    }
    anyhow::bail!(
        "No supported embedding tensor in safetensors (need one of {:?}). First-shard key sample: {:?}. \
         Next: confirm shards are from the base model and include the token embedding table.",
        EMBED_KEYS,
        keys_preview
    )
}

/// Resolved embedding table metadata for the Candle trainer.
#[derive(Debug, Clone)]
pub struct QloraEmbedBundle {
    pub weight_paths: Vec<PathBuf>,
    /// HF `config.json` path (used for layout parse and tracing; keep for operator debugging).
    pub config_path: PathBuf,
    pub tokenizer_path: PathBuf,
    pub embed_key: String,
    pub vocab: usize,
    pub d_model: usize,
    /// HF `config.json` layout (layers, heads, model family).
    pub layout: HfTransformerLayout,
}

/// Fail fast unless HF tokenizer + safetensors shards contain a supported embedding matrix.
pub fn preflight_native_qlora(config: &LoraTrainingConfig) -> anyhow::Result<QloraEmbedBundle> {
    if !matches!(config.tokenizer_mode, MensTokenizerMode::Hf) {
        anyhow::bail!(QLORA_REQUIRES_HF_TOKENIZER);
    }
    let Some(tok_path) = config.tokenizer_path.as_ref() else {
        anyhow::bail!(QLORA_NEEDS_TOKENIZER_PATH);
    };
    if !tok_path.is_file() {
        anyhow::bail!(operator_messages::tokenizer_not_a_file(
            &tok_path.display().to_string()
        ));
    }
    let Some((weight_paths, config_path)) = config.base_model_paths.as_ref() else {
        anyhow::bail!(QLORA_NEEDS_HF_WEIGHTS);
    };
    if !config_path.is_file() {
        anyhow::bail!(operator_messages::hf_config_missing(
            &config_path.display().to_string()
        ));
    }
    if weight_paths.is_empty() {
        anyhow::bail!(operator_messages::no_safetensors_shards());
    }

    let (embed_key, vocab, d_model) = resolve_embedding_table(weight_paths)?;

    let layout = HfTransformerLayout::from_config_path(config_path).with_context(|| {
        format!(
            "parse HF transformer layout from {} (next: use the `config.json` shipped with these safetensors)",
            config_path.display()
        )
    })?;
    if layout.hidden_size != d_model {
        anyhow::bail!(
            "Embedding hidden size from tensor `{}` is {} (second dim) but config.json `hidden_size` / `n_embd` is {}. \
             Next: use the `config.json` from the same HF revision as the safetensors.",
            embed_key,
            d_model,
            layout.hidden_size
        );
    }
    if layout.vocab_size != vocab {
        anyhow::bail!(
            "Embedding vocab from tensor `{}` is {} but config.json `vocab_size` is {}. \
             Next: align tokenizer and base model revision, or fix mixed checkpoint paths.",
            embed_key,
            vocab,
            layout.vocab_size
        );
    }

    let bundle = QloraEmbedBundle {
        weight_paths: weight_paths.clone(),
        config_path: config_path.clone(),
        tokenizer_path: tok_path.clone(),
        embed_key,
        vocab,
        d_model,
        layout,
    };

    let present = super::candle_qlora_weights::tensor_keys_union(&bundle.weight_paths)
        .context("read safetensors key union for Candle QLoRA preflight")?;
    let cov = super::candle_qlora_weights::middle_projection_coverage(&bundle.layout, &present);
    let n_mid = cov.expected;
    let matched_mid = cov.matched;
    let proxy_complete = n_mid == 0 || cov.complete;

    if config.qlora_require_full_proxy_stack && n_mid > 0 && !proxy_complete {
        let missing =
            super::candle_qlora_weights::missing_middle_keys_report(&bundle.layout, &present, 32);
        anyhow::bail!(
            "Candle QLoRA strict proxy stack: need all {} per-layer output-projection weights in shards; found {}. \
             Missing (up to 32): {:?}. \
             Next: pass complete HF `model*.safetensors`, or omit `--qlora-require-full-proxy-stack` for LM-head-only training. \
             See {}.",
            n_mid,
            matched_mid,
            missing,
            operator_messages::POPULI_TRAINING_SSOT_RELPATH
        );
    }

    let full = super::candle_qlora_weights::ordered_full_block_weight_keys(&bundle.layout);
    let matched_full = full.iter().filter(|k| present.contains(k.as_str())).count();
    let sample = super::candle_qlora_weights::sample_present_keys_sorted_from_present(&present, 24);
    tracing::info!(
        target: "vox_populi::mens::qlora_preflight",
        hf_config_path = %bundle.config_path.display(),
        model_type = %bundle.layout.model_type,
        architecture = ?bundle.layout.architecture,
        hidden_size = bundle.layout.hidden_size,
        n_layer = bundle.layout.num_hidden_layers,
        weight_shards = bundle.weight_paths.len(),
        middle_projection_matched = matched_mid,
        middle_projection_expected = n_mid,
        full_block_key_matched = matched_full,
        full_block_key_candidates = full.len(),
        proxy_stack_complete = proxy_complete,
        embed_key = %bundle.embed_key,
        "qlora preflight coverage summary"
    );
    tracing::debug!(
        target: "vox_populi::mens::qlora_preflight",
        ?sample,
        "safetensors key sample (sorted, first 24)"
    );

    super::train_log::info(&format!(
        "QLoRA preflight OK — model_type={} hidden_size={} num_layers={} middle_projection_keys={}/{} \
         proxy_stack_complete={} embed_key={} weight_shards={}",
        bundle.layout.model_type,
        bundle.layout.hidden_size,
        bundle.layout.num_hidden_layers,
        matched_mid,
        n_mid,
        proxy_complete,
        bundle.embed_key,
        bundle.weight_paths.len(),
    ));

    Ok(bundle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use safetensors::serialize_to_file;
    use safetensors::tensor::{Dtype, TensorView};
    use tempfile::tempdir;

    #[test]
    fn preflight_rejects_vox_tokenizer_mode() {
        let c = LoraTrainingConfig {
            tokenizer_mode: MensTokenizerMode::Vox,
            ..Default::default()
        };
        let msg = preflight_native_qlora(&c).unwrap_err().to_string();
        assert!(
            msg.contains("tokenizer hf") || msg.contains("--tokenizer hf"),
            "{msg}"
        );
    }

    #[test]
    fn preflight_rejects_missing_tokenizer_file() {
        let mut c = LoraTrainingConfig {
            tokenizer_mode: MensTokenizerMode::Hf,
            ..Default::default()
        };
        c.tokenizer_path = Some(PathBuf::from("/nonexistent/tokenizer.json"));
        let msg = preflight_native_qlora(&c).unwrap_err().to_string();
        assert!(
            msg.contains("not an existing file") || msg.contains("Tokenizer path"),
            "{msg}"
        );
    }

    fn write_minimal_safetensors(path: &std::path::Path, key: &str, vocab: usize, d_model: usize) {
        let n_bytes = vocab * d_model * 4;
        let raw: Vec<u8> = vec![0u8; n_bytes];
        let tv =
            TensorView::new(Dtype::F32, vec![vocab, d_model], raw.as_slice()).expect("tensor view");
        serialize_to_file([(key, tv)], &None, path).expect("serialize safetensors");
    }

    fn write_tensor_view(path: &std::path::Path, key: &str, shape: Vec<usize>, raw: &[u8]) {
        let tv = TensorView::new(Dtype::F32, shape, raw).expect("tensor view");
        serialize_to_file([(key, tv)], &None, path).expect("serialize safetensors");
    }

    #[test]
    fn preflight_ok_with_wte_weight() {
        let dir = tempdir().expect("tempdir");
        let tok = dir.path().join("tokenizer.json");
        std::fs::write(&tok, "{}").expect("tokenizer placeholder");
        let cfg_path = dir.path().join("config.json");
        std::fs::write(
            &cfg_path,
            "{\"model_type\":\"gpt2\",\"n_embd\":19,\"n_head\":1,\"n_layer\":1,\"vocab_size\":17}",
        )
        .expect("config");
        let st = dir.path().join("shard.safetensors");
        write_minimal_safetensors(&st, "wte.weight", 17, 19);

        let c = LoraTrainingConfig {
            tokenizer_mode: MensTokenizerMode::Hf,
            tokenizer_path: Some(tok),
            base_model_paths: Some((vec![st], cfg_path)),
            ..Default::default()
        };
        let b = preflight_native_qlora(&c).expect("preflight");
        assert_eq!(b.embed_key, "wte.weight");
        assert_eq!(b.vocab, 17);
        assert_eq!(b.d_model, 19);
    }

    #[test]
    fn preflight_prefers_wte_when_it_appears_only_on_second_shard() {
        let dir = tempdir().expect("tempdir");
        let tok = dir.path().join("tokenizer.json");
        std::fs::write(&tok, "{}").expect("tokenizer");
        let cfg_path = dir.path().join("config.json");
        std::fs::write(
            &cfg_path,
            "{\"model_type\":\"gpt2\",\"n_embd\":19,\"n_head\":1,\"n_layer\":1,\"vocab_size\":17}",
        )
        .expect("config");
        let st_a = dir.path().join("a.safetensors");
        write_minimal_safetensors(&st_a, "model.embed_tokens.weight", 17, 19);
        let st_b = dir.path().join("b.safetensors");
        write_minimal_safetensors(&st_b, "wte.weight", 17, 19);

        let c = LoraTrainingConfig {
            tokenizer_mode: MensTokenizerMode::Hf,
            tokenizer_path: Some(tok),
            base_model_paths: Some((vec![st_a, st_b], cfg_path)),
            ..Default::default()
        };
        let b = preflight_native_qlora(&c).expect("preflight");
        assert_eq!(b.embed_key, "wte.weight");
    }

    #[test]
    fn preflight_rejects_rank_mismatch_on_preferred_embed_key() {
        let dir = tempdir().expect("tempdir");
        let tok = dir.path().join("tokenizer.json");
        std::fs::write(&tok, "{}").expect("tokenizer");
        let cfg_path = dir.path().join("config.json");
        std::fs::write(
            &cfg_path,
            "{\"model_type\":\"gpt2\",\"n_embd\":19,\"n_head\":1,\"n_layer\":1,\"vocab_size\":17}",
        )
        .expect("config");
        let st = dir.path().join("bad.safetensors");
        let shape = vec![17usize, 19, 2];
        let n = shape.iter().product::<usize>();
        let raw: Vec<u8> = vec![0u8; n * 4];
        write_tensor_view(&st, "wte.weight", shape, &raw);

        let c = LoraTrainingConfig {
            tokenizer_mode: MensTokenizerMode::Hf,
            tokenizer_path: Some(tok),
            base_model_paths: Some((vec![st], cfg_path)),
            ..Default::default()
        };
        let msg = preflight_native_qlora(&c).unwrap_err().to_string();
        assert!(
            msg.contains("wte.weight") && msg.contains("2D matrix"),
            "{msg}"
        );
    }

    #[test]
    fn preflight_ok_with_embed_tokens_weight() {
        let dir = tempdir().expect("tempdir");
        let tok = dir.path().join("tokenizer.json");
        std::fs::write(&tok, "{}").expect("tokenizer");
        let cfg_path = dir.path().join("config.json");
        std::fs::write(
            &cfg_path,
            "{\"model_type\":\"qwen2\",\"hidden_size\":7,\"num_attention_heads\":1,\"num_hidden_layers\":1,\"vocab_size\":5}",
        )
        .expect("config");
        let st = dir.path().join("m.safetensors");
        write_minimal_safetensors(&st, "model.embed_tokens.weight", 5, 7);

        let c = LoraTrainingConfig {
            tokenizer_mode: MensTokenizerMode::Hf,
            tokenizer_path: Some(tok),
            base_model_paths: Some((vec![st], cfg_path)),
            ..Default::default()
        };
        let b = preflight_native_qlora(&c).expect("preflight");
        assert_eq!(b.embed_key, "model.embed_tokens.weight");
        assert_eq!(b.vocab, 5);
        assert_eq!(b.d_model, 7);
        assert_eq!(b.layout.hidden_size, 7);
        assert_eq!(b.layout.model_type, "qwen2");
    }

    #[test]
    fn preflight_strict_rejects_missing_o_proj() {
        let dir = tempdir().expect("tempdir");
        let tok = dir.path().join("tokenizer.json");
        std::fs::write(&tok, "{}").expect("tokenizer");
        let cfg_path = dir.path().join("config.json");
        std::fs::write(
            &cfg_path,
            "{\"model_type\":\"qwen2\",\"hidden_size\":7,\"num_attention_heads\":1,\"num_hidden_layers\":2,\"vocab_size\":5}",
        )
        .expect("config");
        let st = dir.path().join("m.safetensors");
        write_minimal_safetensors(&st, "model.embed_tokens.weight", 5, 7);

        let c = LoraTrainingConfig {
            tokenizer_mode: MensTokenizerMode::Hf,
            tokenizer_path: Some(tok),
            base_model_paths: Some((vec![st], cfg_path)),
            qlora_require_full_proxy_stack: true,
            ..Default::default()
        };
        let msg = preflight_native_qlora(&c).unwrap_err().to_string();
        assert!(
            msg.contains("strict proxy stack") || msg.contains("o_proj"),
            "{msg}"
        );
    }
}
