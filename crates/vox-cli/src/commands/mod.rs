//! Subcommand implementations for the **`vox`** binary.
//!
//! Each submodule corresponds to one clap variant in [`crate::Cli`] (`src/lib.rs`). Shared behavior (lex/parse/typecheck)
//! is available in [`crate::pipeline`] for a single frontend; `build` and `check` here still use the
//! legacy inline path. New work should route through `pipeline` for consistent diagnostics.

pub mod add;
/// AI subsystem handling training, models, and eval logic (requires features: `gpu` or `mens-dei` or `mens-base`).
#[cfg(any(feature = "gpu", feature = "mens-dei", feature = "mens-base"))]
pub mod ai;
/// Building and codegen orchestration endpoints.
pub mod build;
/// Packaging tools for bundling Vox web apps (e.g., TanStack/Vite wrapper).
pub mod bundle;
/// Validation and static checking (`vox check`).
pub mod check;
/// CI / SSOT guard commands (`vox ci`).
pub mod ci;
/// Centralized secret lifecycle commands (`vox clavis`).
pub mod clavis;
/// Codex integration logic for `vox db` subcommands.
pub mod codex;
/// Training data extraction / mixing pipelines (`vox corpus`).
pub mod corpus;
/// Codex research ingest / reliability helpers (`vox db` research subcommands).
mod db_research;
pub mod remove;
// `db.rs` re-exports this tree; keep a same-file reference for tooling / unwired-module checks.
#[allow(unused_imports)]
use self::db_research as _;
/// Local VoxDB / Codex diagnostics (`vox db`).
pub mod db;
/// Clap entrypoints for `vox db`.
pub mod db_cli;
pub(crate) mod db_retention;
/// DEI decision engine commands (requires `--features dei`).
#[cfg(feature = "dei")]
pub mod dei;
/// Auto-reloading compilation daemon runner (`vox dev`).
pub mod dev;
/// Submodules for `architect`, `doctor`, `clean`, etc.
pub mod diagnostics;
/// API documentation generator wrapper (`vox doc`).
pub mod doc;
/// Supplemental subcommands (snippet, share, ars).
pub mod extras;
/// Socrates / evidence fusion for scientia worthiness (`metadata_json.scientia_evidence`).
pub mod scientia_worthiness_enrich;
/// ARS `vox skill` implementation (`extras::ars`); re-exported for internal call sites and any out-of-tree dispatch shims.
#[cfg(feature = "ars")]
pub use extras::ars;
/// AST formatting and canonicalization (`vox fmt`).
pub mod fmt;
/// `vox info` — package metadata from registry / local Arca store (`vox-pm`).
pub mod info;
/// Web island UI creation handler (`vox island`).
#[cfg(feature = "island")]
pub mod island;
/// Interactive telemetry-enabled execution orchestrator (`vox live`).
#[cfg(feature = "live")]
pub mod live;
pub mod lock;
/// Legacy login command (compat shim to Clavis).
pub mod login;
/// Legacy logout command (compat shim to Clavis).
pub mod logout;
/// Launch Language Server Protocol wrapper (`vox lsp`).
pub mod lsp;
/// `vox openclaw` tools for orchestrator testing.
#[cfg(feature = "ars")]
pub mod openclaw;
pub mod pm;
pub mod pm_lifecycle;
/// Local registry + HTTP control plane (`vox populi status|serve`; requires `populi`).
#[cfg(feature = "populi")]
pub mod populi_cli;
/// One-command populi lifecycle helpers (`vox populi up|down|status`; requires `populi`).
#[cfg(feature = "populi")]
pub mod populi_lifecycle;
pub(crate) mod repo_upgrade;
/// TOESTUB structural testing guard logic.
#[cfg(feature = "stub-check")]
pub mod stub_check;
pub mod sync;
pub(crate) mod toolchain_upgrade;
pub mod upgrade;
/// Ludus gamification systems logic wrapper.
#[cfg(feature = "extras-ludus")]
pub use extras::ludus;
/// AI-powered CodeRabbit review adapter (`vox review`).
#[cfg(any(feature = "mens-dei", feature = "coderabbit"))]
pub mod review;
/// Native execution via local runtime execution (`vox run`).
pub mod run;
/// Extended runtime subtree (`dev`, `info`, `run` script path, shell) — see submodules.
pub mod runtime;
/// Vox Scientia research facade (`vox scientia` → `vox db` research tools).
pub mod scientia;
/// Test suite integration wrapper (`vox test`).
pub mod test;
pub mod update;

/// Speech-to-text and transcript refinement (`vox oratio`).
#[cfg(feature = "oratio")]
pub mod oratio_cmd;

/// ML tooling specific commands (`vox mens`).
#[cfg(any(feature = "mens-base", feature = "gpu"))]
pub mod mens;

/// Training tools (`vox schola`).
#[cfg(feature = "gpu")]
pub mod schola;
