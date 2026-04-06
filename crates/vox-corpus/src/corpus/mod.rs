//! Portable corpus utilities (mix YAML, structured eval helpers, decoding modes).

pub mod augment;
pub mod constrained_decoding;
pub mod coverage;
pub mod extract_docs;
pub mod extract_rs;
pub mod extract_vox;
pub mod mix;
pub mod preflight;
pub mod prompt_gen;
pub mod structured_eval;

pub use mix::{
    ASR_REFINE_INSTRUCTION, MixConfigSchema, MixRunOptions, MixRunReport, MixSourceReportRow,
    normalize_training_jsonl_line, run_mix, run_mix_with_options,
};
