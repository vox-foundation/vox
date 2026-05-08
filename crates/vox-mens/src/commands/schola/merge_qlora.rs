//! `vox schola merge-qlora` — fold QLoRA adapter tensors into base f32 weights.
//!
//! Dispatches to the `mens-candle-cuda` plugin via `MlBackend::merge_adapter`.
//! v2/v3 metadata parsing is done here (serde-only, no candle deps) so that the
//! plugin's `merge_qlora_adapter` entry point can find an `adapter_meta_v2.json`
//! next to the adapter safetensors file.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use vox_bounded_fs::read_utf8_path_capped;
use vox_populi::mens::MERGE_QLORA_REJECTS_BURN_BIN;

// ---------------------------------------------------------------------------
// Inline serde-only schema types (no candle deps).
// These match the on-disk JSON layouts produced by vox-plugin-mens-candle-cuda.
// ---------------------------------------------------------------------------

/// On-disk sidecar format v2 (legacy).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct QloraAdapterMetaV2 {
    pub format: String,
    pub version: u32,
    pub embed_key: String,
    pub vocab: usize,
    pub d_model: usize,
    pub rank: usize,
    pub alpha: usize,
    pub layer_order: Vec<String>,
    pub base_key_map: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_model: Option<String>,
}

impl QloraAdapterMetaV2 {
    const FORMAT: &'static str = "vox_mens_qlora_lora_only_v2";
    const VERSION: u32 = 2;
}

/// On-disk adapter bundle descriptor v3 (current).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopuliAdapterManifestV3 {
    pub format: String,
    pub version: u32,
    pub adapter_method: String,
    pub base_quant: String,
    #[serde(default = "default_true")]
    pub double_quant: bool,
    pub base_key_map: HashMap<String, String>,
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

fn default_true() -> bool {
    true
}

fn v3_to_v2(v3: &PopuliAdapterManifestV3) -> anyhow::Result<QloraAdapterMetaV2> {
    let embed_key = v3.base_key_map.get("lm_head").cloned().unwrap_or_default();
    if embed_key.is_empty() {
        anyhow::bail!("adapter manifest v3: base_key_map missing `lm_head` → HF embed key");
    }
    Ok(QloraAdapterMetaV2 {
        format: QloraAdapterMetaV2::FORMAT.to_string(),
        version: QloraAdapterMetaV2::VERSION,
        embed_key,
        vocab: v3.vocab,
        d_model: v3.d_model,
        rank: v3.rank,
        alpha: v3.alpha,
        layer_order: v3.layer_order.clone(),
        base_key_map: v3.base_key_map.clone(),
        base_model: v3.base_model.clone(),
    })
}

pub fn run_merge_qlora(
    base_shards: Vec<PathBuf>,
    adapter: PathBuf,
    meta: PathBuf,
    output: PathBuf,
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

    // Parse meta (v2 or v3) and normalise to v2.
    let raw = read_utf8_path_capped(&meta).with_context(|| format!("read {}", meta.display()))?;
    let meta_v2: QloraAdapterMetaV2 =
        if let Ok(m) = serde_json::from_str::<QloraAdapterMetaV2>(&raw) {
            m
        } else {
            let v3: PopuliAdapterManifestV3 = serde_json::from_str(&raw)
                .with_context(|| format!("parse meta as v2 or v3 {}", meta.display()))?;
            v3_to_v2(&v3).with_context(|| "adapter manifest v3 → v2 bridge")?
        };

    // The plugin's merge_qlora_adapter reads `adapter_meta_v2.json` (or `meta.json`)
    // from the adapter's parent directory and scans the base_path directory for shards.
    // We write the v2 meta next to the adapter so the plugin can find it.
    let adapter_dir = adapter
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let sidecar_path = adapter_dir.join("adapter_meta_v2.json");
    let sidecar_json =
        serde_json::to_string_pretty(&meta_v2).context("serialize adapter meta v2")?;
    std::fs::write(&sidecar_path, &sidecar_json)
        .with_context(|| format!("write {}", sidecar_path.display()))?;

    // Use the parent directory of the first shard as the base model directory.
    let base_dir = base_shards[0]
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    // Dispatch to the plugin.
    let result = (|| -> anyhow::Result<()> {
        let plugin = vox_plugin_host::cached_code_plugin("mens-candle-cuda")
            .context("mens-candle-cuda plugin not found — install vox-plugin-mens-candle-cuda")?;
        let backend = plugin
            .plugin
            .as_ml_backend()
            .into_option()
            .ok_or_else(|| anyhow::anyhow!("mens-candle-cuda plugin does not provide MlBackend"))?;
        backend
            .merge_adapter(
                base_dir.to_string_lossy().as_ref().into(),
                adapter.to_string_lossy().as_ref().into(),
                output.to_string_lossy().as_ref().into(),
            )
            .into_result()
            .map_err(|e| anyhow::anyhow!("merge_adapter: {e}"))
    })();

    // Clean up the sidecar we wrote regardless of outcome.
    let _ = std::fs::remove_file(&sidecar_path);

    result?;

    eprintln!("Wrote merged tensors (subset) to {}", output.display());
    let base = meta_v2
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
    Ok(())
}
