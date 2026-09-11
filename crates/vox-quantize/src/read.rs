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
    /// One-slot shard cache. `names` is grouped by shard, so a single slot
    /// yields exactly one load per shard and bounds peak RSS to one shard.
    /// Previously `load_f32` called `candle_core::safetensors::load` (fs::read
    /// + full deserialize, no mmap) once per tensor.
    cached: std::cell::RefCell<Option<(PathBuf, HashMap<String, Tensor>)>>,
    loads: std::cell::Cell<usize>,
}

#[derive(serde::Deserialize)]
struct ShardIndex {
    weight_map: HashMap<String, String>,
}

/// Tensor names from a safetensors file's header, reading only the 8-byte
/// length prefix and the JSON header itself — never tensor data.
///
/// `open` previously called `candle_core::safetensors::load` here purely to
/// enumerate `st.keys()`, which materializes the whole checkpoint and then
/// drops it. On the unsharded intermediate `recombine` writes, that is the
/// entire model in RAM before any tensor has been quantized.
fn read_header(path: &Path) -> Result<serde_json::Map<String, serde_json::Value>, QuantizeError> {
    use std::io::Read;
    let mut f = std::fs::File::open(path)?;
    let mut len_buf = [0u8; 8];
    f.read_exact(&mut len_buf)?;
    let header_len = u64::from_le_bytes(len_buf);
    let header_len = usize::try_from(header_len).map_err(|_| {
        QuantizeError::ReadModel(format!(
            "header length overflows usize in {}",
            path.display()
        ))
    })?;
    let mut header = vec![0u8; header_len];
    f.read_exact(&mut header)?;
    let json: serde_json::Value = serde_json::from_slice(&header)
        .map_err(|e| QuantizeError::ReadModel(format!("{}: {e}", path.display())))?;
    json.as_object().cloned().ok_or_else(|| {
        QuantizeError::ReadModel(format!("header is not an object in {}", path.display()))
    })
}

fn header_tensor_names(path: &Path) -> Result<Vec<String>, QuantizeError> {
    Ok(read_header(path)?
        .keys()
        .filter(|k| k.as_str() != "__metadata__")
        .cloned()
        .collect())
}

/// Byte length of a tensor's data section, from its safetensors header entry
/// (`data_offsets: [start, end]`) — no tensor data is read.
fn header_entry_byte_len(entry: &serde_json::Value) -> Option<u64> {
    let offsets = entry.get("data_offsets")?.as_array()?;
    let start = offsets.first()?.as_u64()?;
    let end = offsets.get(1)?.as_u64()?;
    end.checked_sub(start)
}

/// Shape of a tensor from its safetensors header entry (`shape: [..]`) — no
/// tensor data is read.
fn header_entry_shape(entry: &serde_json::Value) -> Option<Vec<usize>> {
    entry
        .get("shape")?
        .as_array()?
        .iter()
        .map(|v| v.as_u64().map(|n| n as usize))
        .collect()
}

impl SafeTensorsSource {
    pub fn open(dir: &Path) -> Result<Self, QuantizeError> {
        let index = dir.join("model.safetensors.index.json");
        let single = dir.join("model.safetensors");
        let mut map = HashMap::new();
        if index.exists() {
            let raw = std::fs::read_to_string(&index)?;
            let idx: ShardIndex =
                serde_json::from_str(&raw).map_err(|e| QuantizeError::ShardIndex(e.to_string()))?;
            for (name, file) in idx.weight_map {
                map.insert(name, dir.join(file));
            }
        } else if single.exists() {
            for name in header_tensor_names(&single)? {
                map.insert(name, single.clone());
            }
        } else {
            return Err(QuantizeError::ReadModel(format!(
                "no model.safetensors or model.safetensors.index.json in {}",
                dir.display()
            )));
        }
        let mut names: Vec<String> = map.keys().cloned().collect();
        // Group by shard (then by name for determinism) so the one-slot cache
        // in `load_f32` sees each shard exactly once.
        names.sort_by(|a, b| (&map[a], a).cmp(&(&map[b], b)));
        Ok(Self {
            map,
            names,
            cached: std::cell::RefCell::new(None),
            loads: std::cell::Cell::new(0),
        })
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

        let mut slot = self.cached.borrow_mut();
        let hit = slot.as_ref().is_some_and(|(p, _)| p == path);
        if !hit {
            // Drop the previous shard before reading the next one so peak RSS
            // stays near one shard rather than the whole checkpoint.
            *slot = None;
            let tensors = candle_core::safetensors::load(path, &Device::Cpu)?;
            self.loads.set(self.loads.get() + 1);
            *slot = Some((path.clone(), tensors));
        }
        let (_, tensors) = slot.as_ref().expect("just populated");
        let t = tensors.get(name).ok_or_else(|| {
            QuantizeError::ReadModel(format!("tensor `{name}` missing from shard"))
        })?;
        Ok(t.to_dtype(candle_core::DType::F32)?)
    }

