//! SSOT / matrix guard implementations for `vox ci`.

use anyhow::{Result, anyhow};
use std::process::Command;

use super::build_timings;
use super::check_links;
use super::cmd_enums::{CiCmd, DocInventoryCmd, EvalMatrixCmd};
use super::command_compliance;
use super::command_sync;
use super::contracts_index;
use super::coverage_gates;
use super::eval_matrix;
use super::line_endings;
use super::release_build;
use super::scaling_audit;
use super::scientia_worthiness_contract;
use super::{cargo_bin, repo_root};

/// Helpers live in `ci/run_body_helpers/`; `#[path]` keeps them out of `ci/run_body/` (submodule rule).
#[path = "run_body_helpers/mod.rs"]
mod run_body_helpers;

use run_body_helpers::{
    MensGateOpts, check_codex_ssot, check_docs_ssot, check_no_vox_dei, check_workflow_scripts,
    run_build_timings, run_clavis_parity, run_cuda_features, run_cuda_release_build,
    run_feature_matrix, run_grammar_drift, run_manifest, run_mens_gate, run_repo_guards,
    run_data_ssot_guards, run_secret_env_guard, run_sql_surface_guard, run_ssot_drift,
    run_toestub_scoped,
    run_toestub_self_apply,
};

/// Run `vox ci` subcommand.
pub async fn run(cmd: CiCmd) -> Result<()> {
    let root = repo_root();
    match cmd {
        CiCmd::Manifest => run_manifest(&root),
        CiCmd::CheckDocsSsot => check_docs_ssot(&root),
        CiCmd::CheckCodexSsot => check_codex_ssot(&root),
        CiCmd::ContractsIndex => contracts_index::run(&root),
        CiCmd::ScientiaWorthinessContract => scientia_worthiness_contract::run(&root),
        CiCmd::SsotDrift => run_ssot_drift(&root),
        CiCmd::DataSsotGuards => run_data_ssot_guards(&root),
        CiCmd::FeatureMatrix => run_feature_matrix(&root),
        CiCmd::NoDeiImport => check_no_vox_dei(&root),
        CiCmd::CheckSummaryDrift => {
            let cargo = cargo_bin();
            let st = Command::new(&cargo)
                .current_dir(&root)
                .args(["run", "-p", "vox-doc-pipeline", "--", "--check"])
                .status()?;
            if !st.success() {
                return Err(anyhow!(
                    "SUMMARY.md is out of sync with docs/src. Run 'cargo run -p vox-doc-pipeline' to fix."
                ));
            }
            println!("SUMMARY.md is up to date.");
            Ok(())
        }
        CiCmd::BuildDocs => {
            let cargo = cargo_bin();
            // 1. Generate SUMMARY.md
            let st = Command::new(&cargo)
                .current_dir(&root)
                .args(["run", "-p", "vox-doc-pipeline"])
                .status()?;
            if !st.success() {
                return Err(anyhow!("failed to generate SUMMARY.md"));
            }
            // 2. Run mdbook build docs (assuming mdbook is on PATH)
            let st = Command::new("mdbook")
                .current_dir(&root)
                .args(["build", "docs"])
                .status()?;
            if !st.success() {
                return Err(anyhow!("mdbook build docs failed"));
            }
            // 3. sitemap.xml (mdbook-sitemap-generator is a post-build CLI, not a preprocessor)
            let domain = std::env::var("MDBOOK_SITEMAP_DOMAIN")
                .unwrap_or_else(|_| "https://vox-foundation.github.io/vox/".to_string());
            let domain_arg = domain.trim_end_matches('/').to_string();
            let st = Command::new("mdbook-sitemap-generator")
                .current_dir(root.join("docs"))
                .args([
                    "--domain",
                    domain_arg.as_str(),
                    "--output",
                    "book/html/sitemap.xml",
                ])
                .status()?;
            if !st.success() {
                return Err(anyhow!(
                    "mdbook-sitemap-generator failed (install: cargo install mdbook-sitemap-generator --version 0.2.0 --locked)"
                ));
            }
            println!("Documentation built successfully.");
            Ok(())
        }
        CiCmd::DocInventory { cmd: sub } => match sub {
            DocInventoryCmd::Generate { output } => {
                let out =
                    output.unwrap_or_else(|| root.join(vox_doc_inventory::DEFAULT_INVENTORY_PATH));
                vox_doc_inventory::generate(&root, &out)?;
                println!("Wrote {}", out.display());
                Ok(())
            }
            DocInventoryCmd::Verify => {
                let committed = root.join(vox_doc_inventory::DEFAULT_INVENTORY_PATH);
                vox_doc_inventory::verify_fresh(&root, &committed)?;
                println!("doc-inventory.json matches generator output (excluding generated_at)");
                Ok(())
            }
        },
        CiCmd::EvalMatrix { cmd: sub } => match sub {
            EvalMatrixCmd::Verify => eval_matrix::run_verify(&root),
            EvalMatrixCmd::Run { milestone } => {
                eval_matrix::run_executions(&root, milestone.as_deref())
            }
        },
        CiCmd::WorkflowScripts { allowlist } => check_workflow_scripts(&root, &allowlist),
        CiCmd::LineEndings { all, base } => line_endings::run(&root, all, base),
        CiCmd::MeshGate {
            profile,
            isolated_runner,
            windows_isolated_runner,
            gate_build_target_dir,
            gate_log_file,
        } => run_mens_gate(
            &root,
            &profile,
            &MensGateOpts {
                isolated_runner: isolated_runner || windows_isolated_runner,
                gate_build_target_dir,
                gate_log_file,
            },
        ),
        CiCmd::CudaReleaseBuild { log_dir } => run_cuda_release_build(&root, log_dir),
        CiCmd::ToestubSelfApply => run_toestub_self_apply(&root),
        CiCmd::ToestubScoped {
            root: scan_root,
            mode,
        } => run_toestub_scoped(&root, &scan_root, mode),
        CiCmd::ScalingAudit { cmd } => scaling_audit::run(&root, cmd),
        CiCmd::CudaFeatures => run_cuda_features(),
        CiCmd::BuildTimings {
            json,
            crates,
            deep,
            persist,
            name,
            profile,
        } => {
            if deep {
                build_timings::bench_build_run(persist.unwrap_or(true), name, Some(profile))
                    .await?;
                Ok(())
            } else {
                run_build_timings(&root, json, crates)
            }
        }
        CiCmd::GrammarDrift { emit } => run_grammar_drift(&root, emit),
        CiCmd::RepoGuards => run_repo_guards(&root),
        CiCmd::SecretEnvGuard { all } => run_secret_env_guard(&root, all),
        CiCmd::SqlSurfaceGuard { all } => run_sql_surface_guard(&root, all),
        CiCmd::ClavisParity => run_clavis_parity(&root),
        CiCmd::CommandCompliance => command_compliance::run(&root),
        CiCmd::CoverageGates {
            summary_json,
            mode,
            config,
        } => coverage_gates::run(summary_json, mode, config),
        CiCmd::CommandSync { write } => command_sync::run(&root, write),
        CiCmd::PmProvenance {
            strict,
            root: provenance_root,
        } => super::pm_provenance::run(&root, &provenance_root, strict),
        CiCmd::CheckLinks => check_links::run(&root),
        CiCmd::ReleaseBuild {
            target,
            version,
            out_dir,
            package,
        } => release_build::run(&root, &target, version.as_deref(), &out_dir, package),
    }
}
