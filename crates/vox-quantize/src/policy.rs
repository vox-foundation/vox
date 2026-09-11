//! Quantization policy: tensor-role classification, named k-quant mixtures,
//! and GGML block-size alignment fallback.

use candle_core::quantized::GgmlDType;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TensorRole {
    Embedding,
    Output,
    DownProj,
    VProj,
    Matrix,
    KeepF32,
}

impl TensorRole {
    pub fn from_key(key: &str) -> Self {
        let k = key.to_ascii_lowercase();
        if k.ends_with("layernorm.weight")
            || k.ends_with(".norm.weight")
            // Dense Qwen3's per-head q_norm/k_norm end in "_norm.weight"
            // (underscore), which ".norm.weight" (dot) does not match.
            || k.ends_with("q_norm.weight")
            || k.ends_with("k_norm.weight")
            || k == "model.language_model.norm.weight"
            || k.ends_with(".a_log")
            || k.ends_with(".dt_bias")
            || k.ends_with(".bias")
            || k.contains("inv_freq")
        {
            return TensorRole::KeepF32;
        }
        if k.contains("embed_tokens") {
            return TensorRole::Embedding;
        }
        if k.contains("lm.head") || k.contains("lm_head") {
            return TensorRole::Output;
        }
        if k.ends_with("down_proj.weight") {
            return TensorRole::DownProj;
        }
        if k.ends_with("v_proj.weight") {
            return TensorRole::VProj;
        }
        TensorRole::Matrix
    }
}

#[derive(Debug, Clone)]
pub enum QuantMixture {
    Q4KM,
    Q5KM,
    Q6K,
    Q8_0,
    Manual(BTreeMap<TensorRole, GgmlDType>),
}

impl QuantMixture {
    pub fn target_for(&self, role: TensorRole) -> Option<GgmlDType> {
        if role == TensorRole::KeepF32 {
            return None;
        }
        match self {
            QuantMixture::Q4KM => Some(match role {
                TensorRole::DownProj
                | TensorRole::VProj
                | TensorRole::Embedding
                | TensorRole::Output => GgmlDType::Q6K,
                _ => GgmlDType::Q4K,
            }),
            QuantMixture::Q5KM => Some(match role {
                TensorRole::DownProj
                | TensorRole::VProj
                | TensorRole::Embedding
                | TensorRole::Output => GgmlDType::Q6K,
                _ => GgmlDType::Q5K,
            }),
            QuantMixture::Q6K => Some(GgmlDType::Q6K),
            QuantMixture::Q8_0 => Some(GgmlDType::Q8_0),
            QuantMixture::Manual(m) => m.get(&role).copied(),
        }
    }
}

/// Bits-per-weight for a GGML quantized dtype, derived from candle_core's
/// real block layout (`type_size` bytes / `block_size` elements) — never a
/// separately hardcoded constant.
pub fn bits_per_weight(dtype: GgmlDType) -> f64 {
    dtype.type_size() as f64 * 8.0 / dtype.block_size() as f64
}

/// Fraction of a dense transformer's quantizable parameters that fall into
/// Q4_K_M/Q5_K_M's "boosted" roles (DownProj + VProj + Embedding + Output,
/// all bumped to Q6K by `QuantMixture::target_for`) rather than the default
/// Matrix role. `target_for` only maps *a* role to *a* dtype — it has no
/// notion of how many parameters each role covers, which is a property of
/// the checkpoint's architecture, not of the policy.
///
/// Measured directly from `Qwen/Qwen3.8-27B`'s safetensors headers by
/// classifying every tensor with `TensorRole::from_key` and summing
/// element counts (27,781,427,952 total params):
///   down_proj  65 x 5120 x 17408 = 5,793,382,400  (64 layers + 1 MTP layer)
///   embed_tokens   248320 x 5120 = 1,271,398,400
///   lm_head        248320 x 5120 = 1,271,398,400  (untied, counted separately)
///   v_proj     17 x 1024 x 5120  =    89,128,960  (16 full-attn + 1 MTP layer)
///                        total   = 8,425,308,160 / 27,781,427,952 = 0.3033
/// See docs/superpowers/specs/2026-09-10-qwen38-27b-hub-design.md §5.1.
pub const QWEN3_27B_BOOSTED_ROLE_FRACTION: f64 = 0.303;

/// Weighted-average bits-per-weight for `mixture`, given the fraction of
/// parameters in "boosted" roles (DownProj/VProj/Embedding/Output) vs the
/// default Matrix role. Uses `QuantMixture::target_for`'s real role→dtype
/// assignments, not a restated bpw constant. Returns `None` for a `Manual`
/// mixture that doesn't map both roles.
pub fn mixture_bpw(mixture: &QuantMixture, boosted_fraction: f64) -> Option<f64> {
    let default_dtype = mixture.target_for(TensorRole::Matrix)?;
    let boosted_dtype = mixture.target_for(TensorRole::DownProj)?;
    Some(
        (1.0 - boosted_fraction) * bits_per_weight(default_dtype)
            + boosted_fraction * bits_per_weight(boosted_dtype),
    )
}

