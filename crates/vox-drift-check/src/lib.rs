//! Workspace-wide multi-language drift and pattern-repetition linter.
//!
//! Detects stale symbol references, naming drift, and copy-paste patterns
//! across Rust, TypeScript, and Vox source files.

pub mod cache;
pub mod config;
pub mod engine;
pub mod extractor;
pub mod extractors;
pub mod features;
pub mod report;
pub mod rules;
pub mod sweep;

pub use vox_code_audit::rules::Severity;
