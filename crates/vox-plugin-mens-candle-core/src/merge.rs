//! Merge exported Candle QLoRA adapter into base f32 weights.
//!
//! Reads the v3 adapter manifest (`adapter_manifest.json`) written by training.
//! The `vox_bounded_fs::read_utf8_path_capped` calls are replaced with plain
//! `std::fs::read_to_string` to avoid pulling `vox-scaling-policy` into the plugin.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Context;
use candle_core::{DType, Device, Tensor};
use safetensors::SafeTensors;
use safetensors::serialize;
use safetensors::tensor::{Dtype, TensorView};

use crate::adapter_schema_v3::PopuliAdapterManifestV3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum AdapterFamily {
    Gpt2Like,
    Qwen2Like,
    Qwen35Like,
    Unknown,
}

fn family_from_base_key(key: &str) -> AdapterFamily {
    if key.starts_with("h.") || key == "wte.weight" {
        AdapterFamily::Gpt2Like
    } else if key.starts_with("model.language_model.") {
        AdapterFamily::Qwen35Like
    } else if key.starts_with("model.layers.") || key.starts_with("model.embed_tokens.") {
        AdapterFamily::Qwen2Like
    } else {
        AdapterFamily::Unknown
    }
}

/// `W' = W + (B @ A) * (alpha / rank)` in f32 (PEFT scaling).
pub fn lora_delta_f32(
    lora_a: &Tensor,
    lora_b: &Tensor,
    alpha: f64,
    rank: usize,
) -> candle_core::Result<Tensor> {
    let scale = (alpha / rank.max(1) as f64) as f32;
    let ba = lora_b.matmul(lora_a)?;
    let s = Tensor::new(scale, ba.device())?;
    ba.broadcast_mul(&s)
}

/// Read one LoRA factor for `logical` out of a trained adapter file.
///
/// `QLoraTrainer::save_adapter` dumps the varmap verbatim, and peft-rs registers
/// each factor as a `candle_nn::Linear` weight under the layer's VarBuilder
/// prefix — so the key on disk is `<logical>.lora_a.weight`, not the bare
/// `<logical>.lora_a` this module originally looked up.
///
/// No fallback to the bare spelling: nothing in this workspace has ever written
/// it, and the old lookup that expected it could never have succeeded on a real
/// adapter. A tolerant fallback here would only let a future spelling drift pass
/// silently, which is the failure mode this whole path exists to kill.
fn adapter_factor(st: &SafeTensors<'_>, logical: &str, which: &str) -> anyhow::Result<Tensor> {
    let key = format!("{logical}.{which}.weight");
    let view = st
        .tensor(&key)
        .with_context(|| format!("adapter missing {key}"))?;
    tensor_from_f32_view(view)
}

/// One adapter layer's trained LoRA factors, with the scaling they were trained
/// under.
///
/// Deliberately the **factors**, not their product. `a` is `[rank, in]` and `b`
/// is `[out, rank]`; the dense `B @ A` delta is `[out, in]` — the full F32 size
/// of the base weight. Holding one product per layer resident would be roughly
/// an entire F32 copy of every linear layer in the model at once (the LM head
/// alone is ~4.7 GiB at 27B), which OOMs before a single token is served. The
/// product is formed transiently in `fold_lora_delta` instead, one weight at a
/// time, mirroring the streaming discipline `merge_qlora_into_base_subset`
/// already has.
pub struct LoraFactors {
    pub a: Tensor,
    pub b: Tensor,
    pub alpha: f64,
    pub rank: usize,
}

impl LoraFactors {
    /// This layer's dense delta `(B @ A) * alpha/rank`. Caller drops it right
    /// after adding it — never store the result.
    pub fn delta(&self) -> candle_core::Result<Tensor> {
        lora_delta_f32(&self.a, &self.b, self.alpha, self.rank)
    }
}

