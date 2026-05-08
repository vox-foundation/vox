//! # vox-container
//!
//! OCI-compatible container runtime abstraction for the Vox toolchain.
//!
//! Provides a unified [`ContainerRuntime`] trait over Docker and Podman,
//! automatic runtime detection (preferring rootless Podman), and Dockerfile /
//! Compose file generation from Vox `environment` declarations.
//!
//! Legacy Python module paths (`pyproject`, `python_dockerfile`, `run_py_setup`) remain for
//! embedders; **`run_py_setup`** and **`PythonEnv::uv_sync` / `uv_add_packages`** hard-error.
//! [`generate_pyproject_toml`](crate::generate_pyproject_toml) emits a **retired** placeholder only.
//!
//! Submodules document their own `pub` items; the facade re-exports ergonomic names for CLI/embedders.

#![allow(clippy::collapsible_if)]

/// Support for bare metal deployments (systemd, etc.)
pub mod bare_metal;
pub mod deploy_target;
pub mod detect;
pub mod docker;
pub mod env;
pub mod generate;
pub mod podman;
pub mod pyproject;
pub mod python_dockerfile;
pub mod setup;

mod runtime;

pub use bare_metal::generate_systemd_unit;
pub use deploy_target::{
    BareMetalTarget, ComposeTarget, ContainerTarget, DeployTarget, KubernetesTarget,
    build_container_target, resolve_target_kind,
};
pub use detect::detect_runtime;
pub use env::PythonEnv;
pub use pyproject::generate_pyproject_toml;
pub use python_dockerfile::generate_python_dockerfile;
pub use runtime::{BuildOpts, ContainerRuntime, RunOpts};
pub use setup::{PySetupOpts, run_py_setup};

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
