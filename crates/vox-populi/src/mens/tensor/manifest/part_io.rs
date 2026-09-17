pub fn load_manifest(run_dir: &Path) -> anyhow::Result<Option<TrainingManifest>> {
    let p = run_dir.join("training_manifest.json");
    if !p.is_file() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&p)?;
    let m: TrainingManifest = serde_json::from_str(&raw)?;
    if m.manifest_schema_version < TRAINING_MANIFEST_SCHEMA_VERSION {
        tracing::debug!(
            path = %p.display(),
            file_schema = m.manifest_schema_version,
            current = TRAINING_MANIFEST_SCHEMA_VERSION,
            "older training manifest (reader tolerant; fields default-filled)"
        );
    }
    Ok(Some(m))
}

/// Ensure checkpoint file exists and is non-trivial; cross-check manifest when present.
pub fn validate_checkpoint_manifest(
    checkpoint: &Path,
    run_dir: &Path,
    params: ValidateParams,
) -> anyhow::Result<()> {
    // A QLoRA checkpoint is a *run directory* (adapter + manifest + tokenizer),
    // which is what `vox mens serve --model` is given and what the serve dispatch
    // arm itself probes with `model.join(...)`. Sizing the directory's own inode
    // (192–224 bytes on macOS) rejected every such run as "too small"; size the
    // weights inside it instead.
    let sized = if checkpoint.is_dir() {
        ["candle_qlora_adapter.safetensors", "merged.safetensors"]
            .iter()
            .map(|f| checkpoint.join(f))
            .find(|p| p.is_file())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "checkpoint directory {} holds no candle_qlora_adapter.safetensors or merged.safetensors",
                    checkpoint.display()
                )
            })?
    } else {
        checkpoint.to_path_buf()
    };
    let meta = std::fs::metadata(&sized)?;
    if meta.len() < 256 {
        anyhow::bail!(
            "checkpoint {} too small ({} bytes)",
            sized.display(),
            meta.len()
        );
    }
    if let Ok(Some(m)) = load_manifest(run_dir) {
        if m.vocab_size != params.vocab_size {
            anyhow::bail!(
                "manifest vocab_size {} != expected {}",
                m.vocab_size,
                params.vocab_size
            );
        }
        if m.d_model != params.d_model
            || m.n_heads != params.n_heads
            || m.n_layers != params.n_layers
        {
            anyhow::bail!(
                "manifest arch mismatch vs checkpoint validation params (d_model/n_heads/n_layers)"
            );
        }
    }
    let _ = params.kind;
    Ok(())
}

#[cfg(test)]
mod part_io_tests {
    use super::validate_checkpoint_manifest;
    use crate::mens::tensor::manifest::{ArchParams, CheckpointKind};

    /// A QLoRA run is a directory, and `vox mens serve --model <run-dir>` passes
    /// it here. Sizing the directory inode itself rejected every real adapter run
    /// with "checkpoint ... too small (224 bytes)" — the weights are inside it.
    #[test]
    fn a_run_directory_is_sized_by_its_adapter_not_its_inode() {
        let dir = std::env::temp_dir().join(format!("vox-ckpt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let params = ArchParams::default().to_validate_params(Some(CheckpointKind::Lora));

        // No weights in the directory at all: still an error, with a message
        // that names what is missing rather than an inode byte count.
        let err = validate_checkpoint_manifest(&dir, &dir, params.clone()).unwrap_err();
        assert!(
            err.to_string().contains("candle_qlora_adapter.safetensors"),
            "got: {err}"
        );

        std::fs::write(dir.join("candle_qlora_adapter.safetensors"), vec![0u8; 512]).unwrap();
        validate_checkpoint_manifest(&dir, &dir, params.clone())
            .expect("a run directory holding a real adapter must validate");

        // A too-small adapter is still rejected.
        std::fs::write(dir.join("candle_qlora_adapter.safetensors"), vec![0u8; 8]).unwrap();
        assert!(validate_checkpoint_manifest(&dir, &dir, params).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
