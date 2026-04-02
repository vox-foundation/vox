//! Operator kill-switches and rollout flags derived from the environment.
//!
//! Values are read at call time (no process-wide cache) so tests and spawned workers see updates.

use serde::Serialize;

/// `1`, `true`, or `yes` after trim; alphabetic tokens are ASCII case-insensitive.
#[must_use]
pub fn env_truthy(name: &str) -> bool {
    std::env::var(name)
        .map(|v| truthy_token(&v))
        .unwrap_or(false)
}

#[must_use]
fn truthy_token(v: &str) -> bool {
    let v = v.trim();
    v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes")
}

/// Matches `DbCircuitBreaker::enabled_from_env` in `vox-db` (`1` / `true` only, lowercase trim).
#[must_use]
pub fn db_circuit_breaker_env_enabled() -> bool {
    std::env::var("VOX_DB_CIRCUIT_BREAKER")
        .map(|v| db_circuit_breaker_token(&v))
        .unwrap_or(false)
}

#[must_use]
fn db_circuit_breaker_token(v: &str) -> bool {
    let v = v.trim().to_ascii_lowercase();
    v == "1" || v == "true"
}

/// When `VOX_ORCH_LINEAGE_OFF` is truthy, skip `orchestration_lineage_events` writes.
#[must_use]
pub fn orchestration_lineage_persist_enabled() -> bool {
    !env_truthy("VOX_ORCH_LINEAGE_OFF")
}

/// Codex workflow journal persistence — disabled when `VOX_WORKFLOW_JOURNAL_CODEX_OFF` is truthy
/// (same tokens as [`env_truthy`]: `1` / `true` / `yes`).
#[must_use]
pub fn workflow_journal_codex_persist_enabled() -> bool {
    !env_truthy("VOX_WORKFLOW_JOURNAL_CODEX_OFF")
}

/// `VOX_DB_SYNC_INTEGRATION` is exactly `1` (opt-in remote `sync_for` test gate).
#[must_use]
pub fn db_sync_remote_integration_gate_armed() -> bool {
    std::env::var("VOX_DB_SYNC_INTEGRATION").ok().as_deref() == Some("1")
}

/// `VOX_DB_EMBEDDED_REPLICA_INTEGRATION` is exactly `1` (opt-in embedded-replica test gate).
#[must_use]
pub fn db_embedded_replica_integration_gate_armed() -> bool {
    std::env::var("VOX_DB_EMBEDDED_REPLICA_INTEGRATION")
        .ok()
        .as_deref()
        == Some("1")
}

/// Serializable snapshot for diagnostics (`vox doctor`, logs).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RolloutFlagSnapshot {
    pub orchestration_lineage_persist: bool,
    pub workflow_journal_codex_persist: bool,
    pub db_circuit_breaker_env: bool,
    pub db_sync_remote_integration_gate: bool,
    pub db_embedded_replica_integration_gate: bool,
}

#[must_use]
pub fn rollout_flag_snapshot() -> RolloutFlagSnapshot {
    RolloutFlagSnapshot {
        orchestration_lineage_persist: orchestration_lineage_persist_enabled(),
        workflow_journal_codex_persist: workflow_journal_codex_persist_enabled(),
        db_circuit_breaker_env: db_circuit_breaker_env_enabled(),
        db_sync_remote_integration_gate: db_sync_remote_integration_gate_armed(),
        db_embedded_replica_integration_gate: db_embedded_replica_integration_gate_armed(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollout_snapshot_json_roundtrip_fields() {
        let s = rollout_flag_snapshot();
        let v = serde_json::to_value(&s).expect("serialize");
        assert!(v.get("orchestration_lineage_persist").is_some());
        assert!(v.get("workflow_journal_codex_persist").is_some());
        assert!(v.get("db_circuit_breaker_env").is_some());
        assert!(v.get("db_sync_remote_integration_gate").is_some());
        assert!(v.get("db_embedded_replica_integration_gate").is_some());
    }

    #[test]
    fn workflow_journal_truthy_tokens_match_parser_contract() {
        for token in ["1", "true", "yes", "True", " YES "] {
            assert!(
                truthy_token(token),
                "token `{token}` should be treated as truthy"
            );
        }
        assert!(!truthy_token("0"));
        assert!(!truthy_token("no"));
    }

    #[test]
    fn db_circuit_breaker_token_contract_is_strict() {
        assert!(db_circuit_breaker_token("1"));
        assert!(db_circuit_breaker_token("true"));
        assert!(db_circuit_breaker_token(" TRUE "));
        assert!(
            !db_circuit_breaker_token("yes"),
            "db circuit breaker only accepts `1` or `true`"
        );
        assert!(!db_circuit_breaker_token("0"));
    }
}
