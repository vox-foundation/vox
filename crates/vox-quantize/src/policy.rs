//! Quantization policy: tensor-role classification, named k-quant mixtures,
//! and GGML block-size alignment fallback.

use crate::error::QuantizeError;
use candle_core::quantized::GgmlDType;
use std::collections::BTreeMap;
use std::path::Path;

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

/// Weights-only footprint in GiB for a `params_b`-billion-parameter model at
/// `bpw` bits per weight.
pub fn needed_gib(params_b: f64, bpw: f64) -> f64 {
    params_b * 1e9 * bpw / 8.0 / (1024.0 * 1024.0 * 1024.0)
}

/// Exact sizing for a `plan_quantize` run: the output artifact's size, the
/// peak RAM the engine needs to produce it, and the derived bpw/params.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuantizePlan {
    /// Total bytes the quantized output will occupy, summed tensor-by-tensor
    /// from candle's real `GgmlDType::{block_size, type_size}`.
    pub output_bytes: u64,
    /// Peak RAM: `2*S_max + output_bytes + 4*4*p_max` (see `plan_quantize`).
    pub peak_bytes: u64,
    /// `output_bytes * 8 / params`.
    pub bits_per_weight: f64,
    /// Total element count across every tensor in the checkpoint.
    pub params: u64,
}

