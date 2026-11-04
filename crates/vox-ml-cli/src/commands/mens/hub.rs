//! Hugging Face Hub operations for `vox mens hub` (upload / download model artifacts).

#[cfg(any(feature = "gpu", feature = "mens-hf-hub"))]
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
    /// Download one ladder adapter from its Hub repo into `mens/runs/qwen3-<size>`, pinned to a version.
    Pull {
        /// Ladder size name (`0.6B`, `1.7B`, `4B`, `8B`, `14B`, `32B`, or `Qwen3-4B`).
        #[arg(long, required = true)]
        size: String,
        /// Version tag (`v2`), branch, or commit. Default: the latest `v<N>` tag.
        #[arg(long)]
        revision: Option<String>,
        /// Hugging Face namespace (`VOX_MENS_HF_NAMESPACE` fallback).
        #[arg(long)]
        namespace: Option<String>,
        /// Accept an adapter published without passing gate receipts (`ungated`).
        #[arg(long, default_value_t = false)]
        allow_ungated: bool,
    },
    /// Report Hub versus local ladder state, including version tags. Never uploads.
    Verify {
        /// Hugging Face namespace (`VOX_MENS_HF_NAMESPACE` fallback).
        #[arg(long)]
        namespace: Option<String>,
        /// Emit JSON instead of a formatted table.
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Compare and synchronize local `mens/runs/qwen3-<size>` adapters with Hugging Face Hub.
    Sync {
        /// Hugging Face namespace (`VOX_MENS_HF_NAMESPACE` fallback).
        #[arg(long)]
        namespace: Option<String>,
        /// Upload local adapters (`local_only` / `diverged`) and refresh `<namespace>/vox-mens-index`.
        #[arg(long, default_value_t = false)]
        push: bool,
        /// Download remote adapters (`hub_only` / `diverged`) into `mens/runs/qwen3-<size>`.
        #[arg(long, default_value_t = false)]
        pull: bool,
        /// Allow uploading an adapter without passing gate receipts (marks card as `ungated`).
        #[arg(long, default_value_t = false)]
        force_ungated: bool,
        /// Create new Hugging Face repositories as private (default: public).
        #[arg(long, default_value_t = false)]
        private: bool,
        /// Emit JSON instead of a formatted table.
        #[arg(long, default_value_t = false)]
        json: bool,
        /// Limit push or pull to one ladder size (`0.6B`, `4B`, `Qwen3-4B`). Other sizes are reported only.
        #[arg(long)]
        size: Option<String>,
    },
}

#[cfg(any(feature = "gpu", feature = "mens-hf-hub"))]
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
        HubAction::Pull {
            size,
            revision,
            namespace,
            allow_ungated,
        } => {
            let workspace_root = vox_corpus::training::contract::find_workspace_root();
            let dir = vox_populi::mens::hub_sync::pull_adapter_revision(
                std::path::Path::new(&size),
                workspace_root.as_deref(),
                namespace.as_deref(),
                !allow_ungated,
                revision.as_deref(),
            )
            .await?;
            println!("==> Adapter ready in {}", dir.display());
            Ok(())
        }
        HubAction::Verify { namespace, json } => {
            Box::pin(run_hub(HubAction::Sync {
                namespace,
                push: false,
                pull: false,
                force_ungated: false,
                private: false,
                json,
                size: None,
            }))
            .await
        }
        HubAction::Sync {
            namespace,
            push,
            pull,
            force_ungated,
            private,
            json,
            size,
        } => {
            let workspace_root = vox_corpus::training::contract::find_workspace_root();
            let rows = vox_populi::mens::hub_sync::sync_all_sizes(
                workspace_root.as_deref(),
                namespace.as_deref(),
                push,
                pull,
                force_ungated,
                private,
                size.as_deref(),
            )
            .await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&rows)?);
            } else {
                println!(
                    "{:<6} {:<12} {:<8} {:<34} {:<10} {:<10} {:<16} {}",
                    "SIZE", "STATUS", "VERSION", "REPO", "PEAK_MB", "LOSS", "GATE", "ACTION"
                );
                for r in &rows {
                    let peak = r
                        .measured_peak_mb
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| "-".to_string());
                    let loss = r
                        .final_loss
                        .map(|l| format!("{l:.4}"))
                        .unwrap_or_else(|| "-".to_string());
                    let gate = r.gate_status.as_deref().unwrap_or("-");
                    let act = r.action_taken.as_deref().unwrap_or("-");
                    let version = r.latest_version.as_deref().unwrap_or("-");
                    println!(
                        "{:<6} {:<12} {:<8} {:<34} {:<10} {:<10} {:<16} {}",
                        r.size,
                        r.sync_status.as_str(),
                        version,
                        r.repo_id,
                        peak,
                        loss,
                        gate,
                        act
                    );
                }
            }
            Ok(())
        }
    }
}

#[cfg(not(any(feature = "gpu", feature = "mens-hf-hub")))]
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

    #[test]
    fn hub_action_verify_variant_does_not_push() {
        let action = HubAction::Verify {
            namespace: None,
            json: true,
        };
        assert!(format!("{action:?}").contains("Verify"));
    }

    #[test]
    fn hub_action_sync_variant() {
        let action = HubAction::Sync {
            namespace: None,
            push: true,
            pull: false,
            force_ungated: false,
            private: true,
            json: true,
            size: Some("4B".to_string()),
        };
        assert!(format!("{action:?}").contains("Sync"));
    }
}
