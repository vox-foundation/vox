//! Helpers for `run_body` (`vox ci` SSOT / matrix guards), split from `include!` shards.

mod corpus_decl_coverage;
mod cuda;
mod cuda_release_build;
mod data_ssot_guards;
mod docs;
mod grammar;
mod guards;
mod hash;
mod matrix;
mod timings;

pub(crate) use corpus_decl_coverage::run_corpus_decl_coverage;
pub(crate) use cuda::run_cuda_features;
pub(crate) use cuda_release_build::run_cuda_release_build;
pub(crate) use data_ssot_guards::run_data_ssot_guards;
pub(crate) use docs::{check_codex_ssot, check_docs_ssot, run_manifest, run_ssot_drift};
pub(crate) use grammar::run_grammar_drift;
pub(crate) use guards::{
    run_clavis_cutover_audit, run_clavis_cutover_gates, run_clavis_parity, run_repo_guards,
    run_secret_env_guard, run_sql_surface_guard,
};
pub(crate) use matrix::{
    MensGateOpts, check_no_vox_dei, check_workflow_scripts, run_feature_matrix, run_mens_gate,
    run_toestub_scoped, run_toestub_self_apply,
};
pub(crate) use timings::run_build_timings;
