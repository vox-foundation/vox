//! Non-git repository roots get distinct `repository_id` values (filesystem SSOT isolation).

use std::fs;
use tempfile::TempDir;

#[test]
fn distinct_repository_ids_for_separate_paths() {
    let a = TempDir::new().expect("tempdir a");
    let b = TempDir::new().expect("tempdir b");
    fs::write(a.path().join("README.md"), "a").unwrap();
    fs::write(b.path().join("README.md"), "b").unwrap();
    let ca = vox_repository::discover_repository_or_fallback(a.path());
    let cb = vox_repository::discover_repository_or_fallback(b.path());
    assert_ne!(
        ca.repository_id, cb.repository_id,
        "expected different ids for different roots"
    );
    assert_eq!(ca.root, a.path().canonicalize().unwrap());
    assert_eq!(cb.root, b.path().canonicalize().unwrap());
}

/// Two repos imply distinct default MCP-style session roots (path isolation).
#[test]
fn distinct_session_directories_per_repository_id() {
    use vox_orchestrator::session::SessionConfig;

    let a = TempDir::new().expect("tempdir a");
    let b = TempDir::new().expect("tempdir b");
    let ca = vox_repository::discover_repository_or_fallback(a.path());
    let cb = vox_repository::discover_repository_or_fallback(b.path());

    let sa = SessionConfig {
        sessions_dir: vox_config::mcp_sessions_dir(&ca.repository_id),
        repository_id: Some(ca.repository_id.clone()),
        ..SessionConfig::default()
    };
    let sb = SessionConfig {
        sessions_dir: vox_config::mcp_sessions_dir(&cb.repository_id),
        repository_id: Some(cb.repository_id.clone()),
        ..SessionConfig::default()
    };

    assert_ne!(sa.sessions_dir, sb.sessions_dir);
    assert_eq!(sa.repository_id.as_deref(), Some(ca.repository_id.as_str()));
    assert_eq!(sb.repository_id.as_deref(), Some(cb.repository_id.as_str()));
}
