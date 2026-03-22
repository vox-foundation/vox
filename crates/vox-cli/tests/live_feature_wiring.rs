//! Integration guard: `vox-cli` feature **`live`** must resolve `vox-orchestrator` (see `commands/live.rs`).

#[test]
fn vox_orchestrator_config_default_for_live_feature() {
    let _ = vox_orchestrator::OrchestratorConfig::default();
}
