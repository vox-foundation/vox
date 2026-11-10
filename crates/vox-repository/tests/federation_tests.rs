use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use vox_repository::*;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn test_verify_cross_repo_contract_parity_real_workspace() {
    let root = workspace_root();
    let report = verify_cross_repo_contract_parity(&root).expect("parity check failed");
    assert!(report.total_repositories >= 3);
    assert!(report.present_repositories >= 3);
    if root.join("../vox-ops").exists() {
        assert!(
            !report
                .skipped_optional_repositories
                .contains(&"vox-ops".to_string())
        );
        assert_eq!(report.present_repositories, 4);
    } else {
        assert!(
            report
                .skipped_optional_repositories
                .contains(&"vox-ops".to_string())
        );
    }
    assert!(report.in_parity, "drifts found: {:?}", report.drifts);
    assert!(report.drifts.is_empty());
}

#[test]
fn test_render_local_cargo_patch_block() {
    let root = workspace_root();
    let enabled = render_local_cargo_patch_block(&root, true).expect("render enabled");
    assert!(enabled.contains("[patch.crates-io]"));
    assert!(enabled.contains("vox-ops = { path = \"../vox-ops\" }"));

    let disabled = render_local_cargo_patch_block(&root, false).expect("render disabled");
    assert!(disabled.contains("# [patch.crates-io]"));
    assert!(disabled.contains("# vox-ops = { path = \"../vox-ops\" }"));
}

#[test]
fn test_refresh_federated_symbol_graph() {
    let root = workspace_root();
    let summary = refresh_federated_symbol_graph(&root).expect("graph refresh failed");
    assert!(summary.repositories_indexed.contains(&"vox".to_string()));
    assert!(
        summary
            .repositories_indexed
            .contains(&"vox-axis".to_string())
    );
    assert!(
        summary
            .repositories_indexed
            .contains(&"voxlang.org".to_string())
    );
    if root.join("../vox-ops").exists() {
        assert!(
            summary
                .repositories_indexed
                .contains(&"vox-ops".to_string())
        );
        assert!(
            !summary
                .skipped_repositories
                .contains(&"vox-ops".to_string())
        );
    } else {
        assert!(
            summary
                .skipped_repositories
                .contains(&"vox-ops".to_string())
        );
    }
    assert!(summary.total_nodes > 0);
    assert!(
        summary
            .crate_nodes
            .iter()
            .any(|c| c.name == "vox-repository")
    );
    assert!(summary.crate_nodes.iter().any(|c| c.name == "vox-gui"));
    assert!(summary.edges.iter().any(|e| e.kind == "shared_contract"));
    assert!(summary.edges.iter().any(|e| e.kind == "cargo_patch"));
}

#[test]
fn test_verify_standalone_sovereignty_real_workspace() {
    let root = workspace_root();
    let report = verify_standalone_sovereignty(&root).expect("sovereignty verification failed");
    assert!(report.is_sovereign, "violations: {:?}", report.violations);
    assert!(report.root_cargo_has_no_ops_dep);
    assert!(report.root_cargo_default_members_ok);
    assert_eq!(report.default_members, vec!["crates/vox-cli".to_string()]);
    assert!(report.telemetry_offline_safe);
    assert!(report.vox_gui_manifest_ok);
    assert!(report.docs_astro_manifest_ok);
    assert!(report.violations.is_empty());
}

#[test]
fn test_contract_parity_detects_drift() {
    let dir = tempdir().unwrap();
    let repo_a = dir.path().join("repo-a");
    let repo_b = dir.path().join("repo-b");
    fs::create_dir_all(repo_a.join(".git")).unwrap();
    fs::create_dir_all(repo_b.join(".git")).unwrap();
    fs::create_dir_all(repo_a.join("contracts/wire")).unwrap();
    fs::create_dir_all(repo_b.join("contracts/wire")).unwrap();

    fs::write(repo_a.join("contracts/wire/proto.json"), b"{\"v\": 1}").unwrap();
    fs::write(repo_b.join("contracts/wire/proto.json"), b"{\"v\": 2}").unwrap();

    fs::create_dir_all(dir.path().join(".vox")).unwrap();
    fs::write(
        dir.path().join(".vox/repositories.yaml"),
        r#"schema_version: 1
repositories:
  - repository_id: repo-a
    display_name: repo-a
    root_path: repo-a
    access_mode: local
    shared_contracts: [contracts/wire/]
  - repository_id: repo-b
    display_name: repo-b
    root_path: repo-b
    access_mode: local
    shared_contracts: [contracts/wire/]
"#,
    )
    .unwrap();

    let report = verify_cross_repo_contract_parity(dir.path()).expect("check");
    assert!(!report.in_parity);
    assert_eq!(report.drifts.len(), 1);
    assert_eq!(
        report.drifts[0].relative_contract_path,
        "contracts/wire/proto.json"
    );
    assert_eq!(report.drifts[0].repo_a, "repo-a");
    assert_eq!(report.drifts[0].repo_b, "repo-b");
}
