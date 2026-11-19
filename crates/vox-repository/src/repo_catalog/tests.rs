use super::{
    QueryFileParams, QueryTextParams, RepoAccessMode, add_repository_to_catalog_with_home,
    external_repo_cache_root_under, get_active_repository_context_with_home,
    onboard_repository_harness_with_home, remove_repository_from_catalog, repo_query_file,
    repo_query_text, resolve_repo_catalog, switch_active_repository_with_home,
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

#[test]
fn onboard_repository_harness_creates_expected_layout_and_is_idempotent() {
    let workspace = tempdir().expect("workspace");
    let home = tempdir().expect("home");
    fs::create_dir_all(workspace.path().join(".git")).expect("git");
    fs::write(
        workspace.path().join("Cargo.toml"),
        "[package]\nname = \"demo-harness\"\nversion = \"0.1.0\"\n",
    )
    .expect("cargo toml");

    let first = onboard_repository_harness_with_home(workspace.path(), false, home.path())
        .expect("onboard");
    assert!(!first.read_only);
    assert!(workspace.path().join("VOX.md").is_file());
    assert!(workspace.path().join("AGENTS.md").is_file());
    assert!(workspace.path().join(".voxignore").is_file());
    assert!(workspace.path().join(".well-known/llms.txt").is_file());
    assert!(workspace.path().join("docs/wiki/README.md").is_file());
    assert!(workspace.path().join(".vox/repositories.yaml").is_file());
    assert!(workspace.path().join(".vox/cache/kb").is_dir());
    assert!(workspace.path().join(".vox/cache/graphify").is_dir());
    assert!(workspace.path().join(".vox/cache/openwiki").is_dir());
    assert!(home.path().join(".vox/library").is_dir());

    fs::write(workspace.path().join("VOX.md"), "# custom user notes\n").expect("custom vox.md");
    let second =
        onboard_repository_harness_with_home(workspace.path(), false, home.path()).expect("rerun");
    assert!(second.created_paths.is_empty());
    assert_eq!(
        fs::read_to_string(workspace.path().join("VOX.md")).expect("read vox.md"),
        "# custom user notes\n"
    );
}

#[test]
fn onboard_repository_harness_read_only_leaves_external_repo_untouched() {
    let external = tempdir().expect("external");
    let home = tempdir().expect("home");
    fs::write(external.path().join("README.md"), "# External Project\n").expect("readme");

    let summary =
        onboard_repository_harness_with_home(external.path(), true, home.path()).expect("onboard");
    assert!(summary.read_only);
    assert!(!external.path().join("VOX.md").exists());
    assert!(!external.path().join("AGENTS.md").exists());
    assert!(!external.path().join(".vox").exists());
    assert!(!external.path().join("docs/wiki").exists());

    let ext_root = external_repo_cache_root_under(home.path(), &summary.repository_id);
    assert!(ext_root.join("repositories.yaml").is_file());
    assert!(ext_root.join("cache/kb").is_dir());
    assert!(ext_root.join("cache/graphify").is_dir());
    assert!(ext_root.join("cache/openwiki").is_dir());
}

#[test]
fn catalog_add_switch_and_remove_persist_active_context() {
    let primary = tempdir().expect("primary");
    let external = tempdir().expect("external");
    let home = tempdir().expect("home");

    fs::create_dir_all(primary.path().join(".git")).expect("primary git");
    fs::create_dir_all(external.path().join(".git")).expect("ext git");
    fs::write(
        external.path().join("README.md"),
        "# Satellite\nhello from ext\n",
    )
    .expect("readme");

    onboard_repository_harness_with_home(primary.path(), false, home.path()).expect("onboard");
    let added = add_repository_to_catalog_with_home(
        primary.path(),
        external.path(),
        Some("ext-sat"),
        true,
        home.path(),
    )
    .expect("add ext");
    assert_eq!(added.repositories.len(), 2);

    // Querying across the catalog works on ReadOnlyLocal repositories.
    let txt = repo_query_text(
        primary.path(),
        &QueryTextParams {
            query: "hello from ext".to_string(),
            repository_ids: None,
            max_matches_per_repo: 5,
            ..QueryTextParams::default()
        },
        None,
    )
    .expect("query ext");
    assert_eq!(txt.hits.len(), 1);
    assert_eq!(txt.hits[0].display_name, "ext-sat");

    let switched =
        switch_active_repository_with_home(primary.path(), "ext-sat", home.path()).expect("switch");
    assert!(switched.read_only);
    assert_eq!(switched.display_name, "ext-sat");
    assert!(!external.path().join(".vox").exists());

    let loaded =
        get_active_repository_context_with_home(primary.path(), home.path()).expect("get active");
    assert_eq!(loaded.repository_id, switched.repository_id);

    let after_remove =
        remove_repository_from_catalog(primary.path(), "ext-sat").expect("remove ext");
    assert_eq!(after_remove.repositories.len(), 1);
}