/// Every adapter layer's trained LoRA factors, keyed by the BASE tensor key the
/// delta must be added to, moved onto `device`.
///
/// This is the inference-side counterpart of `merge_qlora_into_base_subset`:
/// same factors, same scaling, no merged file written, and — like it — no more
/// than one dense delta alive at a time.
pub fn lora_factors_by_base_key(
    adapter_path: &Path,
    meta: &PopuliAdapterManifestV3,
    device: &Device,
) -> anyhow::Result<HashMap<String, LoraFactors>> {
    let bytes = std::fs::read(adapter_path)
        .with_context(|| format!("read adapter {}", adapter_path.display()))?;
    let st = SafeTensors::deserialize(&bytes).context("parse adapter safetensors")?;

    let alpha = meta.alpha as f64;
    let rank = meta.rank;
    let mut out: HashMap<String, LoraFactors> = HashMap::new();

    for logical in &meta.layer_order {
        let Some(base_key) = meta.base_key_map.get(logical) else {
            continue;
        };
        let a = adapter_factor(&st, logical, "lora_a")?;
        let b = adapter_factor(&st, logical, "lora_b")?;
        // Two adapter layers mapping to one base weight would make "which delta
        // wins" silent and load-order dependent. Refuse instead of guessing.
        if out.contains_key(base_key) {
            anyhow::bail!(
                "adapter base_key_map maps more than one layer onto base weight {base_key}; \
                 refusing to guess which delta applies"
            );
        }
        out.insert(
            base_key.clone(),
            LoraFactors {
                a: a.to_device(device)?,
                b: b.to_device(device)?,
                alpha,
                rank,
            },
        );
    }
    Ok(out)
}

/// Load a base shard tensor on CPU and normalize to f32.
fn tensor_from_safetensors_view_f32(view: TensorView<'_>) -> anyhow::Result<Tensor> {
    let shape: Vec<usize> = view.shape().to_vec();
    let candle_dt = match view.dtype() {
        Dtype::F32 => DType::F32,
        Dtype::BF16 => DType::BF16,
        Dtype::F16 => DType::F16,
        d => anyhow::bail!("unsupported base dtype {d:?} for merge (need F32, BF16, or F16)"),
    };
    let t = Tensor::from_raw_buffer(view.data(), candle_dt, &shape, &Device::Cpu)
        .map_err(|e| anyhow::anyhow!("from_raw_buffer: {e}"))?;
    if t.dtype() == DType::F32 {
        Ok(t)
    } else {
        t.to_dtype(DType::F32)
            .map_err(|e| anyhow::anyhow!("cast base tensor to f32: {e}"))
    }
}

fn tensor_from_f32_view(view: TensorView<'_>) -> anyhow::Result<Tensor> {
    let shape = view.shape().to_vec();
    if view.dtype() != Dtype::F32 {
        anyhow::bail!("expected F32 tensor, got {:?}", view.dtype());
    }
    let sl = view.data();
    let n = sl.len() / 4;
    let mut v = Vec::with_capacity(n);
    for i in 0..n {
        let o = i * 4;
        v.push(f32::from_le_bytes([sl[o], sl[o + 1], sl[o + 2], sl[o + 3]]));
    }
    let dev = Device::Cpu;
    Ok(Tensor::from_vec(v, shape, &dev)?)
}

fn load_f32_tensor_from_shards(
    base_paths: &[std::path::PathBuf],
    key: &str,
) -> anyhow::Result<Tensor> {
    for p in base_paths {
        let bytes = std::fs::read(p).with_context(|| format!("read {}", p.display()))?;
        let st =
            SafeTensors::deserialize(&bytes).with_context(|| format!("parse {}", p.display()))?;
        if let Ok(tv) = st.tensor(key) {
            return tensor_from_safetensors_view_f32(tv);
        }
    }
    anyhow::bail!("tensor {key} not found in base safetensors shards")
}

