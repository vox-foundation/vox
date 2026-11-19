use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::{
    ActiveRepoContext, HarnessOnboardSummary, REPO_CATALOG_BASENAME, REPO_CATALOG_SCHEMA_VERSION,
    RepoAccessMode, RepoCapability, RepoCatalog, RepoCatalogRefreshResult, RepositoryDescriptor,
    ResolvedRepoCatalog, ResolvedRepositoryDescriptor,
};

#[derive(Debug, thiserror::Error)]
pub enum RepoCatalogError {
    #[error("repo catalog manifest not found: {path}")]
    MissingManifest { path: String },
    #[error("read repo catalog manifest {path}: {source}")]
    ReadManifest {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("parse repo catalog manifest {path}: {source}")]
    ParseManifest {
        path: String,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("serialize repo catalog snapshot: {0}")]
    SerializeSnapshot(#[from] serde_json::Error),
    #[error("create repo catalog snapshot directory {path}: {source}")]
    CreateSnapshotDir {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("write repo catalog snapshot {path}: {source}")]
    WriteSnapshot {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("sync federated satellite mirror {path}: {source}")]
    SyncSatellite {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid regex query {pattern:?}: {message}")]
    InvalidRegex { pattern: String, message: String },
    #[error("serialize repo catalog manifest: {0}")]
    SerializeManifest(#[from] serde_yaml::Error),
    #[error("write repo catalog manifest {path}: {source}")]
    WriteManifest {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("write harness artifact {path}: {source}")]
    WriteHarnessArtifact {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("repository not found in catalog: {target}")]
    RepositoryNotFound { target: String },
    #[error("unable to determine user home directory for ~/.vox storage")]
    MissingHomeDir,
}

pub fn repo_catalog_manifest_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".vox").join(REPO_CATALOG_BASENAME)
}

fn repo_catalog_snapshot_path(repo_root: &Path, repository_id: &str) -> PathBuf {
    repo_root
        .join(".vox")
        .join("cache")
        .join("repos")
        .join(repository_id)
        .join("repo_catalog_snapshot.json")
}

pub fn load_repo_catalog_from_repo(repo_root: &Path) -> Result<RepoCatalog, RepoCatalogError> {
    let manifest_path = repo_catalog_manifest_path(repo_root);
    if !manifest_path.is_file() {
        return Err(RepoCatalogError::MissingManifest {
            path: manifest_path.display().to_string(),
        });
    }
    let raw = std::fs::read_to_string(&manifest_path).map_err(|source| {
        RepoCatalogError::ReadManifest {
            path: manifest_path.display().to_string(),
            source,
        }
    })?;
    serde_yaml::from_str::<RepoCatalog>(&raw).map_err(|source| RepoCatalogError::ParseManifest {
        path: manifest_path.display().to_string(),
        source,
    })
}

fn normalize_capabilities(
    caps: Vec<RepoCapability>,
    access_mode: &RepoAccessMode,
) -> Vec<RepoCapability> {
    if caps.is_empty() {
        return match access_mode {
            RepoAccessMode::Local | RepoAccessMode::ReadOnlyLocal => vec![
                RepoCapability::ReadFile,
                RepoCapability::ListFiles,
                RepoCapability::TextSearch,
                RepoCapability::HistorySearch,
            ],
            RepoAccessMode::RemoteMcp => vec![
                RepoCapability::ReadFile,
                RepoCapability::ListFiles,
                RepoCapability::TextSearch,
            ],
            RepoAccessMode::RemoteGitHost => vec![RepoCapability::HistorySearch],
            RepoAccessMode::RemoteSearchService => {
                vec![RepoCapability::TextSearch, RepoCapability::SemanticSearch]
            }
        };
    }
    let mut seen = HashSet::new();
    caps.into_iter()
        .filter(|c| seen.insert(c.clone()))
        .collect()
}

fn detect_provider(origin_url: Option<&str>, existing: Option<&str>) -> Option<String> {
    if let Some(existing) = existing.filter(|s| !s.trim().is_empty()) {
        return Some(existing.to_string());
    }
    let origin = origin_url?;
    if origin.contains("github.com") {
        Some("github".to_string())
    } else if origin.contains("gitlab") {
        Some("gitlab".to_string())
    } else if origin.contains("bitbucket") {
        Some("bitbucket".to_string())
    } else {
        Some("local".to_string())
    }
}

fn resolve_descriptor_root(workspace_root: &Path, root_path: &str) -> Result<PathBuf, String> {
    let path = Path::new(root_path);
    let raw = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    };
    Ok(std::fs::canonicalize(&raw).unwrap_or(raw))
}

