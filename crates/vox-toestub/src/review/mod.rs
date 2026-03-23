//! LLM code review: discover providers from env, build prompts, call HTTP APIs, emit SARIF/MD/JSON.
//!
//! [`ReviewClient`] is the orchestration entrypoint; see [`crate::review::providers`] for backends.

/// Async HTTP facade that tries [`ReviewProvider`]s in order until one returns completions.
pub mod client;
/// SARIF, terminal, and Markdown serializers plus loose parsing of model replies into findings.
pub mod formatters;
/// System prompt and per-file/diff prompt builders shared by CLI and tests.
pub mod prompts;
/// Provider enum (OpenRouter, OpenAI-compat, Gemini, Ollama, Pollinations) and `auto_discover_providers`.
pub mod providers;
/// [`ReviewConfig`], [`ReviewResult`], [`ReviewFinding`], and output format parsing.
pub mod types;

pub use client::ReviewClient;
pub use formatters::{format_markdown, format_sarif, format_terminal, parse_review_response};
pub use prompts::{build_diff_review_prompt, build_review_prompt, review_system_prompt};
pub use providers::{ReviewProvider, auto_discover_providers};
pub use types::{ReviewCategory, ReviewConfig, ReviewFinding, ReviewOutputFormat, ReviewResult};
