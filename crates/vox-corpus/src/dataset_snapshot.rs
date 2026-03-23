//! Immutable dataset snapshot metadata (Codex `populi_dataset_snapshots` row shape).

use serde::{Deserialize, Serialize};

/// Source class contributing rows to a snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotSourceClass {
    /// `.vox` sources from the repository tree.
    RepoVox,
    /// Markdown / handbook text.
    Docs,
    /// Rows produced via Codex LLM pipelines.
    CodexLlm,
    /// Tool invocation traces for SFT.
    ToolTrace,
    /// Workflow execution traces.
    WorkflowTrace,
    /// Agent-to-agent coordination traces.
    A2ATrace,
    /// Packaged skill manifests.
    SkillManifest,
    /// Operator-attached or ad-hoc sources.
    Manual,
}

/// Per-source digest and counts for reproducibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceDigest {
    /// Kind of rows hashed in `digest`.
    pub class: SnapshotSourceClass,
    /// blake3 hex of canonical serialized payload
    pub digest: String,
    /// Number of rows from this class in the snapshot.
    pub row_count: u64,
}

/// Training/eval split policy stored alongside the snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitPolicy {
    /// Portion of rows assigned to training (0.0–1.0).
    pub train_fraction: f64,
    /// Portion assigned to evaluation (0.0–1.0).
    pub eval_fraction: f64,
    /// RNG seed for deterministic splits.
    pub seed: u64,
}

/// Full snapshot manifest written next to exported JSONL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetSnapshotManifest {
    /// Stable identifier for this export (often ULID or UUID).
    pub snapshot_id: String,
    /// Creation timestamp in RFC3339.
    pub created_at_rfc3339: String,
    /// Tokenizer id used when materializing rows.
    pub tokenizer_id: String,
    /// Optional base model id for LoRA / continued pretrain metadata.
    pub base_model_id: Option<String>,
    /// Per-class digests contributing to `total_rows`.
    pub sources: Vec<SourceDigest>,
    /// Sum of row counts across sources.
    pub total_rows: u64,
    /// Train/eval split parameters.
    pub split: SplitPolicy,
    /// Free-form operator notes.
    pub notes: Option<String>,
}

impl DatasetSnapshotManifest {
    /// Pretty-printed JSON for sidecar `manifest.json` next to JSONL exports.
    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}
