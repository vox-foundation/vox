//! vox CLI `ci` subcommand: sync-ignore-files, secret-env-guard, doc-pipeline
//! check, plugin-catalog generators, ssot-drift, etc. Extracted from vox-cli
//! to isolate CI-only edits from the main CLI binary's incremental rebuild.
//!
//! ## Current status: workspace boundary marker
//!
//! The 76 implementation files under `vox-cli/src/commands/ci/` cannot yet be
//! moved here without resolving these bidirectional couplings:
//!
//! **ci/ → vox-cli internals (crate:: refs that would need to become external deps):**
//! - `crate::command_registry_model::{RegistryFile, RegistryOperation}` (5 files)
//! - `crate::command_contract::*` (validators.rs)
//! - `crate::VoxCliRoot` (command_compliance/tests.rs — renders the clap help)
//! - `crate::artifact_policy` (workspace_artifacts/{mod,retention}.rs)
//! - `crate::commands::scientia_ledger_contract::*` (scientia_novelty_ledger_contract.rs)
//! - `crate::commands::runtime::shell::check_terminal` (exec_policy_contract.rs)
//!
//! **vox-cli → ci/ (25+ files use these ci/ modules):**
//! - `commands::ci::bounded_read::{read_utf8_path_capped, read_utf8_path_capped_async}`
//!   used by build.rs, fmt.rs, pipeline.rs, compilerd.rs, runtime/shell/*, diagnostics/*, etc.
//! - `commands::ci::sync_ignore_files` (diagnostics/doctor, repo_init.rs)
//! - `commands::ci::run_body::run_body_helpers` (diagnostics/doctor/checks_standard/clavis.rs)
//! - `commands::ci::nomenclature_guard`, `retired_symbol_check` (external consumers)
//!
//! **Resolution path:** extract `bounded_read` → `vox-cli-core` first (it is a thin
//! re-export of `vox-bounded-fs`). Then move `command_registry_model`, `command_contract`,
//! and `artifact_policy` to `vox-cli-core`. Once vox-cli no longer imports from ci/
//! for non-ci purposes, the file move becomes a mechanical rename.
//!
//! Until then, `pub use vox_cli::commands::ci::run` cannot be added here (that would
//! create an L3→L5 layer inversion). The workspace boundary is registered in
//! `docs/src/architecture/layers.toml` and `where-things-live.md` to track intent.
