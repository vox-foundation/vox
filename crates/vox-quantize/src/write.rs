//! Write a quantized SafeTensors-canonical artifact (ADR-043 on-disk format).
//!
//! Quantized tensors are serialized as 1-D `u8` SafeTensors tensors (the raw
//! GGML block bytes from candle), and a `quant-metadata.json` sidecar records
//! the per-tensor GGML dtype, original shape/dtype, and the mixture name.

use crate::error::QuantizeError;
use candle_core::Tensor;
use candle_core::quantized::QTensor;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct TensorMeta {
    pub ggml_dtype: String,
    pub orig_shape: Vec<usize>,
    pub orig_dtype: String,
    pub quantized: bool,
}

#[derive(Debug, Serialize)]
struct QuantMetadata {
    mixture: String,
    writer_version: String,
    tensors: HashMap<String, TensorMeta>,
}

/// Default shard budget for [`ArtifactWriter::new`] -- matches
/// [`recombine`](crate::recombine)'s convention so a streamed quantized
/// artifact shards the same way a streamed recombined one does.
const DEFAULT_SHARD_BYTES: u64 = 5 * 1024 * 1024 * 1024;

pub struct ArtifactWriter {
    /// Directory shards are written into as they flush. For [`Self::new`]
    /// this is a scratch directory (`owns_scratch = true`) moved into the
    /// real output directory at [`Self::finish`], because `new()` takes no
    /// output path and the real one isn't known until `finish` is called.
    out_dir: PathBuf,
    owns_scratch: bool,
    /// Whether `out_dir` has actually been created on disk yet. For
    /// `owns_scratch = true` (constructed via [`Self::new`]) this starts
    /// `false`: creation is deferred to the first call to [`Self::stage`],
    /// which can return `Err`, instead of happening in `new()`, which
    /// cannot. Always `true` for `with_shard_budget`, which creates its
    /// (caller-supplied) directory eagerly since it can fail there.
    scratch_ready: bool,
    shard_bytes: u64,
    pending: HashMap<String, (Vec<u8>, candle_core::DType, Vec<usize>)>,
    pending_bytes: u64,
    /// Shard files already flushed to `out_dir`, named with a placeholder
    /// `-of-tmp` suffix since the final shard count isn't known until
    /// `finish` -- renamed to their real `-of-{n:05}` name there.
    shard_paths: Vec<PathBuf>,
    /// Tensor name -> index into `shard_paths`, resolved to the shard's
    /// final filename at `finish`.
    weight_map: HashMap<String, usize>,
    meta: HashMap<String, TensorMeta>,
}

fn quantized_bytes(q: &QTensor) -> Result<Vec<u8>, QuantizeError> {
    Ok(q.data()?.into_owned())
}

/// Rename `src` to `dst`, falling back to copy+remove when they are on
/// different filesystems (the scratch dir from [`ArtifactWriter::new`] is
/// under the OS temp root, which may not share a device with `out_dir`).
fn move_file(src: &Path, dst: &Path) -> Result<(), QuantizeError> {
    if std::fs::rename(src, dst).is_ok() {
        return Ok(());
    }
    std::fs::copy(src, dst)?;
    std::fs::remove_file(src)?;
    Ok(())
}

impl Default for ArtifactWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl ArtifactWriter {
    /// Streaming writer with no output directory yet -- flushes shards to a
    /// scratch directory as tensors are added and moves them into the real
    /// directory at [`Self::finish`]. Keeps `new()`'s no-argument,
    /// non-fallible signature so `engine.rs` (which doesn't know the output
    /// directory at construction time) is unchanged.
    ///
    /// Creating the scratch directory can fail (disk full, no writable temp
    /// root, ...), but `new()` can't return `Result`, so that creation is
    /// deferred to the first call to [`Self::stage`] (from `add_quantized`
    /// or `add_f32`), which already returns `Result` and can propagate it
    /// via `QuantizeError::Io` instead of panicking.
    pub fn new() -> Self {
        Self {
            out_dir: Self::scratch_dir(),
            owns_scratch: true,
            scratch_ready: false,
            shard_bytes: DEFAULT_SHARD_BYTES,
            pending: HashMap::new(),
            pending_bytes: 0,
            shard_paths: Vec::new(),
            weight_map: HashMap::new(),
            meta: HashMap::new(),
        }
    }

