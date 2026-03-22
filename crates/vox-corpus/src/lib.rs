//! Corpus and training **SSOT** for Populi: paths, preflight, dataset contracts, and corpus helpers.
//!
//! Compiler-coupled extraction stays in `vox-cli` (`commands/corpus.rs`). This crate holds portable
//! metadata, validation, mix/benchmark contracts, and Codex-oriented snapshot types.

pub mod corpus;
pub mod dataset_snapshot;
pub mod tool_workflow_corpus;
pub mod training;
