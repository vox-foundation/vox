use clap::Parser;
use owo_colors::OwoColorize;
use vox_orchestrator::catalog::{ModelCatalog, OpenRouterCatalog};

/// Refresh the model catalog from all sources.
#[derive(Parser)]
pub struct DiscoverArgs {
    /// Force refresh even if cache is warm.
    #[arg(long)]
    pub force: bool,
}

pub async fn run(_args: DiscoverArgs) -> anyhow::Result<()> {
    println!(
        "{} Discovering models...",
        " INFO ".on_blue().white().bold()
    );

    let mut all_models = Vec::new();

    // 1. OpenRouter
    let or_catalog = OpenRouterCatalog::new();
    if let Ok(models) = or_catalog.refresh().await {
        println!(
            "  ✅ Discovered {} models from OpenRouter",
            models.len().green()
        );
        all_models.extend(models);
    }

    // 2. Ollama (Local)
    let ollama_url = vox_config::local_ollama_populi_base_url();
    let ollama_catalog = vox_orchestrator::catalog::OllamaCatalog::new(ollama_url);
    if let Ok(models) = ollama_catalog.refresh().await {
        println!(
            "  ✅ Discovered {} models from local Ollama",
            models.len().green()
        );
        all_models.extend(models);
    }

    // 3. Hugging Face
    let hf_catalog = vox_orchestrator::catalog::HuggingFaceCatalog::new();
    if let Ok(models) = hf_catalog.refresh().await {
        println!(
            "  ✅ Discovered {} models from Hugging Face",
            models.len().green()
        );
        all_models.extend(models);
    }

    // 4. Populi Mesh
    let mesh_catalog = vox_orchestrator::catalog::PopuliMeshCatalog::new();
    if let Ok(models) = mesh_catalog.refresh().await {
        println!(
            "  ✅ Discovered {} models from Populi Mesh",
            models.len().green()
        );
        all_models.extend(models);
    }

    println!(
        "\n✅ Total discovered models: {}",
        all_models.len().green().bold()
    );

    // In a full implementation, we'd persist these to ~/.vox/cache/model-catalog.v1.json

    Ok(())
}
