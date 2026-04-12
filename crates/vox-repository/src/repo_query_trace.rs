//! Cross-repo query helpers that stamp `CrossRepoQueryTrace.source_plane` for CLI vs MCP parity.

use std::path::Path;

use crate::{
    QueryFileParams, QueryHistoryParams, QueryTextParams, RepoCatalogError, RepoFileReadResponse,
    RepoHistoryResponse, RepoTextSearchResponse, repo_query_file, repo_query_history,
    repo_query_text,
};

pub fn repo_query_text_with_plane(
    workspace_root: &Path,
    params: &QueryTextParams,
    source_plane: &str,
    cached_catalog: Option<&crate::ResolvedRepoCatalog>,
) -> Result<RepoTextSearchResponse, RepoCatalogError> {
    let mut r = repo_query_text(workspace_root, params, cached_catalog)?;
    r.trace.source_plane = source_plane.to_string();
    Ok(r)
}

pub fn repo_query_file_with_plane(
    workspace_root: &Path,
    params: &QueryFileParams,
    source_plane: &str,
    cached_catalog: Option<&crate::ResolvedRepoCatalog>,
) -> Result<RepoFileReadResponse, RepoCatalogError> {
    let mut r = repo_query_file(workspace_root, params, cached_catalog)?;
    r.trace.source_plane = source_plane.to_string();
    Ok(r)
}

pub fn repo_query_history_with_plane(
    workspace_root: &Path,
    params: &QueryHistoryParams,
    source_plane: &str,
    cached_catalog: Option<&crate::ResolvedRepoCatalog>,
) -> Result<RepoHistoryResponse, RepoCatalogError> {
    let mut r = repo_query_history(workspace_root, params, cached_catalog)?;
    r.trace.source_plane = source_plane.to_string();
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn query_text_with_plane_overrides_trace() {
        let workspace = tempdir().expect("workspace");
        let repo_a = workspace.path().join("repo-a");
        fs::create_dir_all(repo_a.join(".git")).expect("git dir");
        fs::write(repo_a.join("lib.rs"), "fn alpha() {}\n").expect("source");
        fs::create_dir_all(workspace.path().join(".vox")).expect("dot vox");
        fs::write(
            workspace.path().join(".vox").join("repositories.yaml"),
            r#"schema_version: 1
repositories:
  - display_name: repo-a
    repository_id: null
    root_path: repo-a
    access_mode: local
    capabilities: [read_file, list_files, text_search]
"#,
        )
        .expect("catalog");

        let response = repo_query_text_with_plane(
            workspace.path(),
            &QueryTextParams {
                query: "alpha".to_string(),
                ..QueryTextParams::default()
            },
            "integration-plane",
            None,
        )
        .expect("query");
        assert_eq!(response.trace.source_plane, "integration-plane");
    }
}
