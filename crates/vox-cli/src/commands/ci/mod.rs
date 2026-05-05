//! `vox ci` — repository guard checks (SSOT, manifests, feature matrix) without shell/Python.

mod attention_ledger_parity;
mod attention_parity;
pub(crate) mod bounded_read;
pub mod build_timings;
mod canonical_docs;
mod capability_snapshot;
mod capability_sync;
mod check_links;
mod command_compliance;
mod command_sync;
mod coolify_eval;
pub mod completion_quality;
mod contracts_index;
pub mod data_storage_guard;
mod dep_sprawl;
pub mod deploy_status;
mod determinism_audit;
mod doctest_md;
mod eval_matrix;
mod exec_policy_contract;
mod frozen_crates;
mod grammar_ssot_parity;
mod gui_smoke;
mod install_hooks;
mod kill_stuck_tests;
mod line_endings;
mod mens_scorecard;
pub(crate) mod nomenclature_guard;
mod openclaw_contract;
mod operations_catalog;
mod pm_provenance;
mod pre_push;
mod release_build;
pub(crate) mod retired_symbol_check;
mod scaling_audit;
mod scientia_heuristics_parity;
mod scientia_novelty_ledger_contract;
mod scientia_worthiness_contract;
pub(crate) mod sync_ignore_files;
pub mod watch_run;
pub mod workspace_artifacts;

mod cmd_enums;
mod constants;
mod coverage_gates;
pub(crate) mod run_body;

use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::Result;

pub use cmd_enums::{
    CiCmd, CoolifyEvalCmd, CoverageGateMode, DocInventoryCmd, EvalMatrixCmd, GrammarDriftEmit,
    MensScorecardCmd, OperationsSyncTarget, ScalingAuditCmd,
};

/// Resolve repository root: `VOX_REPO_ROOT`, else walk up from CWD for `AGENTS.md` + `Cargo.toml`.
pub fn repo_root() -> PathBuf {
    vox_repository::resolve_repo_root_for_ci()
}

pub(super) fn cargo_bin() -> PathBuf {
    if let Ok(h) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        let win = PathBuf::from(&h).join(".cargo/bin/cargo.exe");
        if win.is_file() {
            return win;
        }
    }
    PathBuf::from("cargo")
}

/// `nvcc --version` using `CUDA_PATH`/`CUDA_HOME` when set (agent shells often lack full `PATH`).
fn nvcc_version_command() -> Command {
    let try_cuda_bin = |base: &str| -> Option<PathBuf> {
        let root = PathBuf::from(base);
        let exe = if cfg!(windows) {
            root.join("bin").join("nvcc.exe")
        } else {
            root.join("bin").join("nvcc")
        };
        exe.is_file().then_some(exe)
    };
    if let Ok(p) = std::env::var("CUDA_PATH").or_else(|_| std::env::var("CUDA_HOME")) {
        if let Some(exe) = try_cuda_bin(&p) {
            return Command::new(exe);
        }
    }
    Command::new("nvcc")
}

pub(super) fn nvcc_available() -> bool {
    nvcc_version_command()
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Run `vox ci` subcommand.
pub async fn run(cmd: CiCmd) -> Result<()> {
    run_body::run(cmd).await
}
