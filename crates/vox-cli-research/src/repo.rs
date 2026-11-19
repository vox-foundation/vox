//! `vox repo` — repository discovery status, explicit catalog (`.vox/repositories.yaml`), and cross-repo queries.

use anyhow::Result;
use clap::Subcommand;
use serde::Serialize;
use std::path::PathBuf;

use vox_repository::RepoWorkspaceStatus;

#[derive(Subcommand, Debug)]
pub enum RepoCmd {
    /// Show discovered repository root, stable id, and stack markers (from current directory).
    Status {
        /// Emit compact JSON (also when `VOX_CLI_GLOBAL_JSON=1`).
        #[arg(long)]
        json: bool,
        /// Include resolved active multi-repo context & cache paths (`--active`).
        #[arg(long)]
        active: bool,
        /// Run an OpenWiki action (`init`, `build`, `status`, `diff`) for the active repository.
        #[arg(long)]
        wiki: Option<String>,
        /// Persist changes when used with `--wiki build`.
        #[arg(long)]
        write: bool,
    },
    /// Scaffold a new Vox-compatible repository harness (`.voxignore`, `VOX.md`, `AGENTS.md`, `docs/wiki/`, `.vox/`) or external read-only cache (`--read-only`).
    Init {
        /// Project name to initialize repository metadata with
        #[arg(long)]
        name: Option<String>,
        /// Zero-footprint external read-only mode (writes cache only under `~/.vox/repos/<id>/`)
        #[arg(long = "read-only", alias = "external-cache-only")]
        read_only: bool,
    },
    /// Explicit repository catalog operations (`.vox/repositories.yaml`).
    Catalog {
        #[command(subcommand)]
        cmd: RepoCatalogCmd,
    },
    /// Read-only cross-repo queries over the explicit repo catalog.
    Query {
        #[command(subcommand)]
        cmd: RepoQueryCmd,
    },
}

