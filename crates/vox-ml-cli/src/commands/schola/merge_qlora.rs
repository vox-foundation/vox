//! `vox schola merge-qlora` — fold QLoRA adapter tensors into base f32 weights.
//!
//! Dispatches to whichever `MlBackend` plugin matches this host's capabilities
//! (`mens-candle-cuda` on an NVIDIA host, `mens-candle-metal` on Apple Silicon)
//! via `MlBackend::merge_adapter`.
//! The adapter directory must contain `adapter_manifest.json` (v3) written by training.

use std::path::PathBuf;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use vox_bounded_fs::read_utf8_path_capped;
use vox_populi::mens::MERGE_QLORA_REJECTS_BURN_BIN;

// Candidate plugins for the `MlBackend` extension point, mirroring
// catalog.toml's `mens-candle-cuda`/`mens-candle-metal` entries (id +
// requires-tag). vox-plugin-host is deliberately dependency-free and cannot
// read catalog.toml itself, so this is the caller-supplied SSOT-mirror
// `resolve_extension_point` needs. Both plugins implement `merge_adapter`
// (unlike QLoRA *training*, which has no Metal backend yet — see run_train.rs).
// Shared with `commands::mens::eval_local`, which dispatches the same
// `MlBackend` extension point for inference — see that file's use of this
// constant. `crates/vox-plugin-catalog/tests/catalog_validation.rs` pins the
// two `requires-tag` literals on the catalog side so this mirror can't drift
// silently.
// vox:defactored-from vox-plugin-catalog 2026-09-05
pub(crate) const ML_BACKEND_CANDIDATES: &[vox_plugin_host::ExtensionCandidate] = &[
    vox_plugin_host::ExtensionCandidate {
        plugin_id: "mens-candle-cuda",
        requires_tag: Some("nvidia-gpu"),
    },
    vox_plugin_host::ExtensionCandidate {
        plugin_id: "mens-candle-metal",
        requires_tag: Some("apple-silicon"),
    },
];

// ---------------------------------------------------------------------------
// Inline serde-only schema types (no candle deps).
// These match the on-disk JSON layout produced by vox-plugin-mens-candle-cuda.
// ---------------------------------------------------------------------------

/// On-disk adapter bundle descriptor v3 (current, canonical).
///
/// Used here only for structural validation before dispatching to the plugin.
/// The plugin owns the authoritative schema; this struct must accept both the
/// flat legacy layout (`base_quant` at top level) and the canonical nested layout
/// (`quant: { base_quant, double_quant }`) without failing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopuliAdapterManifestV3 {
    pub format: String,
    pub version: u32,
    // Flattened from AdapterMethodFields in the canonical plugin schema.
    #[serde(default)]
    pub adapter_method: String,
    // Legacy flat layout — empty when the canonical nested `quant` field is used instead.
    #[serde(default)]
    pub base_quant: String,
    // Canonical nested quant layout (plugin schema).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quant: Option<serde_json::Value>,
    pub base_key_map: std::collections::HashMap<String, String>,
    pub layer_order: Vec<String>,
    pub vocab: usize,
    pub d_model: usize,
    pub rank: usize,
    pub alpha: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<serde_json::Value>,
}

