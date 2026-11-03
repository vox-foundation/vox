//! Workspace discovery for training SSOT paths (`training_contract.yaml`, system prompt, presets).

use std::path::{Path, PathBuf};

/// Walk up from [`std::env::current_dir`] until a `Cargo.toml` containing a `[workspace]` section is found.
///
/// Returns [`None`] if the current directory cannot be read or no workspace manifest is found.
pub fn find_workspace_root() -> Option<PathBuf> {
    vox_repository::find_cargo_workspace_root()
}

/// Resolve `path` relative to the workspace root when possible; otherwise return `path` as-is.
pub fn resolve_from_workspace(path: &Path) -> PathBuf {
    vox_repository::resolve_from_cargo_workspace(path)
}

/// Anchor a **relative** CLI path to the Cargo workspace (e.g. `--data-dir`, `--output-dir`, `--resume`).
///
/// Absolute paths are returned unchanged. When `workspace_root` is [`None`], falls back to
/// [`resolve_from_workspace`] (workspace discovery from [`std::env::current_dir`]).
pub fn normalize_workspace_relative_path(path: PathBuf, workspace_root: Option<&Path>) -> PathBuf {
    if path.is_absolute() {
        return path;
    }
    if let Some(ws) = workspace_root {
        return ws.join(&path);
    }
    resolve_from_workspace(&path)
}

/// Preferred alias for training data directories (same behavior as [`normalize_workspace_relative_path`]).
#[inline]
pub fn normalize_training_data_dir(data_dir: PathBuf, workspace_root: Option<&Path>) -> PathBuf {
    normalize_workspace_relative_path(data_dir, workspace_root)
}

/// Optional [`normalize_workspace_relative_path`] for checkpoint resume paths when relative to repo root.
#[inline]
pub fn normalize_training_resume_path(resume: PathBuf, workspace_root: Option<&Path>) -> PathBuf {
    normalize_workspace_relative_path(resume, workspace_root)
}

/// Canonical 5-spoke taxonomy for Mens training, verification, evaluation, and audit (`CORPUS-01`).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SpokeKind {
    /// Spoke 1: `.vox` compiler (`lex -> parse -> HIR -> typeck`) and `--interp` / codegen.
    VoxLang,
    /// Spoke 2: First-party Rust workspace (`crates/vox-*`, `cargo check` / `clippy` / `test`).
    RustWorkspace,
    /// Spoke 3: Structured CLI, Nushell (`!nu`), and Vox-Shell (`term_run`).
    StructuredShell,
    /// Spoke 4: MCP tool call & schema-validated handler chain (`vox-mcp-registry`).
    McpToolChain,
    /// Spoke 5: Multi-step Orchestrator / GUI agent harness trajectory.
    AgentHarness,
}

impl SpokeKind {
    /// All 5 canonical spokes in deterministic order.
    pub const ALL: [SpokeKind; 5] = [
        SpokeKind::VoxLang,
        SpokeKind::RustWorkspace,
        SpokeKind::StructuredShell,
        SpokeKind::McpToolChain,
        SpokeKind::AgentHarness,
    ];

    /// Stable snake_case identifier used in Trusty URIs, SQL columns, and JSONL lanes.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            SpokeKind::VoxLang => "vox_lang",
            SpokeKind::RustWorkspace => "rust_workspace",
            SpokeKind::StructuredShell => "structured_shell",
            SpokeKind::McpToolChain => "mcp_tool_chain",
            SpokeKind::AgentHarness => "agent_harness",
        }
    }
}

/// Deterministic verification receipt emitted by a spoke's admission oracle (`CORPUS-01`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OracleReceipt {
    /// Spoke that verified the record.
    pub spoke: SpokeKind,
    /// True only when all required deterministic checks for the spoke passed.
    pub passed: bool,
    /// Verification stage reached (e.g. `typeck+interp`, `syn+quality`, `redact+safety_floor`, `mcp_schema`).
    pub stage: String,
    /// Structured diagnostic messages (empty on clean pass).
    #[serde(default)]
    pub diagnostics: Vec<String>,
    /// Optional subprocess / interpreter exit code (`0` on pass).
    pub exit_code: Option<i32>,
}

/// Provenance metadata travelling with every [`TrainingRecordV2`] (`CORPUS-01`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RecordProvenance {
    /// Origin class (`human`, `repo_source`, `docs`, `ast_mutator`, `audit_repair`, `arca_replay`).
    pub origin: String,
    /// Relative source file path or session identifier.
    pub source_id: String,
    /// Optional task family or category tag.
    pub task_family: String,
    /// Difficulty score (1..=10).
    pub difficulty: u8,
}

