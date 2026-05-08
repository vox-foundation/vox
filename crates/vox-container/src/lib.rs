//! # vox-container
//!
//! OCI-compatible container runtime abstraction for the Vox toolchain.
//!
//! Provides a unified [`ContainerRuntime`] trait over Docker and Podman,
//! and automatic runtime detection (preferring rootless Podman).
//!
//! **Deployment artifact codegen** (Dockerfile, Compose, K8s, Fly, Coolify, systemd)
//! has moved to `vox-deploy-codegen`.
//!
//! This crate now contains only:
//! - [`ContainerRuntime`] trait + [`BuildOpts`] / [`RunOpts`]
//! - Docker and Podman runtime implementations
//! - Runtime auto-detection

#![allow(clippy::collapsible_if)]

pub mod detect;
pub mod docker;
pub mod podman;

mod runtime;

pub use detect::detect_runtime;
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
