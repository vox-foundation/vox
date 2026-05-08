//! OpenClaw sidecar fallback sandbox.
//!
//! When no local Docker/Podman runtime is available, execution of community
//! skills can optionally be delegated to the OpenClaw sidecar (if running)
//! which manages its own Docker container lifecycle.
//!
//! This is a **secondary** path. The primary path uses `vox-container` directly
//! via [`crate::sandbox::runner::SandboxedSkillRunner`].

use vox_ars_runtime::manifest::ResourceLimits;
use vox_ars_runtime::openclaw_adapter::{
    DefaultOpenClawRuntimeAdapter, OpenClawAdapterError, OpenClawRuntimeAdapter,
    connect_default_runtime_adapter,
};

use super::runner::SkillOutput;

/// Error from the OpenClaw sidecar fallback path.
#[derive(Debug, thiserror::Error)]
pub enum FallbackError {
    #[error("OpenClaw sidecar not reachable: {0}")]
    SidecarUnreachable(String),
    #[error("Sidecar delegation failed: {0}")]
    DelegationFailed(String),
    #[error("OpenClaw adapter error: {0}")]
    Adapter(#[from] OpenClawAdapterError),
}

/// Delegates skill execution to the OpenClaw sidecar's Docker sandbox.
///
/// Used when `SandboxedSkillRunner::detect()` fails (no local container runtime).
/// Requires the OpenClaw sidecar to be running and reachable via WS.
pub struct OpenClawSidecarSandbox {
    adapter: DefaultOpenClawRuntimeAdapter,
}

impl OpenClawSidecarSandbox {
    /// Attempt to connect to the OpenClaw sidecar and return a fallback sandbox.
    ///
    /// Returns `Err(FallbackError::SidecarUnreachable)` if the sidecar is not running.
    pub async fn connect() -> Result<Self, FallbackError> {
        let adapter = connect_default_runtime_adapter(None)
            .await
            .map_err(|e| FallbackError::SidecarUnreachable(e.to_string()))?;
        Ok(Self { adapter })
    }

    /// Delegate execution of a shell command to the OpenClaw sidecar sandbox.
    ///
    /// The sidecar runs the command inside its Docker container and returns
    /// the stdout/stderr via the WS frame response (`gateway_call` → `execute.skill`).
    pub async fn delegate_skill(
        &mut self,
        skill_id: &str,
        command: &str,
        _limits: &ResourceLimits,
    ) -> Result<SkillOutput, FallbackError> {
        // Delegate via the WS gateway as an `execute.skill` call.
        let params = serde_json::json!({
            "skill_id": skill_id,
            "command": command,
            "sandbox": true,
        });

        let result = self
            .adapter
            .gateway_call("execute.skill", params)
            .await
            .map_err(|e| FallbackError::DelegationFailed(e.to_string()))?;

        // Parse standard OpenClaw skill response shape.
        let stdout = result
            .get("output")
            .or_else(|| result.get("stdout"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let stderr = result
            .get("stderr")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let exit_code = result
            .get("exit_code")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;

        Ok(SkillOutput {
            stdout,
            stderr,
            exit_code,
            wall_ms: 0, // sidecar does not report timing presently
        })
    }
}
