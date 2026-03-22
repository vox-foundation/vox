//! Subcommand implementations for the **`vox`** binary.
//!
//! Each submodule corresponds to one clap variant in `src/main.rs`. Shared behavior (lex/parse/typecheck)
//! is available in [`crate::pipeline`] for a single frontend; `build` and `check` here still use the
//! legacy inline path. New work should route through `pipeline` for consistent diagnostics.

#[cfg(any(feature = "gpu", feature = "populi-dei", feature = "populi-base"))]
pub mod ai;
pub mod build;
pub mod bundle;
pub mod check;
/// CI / SSOT guard commands (`vox ci`).
pub mod ci;
pub mod codex;
pub mod corpus;
/// Local VoxDB / Codex diagnostics (`vox db`).
pub mod db;
pub mod db_cli;
pub mod dev;
pub mod diagnostics;
pub mod doc;
pub mod extras;
/// ARS `vox skill` implementation (`extras::ars`); re-exported for internal call sites and any out-of-tree dispatch shims.
#[cfg(feature = "ars")]
pub use extras::ars;
pub mod fmt;
/// `vox info` — package metadata from registry / local Arca store (`vox-pm`).
pub mod info;
pub mod install;
#[cfg(feature = "island")]
pub mod island;
#[cfg(feature = "live")]
pub mod live;
pub mod lsp;
#[cfg(feature = "mesh")]
pub mod mesh_cli;
#[cfg(feature = "ars")]
pub mod openclaw;
#[cfg(feature = "stub-check")]
pub mod stub_check;
#[cfg(feature = "extras-ludus")]
pub use extras::ludus;
#[cfg(any(feature = "populi-dei", feature = "coderabbit"))]
pub mod review;
pub mod run;
/// Extended runtime subtree (`dev`, `info`, `run` script path, shell) — see submodules.
pub mod runtime;
/// Vox Scientia research facade (`vox scientia` → `vox db` research tools).
pub mod scientia;
pub mod test;
/// Standalone workflow helpers (interpreted run when `workflow-runtime` is enabled).
pub mod workflow;

#[cfg(any(feature = "populi-base", feature = "gpu"))]
pub mod populi;
