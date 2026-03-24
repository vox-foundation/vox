//! AI and orchestration: generate, train, workflow, serve.
//!
//! Legacy in-process **agent / dei / hud / learn** lived here behind a `dashboard` flag but depended on
//! the workspace-excluded `vox-dei` crate. Dashboard UX is now the VS Code extension — not these modules.

/// Defaults for Populi inference bind/port/temperature (shared with `vox populi serve`).
pub mod inference_defaults;

#[cfg(feature = "populi-dei")]
/// Natural language code generation.
pub mod generate;
#[cfg(feature = "populi-oratio")]
/// Speech-to-text (Oratio); primary UX is `vox populi oratio`.
pub mod oratio;
#[cfg(feature = "gpu")]
/// Model inference server (Axum).
pub mod serve;
#[cfg(all(feature = "gpu", feature = "populi-dei"))]
/// Legacy native training entry (full CLI dispatch).
pub mod train;
#[cfg(feature = "populi-dei")]
/// Automated multi-step workflow execution.
pub mod workflow;
