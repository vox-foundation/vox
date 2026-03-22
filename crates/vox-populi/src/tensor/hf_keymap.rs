//! Shared Hugging Face weight **key naming** for transformer blocks (Burn + Candle).
//!
//! Candle QLoRA also uses [`super::candle_qlora_weights`] for safetensors I/O (key union, logging).

use std::collections::HashSet;

use super::hf_load::{HfArchitecture, HfTransformerLayout};

/// HF key for attention output projection per layer (shape `[d_model, d_model]`).
#[must_use]
pub fn middle_block_projection_key(arch: HfArchitecture, layer: usize) -> String {
    match arch {
        HfArchitecture::Gpt2 => format!("h.{layer}.attn.c_proj.weight"),
        HfArchitecture::Qwen2 => format!("model.layers.{layer}.self_attn.o_proj.weight"),
    }
}

/// Self-attention linear weight keys for one block (inventory / diagnostics).
#[must_use]
pub fn layer_attention_weight_keys(arch: HfArchitecture, layer: usize) -> Vec<String> {
    match arch {
        HfArchitecture::Gpt2 => vec![
            format!("h.{layer}.attn.c_attn.weight"),
            middle_block_projection_key(arch, layer),
        ],
        HfArchitecture::Qwen2 => vec![
            format!("model.layers.{layer}.self_attn.q_proj.weight"),
            format!("model.layers.{layer}.self_attn.k_proj.weight"),
            format!("model.layers.{layer}.self_attn.v_proj.weight"),
            middle_block_projection_key(arch, layer),
        ],
    }
}

/// MLP / FFN linear weight keys for one block.
#[must_use]
pub fn layer_mlp_weight_keys(arch: HfArchitecture, layer: usize) -> Vec<String> {
    match arch {
        HfArchitecture::Gpt2 => vec![
            format!("h.{layer}.mlp.c_fc.weight"),
            format!("h.{layer}.mlp.c_proj.weight"),
        ],
        HfArchitecture::Qwen2 => vec![
            format!("model.layers.{layer}.mlp.gate_proj.weight"),
            format!("model.layers.{layer}.mlp.up_proj.weight"),
            format!("model.layers.{layer}.mlp.down_proj.weight"),
        ],
    }
}

/// All non-embedding tensor name **candidates** for a full transformer (attention + MLP per layer).
#[must_use]
pub fn ordered_full_block_weight_keys(layout: &HfTransformerLayout) -> Vec<String> {
    let mut v = Vec::new();
    for i in 0..layout.dims.n_layer {
        v.extend(layer_attention_weight_keys(layout.architecture, i));
        v.extend(layer_mlp_weight_keys(layout.architecture, i));
    }
    v
}

/// Ordered candidate keys from layer `0 .. n_layer-1` (Candle o_proj proxy stack).
#[must_use]
pub fn ordered_middle_projection_keys(layout: &HfTransformerLayout) -> Vec<String> {
    (0..layout.dims.n_layer)
        .map(|i| middle_block_projection_key(layout.architecture, i))
        .collect()
}

/// Keep `keys` in order, only those present in `present`.
#[must_use]
pub fn filter_keys_in_shard(keys: &[String], present: &HashSet<String>) -> Vec<String> {
    keys.iter()
        .filter(|k| present.contains(k.as_str()))
        .cloned()
        .collect()
}

/// First `max_keys` sorted tensor names from an existing key union (no extra I/O).
#[must_use]
pub fn sample_present_keys_sorted_from_present(
    present: &HashSet<String>,
    max_keys: usize,
) -> Vec<String> {
    let mut v: Vec<_> = present.iter().cloned().collect();
    v.sort();
    v.truncate(max_keys);
    v
}

/// Expected middle keys missing from shard union (diagnostics). Lists up to `max_missing` names.
#[must_use]
pub fn missing_middle_keys_report(
    layout: &HfTransformerLayout,
    present: &HashSet<String>,
    max_missing: usize,
) -> Vec<String> {
    ordered_middle_projection_keys(layout)
        .into_iter()
        .filter(|k| !present.contains(k.as_str()))
        .take(max_missing.max(1))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tensor::hf_load::{HfArchitecture, HfTransformerLayout};

    fn gpt2_layout_one_layer() -> HfTransformerLayout {
        HfTransformerLayout::from_config_json_str(
            "{\"model_type\":\"gpt2\",\"n_embd\":64,\"n_head\":2,\"n_layer\":1,\"vocab_size\":100}",
        )
        .expect("gpt2 config")
    }

    fn qwen_layout_one_layer() -> HfTransformerLayout {
        HfTransformerLayout::from_config_json_str(
            "{\"model_type\":\"qwen2\",\"hidden_size\":32,\"num_attention_heads\":2,\"num_hidden_layers\":1,\"vocab_size\":50}",
        )
        .expect("qwen config")
    }

    fn qwen_layout_two_layers() -> HfTransformerLayout {
        HfTransformerLayout::from_config_json_str(
            "{\"model_type\":\"qwen2\",\"hidden_size\":32,\"num_attention_heads\":2,\"num_hidden_layers\":2,\"vocab_size\":50}",
        )
        .expect("qwen2 two-layer config")
    }

    #[test]
    fn gpt2_middle_key_matches_hf_naming() {
        assert_eq!(
            middle_block_projection_key(HfArchitecture::Gpt2, 0),
            "h.0.attn.c_proj.weight"
        );
        let layout = gpt2_layout_one_layer();
        let keys = ordered_middle_projection_keys(&layout);
        assert_eq!(keys, vec!["h.0.attn.c_proj.weight"]);
    }

    #[test]
    fn qwen2_middle_key_matches_hf_naming() {
        let layout = qwen_layout_one_layer();
        assert_eq!(
            middle_block_projection_key(layout.architecture, 0),
            "model.layers.0.self_attn.o_proj.weight"
        );
    }

    #[test]
    fn qwen2_ordered_middle_keys_track_all_layers() {
        let layout = qwen_layout_two_layers();
        let keys = ordered_middle_projection_keys(&layout);
        assert_eq!(
            keys,
            vec![
                "model.layers.0.self_attn.o_proj.weight",
                "model.layers.1.self_attn.o_proj.weight",
            ]
        );
    }

    #[test]
    fn qwen2_missing_middle_report_respects_max_and_order() {
        use std::collections::HashSet;
        let layout = qwen_layout_two_layers();
        let mut present = HashSet::new();
        present.insert("model.layers.0.self_attn.o_proj.weight".to_string());
        let miss = missing_middle_keys_report(&layout, &present, 8);
        assert_eq!(
            miss,
            vec!["model.layers.1.self_attn.o_proj.weight".to_string()]
        );
    }

    #[test]
    fn qwen2_full_block_keys_include_mlp_projections() {
        let layout = qwen_layout_one_layer();
        let v = ordered_full_block_weight_keys(&layout);
        assert!(v.iter().any(|k| k.contains("mlp.gate_proj")));
        assert!(v.iter().any(|k| k.contains("mlp.up_proj")));
        assert!(v.iter().any(|k| k.contains("mlp.down_proj")));
        assert!(v.iter().any(|k| k.contains("self_attn.q_proj")));
    }

    #[test]
    fn ordered_full_block_includes_attn_and_mlp() {
        let layout = gpt2_layout_one_layer();
        let v = ordered_full_block_weight_keys(&layout);
        assert!(v.iter().any(|k| k.contains("attn.c_attn")));
        assert!(v.iter().any(|k| k.contains("mlp.c_fc")));
    }
}