fn resolved_local_descriptor(
    repo: RepositoryDescriptor,
    declared_id: Option<String>,
    root_path: String,
    capabilities: Vec<RepoCapability>,
    resolved: crate::RepositoryContext,
) -> ResolvedRepositoryDescriptor {
    let mismatch = declared_id.as_ref().map(|id| id != &resolved.repository_id);
    let origin_url = resolved
        .origin_url
        .clone()
        .or_else(|| repo.origin_url.clone());
    let provider = detect_provider(origin_url.as_deref(), repo.provider.as_deref());
    let read_only = repo.read_only || matches!(repo.access_mode, RepoAccessMode::ReadOnlyLocal);
    let access_mode = if read_only {
        RepoAccessMode::ReadOnlyLocal
    } else {
        RepoAccessMode::Local
    };
    let external_cache_dir = repo.external_cache_dir.clone().or_else(|| {
        if read_only {
            external_repo_cache_root(&resolved.repository_id)
                .ok()
                .map(|p| p.display().to_string())
        } else {
            None
        }
    });
    ResolvedRepositoryDescriptor {
        repository_id: Some(resolved.repository_id.clone()),
        declared_repository_id: declared_id,
        display_name: repo.display_name,
        root_path: Some(root_path),
        resolved_root: Some(resolved.root.display().to_string()),
        git_root: resolved.git_root.as_ref().map(|p| p.display().to_string()),
        origin_url,
        provider,
        default_ref: repo.default_ref,
        access_mode,
        capabilities,
        remote: repo.remote,
        metadata: repo.metadata,
        resolution_status: "resolved_local".to_string(),
        resolution_error: None,
        repository_id_mismatch: mismatch,
        role: repo.role,
        optional: repo.optional,
        read_only,
        external_cache_dir,
        shared_contracts: repo.shared_contracts,
        cargo_patch_crates: repo.cargo_patch_crates,
    }
}

fn unresolved_local_descriptor(
    repo: RepositoryDescriptor,
    declared_id: Option<String>,
    root_path: Option<String>,
    capabilities: Vec<RepoCapability>,
    provider: Option<String>,
    resolution_status: &str,
    resolution_error: String,
) -> ResolvedRepositoryDescriptor {
    let read_only = repo.read_only || matches!(repo.access_mode, RepoAccessMode::ReadOnlyLocal);
    let access_mode = if read_only {
        RepoAccessMode::ReadOnlyLocal
    } else {
        RepoAccessMode::Local
    };
    ResolvedRepositoryDescriptor {
        repository_id: declared_id.clone(),
        declared_repository_id: declared_id,
        display_name: repo.display_name,
        root_path,
        resolved_root: None,
        git_root: None,
        origin_url: repo.origin_url,
        provider,
        default_ref: repo.default_ref,
        access_mode,
        capabilities,
        remote: repo.remote,
        metadata: repo.metadata,
        resolution_status: resolution_status.to_string(),
        resolution_error: Some(resolution_error),
        repository_id_mismatch: None,
        role: repo.role,
        optional: repo.optional,
        read_only,
        external_cache_dir: repo.external_cache_dir,
        shared_contracts: repo.shared_contracts,
        cargo_patch_crates: repo.cargo_patch_crates,
    }
}

fn skipped_absent_optional_descriptor(
    repo: RepositoryDescriptor,
    declared_id: Option<String>,
    root_path: Option<String>,
    capabilities: Vec<RepoCapability>,
    provider: Option<String>,
    reason: String,
) -> ResolvedRepositoryDescriptor {
    let read_only = repo.read_only || matches!(repo.access_mode, RepoAccessMode::ReadOnlyLocal);
    let access_mode = if read_only {
        RepoAccessMode::ReadOnlyLocal
    } else {
        RepoAccessMode::Local
    };
    ResolvedRepositoryDescriptor {
        repository_id: declared_id.clone(),
        declared_repository_id: declared_id,
        display_name: repo.display_name,
        root_path,
        resolved_root: None,
        git_root: None,
        origin_url: repo.origin_url,
        provider,
        default_ref: repo.default_ref,
        access_mode,
        capabilities,
        remote: repo.remote,
        metadata: repo.metadata,
        resolution_status: "skipped_absent_optional".to_string(),
        resolution_error: Some(reason),
        repository_id_mismatch: None,
        role: repo.role,
        optional: repo.optional,
        read_only,
        external_cache_dir: repo.external_cache_dir,
        shared_contracts: repo.shared_contracts,
        cargo_patch_crates: repo.cargo_patch_crates,
    }
}

fn remote_descriptor(
    repo: RepositoryDescriptor,
    provider: Option<String>,
    capabilities: Vec<RepoCapability>,
    other_mode: RepoAccessMode,
) -> ResolvedRepositoryDescriptor {
    let repo_id = repo.repository_id.clone();
    let status = if repo_id.is_some() {
        "catalog_only_remote"
    } else {
        "remote_missing_repository_id"
    };
    let error = if repo_id.is_some() {
        None
    } else {
        Some(
            "remote repo entries should declare repository_id so cross-repo results can group stably"
                .to_string(),
        )
    };
    ResolvedRepositoryDescriptor {
        repository_id: repo_id,
        declared_repository_id: repo.repository_id,
        display_name: repo.display_name,
        root_path: repo.root_path,
        resolved_root: None,
        git_root: None,
        origin_url: repo.origin_url,
        provider,
        default_ref: repo.default_ref,
        access_mode: other_mode,
        capabilities,
        remote: repo.remote,
        metadata: repo.metadata,
        resolution_status: status.to_string(),
        resolution_error: error,
        repository_id_mismatch: None,
        role: repo.role,
        optional: repo.optional,
        read_only: repo.read_only,
        external_cache_dir: repo.external_cache_dir,
        shared_contracts: repo.shared_contracts,
        cargo_patch_crates: repo.cargo_patch_crates,
    }
}