/// Size a quantization run exactly from the safetensors headers in
/// `input_dir`, with no fitted parameter anywhere.
///
/// For every tensor in every `*.safetensors` file, classifies its role
/// (`TensorRole::from_key`), resolves the mixture's target dtype for that
/// role (`QuantMixture::target_for`), applies GGML block-size alignment
/// (`resolve_dtype`), and sizes it with candle's real `GgmlDType` block
/// layout — `elems / block_size * type_size` (this also covers `KeepF32` /
/// fallback `GgmlDType::F32`, whose `block_size() == 1` and
/// `type_size() == 4` reduce the same formula to `elems * 4`).
///
/// `peak_bytes = 2*S_max + output_bytes + 4*4*p_max`:
/// - `2*S_max` — the largest shard's on-disk size, doubled for the one-slot
///   shard cache holding one shard while the next is read (see `read.rs`).
/// - `output_bytes` — the exact total computed above.
/// - `4*4*p_max` — `round_trip_mse` (`verify.rs`) holds 4 live F32 buffers
///   (`src`, `deq`, `diff`, `sq`) the size of the largest tensor.
pub fn plan_quantize(
    input_dir: &Path,
    mixture: &QuantMixture,
) -> Result<QuantizePlan, QuantizeError> {
    let mut output_bytes: u64 = 0;
    let mut params: u64 = 0;
    let mut s_max: u64 = 0;
    let mut p_max: u64 = 0;

    for entry in std::fs::read_dir(input_dir)? {
        let path = entry?.path();
        if path.extension().is_none_or(|e| e != "safetensors") {
            continue;
        }
        let file_len = std::fs::metadata(&path)?.len();
        s_max = s_max.max(file_len);

        for (name, entry) in crate::read::read_header(&path)? {
            if name == "__metadata__" {
                continue;
            }
            let shape = crate::read::header_entry_shape(&entry).ok_or_else(|| {
                QuantizeError::ReadModel(format!(
                    "missing shape for `{name}` in {}",
                    path.display()
                ))
            })?;
            let elems = shape.iter().map(|&d| d as u64).product::<u64>();
            let last_dim = *shape.last().unwrap_or(&0);
            params += elems;
            p_max = p_max.max(elems);

            let dtype = match mixture.target_for(TensorRole::from_key(&name)) {
                Some(target) => resolve_dtype(target, last_dim),
                None => GgmlDType::F32,
            };
            output_bytes += elems / dtype.block_size() as u64 * dtype.type_size() as u64;
        }
    }

    let peak_bytes = 2 * s_max + output_bytes + 4 * 4 * p_max;
    let bits_per_weight = if params == 0 {
        0.0
    } else {
        output_bytes as f64 * 8.0 / params as f64
    };

    Ok(QuantizePlan {
        output_bytes,
        peak_bytes,
        bits_per_weight,
        params,
    })
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
    fn alignment_falls_back_below_256() {
        assert_eq!(resolve_dtype(GgmlDType::Q4K, 512), GgmlDType::Q4K);
        assert_eq!(resolve_dtype(GgmlDType::Q4K, 96), GgmlDType::Q8_0);
        assert_eq!(resolve_dtype(GgmlDType::Q4K, 100), GgmlDType::F32);
        assert_eq!(resolve_dtype(GgmlDType::Q8_0, 64), GgmlDType::Q8_0);
    }

    /// Catches: reintroducing a bpw estimate weighted by a fitted
    /// architecture constant. This fixture's boosted-role share is nothing
    /// like Qwen3-27B's 0.303, so a role-fraction estimator and an exact
    /// header walk disagree here; only the walk matches the size the engine
    /// actually writes.
    #[test]
    fn plan_quantize_sizes_the_output_from_real_ggml_blocks() {
        use candle_core::{Device, Tensor};
        use std::collections::HashMap;

        let dir = tempfile::tempdir().unwrap();
        let dev = Device::Cpu;
        let mut map: HashMap<String, Tensor> = HashMap::new();
        // Matrix role -> Q4K; down_proj -> Q6K; a norm -> KeepF32.
        map.insert(
            "model.layers.0.self_attn.q_proj.weight".into(),
            Tensor::zeros((256, 256), candle_core::DType::F32, &dev).unwrap(),
        );
        map.insert(
            "model.layers.0.mlp.down_proj.weight".into(),
            Tensor::zeros((256, 256), candle_core::DType::F32, &dev).unwrap(),
        );
        map.insert(
            "model.layers.0.input_layernorm.weight".into(),
            Tensor::zeros((256,), candle_core::DType::F32, &dev).unwrap(),
        );
        candle_core::safetensors::save(&map, dir.path().join("model.safetensors")).unwrap();

        let plan = plan_quantize(dir.path(), &QuantMixture::Q4KM).unwrap();

        let q4k = GgmlDType::Q4K;
        let q6k = GgmlDType::Q6K;
        let expected = (65536 / q4k.block_size() * q4k.type_size()
            + 65536 / q6k.block_size() * q6k.type_size()
            + 256 * 4) as u64;
        assert_eq!(
            plan.output_bytes, expected,
            "output size must come from candle's real block layout, not a bpw constant"
        );
        assert_eq!(plan.params, 65536 + 65536 + 256);
    }

    /// Catches: dropping any term of the peak formula. Every term is a file
    /// stat or block arithmetic; a peak below the output size or below the
    /// largest-tensor working set would let a run be scheduled that cannot
    /// complete, which is the failure this function exists to prevent.
    #[test]
    fn plan_quantize_peak_covers_the_shard_cache_the_output_and_the_verify_temporaries() {
        use candle_core::{Device, Tensor};
        use std::collections::HashMap;

        let dir = tempfile::tempdir().unwrap();
        let dev = Device::Cpu;
        let mut map: HashMap<String, Tensor> = HashMap::new();
        map.insert(
            "model.layers.0.self_attn.q_proj.weight".into(),
            Tensor::zeros((512, 256), candle_core::DType::F32, &dev).unwrap(),
        );
        candle_core::safetensors::save(&map, dir.path().join("model.safetensors")).unwrap();

        let plan = plan_quantize(dir.path(), &QuantMixture::Q4KM).unwrap();
        let shard_bytes = std::fs::metadata(dir.path().join("model.safetensors"))
            .unwrap()
            .len();
        let p_max = 512u64 * 256;
        assert!(
            plan.peak_bytes >= 2 * shard_bytes + plan.output_bytes + 4 * 4 * p_max,
            "peak {} omits a term: 2*S_max={} O={} 4*4*p_max={}",
            plan.peak_bytes,
            2 * shard_bytes,
            plan.output_bytes,
            4 * 4 * p_max
        );
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
