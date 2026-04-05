//! `vox ci` — repository guard checks (SSOT, manifests, feature matrix) without shell/Python.

pub(crate) mod bounded_read;
pub mod build_timings;
mod capability_sync;
mod check_links;
mod command_compliance;
mod command_sync;
pub mod completion_quality;
mod contracts_index;
mod eval_matrix;
mod exec_policy_contract;
mod line_endings;
mod mens_scorecard;
pub(crate) mod nomenclature_guard;
mod openclaw_contract;
mod operations_catalog;
mod pm_provenance;
mod release_build;
mod scaling_audit;
mod scientia_novelty_ledger_contract;
mod scientia_worthiness_contract;
pub mod workspace_artifacts;

mod cmd_enums;
mod constants;
mod coverage_gates;
mod run_body;

use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::Result;

pub use cmd_enums::{
    CiCmd, CoverageGateMode, DocInventoryCmd, EvalMatrixCmd, GrammarDriftEmit, MensScorecardCmd,
    OperationsSyncTarget, ScalingAuditCmd,
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
