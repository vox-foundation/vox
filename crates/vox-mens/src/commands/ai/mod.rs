//! AI and orchestration: generate, train, workflow, serve.
//!
//! Legacy in-process **agent / dei / hud / learn** lived here behind a `dashboard` flag but depended on
//! the unwired historical `vox-dei` module graph. Dashboard UX is now the VS Code extension — not these modules.

/// Defaults for Mens inference bind/port/temperature (shared with `vox mens serve`).
pub mod inference_defaults;

#[cfg(feature = "mens-dei")]
/// Natural language code generation.
pub mod generate;
#[cfg(feature = "oratio")]
/// Speech-to-text (Oratio); primary UX is **`vox oratio`** → [`crate::commands::oratio_cmd`].
pub mod oratio;
#[cfg(feature = "gpu")]
/// Model inference server (Axum).
pub mod serve;
#[cfg(all(feature = "gpu", feature = "mens-dei"))]
/// Legacy native training entry (full CLI dispatch).
pub mod train;
#[cfg(feature = "mens-dei")]
/// Automated multi-step workflow execution.
pub mod workflow;