/// The catalog of a workspace that declares none: the workspace itself, as one
/// local entry. A repo that has not opted into a polyrepo manifest can still be
/// queried for its own files; other repositories still need an explicit
/// `.vox/repositories.yaml` (no ambient scan).
fn implicit_workspace_catalog(workspace_root: &Path) -> RepoCatalog {
    let display_name = workspace_root.file_name().map_or_else(
        || "workspace".to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    RepoCatalog {
        schema_version: REPO_CATALOG_SCHEMA_VERSION,
        primary_repository_id: None,
        repositories: vec![RepositoryDescriptor {
            display_name,
            root_path: Some(".".to_string()),
            access_mode: RepoAccessMode::Local,
            ..RepositoryDescriptor::default()
        }],
    }
}

#[inline(never)]
pub fn resolve_repo_catalog(repo_root: &Path) -> Result<ResolvedRepoCatalog, RepoCatalogError> {
    let workspace = crate::discover_repository_or_fallback(repo_root);
    let catalog = match load_repo_catalog_from_repo(&workspace.root) {
        Err(RepoCatalogError::MissingManifest { .. }) => {
            implicit_workspace_catalog(&workspace.root)
        }
        other => other?,
    };
    let manifest_path = repo_catalog_manifest_path(&workspace.root);
    let mut repositories = Vec::with_capacity(catalog.repositories.len());

    for repo in catalog.repositories {
        let declared_id = repo.repository_id.clone();
        let provider = detect_provider(repo.origin_url.as_deref(), repo.provider.as_deref());
        let capabilities = normalize_capabilities(repo.capabilities.clone(), &repo.access_mode);
        match repo.access_mode.clone() {
            RepoAccessMode::Local | RepoAccessMode::ReadOnlyLocal => {
                let Some(root_path) = repo.root_path.clone() else {
                    repositories.push(unresolved_local_descriptor(
                        repo,
                        declared_id,
                        None,
                        capabilities,
                        provider,
                        "local_missing_root_path",
                        "local repo entries require root_path in .vox/repositories.yaml"
                            .to_string(),
                    ));
                    continue;
                };
                match resolve_descriptor_root(&workspace.root, &root_path) {
                    Ok(root_candidate) => {
                        if !root_candidate.exists() {
                            if repo.optional {
                                repositories.push(skipped_absent_optional_descriptor(
                                    repo,
                                    declared_id,
                                    Some(root_path),
                                    capabilities,
                                    provider,
                                    format!("optional repository path '{root_candidate:?}' does not exist on disk"),
                                ));
                            } else {
                                repositories.push(unresolved_local_descriptor(
                                    repo,
                                    declared_id,
                                    Some(root_path),
                                    capabilities,
                                    provider,
                                    "local_resolution_failed",
                                    format!("repository path '{root_candidate:?}' does not exist on disk"),
                                ));
                            }
                        } else {
                            match crate::discover_repository(&root_candidate) {
                                Ok(resolved) => repositories.push(resolved_local_descriptor(
                                    repo,
                                    declared_id,
                                    root_path,
                                    capabilities,
                                    resolved,
                                )),
                                Err(e) => {
                                    if repo.optional {
                                        repositories.push(skipped_absent_optional_descriptor(
                                            repo,
                                            declared_id,
                                            Some(root_path),
                                            capabilities,
                                            provider,
                                            e.to_string(),
                                        ));
                                    } else {
                                        repositories.push(unresolved_local_descriptor(
                                            repo,
                                            declared_id,
                                            Some(root_path),
                                            capabilities,
                                            provider,
                                            "local_resolution_failed",
                                            e.to_string(),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        if repo.optional {
                            repositories.push(skipped_absent_optional_descriptor(
                                repo,
                                declared_id,
                                Some(root_path),
                                capabilities,
                                provider,
                                e,
                            ));
                        } else {
                            repositories.push(unresolved_local_descriptor(
                                repo,
                                declared_id,
                                Some(root_path),
                                capabilities,
                                provider,
                                "local_resolution_failed",
                                e,
                            ));
                        }
                    }
                }
            }
            other_mode => repositories.push(remote_descriptor(
                repo,
                provider,
                capabilities,
                other_mode.clone(),
            )),
        }
    }

    let primary_repository_id = catalog
        .primary_repository_id
        .clone()
        .or_else(|| workspace.repository_id.clone().into());
    let primary_resolved_root = primary_repository_id.as_deref().and_then(|primary_id| {
        repositories
            .iter()
            .find(|r| {
                r.repository_id.as_deref() == Some(primary_id)
                    && r.resolution_status == "resolved_local"
            })
            .and_then(|r| r.resolved_root.clone())
    });

    Ok(ResolvedRepoCatalog {
        schema_version: catalog.schema_version.max(REPO_CATALOG_SCHEMA_VERSION),
        manifest_path: manifest_path.display().to_string(),
        primary_repository_id,
        primary_resolved_root,
        repositories,
    })
}

#[inline(never)]
pub fn refresh_repo_catalog(
    repo_root: &Path,
) -> Result<RepoCatalogRefreshResult, RepoCatalogError> {
    let workspace = crate::discover_repository_or_fallback(repo_root);
    let catalog = resolve_repo_catalog(&workspace.root)?;
    let snapshot_path = repo_catalog_snapshot_path(&workspace.root, &workspace.repository_id);
    if let Some(parent) = snapshot_path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| RepoCatalogError::CreateSnapshotDir {
            path: parent.display().to_string(),
            source,
        })?;
    }
    let json = serde_json::to_string_pretty(&catalog)?;
    std::fs::write(&snapshot_path, json).map_err(|source| RepoCatalogError::WriteSnapshot {
        path: snapshot_path.display().to_string(),
        source,
    })?;
    Ok(RepoCatalogRefreshResult {
        manifest_path: catalog.manifest_path.clone(),
        snapshot_path: snapshot_path.display().to_string(),
        catalog,
    })
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

fn sanitize_repo_id(repo_id: &str) -> String {
    let cleaned: String = repo_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "default".to_string()
    } else {
        cleaned
    }
}

/// Resolve the user's home directory without relying on OS-specific hardcoded paths.
pub fn vox_user_home_dir() -> Result<PathBuf, RepoCatalogError> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .ok_or(RepoCatalogError::MissingHomeDir)
}

/// Cross-repo personal knowledge library directory under a given home directory (`<home>/.vox/library`).
pub fn global_library_root_under(home_dir: &Path) -> PathBuf {
    home_dir.join(".vox").join("library")
}

/// Cross-repo personal knowledge library directory (`~/.vox/library`).
pub fn global_library_root() -> Result<PathBuf, RepoCatalogError> {
    Ok(global_library_root_under(&vox_user_home_dir()?))
}

/// External zero-footprint cache root for read-only indexing under a given home directory.
pub fn external_repo_cache_root_under(home_dir: &Path, repository_id: &str) -> PathBuf {
    home_dir
        .join(".vox")
        .join("repos")
        .join(sanitize_repo_id(repository_id))
}

/// External zero-footprint cache root (`~/.vox/repos/<repository_id>`).
pub fn external_repo_cache_root(repository_id: &str) -> Result<PathBuf, RepoCatalogError> {
    Ok(external_repo_cache_root_under(
        &vox_user_home_dir()?,
        repository_id,
    ))
}

/// Persist `.vox/repositories.yaml` under `repo_root`.
pub fn save_repo_catalog(
    repo_root: &Path,
    catalog: &RepoCatalog,
) -> Result<PathBuf, RepoCatalogError> {
    let manifest_path = repo_catalog_manifest_path(repo_root);
    if let Some(parent) = manifest_path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| RepoCatalogError::WriteManifest {
            path: parent.display().to_string(),
            source,
        })?;
    }
    let mut normalized = catalog.clone();
    normalized.schema_version = normalized.schema_version.max(REPO_CATALOG_SCHEMA_VERSION);
    let yaml = serde_yaml::to_string(&normalized)?;
    std::fs::write(&manifest_path, yaml).map_err(|source| RepoCatalogError::WriteManifest {
        path: manifest_path.display().to_string(),
        source,
    })?;
    Ok(manifest_path)
}

fn ensure_dir(path: &Path) -> Result<(), RepoCatalogError> {
    std::fs::create_dir_all(path).map_err(|source| RepoCatalogError::WriteHarnessArtifact {
        path: path.display().to_string(),
        source,
    })
}

fn write_if_missing(
    path: &Path,
    contents: &str,
    created: &mut Vec<String>,
    existing: &mut Vec<String>,
) -> Result<(), RepoCatalogError> {
    if path.exists() {
        existing.push(path.display().to_string());
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    std::fs::write(path, contents).map_err(|source| RepoCatalogError::WriteHarnessArtifact {
        path: path.display().to_string(),
        source,
    })?;
    created.push(path.display().to_string());
    Ok(())
}

fn stack_labels(ctx: &crate::RepositoryContext) -> Vec<&'static str> {
    let mut out = Vec::new();
    if ctx.capabilities.vox_project {
        out.push("Vox");
    }
    if ctx.capabilities.cargo_workspace || ctx.capabilities.cargo_package {
        out.push("Rust/Cargo");
    }
    if ctx.capabilities.node_workspace {
        out.push("Node/TypeScript");
    }
    if ctx.capabilities.python_project {
        out.push("Python");
    }
    if ctx.capabilities.go_module {
        out.push("Go");
    }
    if ctx.capabilities.git {
        out.push("Git");
    }
    if out.is_empty() {
        out.push("Generic");
    }
    out
}

fn render_vox_md(ctx: &crate::RepositoryContext, display_name: &str) -> String {
    let stacks = stack_labels(ctx).join(", ");
    format!(
        "# VOX.md — {display_name}\n\n\
         > Universal repository harness descriptor for Vox KB, Graphify, and OpenWiki.\n\n\
         - **Repository Name:** `{display_name}`\n\
         - **Repository ID:** `{repo_id}`\n\
         - **Detected Stack:** {stacks}\n\n\
         ## Knowledge & Context Topology\n\n\
         - **Harness Instructions:** [`AGENTS.md`](AGENTS.md)\n\
         - **OpenWiki Knowledge Base:** [`docs/wiki/README.md`](docs/wiki/README.md)\n\
         - **Exclusion Policy:** [`.voxignore`](.voxignore)\n\
         - **Agent Discovery:** [`.well-known/llms.txt`](.well-known/llms.txt)\n\
         - **Multi-Repo Catalog:** [`.vox/repositories.yaml`](.vox/repositories.yaml)\n\
         - **Local Machine Cache (gitignored):** `.vox/cache/{{kb,graphify,openwiki}}/`\n",
        repo_id = ctx.repository_id,
    )
}

fn render_agents_md(display_name: &str) -> String {
    format!(
        "# AGENTS.md — {display_name}\n\n\
         Repository instructions for Vox and cross-tool coding agents.\n\n\
         - Read `VOX.md` and `docs/wiki/README.md` before modifying architecture or modules.\n\
         - Keep `.vox/cache/` and `.vox/active-repository.json` gitignored (local machine state).\n\
         - Respect `.voxignore` when indexing or searching files.\n"
    )
}

fn render_voxignore() -> &'static str {
    ".git/\n\
     .vox/cache/\n\
     .vox/active-repository.json\n\
     target/\n\
     node_modules/\n\
     dist/\n\
     build/\n\
     .venv/\n\
     __pycache__/\n"
}

fn render_llms_txt(display_name: &str, stacks: &str) -> String {
    format!(
        "# {display_name}\n\n\
         > Stack: {stacks}\n\n\
         - [VOX.md](/VOX.md): Universal Vox repository context and storage layout\n\
         - [AGENTS.md](/AGENTS.md): Agent operating rules for this repository\n\
         - [OpenWiki](/docs/wiki/README.md): Repository wiki landing page\n"
    )
}

fn render_wiki_readme(ctx: &crate::RepositoryContext, display_name: &str) -> String {
    let stacks = stack_labels(ctx).join(", ");
    format!(
        "---\n\
         title: \"{display_name} OpenWiki Hub\"\n\
         description: \"Five-layer OpenWiki documentation hub and module map for {display_name}.\"\n\
         category: \"Architecture SSOTs\"\n\
         status: \"current\"\n\
         topic: \"doc-pipeline\"\n\
         domain: \"documentation\"\n\
         authorship: \"llm-generated\"\n\
         authorship_model: \"vox-openwiki-v7.9\"\n\
         training_eligible: false\n\
         ---\n\n\
         # {display_name} OpenWiki\n\n\
         <!-- kb:summary -->\n\
         This OpenWiki hub indexes the deterministic architecture, interface, schema, frontend surface, and workflow documentation for {display_name}. Automatically synchronized machine-readable regions reside inside explicit auto-fences so verified code locations, source line anchors, package manifests, cross-layer relationships, and VoxDiagram topologies stay current across repository updates while human engineering rationale remains completely preserved.\n\
         <!-- /kb:summary -->\n\n\
         <!-- openwiki:auto:start id=\"repo-overview\" -->\n\
         - **Repository ID:** `{repo_id}`\n\
         - **Detected Stack:** {stacks}\n\
         - **Harness Config:** `.vox/repositories.yaml`\n\
         <!-- openwiki:auto:end id=\"repo-overview\" -->\n\n\
         ## Modules & Architecture\n\n\
         Document key modules, workflows, and architectural decisions under `docs/wiki/`.\n",
        repo_id = ctx.repository_id,
    )
}

/// Onboard any repository with the Vox hybrid harness (`VOX.md`, `AGENTS.md`, `.voxignore`,
/// `.well-known/llms.txt`, `docs/wiki/`, `.vox/repositories.yaml`, `.vox/cache/{kb,graphify,openwiki}`),
/// or inspect in strict zero-footprint `read_only` mode redirecting state to `<home>/.vox/repos/<id>/`.
pub fn onboard_repository_harness_with_home(
    repo_root: &Path,
    read_only: bool,
    home_dir: &Path,
) -> Result<HarnessOnboardSummary, RepoCatalogError> {
    let workspace = crate::discover_repository_or_fallback(repo_root);
    let global_lib = global_library_root_under(home_dir);
    ensure_dir(&global_lib)?;

    let display_name = workspace.root.file_name().map_or_else(
        || "repository".to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let stacks = stack_labels(&workspace).join(", ");

    let mut created_paths = Vec::new();
    let mut existing_paths = Vec::new();
    let mut cache_dirs = Vec::new();

    if read_only {
        let ext_root = external_repo_cache_root_under(home_dir, &workspace.repository_id);
        let cache_root = ext_root.join("cache");
        for sub in ["kb", "graphify", "openwiki"] {
            let dir = cache_root.join(sub);
            ensure_dir(&dir)?;
            cache_dirs.push(dir.display().to_string());
        }
        let _ = vox_config::graphify::provision_repo_graphify_corpora(
            &cache_root,
            &workspace.repository_id,
        );
        let ext_manifest = ext_root.join(REPO_CATALOG_BASENAME);
        if !ext_manifest.exists() {
            let catalog = RepoCatalog {
                schema_version: REPO_CATALOG_SCHEMA_VERSION,
                primary_repository_id: Some(workspace.repository_id.clone()),
                repositories: vec![RepositoryDescriptor {
                    display_name,
                    repository_id: Some(workspace.repository_id.clone()),
                    root_path: Some(workspace.root.display().to_string()),
                    origin_url: workspace.origin_url.clone(),
                    provider: detect_provider(workspace.origin_url.as_deref(), None),
                    access_mode: RepoAccessMode::ReadOnlyLocal,
                    read_only: true,
                    external_cache_dir: Some(ext_root.display().to_string()),
                    capabilities: vec![
                        RepoCapability::ReadFile,
                        RepoCapability::ListFiles,
                        RepoCapability::TextSearch,
                        RepoCapability::SemanticSearch,
                        RepoCapability::HistorySearch,
                    ],
                    ..Default::default()
                }],
            };
            let yaml = serde_yaml::to_string(&catalog)?;
            std::fs::write(&ext_manifest, yaml).map_err(|source| {
                RepoCatalogError::WriteManifest {
                    path: ext_manifest.display().to_string(),
                    source,
                }
            })?;
            created_paths.push(ext_manifest.display().to_string());
        } else {
            existing_paths.push(ext_manifest.display().to_string());
        }

        return Ok(HarnessOnboardSummary {
            repository_id: workspace.repository_id,
            root_path: workspace.root.display().to_string(),
            read_only: true,
            created_paths,
            existing_paths,
            cache_dirs,
            global_library_dir: global_lib.display().to_string(),
            catalog_path: ext_manifest.display().to_string(),
        });
    }

    let cache_root = workspace.root.join(".vox").join("cache");
    for sub in ["kb", "graphify", "openwiki"] {
        let dir = cache_root.join(sub);
        ensure_dir(&dir)?;
        cache_dirs.push(dir.display().to_string());
    }
    let _ = vox_config::graphify::provision_repo_graphify_corpora(
        &cache_root,
        &workspace.repository_id,
    );

    write_if_missing(
        &workspace.root.join(".vox").join(".gitignore"),
        "cache/\nactive-repository.json\n",
        &mut created_paths,
        &mut existing_paths,
    )?;
    write_if_missing(
        &workspace.root.join("VOX.md"),
        &render_vox_md(&workspace, &display_name),
        &mut created_paths,
        &mut existing_paths,
    )?;
    write_if_missing(
        &workspace.root.join("AGENTS.md"),
        &render_agents_md(&display_name),
        &mut created_paths,
        &mut existing_paths,
    )?;
    write_if_missing(
        &workspace.root.join(".voxignore"),
        render_voxignore(),
        &mut created_paths,
        &mut existing_paths,
    )?;
    write_if_missing(
        &workspace.root.join(".well-known").join("llms.txt"),
        &render_llms_txt(&display_name, &stacks),
        &mut created_paths,
        &mut existing_paths,
    )?;
    write_if_missing(
        &workspace.root.join("docs").join("wiki").join("README.md"),
        &render_wiki_readme(&workspace, &display_name),
        &mut created_paths,
        &mut existing_paths,
    )?;

    let manifest_path = repo_catalog_manifest_path(&workspace.root);
    if !manifest_path.exists() {
        let catalog = RepoCatalog {
            schema_version: REPO_CATALOG_SCHEMA_VERSION,
            primary_repository_id: Some(workspace.repository_id.clone()),
            repositories: vec![RepositoryDescriptor {
                display_name,
                repository_id: Some(workspace.repository_id.clone()),
                root_path: Some(".".to_string()),
                origin_url: workspace.origin_url.clone(),
                provider: detect_provider(workspace.origin_url.as_deref(), None),
                access_mode: RepoAccessMode::Local,
                read_only: false,
                capabilities: vec![
                    RepoCapability::ReadFile,
                    RepoCapability::ListFiles,
                    RepoCapability::TextSearch,
                    RepoCapability::SemanticSearch,
                    RepoCapability::HistorySearch,
                ],
                ..Default::default()
            }],
        };
        save_repo_catalog(&workspace.root, &catalog)?;
        created_paths.push(manifest_path.display().to_string());
    } else {
        existing_paths.push(manifest_path.display().to_string());
    }

    Ok(HarnessOnboardSummary {
        repository_id: workspace.repository_id,
        root_path: workspace.root.display().to_string(),
        read_only: false,
        created_paths,
        existing_paths,
        cache_dirs,
        global_library_dir: global_lib.display().to_string(),
        catalog_path: manifest_path.display().to_string(),
    })
}

/// Onboard a repository using the OS user home directory (`~/.vox/...`).
pub fn onboard_repository_harness(
    repo_root: &Path,
    read_only: bool,
) -> Result<HarnessOnboardSummary, RepoCatalogError> {
    let home = vox_user_home_dir()?;
    onboard_repository_harness_with_home(repo_root, read_only, &home)
}

/// Add (or update) a repository entry in `workspace_root/.vox/repositories.yaml`.
pub fn add_repository_to_catalog_with_home(
    workspace_root: &Path,
    target_root: &Path,
    display_name: Option<&str>,
    read_only: bool,
    home_dir: &Path,
) -> Result<ResolvedRepoCatalog, RepoCatalogError> {
    let workspace = crate::discover_repository_or_fallback(workspace_root);
    let target = crate::discover_repository_or_fallback(target_root);

    let mut catalog = match load_repo_catalog_from_repo(&workspace.root) {
        Ok(c) => c,
        Err(RepoCatalogError::MissingManifest { .. }) => {
            implicit_workspace_catalog(&workspace.root)
        }
        Err(e) => return Err(e),
    };

    if catalog.primary_repository_id.is_none() {
        catalog.primary_repository_id = Some(workspace.repository_id.clone());
    }

    let root_path = if target.root == workspace.root {
        ".".to_string()
    } else if let Ok(rel) = target.root.strip_prefix(&workspace.root) {
        let s = rel.to_string_lossy().replace('\\', "/");
        if s.is_empty() { ".".to_string() } else { s }
    } else {
        target.root.display().to_string()
    };

    let external_cache_dir = if read_only {
        onboard_repository_harness_with_home(&target.root, true, home_dir)?;
        Some(
            external_repo_cache_root_under(home_dir, &target.repository_id)
                .display()
                .to_string(),
        )
    } else {
        None
    };

    let resolved_name = display_name
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            target.root.file_name().map_or_else(
                || "repository".to_string(),
                |n| n.to_string_lossy().into_owned(),
            )
        });

    let descriptor = RepositoryDescriptor {
        display_name: resolved_name,
        repository_id: Some(target.repository_id.clone()),
        root_path: Some(root_path.clone()),
        origin_url: target.origin_url.clone(),
        provider: detect_provider(target.origin_url.as_deref(), None),
        access_mode: if read_only {
            RepoAccessMode::ReadOnlyLocal
        } else {
            RepoAccessMode::Local
        },
        read_only,
        external_cache_dir,
        capabilities: vec![
            RepoCapability::ReadFile,
            RepoCapability::ListFiles,
            RepoCapability::TextSearch,
            RepoCapability::SemanticSearch,
            RepoCapability::HistorySearch,
        ],
        ..Default::default()
    };

    if let Some(existing) = catalog.repositories.iter_mut().find(|entry| {
        entry.repository_id.as_deref() == Some(target.repository_id.as_str())
            || entry.root_path.as_deref() == Some(root_path.as_str())
    }) {
        *existing = descriptor;
    } else {
        catalog.repositories.push(descriptor);
    }

    save_repo_catalog(&workspace.root, &catalog)?;
    resolve_repo_catalog(&workspace.root)
}

