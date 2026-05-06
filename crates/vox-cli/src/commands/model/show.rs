//! Show one model from the on-disk registry cache.

use anyhow::{anyhow, Context};
use clap::Parser;
use vox_orchestrator::models::ModelRegistry;

#[derive(Parser)]
pub struct ShowArgs {
    /// Registry model id (e.g. `openai/gpt-4o`).
    pub id: String,
}

pub async fn run(args: ShowArgs) -> anyhow::Result<()> {
    let reg = ModelRegistry::from_cache();
    let m = reg
        .get(&args.id)
        .ok_or_else(|| anyhow!("model id not found in cache: {}", args.id))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&m).context("serialize ModelSpec")?
    );
    Ok(())
}
