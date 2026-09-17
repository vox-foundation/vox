//! SafeTensors model source reader: single `model.safetensors` or sharded
//! via `model.safetensors.index.json` (HF `weight_map`).

use crate::error::QuantizeError;
use candle_core::{Device, Tensor};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A SafeTensors model source: single `model.safetensors` or sharded via
/// `model.safetensors.index.json` (HF `weight_map`).
pub struct SafeTensorsSource {
    map: HashMap<String, PathBuf>,
    names: Vec<String>,
}

#[derive(serde::Deserialize)]
struct ShardIndex {
    weight_map: HashMap<String, String>,
}

impl SafeTensorsSource {
    pub fn open(dir: &Path) -> Result<Self, QuantizeError> {
        let index = dir.join("model.safetensors.index.json");
        let single = dir.join("model.safetensors");
        let merged = dir.join("merged.safetensors");
        let mut map = HashMap::new();
        if index.exists() {
            let raw = std::fs::read_to_string(&index)?;
            let idx: ShardIndex =
                serde_json::from_str(&raw).map_err(|e| QuantizeError::ShardIndex(e.to_string()))?;
            for (name, file) in idx.weight_map {
                map.insert(name, dir.join(file));
            }
        } else if single.exists() {
            let file = std::fs::File::open(&single)?;
            #[allow(unsafe_code)]
            let mmap = unsafe { memmap2::MmapOptions::new().map(&file)? };
            let st = safetensors::SafeTensors::deserialize(&mmap).map_err(|e| {
                QuantizeError::ReadModel(format!("parse safetensors {}: {e}", single.display()))
            })?;
            for name in st.names() {
                map.insert(name.to_string(), single.clone());
            }
        } else if merged.exists() {
            // Overlay mode: dir has merged.safetensors and points to base model shards
            let base_dir = Self::resolve_base_dir(dir)?;
            let base_index = base_dir.join("model.safetensors.index.json");
            if base_index.exists() {
                let raw = std::fs::read_to_string(&base_index)?;
                let idx: ShardIndex = serde_json::from_str(&raw)
                    .map_err(|e| QuantizeError::ShardIndex(e.to_string()))?;
                for (name, file) in idx.weight_map {
                    map.insert(name, base_dir.join(file));
                }
            } else {
                return Err(QuantizeError::ReadModel(format!(
                    "base model index not found at {}",
                    base_index.display()
                )));
            }

            // Overlay the merged adapted keys from merged.safetensors
            let f = std::fs::File::open(&merged)?;
            #[allow(unsafe_code)]
            let mmap = unsafe { memmap2::MmapOptions::new().map(&f)? };
            let st = safetensors::SafeTensors::deserialize(&mmap)
                .map_err(|e| QuantizeError::ReadModel(format!("parse merged.safetensors: {e}")))?;
            for name in st.names() {
                map.insert(name.to_string(), merged.clone());
            }
        } else {
            return Err(QuantizeError::ReadModel(format!(
                "no model.safetensors, model.safetensors.index.json, or merged.safetensors in {}",
                dir.display()
            )));
        }
        let names: Vec<String> = map.keys().cloned().collect();
        Ok(Self { map, names })
    }

