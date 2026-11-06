use super::{
    QueryFileParams, QueryTextParams, RepoAccessMode, repo_query_file, repo_query_text,
    resolve_repo_catalog,
};
use std::fs;
use tempfile::tempdir;

#[test]
fn resolves_local_catalog_entries_against_workspace_root() {
    let workspace = tempdir().expect("workspace");
    let repo_a = workspace.path().join("repo-a");
    fs::create_dir_all(repo_a.join(".git")).expect("git dir");
    fs::write(
        repo_a.join(".git").join("config"),
        "[remote \"origin\"]\n\turl = https://github.com/example/repo-a.git\n",
    )
    .expect("git config");
    fs::create_dir_all(workspace.path().join(".vox")).expect("dot vox");
    fs::write(
        workspace.path().join(".vox").join("repositories.yaml"),
        r#"schema_version: 1
repositories:
  - display_name: repo-a
    repository_id: null
    root_path: repo-a
    access_mode: local
    capabilities: [read_file, text_search, history_search]
"#,
    )
    .expect("catalog");

    let resolved = resolve_repo_catalog(workspace.path()).expect("resolve");
    assert_eq!(resolved.repositories.len(), 1);
    let repo = &resolved.repositories[0];
    assert_eq!(repo.display_name, "repo-a");
    assert_eq!(repo.resolution_status, "resolved_local");
    assert!(repo.repository_id.is_some());
    assert_eq!(repo.provider.as_deref(), Some("github"));
}

#[test]
fn text_query_groups_hits_by_repository() {
    let workspace = tempdir().expect("workspace");
    let repo_a = workspace.path().join("repo-a");
    fs::create_dir_all(repo_a.join(".git")).expect("git dir");
    fs::write(repo_a.join("lib.rs"), "fn alpha() {}\nfn beta() {}\n").expect("source");
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

    let response = repo_query_text(
        workspace.path(),
        &QueryTextParams {
            query: "alpha".to_string(),
            ..QueryTextParams::default()
        },
        None,
    )
    .expect("query");
    assert_eq!(response.result_count, 1);
    assert_eq!(response.repositories_queried, 1);
    assert_eq!(response.hits[0].display_name, "repo-a");
    assert!(response.trace.trace_id.starts_with("xrepo:"));
}

#[test]
fn workspace_without_a_manifest_resolves_to_itself() {
    let workspace = tempdir().expect("workspace");
    fs::create_dir_all(workspace.path().join(".git")).expect("git dir");
    let resolved = resolve_repo_catalog(workspace.path()).expect("implicit catalog");
    assert_eq!(resolved.repositories.len(), 1);
    assert_eq!(resolved.repositories[0].resolution_status, "resolved_local");
    assert_eq!(resolved.repositories[0].access_mode, RepoAccessMode::Local);
}

#[test]
fn file_query_reads_the_workspace_when_no_manifest_exists() {
    let workspace = tempdir().expect("workspace");
    fs::create_dir_all(workspace.path().join(".git")).expect("git dir");
    fs::write(workspace.path().join("Cargo.toml"), "a\nb\nc\n").expect("file");
    let params = QueryFileParams {
        path: "Cargo.toml".to_string(),
        ..QueryFileParams::default()
    };
    let response = repo_query_file(workspace.path(), &params, None).expect("query");
    assert_eq!(response.files.len(), 1, "{:?}", response.skipped);
    assert_eq!(response.files[0].content, "a\nb\nc\n");
    assert_eq!(response.files[0].total_lines, 3);
    assert_eq!(response.files[0].total_bytes, 6);
    let escape = QueryFileParams {
        path: "../outside.txt".to_string(),
        ..QueryFileParams::default()
    };
    let response = repo_query_file(workspace.path(), &escape, None).expect("query");
    assert!(response.files.is_empty(), "path escape must stay sandboxed");
}

#[test]
fn an_unparseable_manifest_is_still_an_error() {
    let workspace = tempdir().expect("workspace");
    fs::create_dir_all(workspace.path().join(".vox")).expect("dot vox");
    fs::write(
        workspace.path().join(".vox").join("repositories.yaml"),
        "repositories: [not, a, mapping",
    )
    .expect("catalog");
    assert!(resolve_repo_catalog(workspace.path()).is_err());
}