pub fn run_merge_qlora(
    base_shards: Vec<PathBuf>,
    adapter: PathBuf,
    meta: PathBuf,
    output: PathBuf,
    quantize: Option<String>,
    keep_merged: bool,
) -> anyhow::Result<()> {
    if base_shards.is_empty() {
        anyhow::bail!("pass at least one `--base-shard` safetensors path");
    }
    for p in &base_shards {
        if !p.is_file() {
            anyhow::bail!("base shard not found: {}", p.display());
        }
    }
    if !adapter.is_file() {
        anyhow::bail!("adapter not found: {}", adapter.display());
    }
    if adapter
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("bin"))
    {
        anyhow::bail!("{MERGE_QLORA_REJECTS_BURN_BIN}");
    }
    if !meta.is_file() {
        anyhow::bail!("meta JSON not found: {}", meta.display());
    }

    // Parse v3 manifest (validate it's readable before dispatching to plugin).
    let raw = read_utf8_path_capped(&meta).with_context(|| format!("read {}", meta.display()))?;
    let manifest: PopuliAdapterManifestV3 = serde_json::from_str(&raw)
        .with_context(|| format!("parse adapter manifest v3 from {}", meta.display()))?;

    // Ensure adapter_manifest.json exists next to the adapter .safetensors so
    // the plugin can find it. If the user pointed --meta at a file in the adapter
    // dir with a different name, copy it as adapter_manifest.json.
    let adapter_dir = adapter
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let canonical_manifest = adapter_dir.join("adapter_manifest.json");
    if meta.canonicalize().ok() != canonical_manifest.canonicalize().ok() {
        std::fs::write(&canonical_manifest, &raw)
            .with_context(|| format!("write {}", canonical_manifest.display()))?;
    }

    // Use the parent directory of the first shard as the base model directory.
    let base_dir = base_shards[0]
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    // Dispatch to whichever MlBackend plugin matches this host's capabilities
    // (CUDA on an NVIDIA host, Metal on Apple Silicon), not a hardcoded id —
    // see vox_plugin_host::resolve_extension_point.
    let result = (|| -> anyhow::Result<()> {
        let plugin_id = vox_plugin_host::resolve_extension_point(
            "MlBackend",
            ML_BACKEND_CANDIDATES,
            &vox_plugin_host::probe(),
        )
        .context("no ML backend plugin matches this host's capabilities")?;
        let plugin = vox_plugin_host::cached_code_plugin(plugin_id).with_context(|| {
            format!("{plugin_id} plugin not found — install vox-plugin-{plugin_id}")
        })?;
        let backend = plugin
            .plugin
            .as_ml_backend()
            .into_option()
            .ok_or_else(|| anyhow::anyhow!("{plugin_id} plugin does not provide MlBackend"))?;
        backend
            .merge_adapter(
                base_dir.to_string_lossy().as_ref().into(),
                adapter.to_string_lossy().as_ref().into(),
                output.to_string_lossy().as_ref().into(),
            )
            .into_result()
            .map_err(|e| anyhow::anyhow!("merge_adapter: {e}"))
    })();

    result?;

    eprintln!("Wrote merged tensors (subset) to {}", output.display());
    let base = manifest
        .base_model
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("unknown");
    let handoff = vox_populi::mens::tensor::external_serving_handoff::ExternalServingHandoffV1::merged_qlora_subset(
        &output,
        base,
        None,
    );
    let handoff_dir = output
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    if let Err(e) =
        vox_populi::mens::tensor::external_serving_handoff::write_handoff(&handoff_dir, &handoff)
    {
        tracing::warn!("external_serving_handoff_v1.json not written: {e}");
    } else {
        eprintln!(
            "Wrote {}",
            handoff_dir
                .join("external_serving_handoff_v1.json")
                .display()
        );
    }

    // Optional: recombine the merged subset over the full base weights and
    // quantize the result. `output` is the merged-subset FILE; `base_dir` is the
    // directory holding the base model (config.json + shards), derived above as
    // the parent of the first `--base-shard`.
    if let Some(mixture_str) = quantize.as_deref() {
        let mixture = crate::commands::quantize::parse_mixture(mixture_str)?;
        let out_parent = output
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        // recombine writes <recombined>/model.safetensors (or sharded
        // model-*.safetensors + index, for large bases) + copies base config.json
        let recombined = out_parent.join("recombined_full");
        // Clear any stale recombined dir so a prior sharded run's
        // model.safetensors.index.json can't mislead the reader.
        let _ = std::fs::remove_dir_all(&recombined);
        vox_quantize::recombine::recombine(&base_dir, &output, &recombined)
            .with_context(|| format!("recombine over base {}", base_dir.display()))?;
        let q_out = out_parent.join("quantized");
        // Clear any stale quantized dir from a prior run before reuse.
        let _ = std::fs::remove_dir_all(&q_out);
        let report = vox_quantize::quantize(&vox_quantize::QuantizeRequest {
            input_dir: recombined.clone(),
            output_dir: q_out.clone(),
            mixture,
            verify: true,
            device: vox_quantize::DevicePref::Auto, // GPU when available
        })
        .with_context(|| "quantize recombined model")?;
        println!(
            "Quantized merged model -> {} ({:.2}x)",
            q_out.display(),
            report.compression_ratio
        );
        finish_recombined(&recombined, &base_dir, keep_merged)?;
    }

    Ok(())
}

/// Modelfile for a merged checkpoint. `FROM .` imports the SafeTensors
/// directory itself, so `ollama create` runs its own conversion.
///
/// Applies only to base architectures Ollama's converter supports (llama,
/// gemma2). Qwen3 bases are rejected by `ollama create` with
/// `unsupported architecture "Qwen3ForCausalLM"` and go through the
/// llama.cpp lane instead. This renderer does not gate on architecture:
/// Ollama's enumeration is upstream and a hardcoded copy would rot.
///
/// No `ADAPTER` directive: Ollama's safetensors adapter converter covers
/// llama and gemma2 bases only — merged weights are the supported route.
#[must_use]
pub fn render_ollama_modelfile(num_ctx: usize, license: &str) -> String {
    let mut s = String::from("FROM .\n");
    s.push_str(&format!("PARAMETER num_ctx {num_ctx}\n"));
    // Must match training's prompt_format "qwen_chatml_im_start"
    // (vox_populi::mens::tensor::external_serving_handoff); a mismatch serves
    // the model off-distribution.
    s.push_str(
        "TEMPLATE \"\"\"{{ if .System }}<|im_start|>system\n{{ .System }}<|im_end|>\n{{ end }}\
         <|im_start|>user\n{{ .Prompt }}<|im_end|>\n<|im_start|>assistant\n\"\"\"\n",
    );
    s.push_str(&format!("LICENSE \"\"\"{license}\"\"\"\n"));
    s
}

