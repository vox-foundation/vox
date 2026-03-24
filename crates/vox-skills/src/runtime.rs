//! In-process ARS runtime: ties Codex to a lightweight execution harness.

use std::sync::Arc;

use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;
use vox_db::VoxDb;

use crate::domain::ArsSkill;
use crate::hooks::HookRegistry;

/// Runtime errors (synchronous book-keeping only; async errors use [`std::result::Result`]).
#[derive(Debug, Error)]
pub enum ArsRuntimeError {
    /// Invalid run identifier.
    #[error("invalid run: {0}")]
    InvalidRun(String),
}

/// ARS runtime bound to Codex and hooks.
pub struct ArsRuntime {
    _db: Arc<VoxDb>,
    _hooks: Arc<HookRegistry>,
}

impl ArsRuntime {
    /// Construct a runtime handle.
    pub fn new(db: Arc<VoxDb>, hooks: Arc<HookRegistry>) -> Self {
        Self {
            _db: db,
            _hooks: hooks,
        }
    }

    /// Allocate a new logical run id (UUID v4 string).
    pub fn create_run(
        &self,
        _parent: Option<&str>,
        _skill_id: Option<&str>,
        _input: Value,
        _opts: Option<Value>,
    ) -> Result<String, ArsRuntimeError> {
        Ok(Uuid::new_v4().to_string())
    }

    /// Execute `skill` for `run_id`, returning a JSON status envelope.
    ///
    /// Present behavior: echoes structured input with success status so CLIs can dogfood wiring.
    pub async fn execute_skill(
        &self,
        run_id: &str,
        skill: &ArsSkill,
        input: Value,
    ) -> Result<Value, ArsRuntimeError> {
        if run_id.is_empty() {
            return Err(ArsRuntimeError::InvalidRun("empty run_id".into()));
        }
        Ok(serde_json::json!({
            "status": "success",
            "run_id": run_id,
            "skill_id": skill.id,
            "skill_version": skill.version,
            "output": input,
        }))
    }
}