    /// Byte size of each tensor's data, read from safetensors headers only —
    /// no tensor data is loaded. Headers are grouped by file so each is
    /// parsed once regardless of how many tensors it holds.
    pub(crate) fn tensor_byte_sizes(&self) -> Result<HashMap<String, u64>, QuantizeError> {
        let mut sizes = HashMap::new();
        for path in self.unique_paths() {
            for (name, entry) in read_header(path)? {
                if name != "__metadata__"
                    && let Some(len) = header_entry_byte_len(&entry)
                {
                    sizes.insert(name, len);
                }
            }
        }
        Ok(sizes)
    }

    /// Shape of each tensor, read from safetensors headers only — no tensor
    /// data is loaded. Lets a merged-override shape check skip loading the
    /// base tensor entirely.
    pub(crate) fn tensor_shapes(&self) -> Result<HashMap<String, Vec<usize>>, QuantizeError> {
        let mut shapes = HashMap::new();
        for path in self.unique_paths() {
            for (name, entry) in read_header(path)? {
                if name != "__metadata__"
                    && let Some(shape) = header_entry_shape(&entry)
                {
                    shapes.insert(name, shape);
                }
            }
        }
        Ok(shapes)
    }

    fn unique_paths(&self) -> std::collections::HashSet<&PathBuf> {
        self.map.values().collect()
    }

    /// Number of shard files actually deserialized by `load_f32`. Test-only
    /// observability for the read-amplification guard; not part of the API.
    #[cfg(test)]
    fn shard_loads(&self) -> usize {
        self.loads.get()
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

    /// Catches: reverting `load_f32` to a per-tensor
    /// `candle_core::safetensors::load` (the read-amplification bug — four
    /// tensors would cost four shard loads), and dropping the shard grouping
    /// in `open` (which makes the one-slot cache thrash back to one load per
    /// tensor). Both mutations are observable here; neither is observable
    /// from a cache driven by a fake loader.
    #[test]
    fn sharded_reads_group_by_shard_and_load_each_shard_once() {
        let dir = tempfile::tempdir().unwrap();
        let t = || Tensor::zeros((2, 256), candle_core::DType::F32, &Device::Cpu).unwrap();
        write_st(
            dir.path(),
            "model-00001-of-00002.safetensors",
            &[("a1", t()), ("a2", t())],
        );
        write_st(
            dir.path(),
            "model-00002-of-00002.safetensors",
            &[("b1", t()), ("b2", t())],
        );
        std::fs::write(
            dir.path().join("model.safetensors.index.json"),
            r#"{"weight_map":{
                "a1":"model-00001-of-00002.safetensors",
                "a2":"model-00001-of-00002.safetensors",
                "b1":"model-00002-of-00002.safetensors",
                "b2":"model-00002-of-00002.safetensors"}}"#,
        )
        .unwrap();

        let src = SafeTensorsSource::open(dir.path()).unwrap();
        assert_eq!(
            src.tensor_names(),
            &[
                "a1".to_string(),
                "a2".to_string(),
                "b1".to_string(),
                "b2".to_string()
            ],
            "names must be grouped by shard so a one-slot cache suffices"
        );

        for name in src.tensor_names().to_vec() {
            assert_eq!(src.load_f32(&name).unwrap().dims(), &[2, 256]);
        }
        assert_eq!(
            src.shard_loads(),
            2,
            "four tensors across two shards must cost two shard loads"
        );
    }

    /// Catches: reverting `open`'s no-index branch to
    /// `candle_core::safetensors::load(&single, ..)` just to read `st.keys()`.
    /// That loads the whole checkpoint to enumerate names and discards it —
    /// fatal on the ~111 GB unsharded intermediate `recombine` writes.
    ///
    /// Note this does NOT use `shard_loads()` as the observable: that counter
    /// lives on `Self`, which doesn't exist yet while `open`'s branch runs, so
    /// no instance-level counter can ever see what a full-load mutation did
    /// there (verified: asserting `shard_loads() == 0` right after `open()`
    /// stayed green even with the buggy full-load call restored by hand).
    /// Instead the fixture truncates the file to just its header — the
    /// tensor-data section is entirely gone. A header-only reader still
    /// succeeds; `candle_core::safetensors::load` needs the data section and
    /// errors, so `open()` failing is the real, code-path-independent signal.
    #[test]
    fn open_enumerates_a_single_file_model_without_loading_tensor_data() {
        let dir = tempfile::tempdir().unwrap();
        let t = || Tensor::zeros((2, 256), candle_core::DType::F32, &Device::Cpu).unwrap();
        let path = dir.path().join("model.safetensors");
        write_st(dir.path(), "model.safetensors", &[("w1", t()), ("w2", t())]);

        // Truncate to header-only: 8-byte length prefix + that many header
        // bytes, dropping every byte of actual tensor data.
        let bytes = std::fs::read(&path).unwrap();
        let header_len = u64::from_le_bytes(bytes[0..8].try_into().unwrap()) as usize;
        std::fs::write(&path, &bytes[..8 + header_len]).unwrap();

        let src = SafeTensorsSource::open(dir.path())
            .expect("header-only read must not require the (now-missing) tensor data section");
        let mut names = src.tensor_names().to_vec();
        names.sort();
        assert_eq!(names, vec!["w1".to_string(), "w2".to_string()]);

        // load_f32 does need the data section, so it fails against the
        // truncated fixture -- that's expected and irrelevant to this test.
        assert_eq!(src.shard_loads(), 0, "no shard has been loaded yet");
    }
}
