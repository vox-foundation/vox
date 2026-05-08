//! # vox-container
//!
//! OCI container runtime trait and types for the Vox toolchain.
//!
//! This crate provides the [`ContainerRuntime`] trait, [`BuildOpts`], and [`RunOpts`]
//! used by deploy-facing code (`vox-deploy-codegen`, `vox-cli`).
//!
//! **Docker and Podman implementations** have moved to `vox-plugin-runtime-container`.
//! **Deployment artifact codegen** has moved to `vox-deploy-codegen`.
//! **Abstract skill runtime trait** is in `vox-skill-runtime`.
//! **Runtime detection** (`detect_runtime`) has moved to `vox-plugin-runtime-container`.

#![allow(clippy::collapsible_if)]

mod runtime;

pub use runtime::{BuildOpts, ContainerRuntime, RunOpts};

/// Classify the exec risk of a container image or command string and log the result.
///
/// Called before any container run dispatch. Uses `vox-exec-grammar`'s risk classifier
/// to log a risk level for auditing — does not gate or reject commands (low-risk wiring;
/// see ADR-026). Upgrade to a hard gate by checking [`vox_exec_grammar::RiskLevel`] and
/// returning an error when `>= High`.
pub fn log_exec_risk(raw_command: &str) {
    match vox_exec_grammar::parse(raw_command) {
        Ok(mut ast) => {
            let policy = vox_exec_grammar::ExecPolicy::default();
            vox_exec_grammar::risk::classify(&mut ast, &policy);
            tracing::info!(
                command = raw_command,
                risk = ?ast.risk,
                "exec-grammar risk classification"
            );
        }
        Err(e) => {
            tracing::debug!(
                command = raw_command,
                error = %e,
                "exec-grammar could not parse command; skipping risk classification"
            );
        }
    }
}

/// Runtime preference (kept here for backward compat; callers migrating to
/// `vox-plugin-runtime-container::detect::RuntimePreference`).
pub mod detect {
    /// Preferred container runtime selection strategy.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum RuntimePreference {
        /// Prefer Podman, fall back to Docker.
        #[default]
        Auto,
        /// Use Docker only.
        Docker,
        /// Use Podman only.
        Podman,
    }

    impl std::str::FromStr for RuntimePreference {
        type Err = anyhow::Error;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s.to_lowercase().as_str() {
                "auto" => Ok(Self::Auto),
                "docker" => Ok(Self::Docker),
                "podman" => Ok(Self::Podman),
                other => anyhow::bail!(
                    "Unknown runtime preference: {other:?}. Use auto, docker, or podman."
                ),
            }
        }
    }
}

/// Detect and return the best available container runtime (Podman preferred).
///
/// Delegates to `vox-plugin-runtime-container` for actual instantiation.
/// Kept here for backward compatibility; long-term callers should use
/// `vox_plugin_runtime_container::detect_runtime()` directly.
pub use detect::RuntimePreference;

/// Backward-compat re-export. Callers can use
/// `vox_plugin_runtime_container::detect_runtime()` directly.
///
/// Returns the best available OCI runtime (Podman first, then Docker).
pub fn detect_runtime(
    preference: detect::RuntimePreference,
) -> anyhow::Result<Box<dyn ContainerRuntime>> {
    vox_plugin_runtime_container::detect_runtime(preference)
}
