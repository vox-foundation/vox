//! MCP `ServerState` and shared `build_repo_scoped_orchestrator` must agree on repo-scoped identity
//! when rooted at the same working directory (guards against drift between embedders).

use std::fs;
use std::path::PathBuf;

use serial_test::serial;
use vox_orchestrator::mcp_tools::ServerState;
use vox_orchestrator::{OrchestratorConfig, build_repo_scoped_orchestrator};

struct RestoreCwd(PathBuf);

impl Drop for RestoreCwd {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.0);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn mcp_server_state_matches_shared_bootstrap() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_path_buf();
    fs::write(
        root.join("Vox.toml"),
        "[project]\nname = \"mcp_bootstrap_parity\"\n",
    )
    .unwrap();

    let cwd = std::env::current_dir().expect("cwd");
    let _restore = RestoreCwd(cwd);
    std::env::set_current_dir(&root).expect("set cwd");

    let config = OrchestratorConfig::default();
    let build = build_repo_scoped_orchestrator(config.clone(), None);
    let state = ServerState::new_full(config);

    assert_eq!(
        build.repository.repository_id,
        state.repository.repository_id
    );
    assert_eq!(
        build.config.memory.log_dir,
        state.orchestrator_config.memory.log_dir
    );
    assert_eq!(
        build.config.memory.memory_md_path,
        state.orchestrator_config.memory.memory_md_path
    );
}