    fn resolve_base_dir(dir: &Path) -> Result<PathBuf, QuantizeError> {
        let tm_path = dir.join("training_manifest.json");
        if tm_path.is_file() {
            if let Ok(raw) = std::fs::read_to_string(&tm_path) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                    if let Some(tok_path) = v.get("tokenizer_path").and_then(|p| p.as_str()) {
                        let p = PathBuf::from(tok_path);
                        if let Some(parent) = p.parent() {
                            if parent.join("model.safetensors.index.json").is_file() {
                                return Ok(parent.to_path_buf());
                            }
                        }
                    }
                }
            }
        }

        // Try adapter_manifest.json base_model
        let am_path = dir.join("adapter_manifest.json");
        if am_path.is_file() {
            if let Ok(raw) = std::fs::read_to_string(&am_path) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                    if let Some(base) = v.get("base_model").and_then(|p| p.as_str()) {
                        if let Some(home) = dirs::home_dir() {
                            let hub = home
                                .join(".cache/huggingface/hub")
                                .join(format!("models--{}", base.replace('/', "--")))
                                .join("snapshots");
                            if let Ok(entries) = std::fs::read_dir(&hub) {
                                for entry in entries.flatten() {
                                    let ep = entry.path();
                                    if ep.is_dir()
                                        && ep.join("model.safetensors.index.json").is_file()
                                    {
                                        return Ok(ep);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Err(QuantizeError::ReadModel(format!(
            "could not resolve base model directory for overlay in {}",
            dir.display()
        )))
    }

    pub fn tensor_names(&self) -> &[String] {
        &self.names
    }

    /// Load a tensor and cast to f32 on CPU.
    pub fn load_f32(&self, name: &str) -> Result<Tensor, QuantizeError> {
        let path = self
            .map
            .get(name)
            .ok_or_else(|| QuantizeError::ReadModel(format!("tensor `{name}` not found")))?;
        let file = std::fs::File::open(path)?;
        #[allow(unsafe_code)]
        let mmap = unsafe { memmap2::MmapOptions::new().map(&file)? };
        let st = safetensors::SafeTensors::deserialize(&mmap).map_err(|e| {
            QuantizeError::ReadModel(format!("parse safetensors {}: {e}", path.display()))
        })?;
        let view = st.tensor(name).map_err(|e| {
            QuantizeError::ReadModel(format!("tensor `{name}` missing from shard: {e}"))
        })?;
        let shape: Vec<usize> = view.shape().to_vec();
        let candle_dt = match view.dtype() {
            safetensors::tensor::Dtype::F32 => candle_core::DType::F32,
            safetensors::tensor::Dtype::BF16 => candle_core::DType::BF16,
            safetensors::tensor::Dtype::F16 => candle_core::DType::F16,
            d => {
                return Err(QuantizeError::ReadModel(format!(
                    "unsupported dtype {d:?} for {name}"
                )));
            }
        };
        let t = Tensor::from_raw_buffer(view.data(), candle_dt, &shape, &Device::Cpu)?;
        if t.dtype() == candle_core::DType::F32 {
            Ok(t)
        } else {
            Ok(t.to_dtype(candle_core::DType::F32)?)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::{Device, Tensor};
    use std::collections::HashMap;

    fn write_st(dir: &std::path::Path, name: &str, tensors: &[(&str, Tensor)]) {
        let map: HashMap<String, Tensor> = tensors
            .iter()
            .map(|(k, t)| (k.to_string(), t.clone()))
            .collect();
        candle_core::safetensors::save(&map, dir.join(name)).unwrap();
    }

    #[test]
    fn reads_single_file_model() {
        let dir = tempfile::tempdir().unwrap();
        let t = Tensor::zeros((4, 256), candle_core::DType::F32, &Device::Cpu).unwrap();
        write_st(dir.path(), "model.safetensors", &[("w", t)]);
        let src = SafeTensorsSource::open(dir.path()).unwrap();
        let names: Vec<_> = src.tensor_names().to_vec();
        assert_eq!(names, vec!["w".to_string()]);
        let loaded = src.load_f32("w").unwrap();
        assert_eq!(loaded.dims(), &[4, 256]);
    }

    #[test]
    fn reads_sharded_model_via_index() {
        let dir = tempfile::tempdir().unwrap();
        let a = Tensor::zeros((2, 256), candle_core::DType::F32, &Device::Cpu).unwrap();
        let b = Tensor::zeros((2, 256), candle_core::DType::F32, &Device::Cpu).unwrap();
        write_st(dir.path(), "model-00001-of-00002.safetensors", &[("a", a)]);
        write_st(dir.path(), "model-00002-of-00002.safetensors", &[("b", b)]);
        std::fs::write(dir.path().join("model.safetensors.index.json"),
            r#"{"weight_map":{"a":"model-00001-of-00002.safetensors","b":"model-00002-of-00002.safetensors"}}"#).unwrap();
        let src = SafeTensorsSource::open(dir.path()).unwrap();
        let mut names = src.tensor_names().to_vec();
        names.sort();
        assert_eq!(names, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(src.load_f32("b").unwrap().dims(), &[2, 256]);
    }
}
