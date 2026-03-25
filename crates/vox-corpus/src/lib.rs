//! Corpus and training **SSOT** for Mens: paths, preflight, dataset contracts, and corpus helpers.
//!
//! Compiler-coupled extraction stays in `vox-cli` (`commands/corpus.rs`). This crate holds portable
//! metadata, validation, mix/benchmark contracts, and Codex-oriented snapshot types.

pub mod arca_replay;
pub mod codegen_vox;
pub mod corpus;
pub mod dataset_snapshot;
pub mod synthetic_gen;
pub mod synthetic_search_gen;
pub mod tool_workflow_corpus;
pub mod training;
