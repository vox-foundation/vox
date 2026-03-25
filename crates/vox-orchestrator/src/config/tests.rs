use std::sync::Mutex;

use super::{
    ConfigValidationError, CostPreference, OrchestratorConfig, OverflowStrategy,
};
use crate::contract::OrchestrationMigrationFlags;
use crate::types::TaskPriority;

/// Serializes tests that mutate process environment variables.
static ENV_MUTEX: Mutex<()> = Mutex::new(());

#[test]
fn default_config_values() {
    let cfg = OrchestratorConfig::default();
    assert!(cfg.enabled);
    assert_eq!(cfg.max_agents, 8);
    assert_eq!(cfg.default_priority, TaskPriority::Normal);
    assert_eq!(cfg.queue_overflow_strategy, OverflowStrategy::SpawnNewAgent);
    assert_eq!(cfg.lock_timeout_ms, 30_000);
    assert!(cfg.toestub_gate);
    assert!(cfg.fallback_to_single_agent);
    assert_eq!(cfg.min_agents, 1);
    assert!(!cfg.scaling_enabled);
    assert_eq!(cfg.cost_preference, CostPreference::Performance);
}

#[test]
fn config_serialization_roundtrip() {
    let cfg = OrchestratorConfig::default();
    let json = serde_json::to_string(&cfg).expect("serialize");
    let back: OrchestratorConfig = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.max_agents, cfg.max_agents);
    assert_eq!(back.enabled, cfg.enabled);
}

#[test]
fn test_config_values() {
    let cfg = OrchestratorConfig::for_testing();
    assert_eq!(cfg.max_agents, 4);
    assert_eq!(cfg.lock_timeout_ms, 1000);
    assert!(!cfg.toestub_gate);
    assert!(cfg.validate().is_ok());
}

#[test]
fn test_validation_errors() {
    let cfg = OrchestratorConfig {
        max_agents: 0,
        lock_timeout_ms: 50,
        bulletin_capacity: 0,
        ..Default::default()
    };

    let errs = cfg.validate().unwrap_err();
    assert_eq!(errs.len(), 4);
    assert!(errs.contains(&ConfigValidationError::InvalidMaxAgents(0)));
    assert!(errs.contains(&ConfigValidationError::InvalidLockTimeout(50)));
    assert!(errs.contains(&ConfigValidationError::InvalidBulletinCapacity(0)));
    assert!(errs.contains(&ConfigValidationError::InvalidScalingLimits(1, 0)));
}

#[test]
fn missing_toml_section_returns_default() {
    let dir = std::env::temp_dir().join("vox_orch_test");
    std::fs::create_dir_all(&dir).ok();
    let toml_path = dir.join("no_orch.toml");
    std::fs::write(&toml_path, "[package]\nname = \"test\"\n").ok();

    let cfg = OrchestratorConfig::load_from_toml(&toml_path).expect("should load");
    assert_eq!(cfg.max_agents, 8);
}

#[test]
fn orchestration_migration_defaults_match_contract() {
    let c = OrchestratorConfig::default();
    assert!(!c.orchestration_migration.orchestration_v2_enabled);
    assert!(c.orchestration_migration.legacy_orchestration_fallback);
}

#[test]
fn orchestration_migration_deserializes_from_toml_fragment() {
    let flags: OrchestrationMigrationFlags = toml::from_str(
        "orchestration_v2_enabled = true\nlegacy_orchestration_fallback = false\n",
    )
    .expect("parse nested [orchestrator.orchestration_migration]-shaped keys");
    assert!(flags.orchestration_v2_enabled);
    assert!(!flags.legacy_orchestration_fallback);
}

#[test]
fn populi_toml_section_merges_into_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    let toml_path = dir.path().join("Vox.toml");
    std::fs::write(
        &toml_path,
        r#"
[orchestrator]
max_agents = 3

[mens]
control_url = "http://mens.example:9847"
scope_id = "unit-scope"
advertise_gpu = true
labels = ["from=toml"]
"#,
    )
    .expect("write");
    let cfg = OrchestratorConfig::load_from_toml(&toml_path).expect("load");
    assert_eq!(cfg.max_agents, 3);
    assert_eq!(
        cfg.populi_control_url.as_deref(),
        Some("http://mens.example:9847")
    );
    assert_eq!(cfg.populi_scope_id.as_deref(), Some("unit-scope"));
    assert!(cfg.default_agent_capabilities.gpu_cuda);
    assert!(
        cfg.default_agent_capabilities
            .labels
            .contains(&"from=toml".to_string())
    );
}

#[test]
#[allow(unsafe_code)]
fn populi_env_overrides_toml_control_url() {
    let _guard = ENV_MUTEX.lock().expect("env test lock");
    const KEY: &str = "VOX_ORCHESTRATOR_MESH_CONTROL_URL";
    let prev = std::env::var(KEY).ok();
    unsafe {
        std::env::set_var(KEY, "http://env-wins:7777");
    }

    let dir = tempfile::tempdir().expect("tempdir");
    let toml_path = dir.path().join("Vox.toml");
    std::fs::write(
        &toml_path,
        r#"
[mens]
control_url = "http://toml-loses:8888"
"#,
    )
    .expect("write");

    let mut cfg = OrchestratorConfig::load_from_toml(&toml_path).expect("load");
    assert_eq!(
        cfg.populi_control_url.as_deref(),
        Some("http://toml-loses:8888")
    );
    cfg.merge_env_overrides();
    assert_eq!(
        cfg.populi_control_url.as_deref(),
        Some("http://env-wins:7777")
    );

    unsafe {
        match prev {
            None => std::env::remove_var(KEY),
            Some(v) => std::env::set_var(KEY, v),
        }
    }
}
