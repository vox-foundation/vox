//! `vox mens models` — Local Model Registry.

use anyhow::Result;
use owo_colors::OwoColorize;
use std::path::PathBuf;

use vox_bounded_fs::read_utf8_path_capped;

/// Prints all trained Mens models found in the run directories.
pub fn run_models(_verbose: bool) -> Result<()> {
    let runs_dir = PathBuf::from(vox_scaling_policy::DEFAULT_MENS_RUNS_ROOT);
    if !runs_dir.exists() {
        eprintln!(
            "{} No models found ({} does not exist)",
            "ℹ".blue(),
            runs_dir.display()
        );
        return Ok(());
    }

    println!("{} Local Mens Model Registry", "📦".cyan());
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

        if let Ok(manifest_raw) = read_utf8_path_capped(&manifest_path)
            && let Ok(manifest) = serde_json::from_str::<
                vox_populi::mens::tensor::manifest::TrainingManifest,
            >(&manifest_raw)
        {
            found += 1;

            let run_id = manifest.run_id.unwrap_or_else(|| "unknown".to_string());
            let base = manifest.base_model.unwrap_or_else(|| "scratch".to_string());

            println!(
                "\n⭐ {}",
                path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .green()
                    .bold()
            );
            println!("  └─ Run ID:      {}", run_id.dimmed());
            println!("  └─ Base Model:  {}", base);
            println!(
                "  └─ Adapter:     Rank {} / Alpha {}",
                manifest.rank, manifest.alpha
            );
            println!(
                "  └─ Data:        {} ({} epochs)",
                manifest.train_file, manifest.epochs
            );

            if let Some(target) = manifest.training_deployment_target {
                println!("  └─ Target:      {:?}", target.cyan());
            }
        }
    }

    if found == 0 {
        eprintln!(
            "{} No completed training runs with manifests found in {}.",
            "ℹ".blue(),
            runs_dir.display()
        );
    } else {
        println!("\n{} total models found.", found);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh checkout has no training runs: listing must report that and
    /// succeed, not error. (cargo runs tests from the crate dir, which has no
    /// `mens/runs`; the guard keeps this from passing vacuously if it ever does.)
    #[test]
    fn listing_without_a_runs_dir_is_ok() {
        assert!(!std::path::Path::new(vox_scaling_policy::DEFAULT_MENS_RUNS_ROOT).exists());
        assert!(run_models(false).is_ok());
    }
}
