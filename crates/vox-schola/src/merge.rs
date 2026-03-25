//! `vox-schola merge` — merge a QLoRA adapter into base weights.

use anyhow::Context;

use crate::cli::{Args, Cmd};

pub fn run(args: Args) -> anyhow::Result<()> {
    let Cmd::Merge { base_shard, adapter, meta, output } = args.cmd else {
        unreachable!()
    };
    if base_shard.is_empty() {
        anyhow::bail!("pass at least one --base-shard safetensors path");
    }
    for p in &base_shard {
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
        anyhow::bail!("{}", vox_mens::MERGE_QLORA_REJECTS_BURN_BIN);
    }
    if !meta.is_file() {
        anyhow::bail!("meta JSON not found: {}", meta.display());
    }
    let raw = std::fs::read_to_string(&meta)
        .with_context(|| format!("read {}", meta.display()))?;
    let meta_v2 = if let Ok(m) =
        serde_json::from_str::<vox_mens::tensor::candle_qlora_merge::QloraAdapterMetaV2>(&raw)
    {
        m
    } else {
        let v3: vox_mens::tensor::adapter_schema_v3::PopuliAdapterManifestV3 =
            serde_json::from_str(&raw)
                .with_context(|| format!("parse meta as v2 or v3 {}", meta.display()))?;
        vox_mens::tensor::adapter_schema_v3::to_qlora_meta_v2_for_merge(&v3)
            .with_context(|| "adapter manifest v3 → merge bridge")?
    };
    vox_mens::tensor::candle_qlora_merge::merge_qlora_v2_into_base_subset(
        &base_shard,
        &adapter,
        &meta_v2,
        &output,
    )?;
    eprintln!("Wrote merged tensors (subset) to {}", output.display());
    Ok(())
}
