//! HF safetensors key discovery for Candle QLoRA projection stacking.
//!
//! Pure key naming lives in [`super::hf_keymap`]; this module keeps shard I/O.

pub use super::hf_keymap::{
    missing_middle_keys_report, ordered_full_block_weight_keys,
    ordered_middle_projection_keys, sample_present_keys_sorted_from_present,
};

use super::hf_load::HfTransformerLayout;
use anyhow::Context;
use safetensors::SafeTensors;
use std::collections::HashSet;

/// Counts for Candle QLoRA middle (`o_proj` / `c_proj`) projection keys vs safetensors union.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MiddleProjectionCoverage {
    pub matched: usize,
    pub expected: usize,
    /// `true` when `expected == 0` or every expected key is present in `present`.
    pub complete: bool,
}

/// Shared inventory for planner, `qlora_preflight`, and `candle_qlora_train` (avoid drift).
pub fn middle_projection_coverage(
    layout: &HfTransformerLayout,
    present: &HashSet<String>,
) -> MiddleProjectionCoverage {
    let expected_keys = ordered_middle_projection_keys(layout);
    let n_exp = expected_keys.len();
    if n_exp == 0 {
        return MiddleProjectionCoverage {
            matched: 0,
            expected: 0,
            complete: true,
        };
    }
    let matched = expected_keys
        .iter()
        .filter(|k| present.contains(k.as_str()))
        .count();
    MiddleProjectionCoverage {
        matched,
        expected: n_exp,
        complete: matched == n_exp,
    }
}

/// Collect all tensor names across weight shards (union).
pub fn tensor_keys_union(weight_paths: &[std::path::PathBuf]) -> anyhow::Result<HashSet<String>> {
    let mut keys = HashSet::new();
    for wp in weight_paths {
        let bytes =
            std::fs::read(wp).with_context(|| format!("read weight shard {}", wp.display()))?;
        let st =
            SafeTensors::deserialize(&bytes).with_context(|| format!("parse {}", wp.display()))?;
        for (k, _) in st.tensors() {
            keys.insert(k.clone());
        }
    }
    Ok(keys)
}

#[cfg(test)]
mod coverage_tests {
    use super::*;
    use crate::tensor::hf_load::HfTransformerLayout;
    use std::collections::HashSet;

    #[test]
    fn middle_projection_coverage_counts_keys() {
        let layout = HfTransformerLayout::from_config_json_str(
            "{\"model_type\":\"qwen2\",\"hidden_size\":8,\"num_attention_heads\":2,\"num_hidden_layers\":2,\"vocab_size\":10}",
        )
        .expect("layout");
        let mut present = HashSet::new();
        present.insert("model.layers.0.self_attn.o_proj.weight".to_string());
        let cov = middle_projection_coverage(&layout, &present);
        assert_eq!(cov.expected, 2);
        assert_eq!(cov.matched, 1);
        assert!(!cov.complete);
    }
}