    /// Test-only seam: build a `new()`-shaped writer (streaming into a
    /// scratch dir, `finish` moves shards out) but with an explicit,
    /// possibly-uncreatable scratch path, so the lazy-creation error path in
    /// [`Self::stage`] can be exercised without depending on the real OS
    /// temp directory ever failing to create.
    #[cfg(test)]
    fn new_with_scratch_dir(out_dir: PathBuf) -> Self {
        Self {
            out_dir,
            owns_scratch: true,
            scratch_ready: false,
            shard_bytes: DEFAULT_SHARD_BYTES,
            pending: HashMap::new(),
            pending_bytes: 0,
            shard_paths: Vec::new(),
            weight_map: HashMap::new(),
            meta: HashMap::new(),
        }
    }

    fn scratch_dir() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "vox-quantize-artifact-{}-{nanos}",
            std::process::id()
        ))
    }

    /// Streaming writer that flushes shards directly into `out_dir`, once
    /// `shard_bytes` of pending tensor data has accumulated, with no scratch
    /// dir or move at `finish`. `mixture` is accepted for symmetry with
    /// `finish`'s signature; `finish`'s own `mixture` argument is what's
    /// written to `quant-metadata.json`.
    pub fn with_shard_budget(
        out_dir: &Path,
        _mixture: &str,
        shard_bytes: u64,
    ) -> Result<Self, QuantizeError> {
        std::fs::create_dir_all(out_dir)?;
        Ok(Self {
            out_dir: out_dir.to_path_buf(),
            owns_scratch: false,
            scratch_ready: true,
            shard_bytes,
            pending: HashMap::new(),
            pending_bytes: 0,
            shard_paths: Vec::new(),
            weight_map: HashMap::new(),
            meta: HashMap::new(),
        })
    }

    pub fn add_quantized(
        &mut self,
        name: &str,
        q: &QTensor,
        orig_shape: &[usize],
    ) -> Result<(), QuantizeError> {
        let bytes = quantized_bytes(q)?;
        self.meta.insert(
            name.to_string(),
            TensorMeta {
                ggml_dtype: format!("{:?}", q.dtype()),
                orig_shape: orig_shape.to_vec(),
                orig_dtype: "F32".into(),
                quantized: true,
            },
        );
        let len = bytes.len();
        self.stage(name, bytes, candle_core::DType::U8, vec![len])
    }

    pub fn add_f32(&mut self, name: &str, t: &Tensor) -> Result<(), QuantizeError> {
        let shape = t.dims().to_vec();
        let flat = t.flatten_all()?.to_vec1::<f32>()?;
        let mut bytes = Vec::with_capacity(flat.len() * 4);
        for v in &flat {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        self.meta.insert(
            name.to_string(),
            TensorMeta {
                ggml_dtype: "F32".into(),
                orig_shape: shape.clone(),
                orig_dtype: "F32".into(),
                quantized: false,
            },
        );
        self.stage(name, bytes, candle_core::DType::F32, shape)
    }

    /// Add one tensor's bytes to the pending shard, flushing first when
    /// adding it would push the pending shard over `shard_bytes` -- mirrors
    /// `recombine_with_shard_budget`'s grouping (check-before-add, not
    /// add-then-check), so a shard never *starts* already over budget but
    /// also never splits a single tensor across two shards.
    fn stage(
        &mut self,
        name: &str,
        bytes: Vec<u8>,
        dtype: candle_core::DType,
        shape: Vec<usize>,
    ) -> Result<(), QuantizeError> {
        if !self.scratch_ready {
            std::fs::create_dir_all(&self.out_dir)?;
            self.scratch_ready = true;
        }
        let len = bytes.len() as u64;
        if !self.pending.is_empty() && self.pending_bytes + len > self.shard_bytes {
            self.flush()?;
        }
        self.pending_bytes += len;
        self.pending.insert(name.to_string(), (bytes, dtype, shape));
        Ok(())
    }

    /// Serialize every pending tensor into one shard file in `out_dir` and
    /// clear the pending buffer -- the byte budget this whole change exists
    /// to enforce.
    fn flush(&mut self) -> Result<(), QuantizeError> {
        if self.pending.is_empty() {
            return Ok(());
        }
        use candle_core::Device;
        let mut tensors: HashMap<String, Tensor> = HashMap::new();
        for (name, (bytes, dtype, shape)) in self.pending.drain() {
            let t = match dtype {
                candle_core::DType::U8 => Tensor::from_vec(bytes, shape.clone(), &Device::Cpu)?,
                candle_core::DType::F32 => {
                    let floats: Vec<f32> = bytes
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .map(|c| f32::from_le_bytes(*c))
                        .collect();
                    Tensor::from_vec(floats, shape.clone(), &Device::Cpu)?
                }
                _ => {
                    return Err(QuantizeError::Write(format!(
                        "unexpected dtype for `{name}`"
                    )));
                }
            };
            tensors.insert(name, t);
        }
        let shard_idx = self.shard_paths.len();
        let path = self
            .out_dir
            .join(format!("model-{:05}-of-tmp.safetensors", shard_idx + 1));
        candle_core::safetensors::save(&tensors, &path)?;
        for name in tensors.keys() {
            self.weight_map.insert(name.clone(), shard_idx);
        }
        self.shard_paths.push(path);
        self.pending_bytes = 0;
        Ok(())
    }

    pub fn finish(mut self, out_dir: &Path, mixture: &str) -> Result<(), QuantizeError> {
        std::fs::create_dir_all(out_dir)?;
        if !self.owns_scratch && self.out_dir != out_dir {
            return Err(QuantizeError::Write(format!(
                "ArtifactWriter::with_shard_budget was given `{}` but finish was called with `{}`",
                self.out_dir.display(),
                out_dir.display()
            )));
        }
        self.flush()?;

        let shard_count = self.shard_paths.len();
        if shard_count <= 1 {
            let final_path = out_dir.join("model.safetensors");
            match self.shard_paths.first() {
                Some(p) => move_file(p, &final_path)?,
                None => {
                    let empty: HashMap<String, Tensor> = HashMap::new();
                    candle_core::safetensors::save(&empty, &final_path)?;
                }
            }
        } else {
            let mut final_names = Vec::with_capacity(shard_count);
            for (i, path) in self.shard_paths.iter().enumerate() {
                let final_name = format!("model-{:05}-of-{:05}.safetensors", i + 1, shard_count);
                move_file(path, &out_dir.join(&final_name))?;
                final_names.push(final_name);
            }
            let mut weight_map = serde_json::Map::new();
            for (name, &shard_idx) in &self.weight_map {
                weight_map.insert(
                    name.clone(),
                    serde_json::Value::String(final_names[shard_idx].clone()),
                );
            }
            let mut total_size: u64 = 0;
            for final_name in &final_names {
                total_size += std::fs::metadata(out_dir.join(final_name))?.len();
            }
            let index = serde_json::json!({
                "metadata": { "total_size": total_size },
                "weight_map": weight_map,
            });
            let bytes = serde_json::to_vec_pretty(&index)
                .map_err(|e| QuantizeError::Write(format!("failed to serialize index: {e}")))?;
            std::fs::write(out_dir.join("model.safetensors.index.json"), bytes)?;
        }

        if self.owns_scratch {
            // Best-effort: the scratch dir should be empty now that every
            // shard has been moved out. A leftover on error isn't fatal.
            let _ = std::fs::remove_dir_all(&self.out_dir);
        }

        let meta = QuantMetadata {
            mixture: mixture.to_string(),
            writer_version: env!("CARGO_PKG_VERSION").to_string(),
            tensors: self.meta,
        };
        let json =
            serde_json::to_string_pretty(&meta).map_err(|e| QuantizeError::Write(e.to_string()))?;
        std::fs::write(out_dir.join("quant-metadata.json"), json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::quantized::{GgmlDType, QTensor};
    use candle_core::{Device, Tensor};

    #[test]
    fn writes_blocks_and_metadata_that_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let dev = Device::Cpu;
        let t = Tensor::randn(0f32, 1f32, (8, 256), &dev).unwrap();
        let q = QTensor::quantize(&t, GgmlDType::Q4K).unwrap();
        let mut artifact = ArtifactWriter::new();
        artifact.add_quantized("w", &q, &[8, 256]).unwrap();
        artifact
            .add_f32(
                "norm",
                &Tensor::ones((8,), candle_core::DType::F32, &dev).unwrap(),
            )
            .unwrap();
        artifact.finish(dir.path(), "Q4_K_M").unwrap();

        // Parse-and-assert rather than substring matching: `to_string_pretty`
        // inserts spaces after colons, so `"ggml_dtype":"Q4K"` would never
        // appear literally. Asserting against the parsed value keeps the
        // intent (metadata records dtype + mixture) without coupling to
        // serializer whitespace.
        let meta_raw = std::fs::read_to_string(dir.path().join("quant-metadata.json")).unwrap();
        let meta: serde_json::Value = serde_json::from_str(&meta_raw).unwrap();
        assert_eq!(meta["tensors"]["w"]["ggml_dtype"], "Q4K");
        assert_eq!(meta["tensors"]["w"]["quantized"], true);
        assert_eq!(meta["tensors"]["norm"]["ggml_dtype"], "F32");
        assert_eq!(meta["tensors"]["norm"]["quantized"], false);
        assert_eq!(meta["mixture"], "Q4_K_M");
        assert!(dir.path().join("model.safetensors").exists());
    }

    /// Catches: reverting `new()`'s deferred scratch-dir creation to an
    /// eager `.expect()` in `new()` itself (as it was before this test was
    /// added). `new()` can't return `Result`, so a failure to create its
    /// scratch directory has to surface from the first fallible call that
    /// touches it (`add_f32`/`add_quantized`, via `stage`) as a proper
    /// `Err`, not a panic. Forces the failure by pointing the scratch dir at
    /// a path whose parent is a regular file -- `create_dir_all` can never
    /// succeed under a file.
    #[test]
    fn scratch_dir_creation_failure_surfaces_as_err_not_panic() {
        let dir = tempfile::tempdir().unwrap();
        let blocker_file = dir.path().join("blocker");
        std::fs::write(&blocker_file, b"not a directory").unwrap();
        let unusable_scratch = blocker_file.join("scratch");

        let mut artifact = ArtifactWriter::new_with_scratch_dir(unusable_scratch);
        let t = Tensor::ones((4,), candle_core::DType::F32, &Device::Cpu).unwrap();
        let result = artifact.add_f32("w", &t);

        assert!(
            result.is_err(),
            "an unusable scratch dir must surface as Err from add_f32, not panic"
        );
    }

    /// Catches: reverting to accumulate-everything-then-save-once. The
    /// existing roundtrip test passes against both shapes because it only
    /// checks the metadata sidecar and that *a* model file exists, so it
    /// cannot see residency. Here the budget is smaller than the tensors, so
    /// an accumulating writer produces exactly one file and no index.
    #[test]
    fn the_writer_flushes_shards_instead_of_holding_the_whole_output() {
        let dir = tempfile::tempdir().unwrap();
        let dev = Device::Cpu;
        let mut artifact = ArtifactWriter::with_shard_budget(dir.path(), "Q4_K_M", 4096).unwrap();
        for name in ["w1", "w2", "w3", "w4"] {
            let t = Tensor::randn(0f32, 1f32, (8, 256), &dev).unwrap();
            let q = QTensor::quantize(&t, GgmlDType::Q4K).unwrap();
            artifact.add_quantized(name, &q, &[8, 256]).unwrap();
        }
        artifact.finish(dir.path(), "Q4_K_M").unwrap();

        let shards = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".safetensors"))
            .count();
        assert!(
            shards >= 2,
            "output above the shard budget must be flushed in shards, got {shards}"
        );

        let index: serde_json::Value = serde_json::from_reader(
            std::fs::File::open(dir.path().join("model.safetensors.index.json")).unwrap(),
        )
        .unwrap();
        let map = index["weight_map"].as_object().expect("weight_map");
        for name in ["w1", "w2", "w3", "w4"] {
            assert!(map.contains_key(name), "flushing dropped tensor `{name}`");
        }
    }
}
