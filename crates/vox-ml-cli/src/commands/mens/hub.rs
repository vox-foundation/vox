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
        /// Destination directory for downloaded artifacts (required unless `--cache-only` is set).
        #[arg(long, required_unless_present = "cache_only")]
        output_dir: Option<PathBuf>,
        /// Fetch and verify snapshot inside the HF cache only without copying or linking to `--output-dir`.
        #[arg(long, default_value_t = false)]
        cache_only: bool,
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
        /// Optional spoke subfolder (`vox-lang`, `rust`, `tool-selection`, `argument-generation`, `harness`, etc.).
        #[arg(long)]
        spoke: Option<String>,
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
    ///
    /// Always renders every model card and the index into `mens/runs/hub_cards/` from the
    /// run records. Without `--push` nothing is uploaded, so that is the dry run.
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
        /// With `--push`, upload only README.md and MODEL_CARD.md (plus their entries and the gate
        /// label in hf_sync_manifest.json) for every trained size whose gate is not current, then
        /// refresh the index. Never uploads weights or tags a version; a gated-current size is
        /// listed for `vox mens hub sync --push --size <S>`. Without `--push` it prints the plan.
        #[arg(long, default_value_t = false)]
        cards_only: bool,
        /// Optional spoke subfolder (`vox-lang`, `rust`, `tool-selection`, `argument-generation`, `harness`, etc.).
        #[arg(long)]
        spoke: Option<String>,
    },
    /// Stage (and optionally `--push`) an eval or curated data dataset bundle to Hugging Face Hub.
    Dataset {
        /// Dataset kind: `eval` or `data`.
        #[arg(long, default_value = "eval")]
        kind: String,
        /// Suite or spoke slug (e.g. `agent_episodes`, `rust_bench`, `vox-lang`).
        #[arg(long, required = true)]
        slug: String,
        /// Source file to bundle into the dataset repo.
        #[arg(long = "source", required = true)]
        sources: Vec<PathBuf>,
        /// Output staging directory (`mens/runs/hub_datasets/<slug>` by default).
        #[arg(long)]
        stage_dir: Option<PathBuf>,
        /// Hugging Face namespace (`VOX_MENS_HF_NAMESPACE` fallback).
        #[arg(long)]
        namespace: Option<String>,
        /// Create repo as private when pushing.
        #[arg(long, default_value_t = false)]
        private: bool,
        /// Upload the staged bundle to HF Hub (default: stage locally only).
        #[arg(long, default_value_t = false)]
        push: bool,
    },
}

/// `--cards-only` never downloads and never publishes ungated weights.
#[cfg(any(feature = "gpu", feature = "mens-hf-hub"))]
fn check_cards_only_flags(pull: bool, force_ungated: bool) -> Result<()> {
    if pull {
        anyhow::bail!("--cards-only cannot be combined with --pull");
    }
    if force_ungated {
        anyhow::bail!("--cards-only cannot be combined with --force-ungated");
    }
    Ok(())
}