/// Add (or update) a repository entry in `workspace_root/.vox/repositories.yaml`.
pub fn add_repository_to_catalog(
    workspace_root: &Path,
    target_root: &Path,
    display_name: Option<&str>,
    read_only: bool,
) -> Result<ResolvedRepoCatalog, RepoCatalogError> {
    let home = vox_user_home_dir()?;
    add_repository_to_catalog_with_home(workspace_root, target_root, display_name, read_only, &home)
}

/// Remove a repository entry by id, display_name, or root_path from `.vox/repositories.yaml`.
pub fn remove_repository_from_catalog(
    workspace_root: &Path,
    target: &str,
) -> Result<ResolvedRepoCatalog, RepoCatalogError> {
    let workspace = crate::discover_repository_or_fallback(workspace_root);
    let mut catalog = match load_repo_catalog_from_repo(&workspace.root) {
        Ok(c) => c,
        Err(RepoCatalogError::MissingManifest { .. }) => {
            return Err(RepoCatalogError::RepositoryNotFound {
                target: target.to_string(),
            });
        }
        Err(e) => return Err(e),
    };

    let target_trimmed = target.trim();
    let before_len = catalog.repositories.len();
    let mut removed_ids = Vec::new();
    catalog.repositories.retain(|entry| {
        let matches = entry.repository_id.as_deref() == Some(target_trimmed)
            || entry.display_name == target_trimmed
            || entry.root_path.as_deref() == Some(target_trimmed);
        if matches && let Some(id) = &entry.repository_id {
            removed_ids.push(id.clone());
        }
        !matches
    });

    if catalog.repositories.len() == before_len {
        return Err(RepoCatalogError::RepositoryNotFound {
            target: target.to_string(),
        });
    }

    if let Some(primary) = &catalog.primary_repository_id
        && removed_ids.iter().any(|id| id == primary)
    {
        catalog.primary_repository_id = catalog
            .repositories
            .iter()
            .find_map(|r| r.repository_id.clone());
    }

    save_repo_catalog(&workspace.root, &catalog)?;
    resolve_repo_catalog(&workspace.root)
}