/// Merge adapter LoRA tensors into base weights; write only merged keys (f32).
pub fn merge_qlora_into_base_subset(
    base_paths: &[std::path::PathBuf],
    adapter_path: &Path,
    meta: &PopuliAdapterManifestV3,
    out_path: &Path,
) -> anyhow::Result<()> {
    if meta.format != PopuliAdapterManifestV3::FORMAT {
        anyhow::bail!(
            "unsupported adapter manifest format {:?}, expected {}",
            meta.format,
            PopuliAdapterManifestV3::FORMAT
        );
    }

    let adapter_bytes = std::fs::read(adapter_path)
        .with_context(|| format!("read adapter {}", adapter_path.display()))?;
    let adapter_st =
        SafeTensors::deserialize(&adapter_bytes).context("parse adapter safetensors")?;

    let alpha = meta.alpha as f64;
    let rank = meta.rank;

    let mut inferred_family = AdapterFamily::Unknown;
    let mut family_keys: HashMap<AdapterFamily, Vec<String>> = HashMap::new();
    for key in meta.base_key_map.values() {
        let fam = family_from_base_key(key);
        family_keys.entry(fam).or_default().push(key.clone());
        if fam == AdapterFamily::Unknown {
            continue;
        }
        if inferred_family == AdapterFamily::Unknown {
            inferred_family = fam;
            continue;
        }
        if inferred_family != fam {
            let mut summary: Vec<String> = family_keys
                .iter()
                .filter(|(family, _)| **family != AdapterFamily::Unknown)
                .map(|(family, keys)| format!("{family:?}:{}", keys.len()))
                .collect();
            summary.sort();
            anyhow::bail!(
                "adapter base_key_map mixes incompatible model families ({summary:?}); refusing cross-family merge. \
                 Next: regenerate adapter metadata from one base model family and ensure `layer_order`/`base_key_map` come from the same training run."
            );
        }
    }

    let mut buffers: Vec<(String, Vec<u8>, Vec<usize>)> = Vec::new();

    for logical in &meta.layer_order {
        let Some(base_key) = meta.base_key_map.get(logical) else {
            continue;
        };
        let t_a = adapter_factor(&adapter_st, logical, "lora_a")?;
        let t_b = adapter_factor(&adapter_st, logical, "lora_b")?;
        let delta = lora_delta_f32(&t_a, &t_b, alpha, rank).context("lora delta")?;

        let w = load_f32_tensor_from_shards(base_paths, base_key.as_str())?;
        let merged = w.broadcast_add(&delta)?;

        let shape: Vec<usize> = merged.dims().to_vec();
        let flat = merged.flatten_all()?.to_vec1::<f32>()?;
        let mut bytes = Vec::with_capacity(flat.len() * 4);
        for x in flat {
            bytes.extend_from_slice(&x.to_le_bytes());
        }
        buffers.push((base_key.clone(), bytes, shape));
    }

    let mut map: HashMap<String, TensorView<'_>> = HashMap::new();
    for (name, bytes, shape) in &buffers {
        let view = TensorView::new(Dtype::F32, shape.clone(), bytes.as_slice())
            .with_context(|| format!("TensorView for {name}"))?;
        map.insert(name.clone(), view);
    }

    let payload =
        serialize(&map, None).map_err(|e| anyhow::anyhow!("safetensors serialize: {e}"))?;
    std::fs::write(out_path, payload).with_context(|| format!("write {}", out_path.display()))?;
    Ok(())
}