#[derive(Subcommand, Debug)]
pub enum RepoCatalogCmd {
    /// Resolve and print the current repo catalog.
    List,
    /// Re-resolve the current repo catalog, or apply `--add` / `--remove` / `--switch` updates and write a snapshot cache.
    Refresh {
        /// Switch the active repository focus by repository_id, display_name, or root path.
        #[arg(long)]
        switch: Option<String>,
        /// Add or update a local/external repository path in `.vox/repositories.yaml`.
        #[arg(long)]
        add: Option<PathBuf>,
        /// Optional display name used with `--add`.
        #[arg(long)]
        name: Option<String>,
        /// Mark `--add` entry as read-only (`~/.vox/repos/<id>/` external cache, zero target repo writes).
        #[arg(long = "read-only", alias = "external-cache-only")]
        read_only: bool,
        /// Remove a repository entry by repository_id, display_name, or root path.
        #[arg(long)]
        remove: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum RepoQueryCmd {
    /// Text search across cataloged local repositories.
    Text {
        /// Search query string.
        query: String,
        /// Limit to one or more resolved repository ids.
        #[arg(long = "repo-id")]
        repository_ids: Vec<String>,
        /// Treat the query as a regex.
        #[arg(long)]
        regex: bool,
        /// Make matching case-sensitive.
        #[arg(long)]
        case_sensitive: bool,
        /// Maximum matches returned per repository.
        #[arg(long, default_value_t = 50)]
        max_matches_per_repo: usize,
        /// Maximum files scanned per repository.
        #[arg(long, default_value_t = 50_000)]
        max_files_per_repo: usize,
        /// Skip files larger than this many bytes.
        #[arg(long, default_value_t = 262_144)]
        max_file_bytes: usize,
    },
    /// Read one path across cataloged repositories.
    File {
        /// Workspace-relative or repository-relative file path.
        path: String,
        /// Limit to one or more resolved repository ids.
        #[arg(long = "repo-id")]
        repository_ids: Vec<String>,
        /// Maximum bytes returned per file.
        #[arg(long, default_value_t = 131_072)]
        max_bytes: usize,
    },
    /// Read recent Git history per cataloged repository.
    History {
        /// Limit to one or more resolved repository ids.
        #[arg(long = "repo-id")]
        repository_ids: Vec<String>,
        /// Optional path filter passed to `git log -- <path>`.
        #[arg(long)]
        path: Option<String>,
        /// Optional substring filter over rendered `git log --oneline` lines.
        #[arg(long)]
        contains: Option<String>,
        /// Maximum commits requested per repository.
        #[arg(long, default_value_t = 20)]
        max_commits: usize,
    },
}

fn json_output_enabled() -> bool {
    vox_secrets::resolve_secret(vox_secrets::SecretId::VoxCliGlobalJson).expose() == Some("1")
}

fn print_value<T: Serialize>(value: &T) -> Result<()> {
    if json_output_enabled() {
        println!("{}", serde_json::to_string(value)?);
    } else {
        println!("{}", serde_json::to_string_pretty(value)?);
    }
    Ok(())
}

fn current_repo_root() -> Result<std::path::PathBuf> {
    let cwd = std::env::current_dir()?;
    Ok(vox_repository::discover_repository_or_fallback(&cwd).root)
}

pub async fn run_wiki_action(action: &str, write: bool, json: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let workspace_root = vox_repository::discover_repository_or_fallback(&cwd).root;
    let active_ctx = vox_repository::get_active_repository_context(&workspace_root).ok();
    let (target_root, ext_override) = match &active_ctx {
        Some(ctx) if ctx.read_only => (
            PathBuf::from(&ctx.root_path),
            Some(PathBuf::from(&ctx.openwiki_cache_dir)),
        ),
        Some(ctx) => (PathBuf::from(&ctx.root_path), None),
        None => (workspace_root, None),
    };
    let out_opt = ext_override.as_deref();
    let json_mode = json || json_output_enabled();
    let trimmed = action.trim();

    if trimmed == "sync" || trimmed == "heal" {
        let rep = vox_doc_pipeline::pipeline::openwiki::heal_tier1_deterministic(
            &target_root,
            &[],
            write || trimmed == "heal",
            out_opt,
        )
        .map_err(|e| anyhow::anyhow!(e))?;
        if json_mode {
            print_value(&rep)?;
        } else {
            println!(
                "openwiki {trimmed} OK (llm_calls={}, impacted={}, updated={}, repaired_anchors={})",
                rep.llm_calls,
                rep.blast_radius.impacted_docs.len(),
                rep.updated_pages.len(),
                rep.repaired_anchors
            );
        }
        return Ok(());
    }

    if trimmed == "heal-list"
        || trimmed.starts_with("heal-apply:")
        || trimmed.starts_with("heal-reject:")
        || trimmed.starts_with("heal-demote:")
    {
        let db = crate::workspace_db::connect_cli_workspace_voxdb().await?;
        let repo_ctx = vox_repository::discover_repository_or_fallback(&target_root);
        if trimmed == "heal-list" {
            let rows = db
                .kb_heal_proposals_list(Some(&repo_ctx.repository_id), None, 100)
                .await?;
            print_value(&rows)?;
        } else if let Some(id) = trimmed.strip_prefix("heal-apply:") {
            let path = vox_search::promotion::apply_staged_heal_proposal(
                &db,
                &target_root,
                id,
                "cli-operator",
                Some("Applied via `vox repo --wiki`"),
            )
            .await?;
            print_value(&serde_json::json!({
                "status": "applied",
                "proposal_id": id,
                "path": path.display().to_string(),
            }))?;
        } else if let Some(id) = trimmed.strip_prefix("heal-reject:") {
            let ok = db
                .kb_heal_proposal_update_status(
                    id,
                    "rejected",
                    Some("cli-operator"),
                    Some("Rejected via `vox repo --wiki`"),
                )
                .await?;
            print_value(&serde_json::json!({
                "status": "rejected",
                "proposal_id": id,
                "updated": ok,
            }))?;
        } else if let Some(id) = trimmed.strip_prefix("heal-demote:") {
            if let Some(row) = db.kb_heal_proposal_get(id).await? {
                let _ = vox_doc_pipeline::pipeline::openwiki::demote_doc_to_legacy(
                    &target_root,
                    &row.doc_path,
                )
                .map_err(|e| anyhow::anyhow!(e))?;
                let ok = db
                    .kb_heal_proposal_update_status(
                        id,
                        "demoted_to_legacy",
                        Some("cli-operator"),
                        Some("Demoted doc to legacy via `vox repo --wiki`"),
                    )
                    .await?;
                print_value(&serde_json::json!({
                    "status": "demoted_to_legacy",
                    "proposal_id": id,
                    "doc_path": row.doc_path,
                    "updated": ok,
                }))?;
            } else {
                anyhow::bail!("Heal proposal `{id}` not found");
            }
        }
        return Ok(());
    }

    match trimmed {
        "init" => {
            let rep = vox_doc_pipeline::pipeline::openwiki::openwiki_init(&target_root, out_opt)
                .map_err(|e| anyhow::anyhow!(e))?;
            if json_mode {
                print_value(&rep)?;
            } else {
                println!(
                    "openwiki init OK ({} pages, {} updated) -> {}",
                    rep.generated_pages.len(),
                    rep.updated_pages.len(),
                    rep.output_dir
                );
            }
        }
        "build" => {
            let rep =
                vox_doc_pipeline::pipeline::openwiki::openwiki_build(&target_root, write, out_opt)
                    .map_err(|e| anyhow::anyhow!(e))?;
            if json_mode {
                print_value(&rep)?;
            } else {
                println!(
                    "openwiki build OK (write={}, {} pages, {} changed) -> {}",
                    write,
                    rep.generated_pages.len(),
                    rep.updated_pages.len(),
                    rep.output_dir
                );
            }
        }
        "status" => {
            let rep = vox_doc_pipeline::pipeline::openwiki::openwiki_status(&target_root, out_opt)
                .map_err(|e| anyhow::anyhow!(e))?;
            if json_mode {
                print_value(&rep)?;
            } else {
                println!(
                    "openwiki status: initialized={} stale_pages={}/{} dir={}",
                    rep.initialized, rep.stale_pages, rep.total_pages, rep.wiki_dir
                );
            }
        }
        "diff" => {
            let rep = vox_doc_pipeline::pipeline::openwiki::openwiki_diff(&target_root, out_opt)
                .map_err(|e| anyhow::anyhow!(e))?;
            if json_mode {
                print_value(&rep)?;
            } else {
                println!(
                    "openwiki diff: has_changes={} pages={} dir={}",
                    rep.has_changes,
                    rep.diffs.len(),
                    rep.wiki_dir
                );
            }
        }
        other => anyhow::bail!(
            "Unknown --wiki action '{other}' (expected init|build|status|diff|sync|heal|heal-list|heal-apply:<id>|heal-reject:<id>|heal-demote:<id>)"
        ),
    }
    Ok(())
}

pub async fn run(cmd: RepoCmd) -> Result<()> {
    let repo_root = current_repo_root()?;
    match cmd {
        RepoCmd::Init { name, read_only } => {
            crate::repo_init::run_with_options(name.as_deref(), read_only).await?;
        }
        RepoCmd::Status {
            json,
            active,
            wiki,
            write,
        } => {
            if let Some(action) = wiki {
                return run_wiki_action(&action, write, json).await;
            }
            let cwd = std::env::current_dir()?;
            let payload: RepoWorkspaceStatus = vox_repository::repo_workspace_status_for_cwd(&cwd);
            let active_ctx = if active {
                vox_repository::get_active_repository_context(&payload.root).ok()
            } else {
                None
            };
            if json || json_output_enabled() {
                if let Some(ref ctx) = active_ctx {
                    println!(
                        "{}",
                        serde_json::to_string(&serde_json::json!({
                            "workspace": payload,
                            "active_context": ctx,
                        }))?
                    );
                } else {
                    println!("{}", serde_json::to_string(&payload)?);
                }
            } else {
                println!("root:            {}", payload.root.display());
                println!("repository_id:   {}", payload.repository_id);
                if let Some(ref o) = payload.origin_url {
                    println!("origin_url:      {o}");
                }
                if let Some(ref g) = payload.git_root {
                    println!("git_root:        {}", g.display());
                }
                println!("has_vox_agents:  {}", payload.has_vox_agents_dir);
                if let Some(ref v) = payload.vox_toml {
                    println!("Vox.toml:        {}", v.display());
                }
                let c = &payload.capabilities;
                println!(
                    "markers:         vox_project={} cargo_workspace={} cargo_package={} node={} python={} go={} git={}",
                    c.vox_project,
                    c.cargo_workspace,
                    c.cargo_package,
                    c.node_workspace,
                    c.python_project,
                    c.go_module,
                    c.git
                );
                if !payload.cargo_workspace_members.is_empty() {
                    println!("workspace_members:");
                    for m in payload.cargo_workspace_members.iter().take(20) {
                        let rel = m.strip_prefix(&payload.root).unwrap_or(m);
                        println!("  - {}", rel.display());
                    }
                    let n = payload.cargo_workspace_members.len();
                    if n > 20 {
                        println!("  ... and {} more", n - 20);
                    }
                }
                if let Some(ref ctx) = active_ctx {
                    println!(
                        "active_repo:     {} ({})",
                        ctx.display_name, ctx.repository_id
                    );
                    println!("read_only:       {}", ctx.read_only);
                    println!("kb_cache_dir:    {}", ctx.kb_cache_dir);
                    println!("graphify_cache:  {}", ctx.graphify_cache_dir);
                    println!("openwiki_cache:  {}", ctx.openwiki_cache_dir);
                    println!("global_library:  {}", ctx.global_library_dir);
                }
            }
        }
        RepoCmd::Catalog { cmd } => match cmd {
            RepoCatalogCmd::List => {
                let catalog = vox_repository::resolve_repo_catalog(&repo_root)?;
                print_value(&catalog)?;
            }
            RepoCatalogCmd::Refresh {
                switch,
                add,
                name,
                read_only,
                remove,
            } => {
                if let Some(add_path) = add {
                    let target = if add_path.is_absolute() {
                        add_path
                    } else {
                        repo_root.join(&add_path)
                    };
                    let added = vox_repository::add_repository_to_catalog(
                        &repo_root,
                        &target,
                        name.as_deref(),
                        read_only,
                    )?;
                    print_value(&added)?;
                } else if let Some(remove_target) = remove {
                    let remaining =
                        vox_repository::remove_repository_from_catalog(&repo_root, &remove_target)?;
                    print_value(&remaining)?;
                } else if let Some(switch_target) = switch {
                    let switched =
                        vox_repository::switch_active_repository(&repo_root, &switch_target)?;
                    print_value(&switched)?;
                } else {
                    let refreshed = vox_repository::refresh_repo_catalog(&repo_root)?;
                    print_value(&refreshed)?;
                }
            }
        },
        RepoCmd::Query { cmd } => match cmd {
            RepoQueryCmd::Text {
                query,
                repository_ids,
                regex,
                case_sensitive,
                max_matches_per_repo,
                max_files_per_repo,
                max_file_bytes,
            } => {
                let response = vox_repository::repo_query_text_with_plane(
                    &repo_root,
                    &vox_repository::QueryTextParams {
                        query,
                        repository_ids: (!repository_ids.is_empty()).then_some(repository_ids),
                        case_insensitive: !case_sensitive,
                        regex,
                        max_matches_per_repo,
                        max_files_per_repo,
                        max_file_bytes,
                        conversation_id: None,
                    },
                    "cli",
                    None,
                )?;
                print_value(&response)?;
            }
            RepoQueryCmd::File {
                path,
                repository_ids,
                max_bytes,
            } => {
                let response = vox_repository::repo_query_file_with_plane(
                    &repo_root,
                    &vox_repository::QueryFileParams {
                        path,
                        repository_ids: (!repository_ids.is_empty()).then_some(repository_ids),
                        max_bytes,
                        conversation_id: None,
                    },
                    "cli",
                    None,
                )?;
                print_value(&response)?;
            }
            RepoQueryCmd::History {
                repository_ids,
                path,
                contains,
                max_commits,
            } => {
                let response = vox_repository::repo_query_history_with_plane(
                    &repo_root,
                    &vox_repository::QueryHistoryParams {
                        repository_ids: (!repository_ids.is_empty()).then_some(repository_ids),
                        path,
                        contains,
                        max_commits,
                        conversation_id: None,
                    },
                    "cli",
                    None,
                )?;
                print_value(&response)?;
            }
        },
    }
    Ok(())
}
