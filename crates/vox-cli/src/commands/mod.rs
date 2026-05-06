//! Subcommand implementations for the **`vox`** binary.
//!
//! Each submodule corresponds to one clap variant in [`crate::Cli`] (`src/lib.rs`). Shared behavior (lex/parse/typecheck)
//! is available in [`crate::pipeline`] for a single frontend; `build` and `check` here still use the
//! legacy inline path. New work should route through `pipeline` for consistent diagnostics.

pub mod add;

#[cfg(feature = "dei")]
pub mod attention;
/// Identity and master key integration (`vox auth`).
pub mod auth;
/// Canonical login for vault / Clavis (`vox login`, `vox auth connect`, `vox clavis login`).
pub mod login_shared;
/// Building and codegen orchestration endpoints.
pub mod build;
/// Packaging tools for bundling Vox web apps (e.g., TanStack/Vite wrapper).
pub mod bundle;
/// Catalog explicit management commands (`vox catalog`).
pub mod catalog;
/// Validation and static checking (`vox check`).
pub mod check;
/// CI / SSOT guard commands (`vox ci`).
pub mod ci;
/// Centralized secret lifecycle commands (`vox clavis`).
pub mod clavis;
/// Codex integration logic for `vox db` subcommands.
pub mod codex;
/// `vox config` CLI endpoint logic.
pub mod config;
/// Training data extraction / mixing pipelines (`vox corpus`).
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
/// `vox deploy` — `Vox.toml` `[deploy]` execution (`vox-container`).
pub mod deploy;
/// Auto-reloading compilation daemon runner (`vox dev`).
pub mod dev;
/// Submodules for `architect`, `doctor`, `clean`, etc.
pub mod diagnostics;
/// API documentation generator wrapper (`vox doc`).
pub mod doc;
/// Extension lane: unified entry for legacy/ML subcommands (ars, ludus, oratio, schola).
pub mod ext;
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
/// `vox init` — scaffold `Vox.toml` / `src/main.vox` / skill markdown.
pub mod init;
/// Interactive telemetry-enabled execution orchestrator (`vox live`).
#[cfg(feature = "live")]
pub mod live;
pub mod lock;
/// Launch Language Server Protocol wrapper (`vox lsp`).
pub mod lsp;
/// Start the Vox MCP server wrapper (`vox mcp`).
pub mod mcp;
#[cfg(feature = "mcp-server")]
pub mod mcp_server;
/// React interop / web stack migrations (`vox migrate web`, …).
pub mod migrate;
pub mod model;
pub mod new;
/// `vox openclaw` tools for orchestrator testing.
#[cfg(feature = "ars")]
pub mod openclaw;
pub mod play;
pub mod pm;
pub mod pm_lifecycle;
pub mod repair;
#[cfg(feature = "dei")]
pub mod safety;

/// Explicit multi-repo catalog and read-only polyrepo queries (`vox repo`).
pub mod repo;
pub mod repo_init;
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
#[cfg(any(feature = "dei", feature = "coderabbit"))]
pub mod review;
/// Native execution via local runtime execution (`vox run`).
pub mod run;
/// Extended runtime subtree (`dev`, `info`, `run` script path, shell) — see submodules.
pub mod runtime;
/// Vox Scientia research facade (`vox scientia` → `vox db` research tools).
pub mod scientia;
pub(crate) mod scientia_ledger_contract;
/// Optional telemetry upload queue (`vox telemetry`).
pub mod telemetry;
/// Test suite integration wrapper (`vox test`).
pub mod test;
pub mod update;

pub mod grammar;

/// Unified research operations: infrastructure and evaluation.
pub mod research;

/// Manual plan bridging via PlanningOrchestrator
pub mod plan;

/// LLM-native context and prompt generation tools
pub mod llm;

/// Generate Vox code from a prompt using the MENS inference model (`vox generate`).
pub mod generate;

/// Vox Visus: Voice of Vision. Agentic GUI visual intelligence and bug detection.
#[cfg(feature = "dei")]
pub mod visus;

/// Local orchestration dashboard (`vox dashboard`).
#[cfg(feature = "dashboard")]
pub mod dashboard;