/// Entry point called from `backend.rs` `merge_adapter`.
///
/// `base_path` — directory containing base model safetensors shards.
/// `adapter_path` — path to `candle_qlora_adapter.safetensors`.
/// `dest_path` — output path for merged safetensors (e.g. `merged.safetensors`).
pub fn merge_qlora_adapter(
    base_path: &str,
    adapter_path: &str,
    dest_path: &str,
) -> anyhow::Result<()> {
    let base_dir = std::path::Path::new(base_path);
    let adapter_p = std::path::Path::new(adapter_path);
    let out_p = std::path::Path::new(dest_path);

    // Load v3 manifest from adapter directory (sibling of adapter .safetensors)
    let meta_dir = adapter_p.parent().unwrap_or(std::path::Path::new("."));
    let meta_path = if meta_dir.join("adapter_manifest.json").is_file() {
        meta_dir.join("adapter_manifest.json")
    } else {
        anyhow::bail!(
            "adapter manifest (adapter_manifest.json) not found in {}",
            meta_dir.display()
        );
    };
    let meta_raw = std::fs::read_to_string(&meta_path)
        .with_context(|| format!("read {}", meta_path.display()))?;
    let meta: PopuliAdapterManifestV3 =
        serde_json::from_str(&meta_raw).with_context(|| "parse adapter manifest JSON")?;

    // Collect base shards from base_path directory
    let mut base_shards = Vec::new();
    for entry in std::fs::read_dir(base_dir)
        .with_context(|| format!("read base dir {}", base_dir.display()))?
    {
        let p = entry?.path();
        if p.extension().map(|e| e == "safetensors").unwrap_or(false)
            && p.file_name().unwrap().to_string_lossy().contains("model")
        {
            base_shards.push(p);
        }
    }
    base_shards.sort();

    merge_qlora_into_base_subset(&base_shards, adapter_p, &meta, out_p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::DType;
    use safetensors::SafeTensors;

    #[test]
    fn lora_delta_matches_manual_scale() {
        let dev = Device::Cpu;
        let a = Tensor::ones(&[2, 3], DType::F32, &dev).unwrap();
        let b = Tensor::ones(&[4, 2], DType::F32, &dev).unwrap();
        let d = lora_delta_f32(&a, &b, 8.0, 2).unwrap();
        assert_eq!(d.dims(), &[4, 3]);
    }

    /// Write a one-layer adapter under the key spelling `save_adapter` actually
    /// produces (`<logical>.lora_{a,b}.weight`), plus its v3 manifest.
    fn write_adapter_fixture(
        dir: &Path,
        logical: &str,
        base_key: &str,
        a: &Tensor,
        b: &Tensor,
        rank: usize,
        alpha: usize,
    ) -> (std::path::PathBuf, PopuliAdapterManifestV3) {
        let to_bytes = |t: &Tensor| {
            let mut v = Vec::new();
            for x in t.flatten_all().unwrap().to_vec1::<f32>().unwrap() {
                v.extend_from_slice(&x.to_le_bytes());
            }
            v
        };
        let (ab, bb) = (to_bytes(a), to_bytes(b));
        let mut map: HashMap<String, TensorView<'_>> = HashMap::new();
        map.insert(
            format!("{logical}.lora_a.weight"),
            TensorView::new(Dtype::F32, a.dims().to_vec(), ab.as_slice()).unwrap(),
        );
        map.insert(
            format!("{logical}.lora_b.weight"),
            TensorView::new(Dtype::F32, b.dims().to_vec(), bb.as_slice()).unwrap(),
        );
        let path = dir.join("candle_qlora_adapter.safetensors");
        std::fs::write(&path, serialize(&map, None).unwrap()).unwrap();

        let mut base_key_map = HashMap::new();
        base_key_map.insert(logical.to_string(), base_key.to_string());
        let meta = PopuliAdapterManifestV3::new(
            crate::finetune_contract::AdapterMethod::Qlora,
            crate::finetune_contract::BaseQuantMode::Nf4,
            true,
            base_key_map,
            vec![logical.to_string()],
            b.dim(0).unwrap(),
            a.dim(1).unwrap(),
            rank,
            alpha,
            None,
            None,
        );
        (path, meta)
    }

    #[test]
    fn deltas_are_keyed_by_base_key_and_scaled_by_alpha_over_rank() {
        let dir = tempfile::tempdir().expect("tempdir");
        let dev = Device::Cpu;
        let (rank, alpha, d, out) = (2usize, 8usize, 3usize, 4usize);
        let a = Tensor::ones(&[rank, d], DType::F32, &dev).unwrap();
        let b = Tensor::ones(&[out, rank], DType::F32, &dev).unwrap();
        let (path, meta) =
            write_adapter_fixture(dir.path(), "lm_head", "wte.weight", &a, &b, rank, alpha);

        let factors = lora_factors_by_base_key(&path, &meta, &dev).expect("factors");
        let f = factors
            .get("wte.weight")
            .expect("factors must be keyed by the BASE tensor key the model looks up");
        // Stored as factors, not a dense product — that is the whole point.
        assert_eq!(f.a.dims(), &[rank, d]);
        assert_eq!(f.b.dims(), &[out, rank]);
        let d0 = f.delta().expect("delta");
        assert_eq!(d0.dims(), &[out, d]);
        // (B @ A) = rank everywhere; scaled by alpha/rank => alpha.
        for v in d0.flatten_all().unwrap().to_vec1::<f32>().unwrap() {
            assert!((v - alpha as f32).abs() < 1e-5, "got {v}");
        }
    }

    /// The regression this whole fix exists for: training writes
    /// `<logical>.lora_a.weight`, and the old lookup asked for the bare
    /// `<logical>.lora_a`, so every adapter read came back "missing".
    #[test]
    fn the_key_spelling_training_writes_is_the_one_we_read() {
        let dir = tempfile::tempdir().expect("tempdir");
        let dev = Device::Cpu;
        let a = Tensor::ones(&[2, 3], DType::F32, &dev).unwrap();
        let b = Tensor::ones(&[4, 2], DType::F32, &dev).unwrap();
        let (path, meta) = write_adapter_fixture(dir.path(), "lm_head", "wte.weight", &a, &b, 2, 4);

        let bytes = std::fs::read(&path).unwrap();
        let st = SafeTensors::deserialize(&bytes).unwrap();
        assert!(
            st.tensor("lm_head.lora_a").is_err(),
            "fixture must use the on-disk spelling, not the fictitious bare one"
        );
        assert!(
            adapter_factor(&st, "lm_head", "lora_a").is_ok(),
            "the reader must use the spelling save_adapter actually writes"
        );
        assert!(lora_factors_by_base_key(&path, &meta, &dev).is_ok());
    }

    #[test]
    fn merge_v2_applies_lm_head_delta() {
        let dir = tempfile::tempdir().expect("tempdir");
        let d = 3usize;
        let vocab = 4usize;
        let rank = 2usize;
        let alpha = 4usize;
        let dev = Device::Cpu;

        let w: Vec<f32> = (0..vocab * d).map(|i| i as f32 * 0.1).collect();
        let w_mat = Tensor::from_vec(w, (vocab, d), &dev).unwrap();
        let flat = w_mat.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        let mut wb = Vec::with_capacity(flat.len() * 4);
        for x in &flat {
            wb.extend_from_slice(&x.to_le_bytes());
        }
        let mut base_map: HashMap<String, TensorView<'_>> = HashMap::new();
        base_map.insert(
            "wte.weight".into(),
            TensorView::new(Dtype::F32, vec![vocab, d], wb.as_slice()).unwrap(),
        );
        let base_path = dir.path().join("model.safetensors");
        std::fs::write(&base_path, serialize(&base_map, None).unwrap()).unwrap();

        let a = Tensor::ones(&[rank, d], DType::F32, &dev).unwrap();
        let b = Tensor::ones(&[vocab, rank], DType::F32, &dev).unwrap();
        let delta = lora_delta_f32(&a, &b, alpha as f64, rank).unwrap();
        let expected = w_mat.broadcast_add(&delta).unwrap();

        let fa = a.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        let fb = b.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        let mut ab = Vec::new();
        for x in fa {
            ab.extend_from_slice(&x.to_le_bytes());
        }
        let mut bb = Vec::new();
        for x in fb {
            bb.extend_from_slice(&x.to_le_bytes());
        }

        let mut ad_map: HashMap<String, TensorView<'_>> = HashMap::new();
        ad_map.insert(
            "lm_head.lora_a.weight".into(),
            TensorView::new(Dtype::F32, vec![rank, d], ab.as_slice()).unwrap(),
        );
        ad_map.insert(
            "lm_head.lora_b.weight".into(),
            TensorView::new(Dtype::F32, vec![vocab, rank], bb.as_slice()).unwrap(),
        );
        let ad_path = dir.path().join("candle_qlora_adapter.safetensors");
        std::fs::write(&ad_path, serialize(&ad_map, None).unwrap()).unwrap();

        let mut base_key_map = HashMap::new();
        base_key_map.insert("lm_head".into(), "wte.weight".into());
        let meta = PopuliAdapterManifestV3::new(
            crate::finetune_contract::AdapterMethod::Qlora,
            crate::finetune_contract::BaseQuantMode::Nf4,
            true,
            base_key_map,
            vec!["lm_head".into()],
            vocab,
            d,
            rank,
            alpha,
            None,
            None,
        );
        let out_path = dir.path().join("merged.safetensors");
        merge_qlora_into_base_subset(&[base_path], &ad_path, &meta, &out_path).expect("merge");

        let bytes = std::fs::read(&out_path).unwrap();
        let st = SafeTensors::deserialize(&bytes).unwrap();
        let tv = st.tensor("wte.weight").unwrap();
        let got = tensor_from_f32_view(tv).unwrap();
        let exp_flat = expected.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        let got_flat = got.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        assert_eq!(exp_flat.len(), got_flat.len());
        for (e, g) in exp_flat.iter().zip(got_flat.iter()) {
            assert!((e - g).abs() < 1e-5, "expected {e} got {g}");
        }
    }
}
