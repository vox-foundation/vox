//! Shared CLI argument types and enums for various commands.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// Build mode (`app` or `library`).
#[derive(Clone, Copy, Debug, ValueEnum, Default, Serialize, Deserialize, PartialEq)]
pub enum BuildMode {
    /// Emit app code + components (default).
    #[default]
    App,
    /// Emit UI-agnostic models, schemas, and client fetchers.
    Library,
}

/// Bundling mode: `app` (web + backend) or `script` (binary only).
#[derive(Clone, Copy, Debug, ValueEnum, Default, Serialize, Deserialize, PartialEq)]
pub enum BundleMode {
    /// Web application with React frontend and Axum backend.
    #[default]
    App,
    /// Native binary script for mesh/CLI execution.
    Script,
}

/// `vox upgrade` lane: release binary vs local repository checkout.
#[derive(Clone, Copy, Debug, Default, ValueEnum, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpgradeLane {
    /// Checksums-verified release archive into `CARGO_HOME/bin` (default).
    #[default]
    Release,
    /// Fetch / fast-forward then `cargo install`.
    Repo,
}
