//! LoRA parameter / memory estimates (no GPU).

use vox_populi::mens::tensor::lora::{LoraConfig, lora_memory_estimate};

#[test]
fn lora_memory_estimate_matches_formula() {
    let in_f = 256usize;
    let out_f = 128usize;
    let rank = 8usize;
    let (lora, full, _pct) = lora_memory_estimate(in_f, out_f, rank);
    assert_eq!(lora, in_f * rank + rank * out_f);
    assert_eq!(full, in_f * out_f);
    assert!(lora < full);
}

#[test]
fn default_lora_config_alpha_over_rank_scale() {
    let c = LoraConfig::default();
    assert_eq!(c.rank, 8);
    assert_eq!(c.alpha, 16.0);
    let scale = c.alpha / c.rank as f32;
    assert!((scale - 2.0).abs() < 1e-6);
}
