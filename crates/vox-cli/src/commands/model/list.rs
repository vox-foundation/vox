//! List models from the on-disk registry cache (`model-catalog.v1.json`).

use anyhow::anyhow;
use clap::Parser;
use vox_orchestrator::models::{Capability, ModelRegistry};

#[derive(Parser)]
pub struct ListArgs {
    /// Filter by a routing [`Capability`] name (e.g. `supports_tool_use`, `supports_reasoning`).
    #[arg(long)]
    pub capability: Option<String>,
    /// Maximum rows to print.
    #[arg(long, default_value_t = 100)]
    pub limit: usize,
}

pub async fn run(args: ListArgs) -> anyhow::Result<()> {
    let cap = match args.capability.as_deref() {
        Some(s) => Some(parse_capability(s)?),
        None => None,
    };
    let reg = ModelRegistry::from_cache();
    let mut models = reg.list_models();
    if let Some(c) = cap {
        models.retain(|m| m.capabilities.supports(c));
    }
    models.sort_by(|a, b| a.id.cmp(&b.id));
    for m in models.into_iter().take(args.limit) {
        println!("{}", m.id);
    }
    Ok(())
}

fn parse_capability(raw: &str) -> anyhow::Result<Capability> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "supports_tool_use" | "tool_use" | "tools" => Ok(Capability::SupportsToolUse),
        "supports_reasoning" | "reasoning" => Ok(Capability::SupportsReasoning),
        "supports_web_search" | "web_search" => Ok(Capability::SupportsWebSearch),
        "supports_image_generation" | "image_generation" => Ok(Capability::SupportsImageGeneration),
        "supports_vision" | "vision" => Ok(Capability::SupportsVision),
        "supports_json" | "json" => Ok(Capability::SupportsJson),
        "supports_audio_input" | "audio_input" => Ok(Capability::SupportsAudioInput),
        "supports_audio_output" | "audio_output" => Ok(Capability::SupportsAudioOutput),
        other => Err(anyhow!(
            "unknown capability {other:?}; try supports_tool_use, supports_reasoning, …"
        )),
    }
}
