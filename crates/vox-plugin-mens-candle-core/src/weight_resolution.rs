//! Shared base-shard / LoRA-delta resolution for `InferenceEngine::load`.
//!
//! Before this module existed, both `vox-plugin-mens-candle-metal` and
//! `vox-plugin-mens-candle-cuda` hard-required an adapter
//! (`candle_qlora_adapter.safetensors` + `adapter_manifest.json`) to be
//! present in the model directory, with no way to serve a bare base model —
//! so nothing could produce the "baseline" side of a pass@k/BFCL
//! candidate-vs-base comparison. `resolve_weights` adds a second supported
//! shape: a directory with **neither** adapter file is a base-only snapshot,
//! and `model_dir` itself is scanned for the base `*.safetensors` shards
//! (the fine-tuned run case still resolves the base directory from the
//! adapter manifest's `base_model` field, unchanged).
//!
//! A directory with exactly one of the two adapter files present is
//! inconsistent on-disk state and stays a hard error — never a guess.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use candle_core::Device;

use crate::adapter_schema_v3::PopuliAdapterManifestV3;
use crate::merge::LoraFactors;

/// Base weight shards plus any LoRA deltas to fold on top of them.
///
/// `lora_factors` is empty for a base-only load — see module docs.
pub struct ResolvedWeights {
    pub base_shards: Vec<PathBuf>,
    pub lora_factors: HashMap<String, LoraFactors>,
}

/// True when the trained adapter's LoRA deltas still have to be folded in at
/// load time.
///
/// `merged.safetensors` already holds `W + BA·α/r` under the *base* key
/// names, and the weight-source search order puts it ahead of the base
/// shards — so folding the delta again on top of a merged run would apply it
/// twice. Exactly one of the two paths may run.
#[must_use]
pub fn adapter_deltas_must_be_folded(model_dir: &Path) -> bool {
    !model_dir.join("merged.safetensors").is_file()
}

/// Every `*.safetensors` file in `dir` whose name contains `"model"` — the
/// shard-naming convention HF snapshots use (`model-00001-of-00002.safetensors`,
/// or a single `model.safetensors`).
fn glob_model_shards(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut shards = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let p = entry?.path();
        if p.extension().map(|e| e == "safetensors").unwrap_or(false)
            && p.file_name().unwrap().to_string_lossy().contains("model")
        {
            shards.push(p);
        }
    }
    Ok(shards)
}

/// Resolve where `InferenceEngine::load` should read base weights from, and
/// what (if any) trained LoRA delta to fold on top of them.
///
/// - **Adapter present** (`adapter_path` and `meta_path` both exist): base
///   shards come from the manifest's `base_model` directory, and LoRA deltas
///   are read from the adapter file unless `merged.safetensors` already
///   folded them in.
/// - **Base-only** (neither file exists): `model_dir` itself is the base
///   snapshot — its own `*.safetensors` shards are the base weights, and no
///   delta is folded.
/// - **Inconsistent** (exactly one of the two exists): a hard error. Guessing
///   which half of a partially-written run to trust is how a fine-tune
///   silently gets served as its own base model.
pub fn resolve_weights(
    model_dir: &Path,
    adapter_path: &Path,
    meta_path: &Path,
    device: &Device,
) -> Result<ResolvedWeights> {
    match (adapter_path.is_file(), meta_path.is_file()) {
        (false, false) => Ok(ResolvedWeights {
            base_shards: glob_model_shards(model_dir)?,
            lora_factors: HashMap::new(),
        }),
        (true, true) => {
            let meta_raw = std::fs::read_to_string(meta_path)
                .map_err(|e| anyhow::anyhow!("read manifest {}: {e}", meta_path.display()))?;
            let meta: PopuliAdapterManifestV3 = serde_json::from_str(&meta_raw)?;

            let base_shards = match &meta.base_model {
                Some(base) if Path::new(base).is_dir() => glob_model_shards(Path::new(base))?,
                Some(base) => anyhow::bail!(
                    "base_model '{base}' is not a local directory. Hub download is not \
                     supported in the plugin; pre-download the model to a local path and \
                     update adapter_manifest.json."
                ),
                None => anyhow::bail!(
                    "adapter manifest missing `base_model` reference. Cannot load frozen weights."
                ),
            };

            let lora_factors = if adapter_deltas_must_be_folded(model_dir) {
                crate::merge::lora_factors_by_base_key(adapter_path, &meta, device)?
            } else {
                HashMap::new()
            };

            Ok(ResolvedWeights {
                base_shards,
                lora_factors,
            })
        }
        (true, false) => anyhow::bail!(
            "{} has a trained adapter file but no adapter_manifest.json — inconsistent \
             on-disk state, refusing to guess whether to serve the adapter or the base model",
            model_dir.display()
        ),
        (false, true) => anyhow::bail!(
            "{} has an adapter_manifest.json but no candle_qlora_adapter.safetensors — \
             inconsistent on-disk state, refusing to guess whether to serve the adapter or \
             the base model",
            model_dir.display()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The failing-first-test case (TDD): a run directory with NEITHER
    /// adapter file must resolve as base-only — its own shard, no error, no
    /// LoRA factors — rather than the old hard bail. This is the load-path
    /// fix's core regression guard.
    #[test]
    fn a_directory_with_no_adapter_resolves_as_base_only() {
        let d = tempfile::tempdir().unwrap();
        let shard = d.path().join("model-00001-of-00001.safetensors");
        std::fs::write(&shard, b"").unwrap();
        // A decoy that must NOT be picked up as a shard.
        std::fs::write(d.path().join("tokenizer.json"), b"{}").unwrap();

        let resolved = resolve_weights(
            d.path(),
            &d.path().join("candle_qlora_adapter.safetensors"),
            &d.path().join("adapter_manifest.json"),
            &Device::Cpu,
        )
        .expect("a base-only directory must load, not hard-error");

        assert_eq!(resolved.base_shards, vec![shard]);
        assert!(
            resolved.lora_factors.is_empty(),
            "a base-only load must never fold a delta"
        );
    }

    #[test]
    fn adapter_without_manifest_is_a_hard_error() {
        let d = tempfile::tempdir().unwrap();
        let adapter = d.path().join("candle_qlora_adapter.safetensors");
        std::fs::write(&adapter, b"").unwrap();
        let meta_path = d.path().join("adapter_manifest.json");

        let result = resolve_weights(d.path(), &adapter, &meta_path, &Device::Cpu);
        match result {
            Ok(_) => panic!("adapter without manifest must not silently pick a mode"),
            Err(e) => assert!(e.to_string().contains("inconsistent"), "{e}"),
        }
    }

    #[test]
    fn manifest_without_adapter_is_a_hard_error() {
        let d = tempfile::tempdir().unwrap();
        let adapter = d.path().join("candle_qlora_adapter.safetensors");
        let meta_path = d.path().join("adapter_manifest.json");
        std::fs::write(&meta_path, "{}").unwrap();

        let result = resolve_weights(d.path(), &adapter, &meta_path, &Device::Cpu);
        match result {
            Ok(_) => panic!("manifest without adapter must not silently pick a mode"),
            Err(e) => assert!(e.to_string().contains("inconsistent"), "{e}"),
        }
    }

    #[test]
    fn adapter_deltas_must_be_folded_is_false_once_merged() {
        let d = tempfile::tempdir().unwrap();
        assert!(adapter_deltas_must_be_folded(d.path()));
        std::fs::write(d.path().join("merged.safetensors"), b"").unwrap();
        assert!(!adapter_deltas_must_be_folded(d.path()));
    }
}
