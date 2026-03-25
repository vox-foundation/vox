//! Helpers for `run_body` (`vox ci` SSOT / matrix guards), split from `include!` shards.

mod cuda;
mod docs;
mod grammar;
mod guards;
mod hash;
mod matrix;
mod timings;

pub(crate) use cuda::run_cuda_features;
pub(crate) use docs::{check_codex_ssot, check_docs_ssot, run_manifest, run_ssot_drift};
pub(crate) use grammar::run_grammar_drift;
pub(crate) use guards::run_repo_guards;
pub(crate) use matrix::{
    check_no_vox_dei, check_workflow_scripts, run_feature_matrix, run_mens_gate, run_toestub_scoped,
};
pub(crate) use timings::run_build_timings;
