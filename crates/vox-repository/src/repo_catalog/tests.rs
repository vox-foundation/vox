use super::{QueryTextParams, repo_query_text, resolve_repo_catalog};
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
    )
    .expect("query");
    assert_eq!(response.result_count, 1);
    assert_eq!(response.repositories_queried, 1);
    assert_eq!(response.hits[0].display_name, "repo-a");
    assert!(response.trace.trace_id.starts_with("xrepo:"));
}