fn active_repo_state_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join(".vox").join("active-repository.json")
}

fn active_context_from_descriptor(
    desc: &ResolvedRepositoryDescriptor,
    home_dir: &Path,
) -> ActiveRepoContext {
    let repo_id = desc
        .repository_id
        .clone()
        .or_else(|| desc.declared_repository_id.clone())
        .unwrap_or_else(|| "default".to_string());
    let root_path = desc
        .resolved_root
        .clone()
        .or_else(|| desc.root_path.clone())
        .unwrap_or_else(|| ".".to_string());
    let is_ro = desc.read_only || matches!(desc.access_mode, RepoAccessMode::ReadOnlyLocal);
    let cache_root = if is_ro {
        desc.external_cache_dir
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| external_repo_cache_root_under(home_dir, &repo_id))
            .join("cache")
    } else {
        PathBuf::from(&root_path).join(".vox").join("cache")
    };
    ActiveRepoContext {
        repository_id: repo_id,
        display_name: desc.display_name.clone(),
        root_path,
        access_mode: desc.access_mode.clone(),
        read_only: is_ro,
        kb_cache_dir: cache_root.join("kb").display().to_string(),
        graphify_cache_dir: cache_root.join("graphify").display().to_string(),
        openwiki_cache_dir: cache_root.join("openwiki").display().to_string(),
        global_library_dir: global_library_root_under(home_dir).display().to_string(),
        updated_at_ms: now_unix_ms(),
    }
}

