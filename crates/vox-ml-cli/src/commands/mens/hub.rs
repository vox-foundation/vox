//! Hugging Face Hub operations for `vox mens hub` (upload / download model artifacts).

#[cfg(feature = "mens-hf-hub")]
use anyhow::Context;
use anyhow::Result;
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug, Clone)]
pub enum HubAction {
    /// Upload a model directory to Hugging Face Hub.
    Upload {
        /// Hugging Face repository ID (e.g. `owner/model-name`).
        #[arg(long, required = true)]
        repo: String,
        /// Local directory containing model artifacts.
        #[arg(long, required = true)]
        model_dir: PathBuf,
        /// Make repository private.
        #[arg(long, default_value_t = false)]
        private: bool,
        /// Optional commit message.
        #[arg(long)]
        message: Option<String>,
    },
    /// Download a model or adapter from Hugging Face Hub.
    Download {
        /// Hugging Face repository ID (e.g. `Qwen/Qwen3-8B`).
        #[arg(long, required = true)]
        repo: String,
        /// Destination directory for downloaded artifacts.
        #[arg(long, required = true)]
        output_dir: PathBuf,
    },
}

#[cfg(feature = "mens-hf-hub")]
pub async fn run_hub(action: HubAction) -> Result<()> {
    match action {
        HubAction::Upload {
            repo,
            model_dir,
            private,
            message,
        } => {
            println!(
                "==> Uploading model from {} to Hugging Face Hub ({repo})...",
                model_dir.display()
            );
            let commit_oid = vox_populi::mens::hub::upload_model_folder(
                &repo,
                &model_dir,
                private,
                message.as_deref(),
            )
            .await?;
            println!("==> Upload completed successfully! Commit: {commit_oid}");
            println!("==> Hub URL: https://huggingface.co/{repo}");
            Ok(())
        }
        HubAction::Download { repo, output_dir } => {
            println!(
                "==> Downloading model {repo} from Hugging Face Hub to {}...",
                output_dir.display()
            );
            let files = vox_populi::mens::hub::download_model(&repo).await?;
            println!(
                "==> Snapshot fetched to cache: {}",
                files.cache_dir.display()
            );

            std::fs::create_dir_all(&output_dir)
                .with_context(|| format!("failed creating output dir {}", output_dir.display()))?;

            let stage_one = |src: &std::path::Path, dest: &std::path::Path| -> anyhow::Result<()> {
                if dest.exists() {
                    let _ = std::fs::remove_file(dest);
                }
                if std::fs::hard_link(src, dest).is_err() {
                    std::fs::copy(src, dest).with_context(|| {
                        format!("failed to copy {} to {}", src.display(), dest.display())
                    })?;
                }
                Ok(())
            };

            if files.config.exists() {
                if let Some(name) = files.config.file_name() {
                    stage_one(&files.config, &output_dir.join(name))?;
                }
            }
            if let Some(ref tok) = files.tokenizer {
                if tok.exists() {
                    if let Some(name) = tok.file_name() {
                        stage_one(tok, &output_dir.join(name))?;
                    }
                }
            }
            for w in &files.weights {
                if w.exists() {
                    if let Some(name) = w.file_name() {
                        stage_one(w, &output_dir.join(name))?;
                    }
                }
            }
            println!("==> Model artifacts staged into {}", output_dir.display());
            Ok(())
        }
    }
}

#[cfg(not(feature = "mens-hf-hub"))]
pub async fn run_hub(_action: HubAction) -> Result<()> {
    anyhow::bail!("Hugging Face Hub operations require the `mens-hf-hub` or `gpu` feature.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hub_action_upload_debug_and_clone() {
        let action = HubAction::Upload {
            repo: "vox/test-model".to_string(),
            model_dir: PathBuf::from("mens/runs/test"),
            private: true,
            message: Some("Initial upload".to_string()),
        };
        let cloned = action.clone();
        match cloned {
            HubAction::Upload {
                repo,
                private,
                message,
                ..
            } => {
                assert_eq!(repo, "vox/test-model");
                assert!(private);
                assert_eq!(message.as_deref(), Some("Initial upload"));
            }
            _ => panic!("unexpected action variant"),
        }
    }

    #[test]
    fn hub_action_download_variant() {
        let action = HubAction::Download {
            repo: "Qwen/Qwen3-8B".to_string(),
            output_dir: PathBuf::from("target/qwen"),
        };
        assert!(format!("{action:?}").contains("Qwen3-8B"));
    }
}
