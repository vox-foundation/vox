//! # vox-deploy-codegen
//!
//! Deployment artifact codegen for the Vox toolchain.
//!
//! Generates Dockerfiles, Compose files, Kubernetes manifests, Fly.io configs,
//! Coolify configurations, and systemd unit files from Vox `environment`
//! declarations. This is **pure text generation** — no runtime execution.
//!
//! Submodules:
//! - [`deploy_target`] — unified `DeployTarget` enum + per-target execution helpers
//! - [`generate`] — Dockerfile / Compose generation from `EnvironmentSpec`
//! - [`bare_metal`] — systemd unit file generation
//! - [`python_dockerfile`] — Python-flavored Dockerfile snippet generation
//! - [`pyproject`] — pyproject.toml generation (retired, emits placeholder)
//! - [`env`] — `PythonEnv` uv/venv detection (legacy, hard-errors on uv sync)
//! - [`setup`] — `run_py_setup` hard-error stub (Python lane retired)

#![allow(clippy::collapsible_if)]

pub mod bare_metal;
pub mod deploy_target;
pub mod env;
pub mod generate;
pub mod pyproject;
pub mod python_dockerfile;
pub mod setup;

// Re-export the runtime trait from vox-container so deploy_target.rs compiles
// against the same trait object type used by detect_runtime().
// TODO Phase 5: when vox-container is deleted, this re-export is replaced by
// vox-skill-runtime's ContainerRuntime equivalent.
pub use vox_container::{BuildOpts, ContainerRuntime};

pub use bare_metal::generate_systemd_unit;
pub use deploy_target::{
    BareMetalTarget, ComposeTarget, ContainerTarget, DeployTarget, KubernetesTarget,
    build_container_target, resolve_target_kind,
};
pub use env::PythonEnv;
pub use pyproject::generate_pyproject_toml;
pub use python_dockerfile::generate_python_dockerfile;
pub use setup::{PySetupOpts, run_py_setup};
