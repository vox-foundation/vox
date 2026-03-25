//! Shared training data utilities for the Vox CLI.
//!
//! Provides construct extraction, JSONL record emission, and instruction
//! template generation. Used by `vox check --emit-training-jsonl` and
//! `vox corpus` subcommands.

#[cfg(feature = "gpu")]
pub mod native;

mod core;
mod instruction;
mod multiturn;
mod negative;
mod system_prompt;
mod taxonomy;

pub use core::{
    SCHEMA_VERSION, append_jsonl, build_training_record, extract_constructs, timestamp_string,
    walk_vox_files,
};
pub use instruction::{extract_name_from_source, instruction_templates};
#[allow(unused_imports)] // re-exported for CLI / corpus tooling
pub use multiturn::{followup_templates, generate_multiturn_pairs};
pub use negative::generate_negative_examples;
#[allow(unused_imports)] // re-exported for CLI / corpus tooling
pub use system_prompt::{CONSTRUCT_DOCS, SYSTEM_PROMPT_PREAMBLE, generate_system_prompt};
pub use taxonomy::{TAXONOMY, construct_difficulty};
