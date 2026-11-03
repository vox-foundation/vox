use crate::error::QuantizeError;
use crate::read::SafeTensorsSource;
use candle_core::Device;
use std::collections::HashMap;
use std::path::Path;

/// Default shard budget: matches the common Hugging Face convention so
/// recombined output looks like an ordinary sharded checkpoint.
const DEFAULT_SHARD_BYTES: u64 = 5 * 1024 * 1024 * 1024;

/// Build a complete model in `out_dir` by taking every base tensor and
/// overwriting the keys present in the merged subset. Errors if the subset
/// contains a key absent from the base (a sign of an adapter/base mismatch).
pub fn recombine(
    base_dir: &Path,
    merged_subset: &Path,
    out_dir: &Path,
) -> Result<(), QuantizeError> {
    let merged = candle_core::safetensors::load(merged_subset, &Device::Cpu)?;
    recombine_with_shard_budget(base_dir, &merged, out_dir, DEFAULT_SHARD_BYTES)
}

/// Same as [`recombine`], but writes shards of at most `shard_bytes` each
/// instead of one unsharded file, and takes the merged overrides already
/// loaded rather than a path to load them from.
///
/// Never holds more than one shard's worth of tensors resident: sizes for
/// grouping come from safetensors headers (no tensor data read), and each
/// shard's tensors are loaded, written, and dropped before the next group is
/// read.
pub fn recombine_with_shard_budget(
    base_dir: &Path,
    merged: &HashMap<String, candle_core::Tensor>,
    out_dir: &Path,
    shard_bytes: u64,
) -> Result<(), QuantizeError> {
    let base = SafeTensorsSource::open(base_dir)?;

    let base_names: std::collections::HashSet<&str> =
        base.tensor_names().iter().map(|s| s.as_str()).collect();
    for k in merged.keys() {
        if !base_names.contains(k.as_str()) {
            return Err(QuantizeError::ReadModel(format!(
                "merged key `{k}` not present in base model — adapter/base mismatch"
            )));
        }
    }

    // Validate shape agreement up front, from headers only -- a merged
    // override never needs the base tensor's data, only its shape and dtype.
    let base_shapes = base.tensor_shapes()?;
    for (name, m) in merged {
        if let Some(base_dims) = base_shapes.get(name) {
            let merged_dims = m.dims().to_vec();
            if &merged_dims != base_dims {
                return Err(QuantizeError::ReadModel(format!(
                    "merged key `{name}` shape {merged_dims:?} does not match base shape {base_dims:?} — adapter/base mismatch"
                )));
            }
        }
    }

    // Group tensor names into shards by byte size alone -- no tensor data is
    // read for this pass, so the shard count is known before the first write.
    let sizes = base.tensor_byte_sizes()?;
    let mut groups: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    let mut current_bytes: u64 = 0;
    for name in base.tensor_names() {
        let size = sizes.get(name).copied().unwrap_or(0);
        if !current.is_empty() && current_bytes + size > shard_bytes {
            groups.push(std::mem::take(&mut current));
            current_bytes = 0;
        }
        current.push(name.clone());
        current_bytes += size;
    }
    if !current.is_empty() {
        groups.push(current);
    }
    let shard_count = groups.len();
    // A model whose whole checkpoint fits under one budget still writes the
    // plain unsharded `model.safetensors` (no index) that `recombine`
    // produced before this change -- sharding machinery with one shard would
    // otherwise be an unannounced output-format break for every small model.
    let sharded = shard_count > 1;

    std::fs::create_dir_all(out_dir)?;
    let mut weight_map = serde_json::Map::new();
    let mut total_size: u64 = 0;

    for (i, group) in groups.iter().enumerate() {
        let mut shard: HashMap<String, candle_core::Tensor> = HashMap::new();
        for name in group {
            // Every non-overridden tensor comes back from `load_f32` upcast
            // to F32. A merged override must match that, or the recombined
            // checkpoint mixes dtypes across tensors (every base tensor F32,
            // every overridden one whatever dtype the merge produced) even
            // though nothing here signals that split. `to_dtype` is only
            // ever called on the small merged override, never on the
            // (possibly huge) base tensor.
            let t = match merged.get(name) {
                Some(m) if m.dtype() != candle_core::DType::F32 => {
                    m.to_dtype(candle_core::DType::F32)?
                }
                Some(m) => m.clone(),
                None => base.load_f32(name)?,
            };
            total_size += t.elem_count() as u64 * t.dtype().size_in_bytes() as u64;
            shard.insert(name.clone(), t);
        }
        let filename = if sharded {
            format!("model-{:05}-of-{:05}.safetensors", i + 1, shard_count)
        } else {
            "model.safetensors".to_string()
        };
        candle_core::safetensors::save(&shard, out_dir.join(&filename))?;
        for name in group {
            weight_map.insert(name.clone(), serde_json::Value::String(filename.clone()));
        }
        // `shard` drops here, before the next group is loaded.
    }

    if sharded {
        let index = serde_json::json!({
            "metadata": { "total_size": total_size },
            "weight_map": weight_map,
        });
        let bytes = serde_json::to_vec_pretty(&index)
            .map_err(|e| QuantizeError::ReadModel(format!("failed to serialize index: {e}")))?;
        std::fs::write(out_dir.join("model.safetensors.index.json"), bytes)?;
    }

    let cfg = base_dir.join("config.json");
    if cfg.exists() {
        std::fs::copy(&cfg, out_dir.join("config.json"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::{Device, Tensor};
    use std::collections::HashMap;

    /// Write a single-file base checkpoint of F32 tensors with the given
    /// element counts, plus a minimal config.json, and return its directory.
    fn write_fake_base(root: &std::path::Path, tensors: &[(&str, usize)]) -> std::path::PathBuf {
        let dir = root.join("base");
        std::fs::create_dir_all(&dir).unwrap();
        let mut map: HashMap<String, Tensor> = HashMap::new();
        for (name, elems) in tensors {
            map.insert(
                (*name).to_string(),
                Tensor::zeros((*elems,), candle_core::DType::F32, &Device::Cpu).unwrap(),
            );
        }
        candle_core::safetensors::save(&map, dir.join("model.safetensors")).unwrap();
        std::fs::write(dir.join("config.json"), r#"{"model_type":"test"}"#).unwrap();
        dir
    }

    #[test]
    fn recombine_does_not_hold_the_whole_model_resident() {
        let dir = tempfile::tempdir().unwrap();
        let base = write_fake_base(
            dir.path(),
            &[("a", 4096), ("b", 4096), ("c", 4096), ("d", 4096)],
        );
        let out = dir.path().join("out");
        recombine_with_shard_budget(&base, &HashMap::new(), &out, 8192).unwrap();

        let shards = std::fs::read_dir(&out)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".safetensors"))
            .count();
        assert!(
            shards >= 2,
            "a model larger than the shard budget must be written in shards, got {shards}"
        );
        assert!(
            out.join("model.safetensors.index.json").exists(),
            "sharded output needs an index or no reader can load it"
        );
    }

    #[test]
    fn recombine_preserves_every_tensor_across_shards() {
        let dir = tempfile::tempdir().unwrap();
        let base = write_fake_base(
            dir.path(),
            &[("a", 4096), ("b", 4096), ("c", 4096), ("d", 4096)],
        );
        let out = dir.path().join("out");
        recombine_with_shard_budget(&base, &HashMap::new(), &out, 8192).unwrap();
        let index: serde_json::Value = serde_json::from_reader(
            std::fs::File::open(out.join("model.safetensors.index.json")).unwrap(),
        )
        .unwrap();
        let map = index["weight_map"].as_object().expect("weight_map");
        for name in ["a", "b", "c", "d"] {
            assert!(map.contains_key(name), "sharding dropped tensor `{name}`");
        }
    }

    #[test]
    fn merged_subset_overrides_base_keys() {
        let dev = Device::Cpu;
        let base = tempfile::tempdir().unwrap();
        let merged = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();

        let mut b: HashMap<String, Tensor> = HashMap::new();
        b.insert(
            "w_adapted".into(),
            Tensor::zeros((256, 256), candle_core::DType::F32, &dev).unwrap(),
        );
        b.insert(
            "w_frozen".into(),
            Tensor::ones((256, 256), candle_core::DType::F32, &dev).unwrap(),
        );
        candle_core::safetensors::save(&b, base.path().join("model.safetensors")).unwrap();
        std::fs::write(base.path().join("config.json"), r#"{"model_type":"qwen3_5","architectures":["Qwen35ForCausalLM"],"hidden_size":256,"num_attention_heads":8,"num_hidden_layers":1,"vocab_size":512}"#).unwrap();

        let mut m: HashMap<String, Tensor> = HashMap::new();
        m.insert(
            "w_adapted".into(),
            Tensor::full(2.0f32, (256, 256), &dev).unwrap(),
        );
        candle_core::safetensors::save(&m, merged.path().join("merged.safetensors")).unwrap();

        recombine(
            base.path(),
            &merged.path().join("merged.safetensors"),
            out.path(),
        )
        .unwrap();

        let result =
            candle_core::safetensors::load(out.path().join("model.safetensors"), &dev).unwrap();
        assert_eq!(
            result["w_adapted"]
                .mean_all()
                .unwrap()
                .to_scalar::<f32>()
                .unwrap(),
            2.0
        );
        assert_eq!(
            result["w_frozen"]
                .mean_all()
                .unwrap()
                .to_scalar::<f32>()
                .unwrap(),
            1.0
        );
        assert!(out.path().join("config.json").exists());
    }

    #[test]
    fn merged_key_absent_from_base_errors() {
        let dev = Device::Cpu;
        let base = tempfile::tempdir().unwrap();
        let merged = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let mut b: HashMap<String, Tensor> = HashMap::new();
        b.insert(
            "w_frozen".into(),
            Tensor::ones((256, 256), candle_core::DType::F32, &dev).unwrap(),
        );
        candle_core::safetensors::save(&b, base.path().join("model.safetensors")).unwrap();
        let mut m: HashMap<String, Tensor> = HashMap::new();
        m.insert(
            "not_in_base".into(),
            Tensor::zeros((256, 256), candle_core::DType::F32, &dev).unwrap(),
        );
        candle_core::safetensors::save(&m, merged.path().join("merged.safetensors")).unwrap();
        assert!(
            recombine(
                base.path(),
                &merged.path().join("merged.safetensors"),
                out.path()
            )
            .is_err()
        );
    }

    #[test]
    fn merged_key_shape_mismatch_errors() {
        let dev = candle_core::Device::Cpu;
        let base = tempfile::tempdir().unwrap();
        let merged = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let mut b = std::collections::HashMap::new();
        b.insert(
            "w".to_string(),
            candle_core::Tensor::ones((256, 256), candle_core::DType::F32, &dev).unwrap(),
        );
        candle_core::safetensors::save(&b, base.path().join("model.safetensors")).unwrap();
        let mut m = std::collections::HashMap::new();
        m.insert(
            "w".to_string(),
            candle_core::Tensor::ones((128, 256), candle_core::DType::F32, &dev).unwrap(),
        );
        candle_core::safetensors::save(&m, merged.path().join("merged.safetensors")).unwrap();
        let err = recombine(
            base.path(),
            &merged.path().join("merged.safetensors"),
            out.path(),
        );
        assert!(err.is_err(), "shape mismatch must error");
    }

    /// Catches: writing a merged override's dtype unconditionally
    /// (`Some(m) => m.clone()` with no dtype check at all). A merged tensor
    /// with a non-F32 dtype must come out of `recombine` as F32, not
    /// silently keep its own -- matching what `load_f32` already does for
    /// every non-overridden tensor.
    #[test]
    fn merged_override_dtype_mismatch_is_cast_to_f32() {
        let dev = Device::Cpu;
        let base = tempfile::tempdir().unwrap();
        let merged = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();

        let mut b: HashMap<String, Tensor> = HashMap::new();
        b.insert("w".into(), Tensor::full(1.0f32, (256, 256), &dev).unwrap());
        candle_core::safetensors::save(&b, base.path().join("model.safetensors")).unwrap();
        std::fs::write(base.path().join("config.json"), r#"{"model_type":"test"}"#).unwrap();

        // Merged override is BF16 while the base tensor is F32.
        let mut m: HashMap<String, Tensor> = HashMap::new();
        m.insert(
            "w".into(),
            Tensor::full(2.0f32, (256, 256), &dev)
                .unwrap()
                .to_dtype(candle_core::DType::BF16)
                .unwrap(),
        );
        candle_core::safetensors::save(&m, merged.path().join("merged.safetensors")).unwrap();

        recombine(
            base.path(),
            &merged.path().join("merged.safetensors"),
            out.path(),
        )
        .unwrap();

        let result =
            candle_core::safetensors::load(out.path().join("model.safetensors"), &dev).unwrap();
        assert_eq!(
            result["w"].dtype(),
            candle_core::DType::F32,
            "a merged override with a non-F32 dtype must be cast to F32, not written as-is"
        );
        assert_eq!(
            result["w"].mean_all().unwrap().to_scalar::<f32>().unwrap(),
            2.0
        );
    }

    /// Catches: casting a merged override to the BASE's on-disk dtype
    /// instead of F32 (the bug this fix corrects). Against a real BF16 Qwen3
    /// checkpoint, `load_f32` upcasts every non-overridden tensor to F32
    /// while a base-dtype cast leaves the overridden tensor BF16 -- exactly
    /// the mixed-dtype output the comment above claims this code prevents.
    /// Asserts every tensor in the output is F32, not just the override.
    #[test]
    fn bf16_base_with_override_recombines_uniformly_to_f32() {
        let dev = Device::Cpu;
        let base = tempfile::tempdir().unwrap();
        let merged = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();

        // Base checkpoint is BF16 throughout, as a real Qwen3 checkpoint is.
        let mut b: HashMap<String, Tensor> = HashMap::new();
        b.insert(
            "w_adapted".into(),
            Tensor::full(1.0f32, (256, 256), &dev)
                .unwrap()
                .to_dtype(candle_core::DType::BF16)
                .unwrap(),
        );
        b.insert(
            "w_frozen".into(),
            Tensor::full(3.0f32, (256, 256), &dev)
                .unwrap()
                .to_dtype(candle_core::DType::BF16)
                .unwrap(),
        );
        candle_core::safetensors::save(&b, base.path().join("model.safetensors")).unwrap();
        std::fs::write(base.path().join("config.json"), r#"{"model_type":"test"}"#).unwrap();

        // Merge overrides one tensor, itself BF16 (as candle's training path
        // produces), leaving the other untouched.
        let mut m: HashMap<String, Tensor> = HashMap::new();
        m.insert(
            "w_adapted".into(),
            Tensor::full(2.0f32, (256, 256), &dev)
                .unwrap()
                .to_dtype(candle_core::DType::BF16)
                .unwrap(),
        );
        candle_core::safetensors::save(&m, merged.path().join("merged.safetensors")).unwrap();

        recombine(
            base.path(),
            &merged.path().join("merged.safetensors"),
            out.path(),
        )
        .unwrap();

        let result =
            candle_core::safetensors::load(out.path().join("model.safetensors"), &dev).unwrap();
        for (name, tensor) in &result {
            assert_eq!(
                tensor.dtype(),
                candle_core::DType::F32,
                "tensor `{name}` must be F32 -- recombine must not mix dtypes"
            );
        }
    }
}
