//! `.vox` source code corpus extractor for Mens training data.
//!
//! Walks `examples/golden/**/*.vox`, `crates/**/tests/**/*.vox`, integration fixtures, and other
//! `.vox` files under the repo root, emitting `prompt`/`response` training pairs.
//!
//! With **`ast-extract`**, per-declaration slices come from the **`vox-compiler`** parse tree
//! (no line-heuristic drift). Without it, a legacy line scanner is used.
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

#[cfg(feature = "ast-extract")]
pub(crate) mod part_ast;

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
    (
        "import",
        &[
            "Write a Vox import line for `{name}`",
            "Add the correct Vox `import` for `{name}`",
        ],
    ),
    (
        "http_route",
        &[
            "Write a Vox HTTP route for `{name}`",
            "Declare an HTTP handler in Vox matching `{name}`",
        ],
    ),
    (
        "mcp_resource",
        &[
            "Define an MCP resource in Vox related to `{name}`",
            "Write a `@mcp.resource` handler for `{name}`",
        ],
    ),
    (
        "server_fn",
        &[
            "Write a `@server` function `{name}` in Vox",
            "Implement server-side handler `{name}` in Vox",
        ],
    ),
    (
        "const",
        &[
            "Declare a Vox constant `{name}`",
            "Define `const {name}` with an explicit type in Vox",
        ],
    ),
    (
        "skill",
        &[
            "Define a Vox skill `{name}`",
            "Write a skill block named `{name}` for agent tooling",
        ],
    ),
    (
        "agent_def",
        &[
            "Define a Vox agent `{name}`",
            "Write an agent specification named `{name}` in Vox",
        ],
    ),
    (
        "reactive_component",
        &[
            "Create a reactive Vox component `{name}`",
            "Write a stateful UI component `{name}` with `state` / `view`",
        ],
    ),
    (
        "routes",
        &[
            "Wire up Vox routes in the `{name}` group",
            "Define route wiring for `{name}` in Vox",
        ],
    ),
    (
        "collection",
        &[
            "Define a Vox collection schema `{name}`",
            "Declare collection `{name}` for persistence in Vox",
        ],
    ),
    (
        "index",
        &[
            "Add a Vox index `{name}` for queries",
            "Define database index `{name}` in Vox",
        ],
    ),
    (
        "v0_component",
        &[
            "Write a V0 component `{name}` in Vox",
            "Scaffold `{name}` using `@v0` patterns in Vox",
        ],
    ),
    (
        "agent",
        &[
            "Declare a Vox agent instance `{name}`",
            "Configure agent `{name}` in Vox",
        ],
    ),
];

include!("part_helpers.rs");
include!("part_walk.rs");
