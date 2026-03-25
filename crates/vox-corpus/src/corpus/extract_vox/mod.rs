//! `.vox` source code corpus extractor for Mens training data.
//!
//! Walks `crates/vox-parser/tests/golden/**/*.vox`, `crates/vox-integration-tests/**/*.vox`,
//! and any other `.vox` files under the repo root, extracting complete file contents as
//! `prompt`/`response` training pairs.
//!
//! ## Extraction strategy
//! - **Per-file extraction**: each `.vox` file becomes one training pair.
//! - **Prompt**: derived from file-level `#` comments or auto-generated from filename.
//! - **Response**: the complete file contents (valid Vox syntax).
//! - **Category**: inferred from the file path and content constructs.
//! - **Per-block extraction**: each top-level construct (fn, actor, workflow, etc.) becomes
//!   a separate training pair with a targeted prompt.

use std::path::{Path, PathBuf};

use anyhow::Context;
use serde_json::json;

/// Configuration for Vox source extraction.
#[derive(Debug, Clone)]
pub struct ExtractVoxConfig {
    /// Root directory to walk (usually the repo root).
    pub root: PathBuf,
    /// Minimum number of non-comment, non-empty lines to include a file.
    pub min_content_lines: usize,
    /// Maximum number of pairs to emit (0 = unlimited).
    pub limit: usize,
    /// Default quality rating for Vox source pairs.
    pub default_rating: u8,
}

impl Default for ExtractVoxConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            min_content_lines: 2,
            limit: 0,
            default_rating: 5, // higher than Rust source — Vox is the target language
        }
    }
}

/// One extracted training pair from a `.vox` source.
#[derive(Debug, Clone)]
pub struct VoxTrainingPair {
    /// Source file path (relative to root).
    pub source_path: PathBuf,
    /// Inferred construct category.
    pub category: String,
    /// The prompt (doc comment text or generated imperative).
    pub prompt: String,
    /// The Vox source code block.
    pub response: String,
    /// Quality rating.
    pub rating: u8,
}

impl VoxTrainingPair {
    /// Serialize to a JSONL row compatible with `vox_tensor::data::TrainingPair`.
    #[must_use]
    pub fn to_jsonl(&self) -> String {
        let v = json!({
            "prompt": self.prompt,
            "response": self.response,
            "category": self.category,
            "rating": self.rating,
            "source": self.source_path.display().to_string(),
            "format": "vox_source",
        });
        v.to_string()
    }
}

/// Prompt templates for different construct types discovered in Vox files.
const CONSTRUCT_PROMPTS: &[(&str, &[&str])] = &[
    (
        "fn",
        &[
            "Write a Vox function called `{name}`",
            "Implement the `{name}` function in Vox",
            "Show me a Vox function named `{name}`",
        ],
    ),
    (
        "actor",
        &[
            "Define a Vox actor called `{name}`",
            "Create an actor named `{name}` in Vox with state and message handlers",
        ],
    ),
    (
        "workflow",
        &[
            "Write a durable Vox workflow called `{name}`",
            "Implement the `{name}` workflow with retry semantics in Vox",
        ],
    ),
    (
        "activity",
        &[
            "Define a Vox activity called `{name}`",
            "Write an activity function named `{name}` in Vox",
        ],
    ),
    (
        "component",
        &[
            "Create a Vox UI component called `{name}`",
            "Write a component function named `{name}` that returns Element",
        ],
    ),
    (
        "table",
        &[
            "Define a Vox @table schema called `{name}`",
            "Write a database table definition named `{name}` in Vox",
        ],
    ),
    (
        "type",
        &[
            "Define a Vox type called `{name}`",
            "Create a tagged union type named `{name}` in Vox",
        ],
    ),
    (
        "query",
        &[
            "Write a Vox @query function called `{name}`",
            "Implement a read-only data query named `{name}` in Vox",
        ],
    ),
    (
        "mutation",
        &[
            "Write a Vox @mutation function called `{name}`",
            "Implement a data mutation named `{name}` in Vox",
        ],
    ),
    (
        "mcp_tool",
        &[
            "Define an MCP tool called `{name}` in Vox",
            "Write a @mcp.tool function named `{name}`",
        ],
    ),
    (
        "test",
        &[
            "Write a Vox test called `{name}`",
            "Create a unit test named `{name}` in Vox",
        ],
    ),
];

include!("part_helpers.rs");
include!("part_walk.rs");