/// Post-quantize disposition of `recombined_full/`. With `keep_merged`, copy
/// the tokenizer files Ollama's converter needs and write a Modelfile beside
/// the weights; without it, delete the directory as before.
///
/// Extracted from the body of `run_merge_qlora` so the keep-vs-delete
/// decision is testable without a real base checkpoint and adapter.
fn finish_recombined(
    recombined: &std::path::Path,
    base_dir: &std::path::Path,
    keep_merged: bool,
) -> anyhow::Result<()> {
    if !keep_merged {
        let _ = std::fs::remove_dir_all(recombined);
        return Ok(());
    }
    for f in [
        "tokenizer.json",
        "tokenizer_config.json",
        "generation_config.json",
    ] {
        let src = base_dir.join(f);
        if src.exists() {
            std::fs::copy(&src, recombined.join(f))
                .with_context(|| format!("copy {f} into recombined_full"))?;
        }
    }
    std::fs::write(
        recombined.join("Modelfile"),
        render_ollama_modelfile(8192, "apache-2.0"),
    )
    .context("write Modelfile")?;
    println!(
        "Merged model kept at {} — for a llama/gemma2 base, publish with:\n  cd {} && ollama create -q q4_K_M <name> -f Modelfile\nFor a Qwen3 base, `ollama create` rejects the architecture; use `vox mens merge-qlora --gguf-out <file> --llama-cpp <dir>` instead.",
        recombined.display(),
        recombined.display()
    );
    Ok(())
}

#[cfg(test)]
mod ollama_publish_tests {
    use super::*;

    /// Catches: emitting `FROM model.safetensors` or `FROM <abs path>`.
    /// `ollama create` converts a *directory*; a file path is rejected, and
    /// an absolute path breaks the moment the run dir is copied anywhere.
    #[test]
    fn modelfile_imports_the_directory_relatively() {
        let m = render_ollama_modelfile(8192, "apache-2.0");
        assert!(m.starts_with("FROM .\n"), "got: {m}");
        assert!(m.contains("PARAMETER num_ctx 8192"), "got: {m}");
    }

    /// Catches: dropping the TEMPLATE. MENS trains with prompt_format
    /// "qwen_chatml_im_start" (external_serving_handoff.rs:40); a Modelfile
    /// without the matching ChatML template serves the model
    /// off-distribution, which looks like a bad checkpoint rather than a
    /// bad Modelfile.
    #[test]
    fn modelfile_template_matches_the_training_prompt_format() {
        let m = render_ollama_modelfile(8192, "apache-2.0");
        assert!(m.contains("<|im_start|>"), "got: {m}");
        assert!(m.contains("<|im_end|>"), "got: {m}");
    }

    /// Catches: emitting an ADAPTER directive. Ollama's safetensors adapter
    /// converter handles base architectures llama and gemma2 only, and it
    /// would fail on a LoRA adapter regardless of the merged-weight route
    /// this Modelfile describes.
    #[test]
    fn modelfile_emits_no_adapter_directive() {
        let m = render_ollama_modelfile(8192, "apache-2.0");
        assert!(!m.contains("ADAPTER"), "got: {m}");
    }

    /// Catches: reverting `if keep_merged { .. } else { remove_dir_all }` at
    /// :219 back to an unconditional delete -- i.e. undoing this entire
    /// feature. The three renderer tests above are all pure string checks
    /// and stay green against that revert, so without this one the task has
    /// no regression guard at all.
    ///
    /// Drives the gated block directly rather than through run_merge_qlora,
    /// which needs a real base checkpoint and an adapter.
    #[test]
    fn keep_merged_leaves_the_recombined_directory_and_its_modelfile_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let recombined = dir.path().join("recombined_full");
        std::fs::create_dir_all(&recombined).unwrap();
        std::fs::write(recombined.join("model.safetensors"), b"stub").unwrap();

        finish_recombined(&recombined, dir.path(), true).unwrap();
        assert!(
            recombined.join("model.safetensors").is_file(),
            "--keep-merged must keep the merged artifact; it was deleted"
        );
        assert!(
            recombined.join("Modelfile").is_file(),
            "--keep-merged must write a Modelfile next to the weights"
        );

        finish_recombined(&recombined, dir.path(), false).unwrap();
        assert!(
            !recombined.exists(),
            "without --keep-merged the recombined dir is still deleted"
        );
    }
}
