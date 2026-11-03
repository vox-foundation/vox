//! Core types for documentation mention extraction and resolution.

use serde::{Deserialize, Serialize};

/// The semantic kind of code or system mention found in documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MentionKind {
    /// Repository file path (e.g. `crates/vox-cli/src/main.rs`).
    FilePath,
    /// Workspace or ecosystem crate name (e.g. `vox-cli`, `vox-orchestrator`).
    CrateName,
    /// Path-qualified Rust symbol (e.g. `vox_actor_runtime::llm`, `MemoryManager::persist_fact`).
    RustSymbol,
    /// CLI command or flag invocation (e.g. `vox audit docs`, `vox check`).
    CliCommand,
    /// Environment variable (e.g. `VOX_DB_URL`, `RUST_LOG`).
    EnvVar,
    /// Dotted configuration key from contracts/config (e.g. `server.port`).
    ConfigKey,
}

/// The prescriptive role of the mention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MentionRole {
    /// Normative prescription: user is instructed to run, import, configure, or use this.
    /// Deficits here are actionable documentation defects.
    Prescribes,
    /// Illustrative or descriptive mention: historical explanation, migration context,
    /// or explicitly skipped example (`// vox:skip` or fenced with `text`).
    Mentions,
}

/// Status of resolving an extracted mention against repository ground truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionStatus {
    /// The target exists and is verified in repo ground truth.
    Resolved,
    /// The target could not be found anywhere in repo ground truth.
    Missing,
    /// The target is retired/deprecated according to retired-symbols contract.
    Retired,
    /// The target matches multiple distinct symbols or is insufficiently qualified.
    Ambiguous,
}

/// A single code or platform token extracted from a documentation file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractedMention {
    /// The classification of the mention.
    pub kind: MentionKind,
    /// Whether the mention is prescriptive or merely descriptive/illustrative.
    pub role: MentionRole,
    /// The raw substring as it appeared in source text.
    pub raw_text: String,
    /// Canonical normalized identifier (e.g. trimmed, unquoted, lowercase for commands).
    pub normalized_target: String,
    /// Relative path of the document containing this mention.
    pub source_file: String,
    /// 1-indexed line number where the mention begins.
    pub line_number: usize,
    /// Byte offset in the file.
    pub byte_offset: usize,
    /// Info string of the enclosing code fence if inside one (e.g. "vox", "rust", "bash", "text").
    pub fence_info: Option<String>,
}

/// The result of resolving an extracted mention against ground truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedMention {
    /// The underlying extracted mention.
    pub extracted: ExtractedMention,
    /// Resolution verdict.
    pub status: ResolutionStatus,
    /// Suggested replacement if retired or renamed.
    pub replacement: Option<String>,
    /// Explanation if retired or ambiguous.
    pub rationale: Option<String>,
}

impl ResolvedMention {
    /// Whether this mention represents a deficit (missing or retired while prescribing behavior).
    #[must_use]
    pub fn is_actionable_deficit(&self) -> bool {
        self.extracted.role == MentionRole::Prescribes
            && matches!(
                self.status,
                ResolutionStatus::Missing | ResolutionStatus::Retired
            )
    }
}
