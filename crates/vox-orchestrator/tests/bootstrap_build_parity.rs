//! Parity: repeated repo-scoped bootstrap yields identical repository identity and memory paths.

use std::fs;

use vox_orchestrator::{
    OrchestratorConfig, build_repo_scoped_orchestrator,
    build_repo_scoped_orchestrator_for_repository,
};

#[test]
fn two_builds_same_repo_match_ids_and_memory_shard() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_path_buf();
    fs::write(root.join("Vox.toml"), "[project]\nname = \"parity\"\n").expect("Vox.toml");

    let cfg = OrchestratorConfig::default();
    let a = build_repo_scoped_orchestrator(cfg.clone(), Some(root.as_path()));
    let b = build_repo_scoped_orchestrator(cfg, Some(root.as_path()));

    assert_eq!(a.repository.repository_id, b.repository.repository_id);
    assert_eq!(a.config.memory.log_dir, b.config.memory.log_dir);
    assert_eq!(
        a.config.memory.memory_md_path,
        b.config.memory.memory_md_path
    );

    let c =
        build_repo_scoped_orchestrator_for_repository(OrchestratorConfig::default(), &a.repository);
    assert_eq!(c.repository.repository_id, a.repository.repository_id);
    assert_eq!(c.config.memory.log_dir, a.config.memory.log_dir);
}
