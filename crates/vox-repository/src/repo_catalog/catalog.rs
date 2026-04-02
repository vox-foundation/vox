use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::{
    REPO_CATALOG_BASENAME, REPO_CATALOG_SCHEMA_VERSION, RepoAccessMode, RepoCapability,
    RepoCatalog, RepoCatalogRefreshResult, RepositoryDescriptor, ResolvedRepoCatalog,
    ResolvedRepositoryDescriptor,
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
    #[error("invalid regex query {pattern:?}: {message}")]
    InvalidRegex { pattern: String, message: String },
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
            RepoAccessMode::Local => vec![
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
    let mut out = Vec::new();
    for cap in caps {
        let key = format!("{cap:?}");
        if seen.insert(key) {
            out.push(cap);
        }
    }
    out
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

fn resolve_descriptor_root(workspace_root: &Path, root_path: &str) -> PathBuf {
    let path = Path::new(root_path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    }
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
        access_mode: RepoAccessMode::Local,
        capabilities,
        remote: repo.remote,
        metadata: repo.metadata,
        resolution_status: "resolved_local".to_string(),
        resolution_error: None,
        repository_id_mismatch: mismatch,
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
        access_mode: RepoAccessMode::Local,
        capabilities,
        remote: repo.remote,
        metadata: repo.metadata,
        resolution_status: resolution_status.to_string(),
        resolution_error: Some(resolution_error),
        repository_id_mismatch: None,
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
    }
}

pub fn resolve_repo_catalog(repo_root: &Path) -> Result<ResolvedRepoCatalog, RepoCatalogError> {
    let workspace = crate::discover_repository_or_fallback(repo_root);
    let catalog = load_repo_catalog_from_repo(&workspace.root)?;
    let manifest_path = repo_catalog_manifest_path(&workspace.root);
    let mut repositories = Vec::with_capacity(catalog.repositories.len());

    for repo in catalog.repositories {
        let declared_id = repo.repository_id.clone();
        let provider = detect_provider(repo.origin_url.as_deref(), repo.provider.as_deref());
        let capabilities = normalize_capabilities(repo.capabilities.clone(), &repo.access_mode);
        match repo.access_mode.clone() {
            RepoAccessMode::Local => {
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
                let root_candidate = resolve_descriptor_root(&workspace.root, &root_path);
                match crate::discover_repository(&root_candidate) {
                    Ok(resolved) => repositories.push(resolved_local_descriptor(
                        repo,
                        declared_id,
                        root_path,
                        capabilities,
                        resolved,
                    )),
                    Err(e) => repositories.push(unresolved_local_descriptor(
                        repo,
                        declared_id,
                        Some(root_path),
                        capabilities,
                        provider,
                        "local_resolution_failed",
                        e.to_string(),
                    )),
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

    Ok(ResolvedRepoCatalog {
        schema_version: catalog.schema_version.max(REPO_CATALOG_SCHEMA_VERSION),
        manifest_path: manifest_path.display().to_string(),
        repositories,
    })
}

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
