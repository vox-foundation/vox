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
        repo_catalog_manifest_path(dir.path()),
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

#[test]
fn test_sync_federated_satellites_and_mirror_parity() {
    let dir = tempdir().expect("tempdir");
    let vox_root = dir.path().join("vox");
    let axis_root = dir.path().join("vox-axis");

    fs::create_dir_all(vox_root.join(".vox")).unwrap();
    fs::create_dir_all(vox_root.join("contracts/gui")).unwrap();
    fs::create_dir_all(vox_root.join("crates/vox-gui")).unwrap();
    fs::create_dir_all(axis_root.join("contracts/gui")).unwrap();
    fs::create_dir_all(axis_root.join("crates/vox-gui")).unwrap();

    fs::write(
        repo_catalog_manifest_path(&vox_root),
        r#"schema_version: 1
repositories:
  - repository_id: vox
    display_name: vox
    root_path: .
    shared_contracts:
      - contracts/gui
  - repository_id: vox-axis
    display_name: vox-axis
    root_path: crates/vox-gui
    shared_contracts:
      - contracts/gui
"#,
    )
    .unwrap();

    fs::write(vox_root.join("contracts/gui/surface.json"), "{\"v\":1}").unwrap();
    fs::write(axis_root.join("contracts/gui/surface.json"), "{\"v\":0}").unwrap();
    fs::write(vox_root.join("crates/vox-gui/README.md"), "# Axis V2").unwrap();
    fs::write(axis_root.join("README.md"), "# Axis V1").unwrap();

    let before = verify_cross_repo_contract_parity(&vox_root).expect("before parity");
    assert!(!before.in_parity);
    assert!(before.drifts.len() >= 2);

    let summary = sync_federated_satellites(&vox_root).expect("sync failed");
    assert!(
        summary
            .present_repositories
            .contains(&"vox-axis".to_string())
    );
    assert!(summary.files_copied >= 2);

    let after = verify_cross_repo_contract_parity(&vox_root).expect("after parity");
    assert!(after.in_parity, "drifts remaining: {:?}", after.drifts);
}

#[test]
fn test_core_module_leaf_facade_exports() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("Vox.toml"), "[vox]\nmodel = \"x\"\n").unwrap();
    let ctx = vox_repository::core::discover_repository_or_fallback(dir.path());
    assert!(!ctx.repository_id.is_empty());
    let manifest_root = vox_repository::core::find_project_manifest_root(dir.path());
    assert!(manifest_root.is_some());
    let _ci_root = vox_repository::core::resolve_repo_root_for_ci();
    let host = vox_repository::core::probe_host_capabilities();
    assert!(host.arch.is_some());
}

#[test]
fn test_hybrid_in_monorepo_surfaces_without_external_siblings() {
    let dir = tempdir().expect("tempdir");
    let vox_root = dir.path().join("vox");
    let ops_root = dir.path().join("vox-ops");

    fs::create_dir_all(vox_root.join(".vox")).unwrap();
    fs::create_dir_all(vox_root.join("contracts/telemetry")).unwrap();
    fs::create_dir_all(vox_root.join("contracts/gui")).unwrap();
    fs::create_dir_all(vox_root.join("contracts/documentation")).unwrap();
    fs::create_dir_all(vox_root.join("crates/vox-gui/contracts/gui")).unwrap();
    fs::create_dir_all(vox_root.join("crates/vox-gui/contracts/telemetry")).unwrap();
    fs::create_dir_all(vox_root.join("docs-astro/contracts/documentation")).unwrap();
    fs::create_dir_all(ops_root.join("contracts/telemetry")).unwrap();

    fs::write(
        vox_root.join("contracts/telemetry/events.v1.yaml"),
        "schema_version: 1\n",
    )
    .unwrap();
    fs::write(
        ops_root.join("contracts/telemetry/events.v1.yaml"),
        "schema_version: 1\n",
    )
    .unwrap();
    fs::write(
        vox_root.join("contracts/gui/surface.json"),
        "{\"version\":1}",
    )
    .unwrap();
    fs::write(
        vox_root.join("crates/vox-gui/contracts/gui/surface.json"),
        "{\"version\":1}",
    )
    .unwrap();
    fs::write(
        vox_root.join("crates/vox-gui/contracts/telemetry/events.v1.yaml"),
        "schema_version: 1\n",
    )
    .unwrap();
    fs::write(
        vox_root.join("contracts/documentation/taxonomy.yaml"),
        "version: 1\n",
    )
    .unwrap();
    fs::write(
        vox_root.join("docs-astro/contracts/documentation/taxonomy.yaml"),
        "version: 1\n",
    )
    .unwrap();

    fs::write(
        repo_catalog_manifest_path(&vox_root),
        r#"schema_version: 1
primary_repository_id: vox
repositories:
  - repository_id: vox
    display_name: vox
    root_path: .
    shared_contracts:
      - contracts/telemetry/
      - contracts/gui/
      - contracts/documentation/
  - repository_id: vox-ops
    display_name: vox-ops
    root_path: ../vox-ops
    optional: true
    shared_contracts:
      - contracts/telemetry/
  - repository_id: vox-axis
    display_name: vox-axis
    root_path: crates/vox-gui
    shared_contracts:
      - contracts/gui/
      - contracts/telemetry/
  - repository_id: voxlang.org
    display_name: voxlang.org
    root_path: docs-astro
    shared_contracts:
      - contracts/documentation/
"#,
    )
    .unwrap();

    // Neither ../vox-axis nor ../voxlang.org exists on disk: hybrid resolution must still
    // count 4/4 present repositories via canonical in-monorepo paths and skip absent external mirrors cleanly.
    let parity = verify_cross_repo_contract_parity(&vox_root).expect("hybrid parity");
    assert_eq!(parity.total_repositories, 4);
    assert_eq!(parity.present_repositories, 4);
    assert!(parity.in_parity, "drifts: {:?}", parity.drifts);

    let sync = sync_federated_satellites(&vox_root).expect("hybrid sync");
    assert_eq!(sync.present_repositories, vec!["vox-ops".to_string()]);
    assert!(sync.skipped_repositories.contains(&"vox-axis".to_string()));
    assert!(
        sync.skipped_repositories
            .contains(&"voxlang.org".to_string())
    );
}