/// Versioned (`v2`), schema-validated multi-spoke training record (`CORPUS-01`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TrainingRecordV2 {
    /// Schema discriminator (`"vox.training_record.v2"`).
    pub schema_version: String,
    /// Target spoke (`VoxLang`, `RustWorkspace`, `StructuredShell`, `McpToolChain`, `AgentHarness`).
    pub spoke: SpokeKind,
    /// User / task instruction prompt.
    pub prompt: String,
    /// System prompt or contextual preamble.
    pub system_context: String,
    /// Verified assistant response (code, structured shell pipeline, MCP tool call JSON, or trajectory).
    pub response: String,
    /// Optional rejected completion when this record also serves as a DPO contrast pair.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejected_response: Option<String>,
    /// Deterministic oracle verification receipt proving admission validity.
    pub oracle_receipt: OracleReceipt,
    /// 128-bit canonical content digest (`xxh3_128` 32-char lowercase hex).
    pub content_digest: String,
    /// Provenance metadata.
    pub provenance: RecordProvenance,
}

impl TrainingRecordV2 {
    /// Current schema discriminator constant.
    pub const SCHEMA_VERSION: &'static str = "vox.training_record.v2";

    /// Construct a new [`TrainingRecordV2`] and compute its canonical content digest.
    #[must_use]
    pub fn new(
        spoke: SpokeKind,
        prompt: impl Into<String>,
        system_context: impl Into<String>,
        response: impl Into<String>,
        oracle_receipt: OracleReceipt,
        provenance: RecordProvenance,
    ) -> Self {
        let prompt = prompt.into();
        let system_context = system_context.into();
        let response = response.into();
        let content_digest =
            Self::compute_content_digest(spoke, &prompt, &system_context, &response);
        Self {
            schema_version: Self::SCHEMA_VERSION.to_string(),
            spoke,
            prompt,
            system_context,
            response,
            rejected_response: None,
            oracle_receipt,
            content_digest,
            provenance,
        }
    }

    /// Compute the deterministic 128-bit content digest over `(spoke, prompt, system_context, response)`.
    #[must_use]
    pub fn compute_content_digest(
        spoke: SpokeKind,
        prompt: &str,
        system_context: &str,
        response: &str,
    ) -> String {
        let canonical = format!(
            "{}\0{}\0{}\0{}",
            spoke.as_str(),
            prompt.trim(),
            system_context.trim(),
            response.trim()
        );
        let h = xxhash_rust::xxh3::xxh3_128(canonical.as_bytes());
        format!("{h:032x}")
    }

    /// Format the record's immutable Trusty URI (`trusty://vox-corpus/<spoke>/<content_digest>`).
    #[must_use]
    pub fn trusty_uri(&self) -> String {
        format!(
            "trusty://vox-corpus/{}/{}",
            self.spoke.as_str(),
            self.content_digest
        )
    }

    /// Verify schema version, non-empty prompt/response, oracle pass status, and content digest integrity.
    #[must_use]
    pub fn verify_integrity(&self) -> bool {
        if self.schema_version != Self::SCHEMA_VERSION {
            return false;
        }
        if self.prompt.trim().is_empty() || self.response.trim().is_empty() {
            return false;
        }
        if self.oracle_receipt.spoke != self.spoke || !self.oracle_receipt.passed {
            return false;
        }
        let expected = Self::compute_content_digest(
            self.spoke,
            &self.prompt,
            &self.system_context,
            &self.response,
        );
        self.content_digest == expected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_absolute_is_unchanged_with_workspace() {
        let ws = PathBuf::from("/repo");
        // vox-arch-check: allow abs-path
        let abs = PathBuf::from(if cfg!(windows) {
            r"C:\abs\train"
        } else {
            "/abs/train"
        });
        let got = normalize_workspace_relative_path(abs.clone(), Some(ws.as_path()));
        assert_eq!(got, abs);
    }

    #[test]
    fn normalize_relative_joins_workspace() {
        // vox-arch-check: allow abs-path
        let ws = PathBuf::from(if cfg!(windows) {
            r"C:\repo\vox"
        } else {
            "/repo/vox"
        });
        let got =
            normalize_workspace_relative_path(PathBuf::from("target/dogfood"), Some(ws.as_path()));
        assert_eq!(got, ws.join("target/dogfood"));
    }

    #[test]
    fn training_record_v2_computes_digest_and_verifies_integrity() {
        let receipt = OracleReceipt {
            spoke: SpokeKind::VoxLang,
            passed: true,
            stage: "lex+parse+hir+typeck".to_string(),
            diagnostics: Vec::new(),
            exit_code: Some(0),
        };
        let prov = RecordProvenance {
            origin: "repo_source".to_string(),
            source_id: "examples/golden/hello.vox".to_string(),
            task_family: "vox_fn".to_string(),
            difficulty: 4,
        };
        let rec = TrainingRecordV2::new(
            SpokeKind::VoxLang,
            "Write a Vox function `add`",
            "system prompt",
            "fn add(a: int, b: int) to int { return a + b }",
            receipt,
            prov,
        );
        assert_eq!(rec.schema_version, TrainingRecordV2::SCHEMA_VERSION);
        assert_eq!(rec.content_digest.len(), 32);
        assert!(
            rec.trusty_uri()
                .starts_with("trusty://vox-corpus/vox_lang/")
        );
        assert!(rec.verify_integrity());

        let mut tampered = rec.clone();
        tampered.response.push_str("\n// tampered");
        assert!(
            !tampered.verify_integrity(),
            "tampered response must fail digest verification"
        );
    }
}