/// Switch the active repository context stored in `<workspace_root>/.vox/active-repository.json`.
pub fn switch_active_repository_with_home(
    workspace_root: &Path,
    target: &str,
    home_dir: &Path,
) -> Result<ActiveRepoContext, RepoCatalogError> {
    let workspace = crate::discover_repository_or_fallback(workspace_root);
    let catalog = resolve_repo_catalog(&workspace.root)?;
    let needle = target.trim();

    let found = catalog.repositories.iter().find(|repo| {
        repo.repository_id.as_deref() == Some(needle)
            || repo.declared_repository_id.as_deref() == Some(needle)
            || repo.display_name == needle
            || repo.root_path.as_deref() == Some(needle)
            || repo.resolved_root.as_deref() == Some(needle)
    });

    let desc = found.ok_or_else(|| RepoCatalogError::RepositoryNotFound {
        target: target.to_string(),
    })?;

    let ctx = active_context_from_descriptor(desc, home_dir);
    let state_path = active_repo_state_path(&workspace.root);
    if let Some(parent) = state_path.parent() {
        ensure_dir(parent)?;
    }
    let json = serde_json::to_string_pretty(&ctx)?;
    std::fs::write(&state_path, json).map_err(|source| RepoCatalogError::WriteSnapshot {
        path: state_path.display().to_string(),
        source,
    })?;
    Ok(ctx)
}

