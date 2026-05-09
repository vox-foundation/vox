//! Declarative rule-pack loader. Loads YAML rule definitions used by
//! `vox-code-audit` detectors and `vox-publisher` Scientia heuristics.
//!
//! See `docs/src/architecture/2026-05-09-detector-rule-ssot-design.md`.

#![deny(rust_2018_idioms)]

pub mod error;
pub use error::{RulePackError, RulePackResult};

pub mod types;
pub use types::{RuleConfidence, RuleLanguage, RuleSeverity};

pub mod schema;
pub use schema::{FixtureSpec, MatchKind, MatchSpec, RuleFile, RuleSpec, SkipScope};

pub mod pack;
pub use pack::{CompiledRule, RulePack};

pub mod bench;
pub use bench::{BenchReport, RuleBenchResult, run_bench};
