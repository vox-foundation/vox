//! `vox populi models` — Local Model Registry.

use anyhow::Result;
use std::path::{Path, PathBuf};
use owo_colors::OwoColorize;

/// Prints all trained Populi models found in the run directories.
pub fn run_models(_verbose: bool) -> Result<()> {
    let runs_dir = PathBuf::from("populi/runs");
    if !runs_dir.exists() {
        eprintln!("{} No models found ({} does not exist)", "ℹ".blue(), runs_dir.display());
        return Ok(());
    }

    println!("{} Local Populi Model Registry", "📦".cyan());
    println!("════════════════════════════════════════════════");

    let mut found = 0;
    for entry in std::fs::read_dir(&runs_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if !path.is_dir() {
            continue;
        }

        let manifest_path = path.join("training_manifest.json");
        if !manifest_path.exists() {
            continue;
        }

        if let Ok(manifest_raw) = std::fs::read_to_string(&manifest_path) {
            if let Ok(manifest) = serde_json::from_str::<vox_populi::tensor::manifest::TrainingManifest>(&manifest_raw) {
                found += 1;
                
                let run_id = manifest.run_id.unwrap_or_else(|| "unknown".to_string());
                let base = manifest.base_model.unwrap_or_else(|| "scratch".to_string());
                
                println!("\n⭐ {}", path.file_name().unwrap_or_default().to_string_lossy().green().bold());
                println!("  └─ Run ID:      {}", run_id.dimmed());
                println!("  └─ Base Model:  {}", base);
                println!("  └─ Adapter:     Rank {} / Alpha {}", manifest.rank, manifest.alpha);
                println!("  └─ Data:        {} ({} epochs)", manifest.train_file, manifest.epochs);
                
                if let Some(target) = manifest.training_deployment_target {
                    println!("  └─ Target:      {:?}", target.cyan());
                }
            }
        }
    }

    if found == 0 {
        eprintln!("{} No completed training runs with manifests found in {}.", "ℹ".blue(), runs_dir.display());
    } else {
        println!("\n{} total models found.", found);
    }

    Ok(())
}
