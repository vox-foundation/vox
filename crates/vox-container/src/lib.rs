//! # vox-container
//!
//! OCI container runtime trait and types for the Vox toolchain.
//!
//! Provides [`ContainerRuntime`] trait + [`BuildOpts`] / [`RunOpts`].
//!
//! **Docker/Podman implementations** → `vox-plugin-runtime-container`
//! **Deployment artifact codegen** → `vox-deploy-codegen`
//! **Abstract skill runtime trait** → `vox-skill-runtime`
//! **Runtime detection** → `vox-plugin-runtime-container::detect_runtime`

#![allow(clippy::collapsible_if)]

mod runtime;

pub use runtime::{BuildOpts, ContainerRuntime, RunOpts};

/// Runtime preference enum — kept here for backward compat.
/// Callers should migrate to `vox_plugin_runtime_container::RuntimePreference`.
pub mod detect {
    /// Preferred container runtime selection strategy.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum RuntimePreference {
        /// Prefer Podman (rootless, daemonless), fall back to Docker.
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

/// Classify the exec risk of a container image or command string and log the result.
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
