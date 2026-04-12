//! `vox repo` — repository discovery status, explicit catalog (`.vox/repositories.yaml`), and cross-repo queries.

use anyhow::Result;
use clap::Subcommand;
use serde::Serialize;

use vox_repository::RepoWorkspaceStatus;

#[derive(Subcommand, Debug)]
pub enum RepoCmd {
    /// Show discovered repository root, stable id, and stack markers (from current directory).
    Status {
        /// Emit compact JSON (also when `VOX_CLI_GLOBAL_JSON=1`).
        #[arg(long)]
        json: bool,
    },
    /// Scaffold a new Vox-compatible repository structure (.voxignore, AGENTS.md, etc.)
    Init {
        /// Project name to initialize repository metadata with
        #[arg(long)]
        name: Option<String>,
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
    /// Re-resolve the current repo catalog and write a snapshot cache.
    Refresh,
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
    vox_clavis::resolve_secret(vox_clavis::SecretId::VoxCliGlobalJson)
        .expose()
        .as_deref() == Some("1")
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

pub async fn run(cmd: RepoCmd) -> Result<()> {
    let repo_root = current_repo_root()?;
    match cmd {
        RepoCmd::Init { name } => {
            crate::commands::repo_init::run(name.as_deref()).await?;
        }
        RepoCmd::Status { json } => {
            let cwd = std::env::current_dir()?;
            let payload: RepoWorkspaceStatus = vox_repository::repo_workspace_status_for_cwd(&cwd);
            if json || json_output_enabled() {
                println!("{}", serde_json::to_string(&payload)?);
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
            }
        }
        RepoCmd::Catalog { cmd } => match cmd {
            RepoCatalogCmd::List => {
                let catalog = vox_repository::resolve_repo_catalog(&repo_root)?;
                print_value(&catalog)?;
            }
            RepoCatalogCmd::Refresh => {
                let refreshed = vox_repository::refresh_repo_catalog(&repo_root)?;
                print_value(&refreshed)?;
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
