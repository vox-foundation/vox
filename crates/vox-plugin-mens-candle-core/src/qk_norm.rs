//! Dense Qwen3's per-head Q/K RMSNorm loader, shared by the Metal and CUDA
//! training model builds.
//!
//! Qwen3 checkpoints store `…self_attn.q_norm.weight` / `…k_norm.weight`
//! (shape `[head_dim]`). Qwen2/Qwen2.5 checkpoints have neither, so a missing
//! tensor is `None`, not an error. The key is derived from the projection
//! key (`…self_attn.q_proj.weight` → `…self_attn.q_norm.weight`) — a plain
//! `.weight` suffix swap would wrongly produce `q_proj_norm.weight`.

use candle_core::DType;
use candle_nn::{RmsNorm, VarBuilder};

/// RMSNorm epsilon Qwen3 uses for its Q/K norms.
const QK_NORM_EPS: f64 = 1e-6;

/// Load the Q or K norm that belongs to `proj_weight_key`, as F32.
pub fn load_qk_norm(vb: &VarBuilder, proj_weight_key: &str, head_dim: usize) -> Option<RmsNorm> {
    let norm_key = proj_weight_key.replace("_proj.weight", "_norm.weight");
    vb.get((head_dim,), &norm_key)
        .ok()
        .and_then(|t| t.to_dtype(DType::F32).ok())
        .map(|w| RmsNorm::new(w, QK_NORM_EPS))
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::{Device, Tensor};
    use std::collections::HashMap;

    /// `vb_dtype` is the dtype the VarBuilder hands tensors out as — use BF16
    /// to model a builder that does NOT cast for us, so the helper's own F32
    /// cast is what's under test.
    fn vb_with(entries: &[(&str, Tensor)], vb_dtype: DType) -> VarBuilder<'static> {
        let map: HashMap<String, Tensor> = entries
            .iter()
            .map(|(k, t)| ((*k).to_string(), t.clone()))
            .collect();
        VarBuilder::from_tensors(map, vb_dtype, &Device::Cpu)
    }

    #[test]
    fn loads_the_norm_named_after_the_projection() {
        let dev = Device::Cpu;
        let w = Tensor::new(&[2.0f32, 0.5, 3.0, 1.5], &dev).unwrap();
        let vb = vb_with(&[("model.layers.0.self_attn.q_norm.weight", w)], DType::F32);
        let norm = load_qk_norm(&vb, "model.layers.0.self_attn.q_proj.weight", 4)
            .expect("q_norm must be found from the q_proj key");
        // All-ones input has RMS 1, so the output is the weight (up to epsilon).
        let x = Tensor::ones((1, 4), DType::F32, &dev).unwrap();
        let y = norm
            .forward_diff(&x)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        for (got, want) in y.iter().zip([2.0f32, 0.5, 3.0, 1.5]) {
            assert!(
                (got - want).abs() < 1e-4,
                "stored weight not applied: got {y:?}"
            );
        }
    }

    #[test]
    fn missing_norm_is_none_not_an_error() {
        let vb = vb_with(&[], DType::F32);
        assert!(load_qk_norm(&vb, "model.layers.0.self_attn.k_proj.weight", 4).is_none());
    }

    #[test]
    fn the_norm_weight_is_f32_even_from_a_bf16_builder() {
        let dev = Device::Cpu;
        let w = Tensor::new(&[1.0f32, 1.0, 1.0, 1.0], &dev).unwrap();
        let vb = vb_with(&[("m.self_attn.k_norm.weight", w)], DType::BF16);
        let norm = load_qk_norm(&vb, "m.self_attn.k_proj.weight", 4).unwrap();
        // A BF16 weight against F32 activations is a dtype-mismatch error in
        // forward_diff; this only succeeds if the helper cast the weight to F32.
        let x = Tensor::ones((1, 4), DType::F32, &dev).unwrap();
        assert!(
            norm.forward_diff(&x).is_ok(),
            "norm weight must be F32 to match F32 activations"
        );
    }
}
