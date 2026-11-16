use crate::error::QuantizeError;
use crate::read::SafeTensorsSource;
use candle_core::{DType, Device};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Default shard budget: matches the common Hugging Face convention so
/// recombined output looks like an ordinary sharded checkpoint.
const DEFAULT_SHARD_BYTES: u64 = 5 * 1024 * 1024 * 1024;

/// Accompanying tokenizer and generation files copied verbatim alongside
/// CausalLM exports when present in `base_dir`.
const ACCOMPANYING_FILES: &[&str] = &[
    "tokenizer.json",
    "tokenizer_config.json",
    "special_tokens_map.json",
    "generation_config.json",
    "chat_template.jinja",
];

/// Top-level keys stripped when flattening a multimodal Qwen3.5/3.8
/// (`Qwen3_5ForConditionalGeneration`) config to `Qwen3_5ForCausalLM`.
const MULTIMODAL_STRIP_KEYS: &[&str] = &[
    "vision_config",
    "mtp_num_hidden_layers",
    "image_token_id",
    "video_token_id",
    "vision_start_token_id",
    "vision_end_token_id",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum RecombineMode {
    UniformF32,
    HfCausalLm,
}

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
    recombine_impl(
        base_dir,
        merged,
        out_dir,
        shard_bytes,
        RecombineMode::UniformF32,
    )
}

/// Export a Hugging Face CausalLM checkpoint in `out_dir` by combining `base_dir`
/// and `merged_subset` while preserving base native tensor dtypes (`BF16`/`F16`/`F32`),
/// flattening multimodal `Qwen3_5ForConditionalGeneration` configs/keys to
/// `Qwen3_5ForCausalLM` (`qwen3_5_text`), and copying accompanying tokenizer/config files.
pub fn recombine_hf_causal_lm(
    base_dir: &Path,
    merged_subset: &Path,
    out_dir: &Path,
) -> Result<(), QuantizeError> {
    let merged = candle_core::safetensors::load(merged_subset, &Device::Cpu)?;
    recombine_hf_causal_lm_with_shard_budget(base_dir, &merged, out_dir, DEFAULT_SHARD_BYTES)
}

/// Same as [`recombine_hf_causal_lm`], with an explicit per-shard byte budget
/// and pre-loaded `merged` override map.
pub fn recombine_hf_causal_lm_with_shard_budget(
    base_dir: &Path,
    merged: &HashMap<String, candle_core::Tensor>,
    out_dir: &Path,
    shard_bytes: u64,
) -> Result<(), QuantizeError> {
    recombine_impl(
        base_dir,
        merged,
        out_dir,
        shard_bytes,
        RecombineMode::HfCausalLm,
    )
}

fn resolve_base_key(base_names: &HashSet<&str>, merged_key: &str) -> Option<String> {
    if base_names.contains(merged_key) {
        return Some(merged_key.to_string());
    }
    if let Some(rest) = merged_key.strip_prefix("model.language_model.") {
        let candidate = format!("model.{rest}");
        if base_names.contains(candidate.as_str()) {
            return Some(candidate);
        }
    } else if let Some(rest) = merged_key.strip_prefix("model.") {
        let candidate = format!("model.language_model.{rest}");
        if base_names.contains(candidate.as_str()) {
            return Some(candidate);
        }
    }
    None
}

fn is_multimodal_qwen35_config(v: &serde_json::Value) -> bool {
    let has_text_config = v.get("text_config").is_some_and(|tc| tc.is_object());
    let is_qwen35_model_type = v
        .get("model_type")
        .and_then(|x| x.as_str())
        .is_some_and(|mt| mt == "qwen3_5");
    let has_cond_gen_arch = v
        .get("architectures")
        .and_then(|a| a.as_array())
        .is_some_and(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str())
                .any(|s| s == "Qwen3_5ForConditionalGeneration")
        });
    has_text_config || is_qwen35_model_type || has_cond_gen_arch
}

fn emitted_tensor_name(base_key: &str, flatten_multimodal: bool) -> String {
    if flatten_multimodal && let Some(rest) = base_key.strip_prefix("model.language_model.") {
        return format!("model.{rest}");
    }
    base_key.to_string()
}

