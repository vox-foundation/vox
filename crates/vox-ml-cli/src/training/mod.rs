//! Shared training data utilities for the Vox CLI.
//!
//! Provides construct extraction, JSONL record emission, and instruction
//! template generation. Used by `vox check --emit-training-jsonl` and
//! `vox corpus` subcommands.

pub mod core;
mod decl_pairs;
mod instruction;
mod system_prompt;
mod taxonomy;

pub use core::{
    SCHEMA_VERSION, append_jsonl, build_training_record, extract_constructs, timestamp_string,
    walk_vox_files,
};
pub use decl_pairs::{PairStats, pairs_for_file};
pub use instruction::{extract_name_from_source, instruction_templates, split_training_metadata};
#[allow(unused_imports)] // re-exported for CLI / corpus tooling
pub use system_prompt::{CONSTRUCT_DOCS, SYSTEM_PROMPT_PREAMBLE, generate_system_prompt};
pub use taxonomy::{TAXONOMY, construct_difficulty};