/// Switch the active repository context using the OS user home directory.
pub fn switch_active_repository(
    workspace_root: &Path,
    target: &str,
) -> Result<ActiveRepoContext, RepoCatalogError> {
    let home = vox_user_home_dir()?;
    switch_active_repository_with_home(workspace_root, target, &home)
}

/// Read the active repository context, or synthesize a fallback context from the workspace root.
pub fn get_active_repository_context_with_home(
    workspace_root: &Path,
    home_dir: &Path,
) -> Result<ActiveRepoContext, RepoCatalogError> {
    let workspace = crate::discover_repository_or_fallback(workspace_root);
    let state_path = active_repo_state_path(&workspace.root);
    if state_path.is_file()
        && let Ok(raw) = std::fs::read_to_string(&state_path)
        && let Ok(parsed) = serde_json::from_str::<ActiveRepoContext>(&raw)
    {
        return Ok(parsed);
    }

    let catalog = resolve_repo_catalog(&workspace.root)?;
    if let Some(primary) = catalog
        .primary_repository_id
        .as_deref()
        .and_then(|id| {
            catalog
                .repositories
                .iter()
                .find(|r| r.repository_id.as_deref() == Some(id))
        })
        .or_else(|| catalog.repositories.first())
    {
        return Ok(active_context_from_descriptor(primary, home_dir));
    }

    let cache_root = workspace.root.join(".vox").join("cache");
    let display_name = workspace.root.file_name().map_or_else(
        || "workspace".to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    Ok(ActiveRepoContext {
        repository_id: workspace.repository_id,
        display_name,
        root_path: workspace.root.display().to_string(),
        access_mode: RepoAccessMode::Local,
        read_only: false,
        kb_cache_dir: cache_root.join("kb").display().to_string(),
        graphify_cache_dir: cache_root.join("graphify").display().to_string(),
        openwiki_cache_dir: cache_root.join("openwiki").display().to_string(),
        global_library_dir: global_library_root_under(home_dir).display().to_string(),
        updated_at_ms: now_unix_ms(),
    })
}

/// Read the active repository context using the OS user home directory.
pub fn get_active_repository_context(
    workspace_root: &Path,
) -> Result<ActiveRepoContext, RepoCatalogError> {
    let home = vox_user_home_dir()?;
    get_active_repository_context_with_home(workspace_root, &home)
}

#[cfg(test)]
mod catalog_unit_tests {
    use super::*;

    #[test]
    fn implicit_catalog_is_one_local_entry_at_the_workspace_root() {
        let catalog = implicit_workspace_catalog(Path::new("/work/demo"));
        assert_eq!(catalog.repositories.len(), 1);
        let repo = &catalog.repositories[0];
        assert_eq!(repo.display_name, "demo");
        assert_eq!(repo.root_path.as_deref(), Some("."));
        assert_eq!(repo.access_mode, RepoAccessMode::Local);
    }

    #[test]
    fn manifest_path_lives_under_dot_vox() {
        let path = repo_catalog_manifest_path(Path::new("/work/demo"));
        assert!(path.ends_with(Path::new(".vox").join(REPO_CATALOG_BASENAME)));
    }
}