fn flatten_multimodal_qwen35_config(v: &mut serde_json::Value, synthesized_lm_head: bool) {
    let Some(obj) = v.as_object_mut() else {
        return;
    };
    let text_cfg = obj.remove("text_config");
    for key in MULTIMODAL_STRIP_KEYS {
        obj.remove(*key);
    }
    if let Some(serde_json::Value::Object(tc_map)) = text_cfg {
        for (k, val) in tc_map {
            obj.insert(k, val);
        }
    }
    obj.insert(
        "architectures".to_string(),
        serde_json::json!(["Qwen3_5ForCausalLM"]),
    );
    obj.insert(
        "model_type".to_string(),
        serde_json::Value::String("qwen3_5_text".to_string()),
    );
    if synthesized_lm_head {
        obj.insert(
            "tie_word_embeddings".to_string(),
            serde_json::Value::Bool(false),
        );
    }
}

fn untie_config_json(v: &mut serde_json::Value) {
    let Some(obj) = v.as_object_mut() else {
        return;
    };
    obj.insert(
        "tie_word_embeddings".to_string(),
        serde_json::Value::Bool(false),
    );
    if let Some(tc) = obj.get_mut("text_config").and_then(|x| x.as_object_mut()) {
        tc.insert(
            "tie_word_embeddings".to_string(),
            serde_json::Value::Bool(false),
        );
    }
}