/// Print sync rows as JSON or the status table.
#[cfg(any(feature = "gpu", feature = "mens-hf-hub"))]
fn print_sync_rows(rows: &[vox_populi::mens::hub_sync::SizeSyncRow], json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(rows)?);
        return Ok(());
    }
    println!(
        "{:<6} {:<12} {:<8} {:<34} {:<10} {:<10} {:<16} ACTION",
        "SIZE", "STATUS", "VERSION", "REPO", "PEAK_MB", "LOSS", "GATE"
    );
    for r in rows {
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
    Ok(())
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
        HubAction::Download {
            repo,
            output_dir,
            cache_only,
        } => {
            if cache_only {
                println!("==> Downloading model {repo} from Hugging Face Hub (cache-only)...");
            } else if let Some(ref dest) = output_dir {
                println!(
                    "==> Downloading model {repo} from Hugging Face Hub to {}...",
                    dest.display()
                );
            } else {
                println!("==> Downloading model {repo} from Hugging Face Hub...");
            }
            let files = vox_populi::mens::hub::download_model(&repo).await?;
            println!(
                "==> Snapshot fetched to cache: {}",
                files.cache_dir.display()
            );
            if !vox_populi::mens::hub::is_snapshot_complete(&files.cache_dir) {
                anyhow::bail!(
                    "downloaded snapshot at {} failed completeness check",
                    files.cache_dir.display()
                );
            }

            if cache_only || output_dir.is_none() {
                if cache_only && output_dir.is_some() {
                    println!("==> --cache-only is set; skipping staging into --output-dir");
                }
                let has_generation_config = files
                    .generation_config
                    .as_ref()
                    .is_some_and(|p| p.is_file())
                    || files.cache_dir.join("generation_config.json").is_file();
                let has_tokenizer_json = files.cache_dir.join("tokenizer.json").is_file()
                    || files.tokenizer.as_ref().is_some_and(|p| {
                        p.file_name().is_some_and(|n| n == "tokenizer.json") && p.is_file()
                    });
                println!(
                    "==> Cache verified: dir={}, weights={} shard(s), generation_config.json={}, tokenizer.json={}",
                    files.cache_dir.display(),
                    files.weights.len(),
                    has_generation_config,
                    has_tokenizer_json,
                );
                return Ok(());
            }

            let output_dir =
                output_dir.context("--output-dir is required unless --cache-only is set")?;
            std::fs::create_dir_all(&output_dir)
                .with_context(|| format!("failed creating output dir {}", output_dir.display()))?;

            let stage_one = |src: &std::path::Path, dest: &std::path::Path| -> anyhow::Result<()> {
                if dest.exists() || std::fs::symlink_metadata(dest).is_ok() {
                    let _ = std::fs::remove_file(dest);
                }
                if std::fs::hard_link(src, dest).is_err() {
                    std::fs::copy(src, dest).with_context(|| {
                        format!("failed to copy {} to {}", src.display(), dest.display())
                    })?;
                }
                Ok(())
            };
            let stage_if_present = |src: &std::path::Path| -> anyhow::Result<()> {
                if src.is_file()
                    && let Some(name) = src.file_name()
                {
                    stage_one(src, &output_dir.join(name))?;
                }
                Ok(())
            };

            stage_if_present(&files.config)?;
            for opt in [
                files.tokenizer.as_deref(),
                files.tokenizer_config.as_deref(),
                files.chat_template.as_deref(),
                files.generation_config.as_deref(),
                files.special_tokens_map.as_deref(),
            ]
            .into_iter()
            .flatten()
            {
                stage_if_present(opt)?;
            }
            let index_path = files.cache_dir.join("model.safetensors.index.json");
            if index_path.is_file() {
                stage_if_present(&index_path)?;
            }
            for w in &files.weights {
                stage_if_present(w)?;
            }
            println!("==> Model artifacts staged into {}", output_dir.display());
            Ok(())
        }
        HubAction::Pull {
            size,
            revision,
            namespace,
            allow_ungated,
            spoke,
        } => {
            if let Some(sp) = spoke.as_deref() {
                vox_populi::mens::hub_repo::spoke_rel_dir(sp)?;
            }
            let workspace_root = vox_corpus::training::contract::find_workspace_root();
            let dir = vox_populi::mens::hub_sync::pull_adapter_revision_with_spoke(
                std::path::Path::new(&size),
                workspace_root.as_deref(),
                namespace.as_deref(),
                !allow_ungated,
                revision.as_deref(),
                spoke.as_deref(),
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
                cards_only: false,
                spoke: None,
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
            cards_only,
            spoke,
        } => {
            if let Some(sp) = spoke.as_deref() {
                vox_populi::mens::hub_repo::spoke_rel_dir(sp)?;
            }
            let workspace_root = vox_corpus::training::contract::find_workspace_root();
            let rows = if cards_only {
                check_cards_only_flags(pull, force_ungated)?;
                if let Some(sp) = spoke.as_deref() {
                    eprintln!(
                        "  ⚠ `--spoke {sp}` is ignored with `--cards-only` (cards cover all spokes)"
                    );
                }
                vox_populi::mens::hub_cards_push::sync_cards_only(
                    workspace_root.as_deref(),
                    namespace.as_deref(),
                    private,
                    size.as_deref(),
                    push,
                )
                .await?
            } else {
                vox_populi::mens::hub_sync::sync_all_sizes_with_spoke(
                    workspace_root.as_deref(),
                    namespace.as_deref(),
                    push,
                    pull,
                    force_ungated,
                    private,
                    size.as_deref(),
                    spoke.as_deref(),
                )
                .await?
            };
            eprintln!(
                "==> Model cards and index rendered into {}",
                workspace_root
                    .as_deref()
                    .unwrap_or(std::path::Path::new("."))
                    .join(vox_populi::mens::hub_sync::LOCAL_CARDS_DIR)
                    .display()
            );
            print_sync_rows(&rows, json)
        }
        HubAction::Dataset {
            kind,
            slug,
            sources,
            stage_dir,
            namespace,
            private,
            push,
        } => {
            let ds_kind = match kind.trim().to_ascii_lowercase().as_str() {
                "eval" => vox_populi::mens::hub_dataset::DatasetKind::Eval,
                "data" => vox_populi::mens::hub_dataset::DatasetKind::Data,
                other => anyhow::bail!("unknown --kind `{other}` (expected `eval` or `data`)"),
            };
            let ns_env = std::env::var(vox_populi::mens::hub_repo::NAMESPACE_ENV).ok();
            let ns = vox_populi::mens::hub_repo::resolve_namespace(
                namespace.as_deref().or(ns_env.as_deref()),
            )?;
            let out_dir = match stage_dir {
                Some(p) => {
                    vox_corpus::training::contract::normalize_workspace_relative_path(p, None)
                }
                None => vox_corpus::training::contract::resolve_from_workspace(
                    std::path::Path::new("mens/runs/hub_datasets"),
                )
                .join(slug.trim()),
            };
            let mapped_sources = sources
                .into_iter()
                .map(|p| {
                    let rel = p
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("data.jsonl")
                        .to_string();
                    vox_populi::mens::hub_dataset::DatasetSource {
                        local_path: p,
                        repo_rel_path: rel,
                        license: Some("apache-2.0".to_string()),
                    }
                })
                .collect();
            let spec = vox_populi::mens::hub_dataset::DatasetBundleSpec {
                kind: ds_kind,
                slug: slug.clone(),
                namespace: ns,
                description: format!("Vox MENS dataset bundle `{slug}` ({kind})."),
                sources: mapped_sources,
            };
            let staged = vox_populi::mens::hub_dataset::publish_dataset_bundle(
                &spec, &out_dir, push, private,
            )
            .await?;
            println!(
                "==> {} `{}` ({} files) staged at {}",
                if push {
                    "Published"
                } else {
                    "Staged (dry run)"
                },
                staged.repo_id,
                staged.files.len(),
                staged.stage_dir.display()
            );
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
            output_dir: Some(PathBuf::from("target/qwen")),
            cache_only: false,
        };
        assert!(format!("{action:?}").contains("Qwen3-8B"));
    }

    #[test]
    fn hub_download_parses_cache_only_and_output_dir() {
        use clap::Parser;
        #[derive(Parser)]
        struct Wrap {
            #[command(subcommand)]
            action: HubAction,
        }

        let cache_only = Wrap::try_parse_from([
            "hub",
            "download",
            "--repo",
            "Qwen/Qwen3.5-2B",
            "--cache-only",
        ])
        .expect("download --cache-only must parse");
        assert!(matches!(
            cache_only.action,
            HubAction::Download {
                ref repo,
                output_dir: None,
                cache_only: true,
            } if repo == "Qwen/Qwen3.5-2B"
        ));

        let with_out = Wrap::try_parse_from([
            "hub",
            "download",
            "--repo",
            "Qwen/Qwen3.5-2B",
            "--output-dir",
            "/tmp/out",
        ])
        .expect("download --output-dir must parse");
        assert!(matches!(
            with_out.action,
            HubAction::Download {
                ref repo,
                output_dir: Some(ref p),
                cache_only: false,
            } if repo == "Qwen/Qwen3.5-2B" && p == std::path::Path::new("/tmp/out")
        ));

        assert!(
            Wrap::try_parse_from(["hub", "download", "--repo", "Qwen/Qwen3.5-2B"]).is_err(),
            "download without --output-dir or --cache-only must fail clap validation"
        );
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
            cards_only: false,
            spoke: None,
        };
        assert!(format!("{action:?}").contains("Sync"));
    }

    #[cfg(any(feature = "gpu", feature = "mens-hf-hub"))]
    #[test]
    fn cards_only_flag_parses() {
        use clap::Parser;
        #[derive(Parser)]
        struct Wrap {
            #[command(subcommand)]
            action: HubAction,
        }
        let w = Wrap::try_parse_from(["hub", "sync", "--cards-only", "--push"]).unwrap();
        assert!(matches!(
            w.action,
            HubAction::Sync {
                cards_only: true,
                push: true,
                ..
            }
        ));
        let w = Wrap::try_parse_from(["hub", "sync"]).unwrap();
        assert!(matches!(
            w.action,
            HubAction::Sync {
                cards_only: false,
                spoke: None,
                ..
            }
        ));
        let w_spoke = Wrap::try_parse_from(["hub", "sync", "--spoke", "rust"]).unwrap();
        assert!(matches!(
            w_spoke.action,
            HubAction::Sync {
                spoke: Some(ref s),
                ..
            } if s == "rust"
        ));
        let w_pull_spoke =
            Wrap::try_parse_from(["hub", "pull", "--size", "4B", "--spoke", "rust"]).unwrap();
        assert!(matches!(
            w_pull_spoke.action,
            HubAction::Pull {
                spoke: Some(ref s),
                ..
            } if s == "rust"
        ));
    }

    #[cfg(any(feature = "gpu", feature = "mens-hf-hub"))]
    #[test]
    fn cards_only_refuses_pull_and_force_ungated() {
        let e = check_cards_only_flags(true, false).unwrap_err().to_string();
        assert!(e.contains("--pull"), "{e}");
        let e = check_cards_only_flags(false, true).unwrap_err().to_string();
        assert!(e.contains("--force-ungated"), "{e}");
        assert!(check_cards_only_flags(false, false).is_ok());
    }

    /// The eval writer and the cards must agree on the serving path that makes adapter
    /// evidence current (phase 335 `item_checkpoint.rs` vs phase 339 `hub_card/facts.rs`).
    #[cfg(feature = "gpu")]
    #[test]
    fn adapter_serving_matches_eval_identity() {
        assert_eq!(
            vox_populi::mens::hub_card::CURRENT_ADAPTER_SERVING,
            crate::commands::mens::item_checkpoint::ADAPTER_SERVING,
            "CURRENT_ADAPTER_SERVING (vox-populi hub_card/facts.rs) drifted from ADAPTER_SERVING \
             (vox-ml-cli commands/mens/item_checkpoint.rs)"
        );
    }
}
