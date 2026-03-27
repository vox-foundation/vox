//! # vox-toestub
//!
//! **T**odo, **O**mitted wiring, **E**mpty bodies, **S**tub functions,
//! **T**oo-early victory, **U**nresolved references, **B**roken DRY — detector.
//!
//! TOESTUB mechanically detects AI coding anti-patterns that are banned by
//! AGENTS.md but otherwise only caught during manual review.
//!
//! Public modules and re-exports are intentionally thin; each detector/rule is documented in its
//! own file where non-obvious heuristics exist.

mod bounded_fs;

/// Optional LLM-backed triage: wraps provider-specific clients behind a small `AiAnalyzer` API.
pub mod ai_analyze;
/// Token maps, optional `syn` AST, and other shared analysis for detectors.
pub mod analysis;
/// Concrete TOESTUB rules (stubs, empty bodies, secrets, DRY, …) registered by [`detectors::all_rules`].
pub mod detectors;
/// Per-run canary / rollout flags for detectors (set by [`engine::ToestubEngine`]).
pub mod run_context;

/// Structured suppression store (`contracts/toestub/suppression.v1.schema.json`).
pub mod suppression;

/// Runs configured detectors over a [`scanner::Scanner`] snapshot and aggregates [`rules::Finding`]s.
pub mod engine;
/// Renders findings to the terminal, JSON, or Markdown for CI and local CLI output.
pub mod report;
/// End-to-end **code review** flow: prompts, provider adapters (OpenAI, Ollama, …), SARIF/MD emit.
pub mod review;
/// Shared model for a single finding, severity/language enums, and the [`rules::DetectionRule`] trait.
pub mod rules;
/// Collects `SourceFile` entries from a repo path with language detection from extensions.
pub mod scanner;
/// In-memory bounded work queue used to cap parallel file/review tasks.
pub mod task_queue;

pub use ai_analyze::{AiAnalyzer, AiProvider};
pub use analysis::{NonCodeKind, RustFileContext, TokenMap};
pub use engine::{ToestubConfig, ToestubEngine, ToestubRunMode};
pub use report::{OutputFormat, Reporter, RunSnapshot, ToestubJsonReportV1};
pub use review::{
    ReviewCategory, ReviewClient, ReviewConfig, ReviewFinding, ReviewOutputFormat, ReviewProvider,
    ReviewResult, auto_discover_providers, build_diff_review_prompt, build_review_prompt,
    format_markdown, format_sarif, format_terminal, parse_review_response, review_system_prompt,
};
pub use rules::{DetectionRule, Finding, FindingConfidence, Language, Severity};
pub use run_context::ToestubTestsMode;
pub use scanner::Scanner;
pub use task_queue::TaskQueue;