fn recombine_impl(
    base_dir: &Path,
    merged: &HashMap<String, candle_core::Tensor>,
    out_dir: &Path,
    shard_bytes: u64,
    mode: RecombineMode,
) -> Result<(), QuantizeError> {
    let base = SafeTensorsSource::open(base_dir)?;

    let base_names: HashSet<&str> = base.tensor_names().iter().map(|s| s.as_str()).collect();
    // Validate shape agreement up front, from headers only -- a merged
    // override never needs the base tensor's data, only its shape and dtype.
    let base_shapes = base.tensor_shapes()?;
    let base_dtypes = match mode {
        RecombineMode::UniformF32 => None,
        RecombineMode::HfCausalLm => Some(base.tensor_dtypes()?),
    };
    let embed_key = [
        "model.embed_tokens.weight",
        "model.language_model.embed_tokens.weight",
    ]
    .into_iter()
    .find(|k| base_shapes.contains_key(*k));

    let mut resolved_merged: HashMap<String, (&candle_core::Tensor, DType)> = HashMap::new();
    let mut synthesized_lm_head_dtype: Option<DType> = None;

    for (k, m) in merged {
        let matched = match mode {
            RecombineMode::UniformF32 => base_names.contains(k.as_str()).then(|| k.clone()),
            RecombineMode::HfCausalLm => resolve_base_key(&base_names, k),
        };
        if let Some(base_key) = matched {
            if let Some(base_dims) = base_shapes.get(&base_key) {
                let merged_dims = m.dims().to_vec();
                if &merged_dims != base_dims {
                    return Err(QuantizeError::ReadModel(format!(
                        "merged key `{k}` shape {merged_dims:?} does not match base shape {base_dims:?} — adapter/base mismatch"
                    )));
                }
            }
            let target_dtype = match mode {
                RecombineMode::UniformF32 => DType::F32,
                RecombineMode::HfCausalLm => base_dtypes
                    .as_ref()
                    .and_then(|d| d.get(&base_key).copied())
                    .unwrap_or_else(|| m.dtype()),
            };
            if resolved_merged
                .insert(base_key.clone(), (m, target_dtype))
                .is_some()
            {
                return Err(QuantizeError::ReadModel(format!(
                    "multiple merged keys resolve to base tensor `{base_key}`"
                )));
            }
        } else if k == "lm_head.weight"
            && let Some(ek) = embed_key
        {
            let expected_dims = &base_shapes[ek];
            let lm_dims = m.dims().to_vec();
            if &lm_dims != expected_dims {
                return Err(QuantizeError::ReadModel(format!(
                    "synthesized `{k}` shape {lm_dims:?} does not match tied embedding shape {expected_dims:?} — adapter/base mismatch"
                )));
            }
            let target_dtype = match mode {
                RecombineMode::UniformF32 => DType::F32,
                RecombineMode::HfCausalLm => base_dtypes
                    .as_ref()
                    .and_then(|d| d.get(ek).copied())
                    .unwrap_or_else(|| m.dtype()),
            };
            synthesized_lm_head_dtype = Some(target_dtype);
            resolved_merged.insert(k.clone(), (m, target_dtype));
        } else {
            return Err(QuantizeError::ReadModel(format!(
                "merged key `{k}` not present in base model — adapter/base mismatch"
            )));
        }
    }

    let cfg_path = base_dir.join("config.json");
    let parsed_config = if cfg_path.exists() {
        std::fs::read_to_string(&cfg_path)
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
    } else {
        None
    };
    let flatten_multimodal = mode == RecombineMode::HfCausalLm
        && parsed_config
            .as_ref()
            .is_some_and(is_multimodal_qwen35_config);

    // Group tensor names into shards by byte size alone -- no tensor data is
    // read for this pass, so the shard count is known before the first write.
    let sizes = base.tensor_byte_sizes()?;
    let mut groups: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    let mut current_bytes: u64 = 0;
    for name in base.tensor_names() {
        if mode == RecombineMode::HfCausalLm && crate::policy::is_vision_or_mtp_key(name) {
            continue;
        }
        let size = sizes.get(name).copied().unwrap_or(0);
        if !current.is_empty() && current_bytes + size > shard_bytes {
            groups.push(std::mem::take(&mut current));
            current_bytes = 0;
        }
        current.push(name.clone());
        current_bytes += size;
    }
    if let Some(lm_dtype) = synthesized_lm_head_dtype {
        let elem_bytes = match mode {
            RecombineMode::UniformF32 => 4,
            RecombineMode::HfCausalLm => lm_dtype.size_in_bytes() as u64,
        };
        let size = merged["lm_head.weight"].elem_count() as u64 * elem_bytes;
        if !current.is_empty() && current_bytes + size > shard_bytes {
            groups.push(std::mem::take(&mut current));
        }
        current.push("lm_head.weight".to_string());
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
        let mut emitted_names = Vec::with_capacity(group.len());
        for name in group {
            // In `UniformF32`, non-overridden tensors come back from `load_f32`
            // upcast to F32 and merged overrides are cast to F32 to match.
            // In `HfCausalLm`, non-overridden tensors stay in their native
            // storage dtype (`load_native`) and merged overrides cast to their
            // matched base tensor's native dtype.
            let t = match resolved_merged.get(name) {
                Some(&(m, target_dtype)) if m.dtype() != target_dtype => {
                    m.to_dtype(target_dtype)?
                }
                Some(&(m, _)) => m.clone(),
                None => match mode {
                    RecombineMode::UniformF32 => base.load_f32(name)?,
                    RecombineMode::HfCausalLm => base.load_native(name)?,
                },
            };
            let out_name = emitted_tensor_name(name, flatten_multimodal);
            total_size += t.elem_count() as u64 * t.dtype().size_in_bytes() as u64;
            shard.insert(out_name.clone(), t);
            emitted_names.push(out_name);
        }
        let filename = if sharded {
            format!("model-{:05}-of-{:05}.safetensors", i + 1, shard_count)
        } else {
            "model.safetensors".to_string()
        };
        candle_core::safetensors::save(&shard, out_dir.join(&filename))?;
        for out_name in emitted_names {
            weight_map.insert(out_name, serde_json::Value::String(filename.clone()));
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

    if cfg_path.exists() {
        let out_cfg = out_dir.join("config.json");
        let synthesized = synthesized_lm_head_dtype.is_some();
        let transformed = match parsed_config {
            Some(mut v) if flatten_multimodal => {
                flatten_multimodal_qwen35_config(&mut v, synthesized);
                Some(v)
            }
            Some(mut v) if synthesized && v.is_object() => {
                untie_config_json(&mut v);
                Some(v)
            }
            _ => None,
        };
        if let Some(v) = transformed {
            let bytes = serde_json::to_vec_pretty(&v).map_err(|e| {
                QuantizeError::ReadModel(format!("failed to serialize config.json: {e}"))
            })?;
            std::fs::write(&out_cfg, bytes)?;
        } else {
            std::fs::copy(&cfg_path, &out_cfg)?;
        }
    }

    if mode == RecombineMode::HfCausalLm {
        for filename in ACCOMPANYING_FILES {
            let src = base_dir.join(filename);
            if src.is_file() {
                std::fs::copy(&src, out_dir.join(filename))?;
            }
        }
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

    #[test]
    fn recombine_synthesizes_lm_head_on_tied_base_and_unties_config() {
        let dev = Device::Cpu;
        let base = tempfile::tempdir().unwrap();
        let merged = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();

        let mut b: HashMap<String, Tensor> = HashMap::new();
        b.insert(
            "model.language_model.embed_tokens.weight".into(),
            Tensor::ones((16, 256), candle_core::DType::BF16, &dev).unwrap(),
        );
        candle_core::safetensors::save(&b, base.path().join("model.safetensors")).unwrap();
        std::fs::write(
            base.path().join("config.json"),
            r#"{"model_type":"qwen3_5","tie_word_embeddings":true,"text_config":{"hidden_size":256,"tie_word_embeddings":true}}"#,
        )
        .unwrap();

        let mut m: HashMap<String, Tensor> = HashMap::new();
        m.insert(
            "lm_head.weight".into(),
            Tensor::full(4.0f32, (16, 256), &dev).unwrap(),
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
        assert_eq!(result["lm_head.weight"].dtype(), candle_core::DType::F32);
        assert_eq!(
            result["lm_head.weight"]
                .mean_all()
                .unwrap()
                .to_scalar::<f32>()
                .unwrap(),
            4.0
        );
        assert_eq!(
            result["model.language_model.embed_tokens.weight"]
                .mean_all()
                .unwrap()
                .to_scalar::<f32>()
                .unwrap(),
            1.0
        );
        let cfg: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(out.path().join("config.json")).unwrap())
                .unwrap();
        assert_eq!(cfg["tie_word_embeddings"], serde_json::json!(false));
        assert_eq!(
            cfg["text_config"]["tie_word_embeddings"],
            serde_json::json!(false)
        );
    }

    #[test]
    fn hf_causal_lm_preserves_bf16_and_casts_f32_overrides() {
        let dev = Device::Cpu;
        let base = tempfile::tempdir().unwrap();
        let merged = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();

        let bf16 = |val: f32| {
            Tensor::full(val, (32, 64), &dev)
                .unwrap()
                .to_dtype(candle_core::DType::BF16)
                .unwrap()
        };
        let mut b: HashMap<String, Tensor> = HashMap::new();
        b.insert("model.embed_tokens.weight".into(), bf16(1.0));
        b.insert("model.layers.0.self_attn.q_proj.weight".into(), bf16(1.0));
        b.insert("model.layers.0.mlp.down_proj.weight".into(), bf16(3.0));
        candle_core::safetensors::save(&b, base.path().join("model.safetensors")).unwrap();
        std::fs::write(
            base.path().join("config.json"),
            r#"{"architectures":["Qwen3ForCausalLM"],"model_type":"qwen3","torch_dtype":"bfloat16"}"#,
        )
        .unwrap();

        let mut m: HashMap<String, Tensor> = HashMap::new();
        m.insert(
            "model.layers.0.self_attn.q_proj.weight".into(),
            Tensor::full(2.0f32, (32, 64), &dev).unwrap(),
        );
        candle_core::safetensors::save(&m, merged.path().join("merged.safetensors")).unwrap();

        recombine_hf_causal_lm(
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
                candle_core::DType::BF16,
                "tensor `{name}` must stay BF16"
            );
        }
        assert_eq!(
            result["model.layers.0.self_attn.q_proj.weight"]
                .to_dtype(candle_core::DType::F32)
                .unwrap()
                .mean_all()
                .unwrap()
                .to_scalar::<f32>()
                .unwrap(),
            2.0
        );
        assert_eq!(
            result["model.layers.0.mlp.down_proj.weight"]
                .to_dtype(candle_core::DType::F32)
                .unwrap()
                .mean_all()
                .unwrap()
                .to_scalar::<f32>()
                .unwrap(),
            3.0
        );
    }

    #[test]
    fn hf_causal_lm_flattens_qwen35_conditional_generation_and_rewrites_index() {
        let dev = Device::Cpu;
        let base = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();

        let bf16 = |val: f32, rows: usize, cols: usize| {
            Tensor::full(val, (rows, cols), &dev)
                .unwrap()
                .to_dtype(candle_core::DType::BF16)
                .unwrap()
        };
        let bf16_1d = |val: f32, len: usize| {
            Tensor::full(val, (len,), &dev)
                .unwrap()
                .to_dtype(candle_core::DType::BF16)
                .unwrap()
        };

        let mut shard1: HashMap<String, Tensor> = HashMap::new();
        shard1.insert(
            "model.language_model.embed_tokens.weight".into(),
            bf16(1.0, 16, 64),
        );
        shard1.insert(
            "model.language_model.layers.0.self_attn.q_proj.weight".into(),
            bf16(1.0, 64, 64),
        );
        shard1.insert("model.visual.patch_embed.weight".into(), bf16(9.0, 8, 8));
        candle_core::safetensors::save(
            &shard1,
            base.path().join("model-00001-of-00002.safetensors"),
        )
        .unwrap();

        let mut shard2: HashMap<String, Tensor> = HashMap::new();
        shard2.insert(
            "model.language_model.layers.0.mlp.down_proj.weight".into(),
            bf16(3.0, 64, 64),
        );
        shard2.insert("model.language_model.norm.weight".into(), bf16_1d(1.0, 64));
        shard2.insert("visual.encoder.weight".into(), bf16(9.0, 8, 8));
        shard2.insert("mtp.layers.0.weight".into(), bf16(9.0, 8, 8));
        candle_core::safetensors::save(
            &shard2,
            base.path().join("model-00002-of-00002.safetensors"),
        )
        .unwrap();

        std::fs::write(
            base.path().join("model.safetensors.index.json"),
            serde_json::json!({
                "weight_map": {
                    "model.language_model.embed_tokens.weight": "model-00001-of-00002.safetensors",
                    "model.language_model.layers.0.self_attn.q_proj.weight": "model-00001-of-00002.safetensors",
                    "model.visual.patch_embed.weight": "model-00001-of-00002.safetensors",
                    "model.language_model.layers.0.mlp.down_proj.weight": "model-00002-of-00002.safetensors",
                    "model.language_model.norm.weight": "model-00002-of-00002.safetensors",
                    "visual.encoder.weight": "model-00002-of-00002.safetensors",
                    "mtp.layers.0.weight": "model-00002-of-00002.safetensors"
                }
            })
            .to_string(),
        )
        .unwrap();

        std::fs::write(
            base.path().join("config.json"),
            serde_json::json!({
                "architectures": ["Qwen3_5ForConditionalGeneration"],
                "model_type": "qwen3_5",
                "torch_dtype": "bfloat16",
                "eos_token_id": 248044,
                "vision_config": { "depth": 27 },
                "mtp_num_hidden_layers": 1,
                "image_token_id": 248056,
                "video_token_id": 248057,
                "vision_start_token_id": 248053,
                "vision_end_token_id": 248054,
                "text_config": {
                    "hidden_size": 64,
                    "num_hidden_layers": 1,
                    "num_attention_heads": 4,
                    "vocab_size": 16,
                    "tie_word_embeddings": true
                }
            })
            .to_string(),
        )
        .unwrap();

        for (file, body) in [
            ("tokenizer.json", "{}"),
            ("tokenizer_config.json", r#"{"model_max_length":32768}"#),
            (
                "special_tokens_map.json",
                r#"{"eos_token":"<|endoftext|>"}"#,
            ),
            ("generation_config.json", r#"{"do_sample":true}"#),
            ("chat_template.jinja", "{{ messages }}"),
        ] {
            std::fs::write(base.path().join(file), body).unwrap();
        }

        // Merged overrides use CausalLM `model.*` keys while base uses `model.language_model.*`.
        let mut merged: HashMap<String, Tensor> = HashMap::new();
        merged.insert(
            "model.layers.0.self_attn.q_proj.weight".into(),
            Tensor::full(2.0f32, (64, 64), &dev).unwrap(),
        );
        merged.insert(
            "model.norm.weight".into(),
            Tensor::full(5.0f32, (64,), &dev).unwrap(),
        );

        // 8192-byte shard budget forces >1 shard across the ~18 KB BF16 payload.
        recombine_hf_causal_lm_with_shard_budget(base.path(), &merged, out.path(), 8192).unwrap();

        let index: serde_json::Value = serde_json::from_reader(
            std::fs::File::open(out.path().join("model.safetensors.index.json")).unwrap(),
        )
        .unwrap();
        let weight_map = index["weight_map"].as_object().expect("weight_map");
        for expected in [
            "model.embed_tokens.weight",
            "model.layers.0.self_attn.q_proj.weight",
            "model.layers.0.mlp.down_proj.weight",
            "model.norm.weight",
        ] {
            assert!(
                weight_map.contains_key(expected),
                "flattened index missing `{expected}`"
            );
        }
        for key in weight_map.keys() {
            assert!(
                !key.starts_with("model.language_model.")
                    && !key.starts_with("model.visual.")
                    && !key.starts_with("visual.")
                    && !key.starts_with("mtp."),
                "unexpected unflattened/multimodal key in index: `{key}`"
            );
        }

        let mut loaded: HashMap<String, Tensor> = HashMap::new();
        let shard_files: HashSet<String> = weight_map
            .values()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        assert!(shard_files.len() >= 2, "expected multiple output shards");
        for shard_file in shard_files {
            let part = candle_core::safetensors::load(out.path().join(shard_file), &dev).unwrap();
            for (k, t) in part {
                assert_eq!(t.dtype(), candle_core::DType::BF16);
                assert!(!k.starts_with("model.language_model."));
                loaded.insert(k, t);
            }
        }
        assert_eq!(loaded.len(), 4);
        assert_eq!(
            loaded["model.layers.0.self_attn.q_proj.weight"]
                .to_dtype(candle_core::DType::F32)
                .unwrap()
                .mean_all()
                .unwrap()
                .to_scalar::<f32>()
                .unwrap(),
            2.0
        );
        assert_eq!(
            loaded["model.norm.weight"]
                .to_dtype(candle_core::DType::F32)
                .unwrap()
                .mean_all()
                .unwrap()
                .to_scalar::<f32>()
                .unwrap(),
            5.0
        );

        let cfg: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(out.path().join("config.json")).unwrap())
                .unwrap();
        assert_eq!(
            cfg["architectures"],
            serde_json::json!(["Qwen3_5ForCausalLM"])
        );
        assert_eq!(cfg["model_type"], serde_json::json!("qwen3_5_text"));
        assert_eq!(cfg["hidden_size"], serde_json::json!(64));
        assert_eq!(cfg["num_hidden_layers"], serde_json::json!(1));
        assert_eq!(cfg["vocab_size"], serde_json::json!(16));
        assert_eq!(cfg["tie_word_embeddings"], serde_json::json!(true));
        assert_eq!(cfg["torch_dtype"], serde_json::json!("bfloat16"));
        assert_eq!(cfg["eos_token_id"], serde_json::json!(248044));
        for removed in [
            "text_config",
            "vision_config",
            "mtp_num_hidden_layers",
            "image_token_id",
            "video_token_id",
            "vision_start_token_id",
            "vision_end_token_id",
        ] {
            assert!(cfg.get(removed).is_none(), "`{removed}` must be stripped");
        }

        for file in ACCOMPANYING_FILES {
            assert!(out.path().join(file).is_file(), "missing `{file}`");
            assert_eq!(
                std::fs::read_to_string(out.path().join(file)).unwrap(),
                std::fs::read_to_string(base.path().join(file)).unwrap()
            );
        }
    }

    #[test]
    fn hf_causal_lm_synthesizes_bf16_lm_head_and_unties_promoted_config() {
        let dev = Device::Cpu;
        let base = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();

        let mut b: HashMap<String, Tensor> = HashMap::new();
        b.insert(
            "model.language_model.embed_tokens.weight".into(),
            Tensor::ones((16, 64), candle_core::DType::BF16, &dev).unwrap(),
        );
        candle_core::safetensors::save(&b, base.path().join("model.safetensors")).unwrap();
        std::fs::write(
            base.path().join("config.json"),
            r#"{"architectures":["Qwen3_5ForConditionalGeneration"],"model_type":"qwen3_5","text_config":{"hidden_size":64,"tie_word_embeddings":true}}"#,
        )
        .unwrap();

        let mut merged: HashMap<String, Tensor> = HashMap::new();
        merged.insert(
            "lm_head.weight".into(),
            Tensor::full(4.0f32, (16, 64), &dev).unwrap(),
        );

        recombine_hf_causal_lm_with_shard_budget(
            base.path(),
            &merged,
            out.path(),
            DEFAULT_SHARD_BYTES,
        )
        .unwrap();

        let result =
            candle_core::safetensors::load(out.path().join("model.safetensors"), &dev).unwrap();
        assert_eq!(result["lm_head.weight"].dtype(), candle_core::DType::BF16);
        assert_eq!(
            result["model.embed_tokens.weight"].dtype(),
            candle_core::DType::BF16
        );
        let cfg: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(out.path().join("config.json")).unwrap())
                .unwrap();
        assert_eq!(
            cfg["architectures"],
            serde_json::json!(["Qwen3_5ForCausalLM"])
        );
        assert_eq!(cfg["model_type"], serde_json::json!("qwen3_5_text"));
        assert_eq!(cfg["tie_word_embeddings"], serde_json::json!(false));
    }

    #[test]
    fn hf_causal_lm_qwen3_passthrough_preserves_config_and_bf16() {
        let dev = Device::Cpu;
        let base = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();

        let bf16 = |val: f32, rows: usize, cols: usize| {
            Tensor::full(val, (rows, cols), &dev)
                .unwrap()
                .to_dtype(candle_core::DType::BF16)
                .unwrap()
        };
        let mut b: HashMap<String, Tensor> = HashMap::new();
        b.insert("model.embed_tokens.weight".into(), bf16(1.0, 16, 64));
        b.insert(
            "model.layers.0.self_attn.q_proj.weight".into(),
            bf16(1.0, 64, 64),
        );
        b.insert("lm_head.weight".into(), bf16(1.5, 16, 64));
        candle_core::safetensors::save(&b, base.path().join("model.safetensors")).unwrap();

        let orig_cfg = serde_json::json!({
            "architectures": ["Qwen3ForCausalLM"],
            "model_type": "qwen3",
            "hidden_size": 64,
            "num_hidden_layers": 1,
            "vocab_size": 16,
            "torch_dtype": "bfloat16"
        })
        .to_string();
        std::fs::write(base.path().join("config.json"), &orig_cfg).unwrap();
        std::fs::write(base.path().join("tokenizer.json"), "{}").unwrap();
        std::fs::write(base.path().join("generation_config.json"), "{}").unwrap();

        let mut merged: HashMap<String, Tensor> = HashMap::new();
        merged.insert(
            "model.layers.0.self_attn.q_proj.weight".into(),
            Tensor::full(3.0f32, (64, 64), &dev).unwrap(),
        );

        recombine_hf_causal_lm_with_shard_budget(
            base.path(),
            &merged,
            out.path(),
            DEFAULT_SHARD_BYTES,
        )
        .unwrap();

        let result =
            candle_core::safetensors::load(out.path().join("model.safetensors"), &dev).unwrap();
        assert_eq!(result.len(), 3);
        for (name, tensor) in &result {
            assert_eq!(
                tensor.dtype(),
                candle_core::DType::BF16,
                "tensor `{name}` must stay BF16"
            );
        }
        assert_eq!(
            std::fs::read_to_string(out.path().join("config.json")).unwrap(),
            orig_cfg
        );
        assert!(out.path().join("tokenizer.json").is_file());
        assert!(out.path().join("generation_config.json").is_file());
    }
}