/// Weights-only footprint in GiB for a `params_b`-billion-parameter model at
/// `bpw` bits per weight.
pub fn needed_gib(params_b: f64, bpw: f64) -> f64 {
    params_b * 1e9 * bpw / 8.0 / (1024.0 * 1024.0 * 1024.0)
}

/// Does a `params_b`-billion-parameter model at `mixture` fit in
/// `usable_gib` of usable VRAM? Weights only — no KV cache or compute-buffer
/// margin; callers wanting headroom should pass a smaller `usable_gib`.
/// A `Manual` mixture without a usable bpw estimate is never rejected here
/// (unknown, not infeasible).
pub fn fits_target_tier(params_b: f64, mixture: &QuantMixture, usable_gib: f64) -> bool {
    match mixture_bpw(mixture, QWEN3_27B_BOOSTED_ROLE_FRACTION) {
        Some(bpw) => needed_gib(params_b, bpw) <= usable_gib,
        None => true,
    }
}

/// Enforce GGML block-size alignment against the tensor's last dimension.
pub fn resolve_dtype(target: GgmlDType, last_dim: usize) -> GgmlDType {
    let is_kquant = matches!(
        target,
        GgmlDType::Q2K
            | GgmlDType::Q3K
            | GgmlDType::Q4K
            | GgmlDType::Q5K
            | GgmlDType::Q6K
            | GgmlDType::Q8K
    );
    if is_kquant {
        if last_dim.is_multiple_of(256) {
            return target;
        }
        if last_dim.is_multiple_of(32) {
            return GgmlDType::Q8_0;
        }
        return GgmlDType::F32;
    }
    if last_dim.is_multiple_of(32) {
        target
    } else {
        GgmlDType::F32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::quantized::GgmlDType;

    #[test]
    fn role_classification_keeps_qk_norm_f32() {
        // Real dense Qwen3 checkpoints (e.g. Qwen/Qwen3-0.6B) carry a per-head
        // RMSNorm on Q/K before RoPE. `q_norm.weight`/`k_norm.weight` end in
        // "_norm.weight" (underscore), not ".norm.weight" (dot) like
        // input_layernorm/post_attention_layernorm — so the existing
        // `.ends_with(".norm.weight")` check misses them, and this small,
        // precision-sensitive per-head vector was falling through to the
        // default Matrix role and getting quantized (confirmed: produced a
        // Q8_0 fallback on a real checkpoint this session, which downstream
        // code that only reads the F32 map cannot even load, since ADR-043's
        // policy is that norms stay F32 on disk).
        assert_eq!(
            TensorRole::from_key("model.language_model.layers.3.self_attn.q_norm.weight"),
            TensorRole::KeepF32
        );
        assert_eq!(
            TensorRole::from_key("model.language_model.layers.3.self_attn.k_norm.weight"),
            TensorRole::KeepF32
        );
    }

    #[test]
    fn role_classification_keeps_norms_f32() {
        assert_eq!(
            TensorRole::from_key("model.language_model.layers.3.input_layernorm.weight"),
            TensorRole::KeepF32
        );
        assert_eq!(
            TensorRole::from_key("model.language_model.layers.3.linear_attn.A_log"),
            TensorRole::KeepF32
        );
        assert_eq!(
            TensorRole::from_key("model.language_model.layers.3.linear_attn.dt_bias"),
            TensorRole::KeepF32
        );
        assert_eq!(
            TensorRole::from_key("model.language_model.layers.3.mlp.down_proj.weight"),
            TensorRole::DownProj
        );
        assert_eq!(
            TensorRole::from_key("model.language_model.layers.3.self_attn.v_proj.weight"),
            TensorRole::VProj
        );
        assert_eq!(TensorRole::from_key("lm.head.weight"), TensorRole::Output);
        assert_eq!(
            TensorRole::from_key("model.language_model.embed_tokens.weight"),
            TensorRole::Embedding
        );
        assert_eq!(
            TensorRole::from_key("model.language_model.layers.3.mlp.gate_proj.weight"),
            TensorRole::Matrix
        );
    }

    #[test]
    fn q4km_bumps_downproj_and_vproj_to_q6k() {
        let m = QuantMixture::Q4KM;
        assert_eq!(m.target_for(TensorRole::Matrix), Some(GgmlDType::Q4K));
        assert_eq!(m.target_for(TensorRole::DownProj), Some(GgmlDType::Q6K));
        assert_eq!(m.target_for(TensorRole::VProj), Some(GgmlDType::Q6K));
        assert_eq!(m.target_for(TensorRole::Embedding), Some(GgmlDType::Q6K));
        assert_eq!(m.target_for(TensorRole::KeepF32), None);
    }

    #[test]
    fn bits_per_weight_matches_ggml_block_layout() {
        // 144-byte BlockQ4K / 256-elem block, 210-byte BlockQ6K / 256-elem
        // block — the exact figures the spec restates as 4.500 / 6.5625.
        assert_eq!(bits_per_weight(GgmlDType::Q4K), 4.5);
        assert_eq!(bits_per_weight(GgmlDType::Q6K), 6.5625);
    }

    #[test]
    fn q4km_mixture_bpw_matches_spec_arithmetic() {
        // 0.697 x 4.500 + 0.303 x 6.5625 = 5.1249375 (spec §5.1, rounded
        // there to 5.125). The dtype bpw terms come from bits_per_weight
        // (real candle_core block layout); only the 0.697/0.303 role split
        // is a named architecture constant.
        let bpw = mixture_bpw(&QuantMixture::Q4KM, QWEN3_27B_BOOSTED_ROLE_FRACTION).unwrap();
        assert!(
            (bpw - 5.1249375).abs() < 1e-9,
            "expected ~5.1249375 bpw, got {bpw}"
        );
    }

    #[test]
    fn q4_k_m_27b_is_rejected_for_16gb_tier() {
        // 27e9 params * 5.1249375 bpw / 8 / GiB ~= 16.11 GiB > 15.1 GiB
        // usable. (At the checkpoint's real 27.78e9 params: ~16.57 GiB.)
        assert!(!fits_target_tier(27.0, &QuantMixture::Q4KM, 15.1));
        // A genuinely sufficient tier (spec's 24 GB minimum consume target)
        // fits the same mixture.
        assert!(fits_target_tier(27.0, &QuantMixture::Q4KM, 24.0));
    }

    #[test]
    fn alignment_falls_back_below_256() {
        assert_eq!(resolve_dtype(GgmlDType::Q4K, 512), GgmlDType::Q4K);
        assert_eq!(resolve_dtype(GgmlDType::Q4K, 96), GgmlDType::Q8_0);
        assert_eq!(resolve_dtype(GgmlDType::Q4K, 100), GgmlDType::F32);
        assert_eq!(resolve_dtype(GgmlDType::Q8_0, 64), GgmlDType::Q8_0);
    }
}

#[cfg(test)]
mod semcov_wave5_tests {
    use super::*;
    use candle_core::quantized::GgmlDType;
    use std::collections::BTreeMap;

    #[test]
    fn q5km_uses_q5k_for_matrix_and_q6k_for_sensitive_roles() {
        let m = QuantMixture::Q5KM;
        assert_eq!(m.target_for(TensorRole::Matrix), Some(GgmlDType::Q5K));
        assert_eq!(m.target_for(TensorRole::DownProj), Some(GgmlDType::Q6K));
        assert_eq!(m.target_for(TensorRole::VProj), Some(GgmlDType::Q6K));
        assert_eq!(m.target_for(TensorRole::Embedding), Some(GgmlDType::Q6K));
        assert_eq!(m.target_for(TensorRole::Output), Some(GgmlDType::Q6K));
        assert_eq!(m.target_for(TensorRole::KeepF32), None);
    }

    #[test]
    fn q6k_maps_all_non_keepf32_roles_to_q6k() {
        let m = QuantMixture::Q6K;
        assert_eq!(m.target_for(TensorRole::Matrix), Some(GgmlDType::Q6K));
        assert_eq!(m.target_for(TensorRole::DownProj), Some(GgmlDType::Q6K));
        assert_eq!(m.target_for(TensorRole::Embedding), Some(GgmlDType::Q6K));
        assert_eq!(m.target_for(TensorRole::KeepF32), None);
    }

    #[test]
    fn q8_0_mixture_maps_all_non_keepf32_to_q8_0() {
        let m = QuantMixture::Q8_0;
        assert_eq!(m.target_for(TensorRole::Matrix), Some(GgmlDType::Q8_0));
        assert_eq!(m.target_for(TensorRole::DownProj), Some(GgmlDType::Q8_0));
        assert_eq!(m.target_for(TensorRole::VProj), Some(GgmlDType::Q8_0));
        assert_eq!(m.target_for(TensorRole::KeepF32), None);
    }

    #[test]
    fn manual_mixture_returns_mapped_dtype_or_none_for_absent_key() {
        let mut map = BTreeMap::new();
        map.insert(TensorRole::Matrix, GgmlDType::Q4K);
        let m = QuantMixture::Manual(map);
        assert_eq!(m.target_for(TensorRole::Matrix), Some(GgmlDType::Q4K));
        // DownProj is not in the map — expect None (no fallback to KeepF32 logic)
        assert_eq!(m.target_for(TensorRole::DownProj), None);
        // KeepF32 is short-circuited before the BTreeMap lookup
        assert_eq!(m.target_for(TensorRole::KeepF32), None);
    }
}
