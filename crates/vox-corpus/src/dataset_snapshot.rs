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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

pub fn create_snapshot(
    src_dir: &std::path::Path,
    dest_base: &std::path::Path,
) -> anyhow::Result<String> {
    use std::fs;
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let version = format!("v_{}", timestamp);
    let dest_dir = dest_base.join(&version);

    fs::create_dir_all(&dest_dir)?;

    let mut total_rows = 0;

    for entry in fs::read_dir(src_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "jsonl") {
            let file_name = path.file_name().unwrap();
            let dest_path = dest_dir.join(file_name);

            // Count rows
            let content = fs::read_to_string(&path)?;
            total_rows += content.lines().filter(|l| !l.trim().is_empty()).count();

            fs::copy(&path, &dest_path)?;
        }
    }

    let manifest = DatasetSnapshotManifest {
        snapshot_id: version.clone(),
        created_at_rfc3339: chrono::Utc::now().to_rfc3339(),
        tokenizer_id: "vox_qwen_v1".to_string(), // Default
        base_model_id: None,
        sources: vec![], // In production we'd populate this per file
        total_rows: total_rows as u64,
        split: SplitPolicy {
            train_fraction: 0.95,
            eval_fraction: 0.05,
            seed: 42,
        },
        notes: Some("Automatic dogfood snapshot".to_string()),
    };

    let manifest_json = manifest.to_json()?;
    fs::write(dest_dir.join("manifest.json"), manifest_json)?;

    Ok(version)
}

/// Per-spoke row count and canonical digest inside a [`TrustyDatasetSnapshot`] (`CORPUS-06`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpokeMixtureSummary {
    /// Spoke identifier (`VoxLang`, `RustWorkspace`, `StructuredShell`, `McpToolChain`, `AgentHarness`).
    pub spoke: crate::training::SpokeKind,
    /// Number of verified `TrainingRecordV2` rows for this spoke in the snapshot.
    pub row_count: u64,
    /// 128-bit digest of this spoke's sorted record digests (`xxh3_128` hex).
    pub spoke_digest: String,
}

/// Immutable, Trusty-URI-identified multi-spoke dataset snapshot persisted in `VoxDb` (`CORPUS-06`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrustyDatasetSnapshot {
    /// Logical snapshot identifier (e.g. `mens_v1_6_snap_001`).
    pub snapshot_id: String,
    /// Immutable Trusty URI (`trusty://vox-dataset/<content_digest>`).
    pub trusty_uri: String,
    /// Canonical content digest of all records and split metadata (`xxh3_128` hex or `VoxDb` CAS hash).
    pub content_digest: String,
    /// Creation timestamp (RFC3339).
    pub created_at_rfc3339: String,
    /// Per-spoke breakdown in deterministic `SpokeKind` order.
    pub spoke_mixtures: Vec<SpokeMixtureSummary>,
    /// Total verified records in the snapshot.
    pub total_rows: u64,
    /// Train/eval split policy.
    pub split: SplitPolicy,
}

/// Namespace in `VoxDb` `names` table mapping `snapshot_id -> CAS object hash`.
pub const MENS_SNAPSHOT_NAMESPACE: &str = "mens_dataset_snapshot";

/// Construct a deterministic [`TrustyDatasetSnapshot`] and canonical JSONL bundle from verified
/// [`crate::training::TrainingRecordV2`] records (`CORPUS-06`).
#[must_use]
pub fn build_trusty_snapshot(
    snapshot_id: &str,
    records: &[crate::training::TrainingRecordV2],
    split: SplitPolicy,
) -> (TrustyDatasetSnapshot, String) {
    let mut spoke_mixtures = Vec::new();
    for spoke in crate::training::SpokeKind::ALL {
        let mut digests: Vec<&str> = records
            .iter()
            .filter(|r| r.spoke == spoke)
            .map(|r| r.content_digest.as_str())
            .collect();
        digests.sort_unstable();
        let row_count = digests.len() as u64;
        let joined = digests.join("\n");
        let spoke_digest = format!("{:032x}", xxhash_rust::xxh3::xxh3_128(joined.as_bytes()));
        spoke_mixtures.push(SpokeMixtureSummary {
            spoke,
            row_count,
            spoke_digest,
        });
    }

    let mut lines = Vec::with_capacity(records.len());
    for rec in records {
        if let Ok(line) = serde_json::to_string(rec) {
            lines.push(line);
        }
    }
    let jsonl = lines.join("\n");
    let content_digest = format!("{:032x}", xxhash_rust::xxh3::xxh3_128(jsonl.as_bytes()));
    let trusty_uri = format!("trusty://vox-dataset/{content_digest}");

    let snapshot = TrustyDatasetSnapshot {
        snapshot_id: snapshot_id.to_string(),
        trusty_uri,
        content_digest,
        created_at_rfc3339: chrono::Utc::now().to_rfc3339(),
        spoke_mixtures,
        total_rows: records.len() as u64,
        split,
    };
    (snapshot, jsonl)
}

#[cfg(feature = "database")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredSnapshotEnvelope {
    snapshot: TrustyDatasetSnapshot,
    records: Vec<crate::training::TrainingRecordV2>,
}

/// Persist an immutable [`TrustyDatasetSnapshot`] and its [`crate::training::TrainingRecordV2`] rows
/// into `VoxDb` (`objects` CAS + `names` binding + `mens_corpus_quality` rows) (`CORPUS-06`).
#[cfg(feature = "database")]
pub async fn persist_snapshot_to_db(
    db: &vox_db::VoxDb,
    snapshot_id: &str,
    records: &[crate::training::TrainingRecordV2],
    split: SplitPolicy,
) -> anyhow::Result<TrustyDatasetSnapshot> {
    let (mut snapshot, _jsonl) = build_trusty_snapshot(snapshot_id, records, split);
    let envelope = StoredSnapshotEnvelope {
        snapshot: snapshot.clone(),
        records: records.to_vec(),
    };
    let bytes = serde_json::to_vec(&envelope)?;
    let cas_hash = db
        .store("mens_dataset_snapshot", &bytes)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    snapshot.content_digest = cas_hash.clone();
    snapshot.trusty_uri = format!("trusty://vox-dataset/{cas_hash}");

    // Re-store with canonical CAS Trusty URI stamped into the envelope and bind both
    // the human-readable `snapshot_id` and `trusty_uri` in `names`.
    let final_envelope = StoredSnapshotEnvelope {
        snapshot: snapshot.clone(),
        records: records.to_vec(),
    };
    let final_bytes = serde_json::to_vec(&final_envelope)?;
    let final_hash = db
        .store("mens_dataset_snapshot", &final_bytes)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    db.bind_name(MENS_SNAPSHOT_NAMESPACE, snapshot_id, &final_hash)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    db.bind_name(MENS_SNAPSHOT_NAMESPACE, &snapshot.trusty_uri, &final_hash)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    for rec in records {
        let _ = db
            .upsert_corpus_quality(
                &rec.content_digest,
                rec.spoke.as_str(),
                rec.oracle_receipt.passed,
                rec.provenance.difficulty as usize,
                1,
                if rec.oracle_receipt.passed { 1.0 } else { 0.0 },
                snapshot_id,
            )
            .await;
    }

    Ok(snapshot)
}

/// Deterministically reload a [`TrustyDatasetSnapshot`] and its [`crate::training::TrainingRecordV2`]
/// rows from `VoxDb` by `snapshot_id` or `trusty_uri` (`CORPUS-06`).
#[cfg(feature = "database")]
pub async fn load_snapshot_from_db(
    db: &vox_db::VoxDb,
    snapshot_id_or_uri: &str,
) -> anyhow::Result<(
    TrustyDatasetSnapshot,
    Vec<crate::training::TrainingRecordV2>,
)> {
    let bindings = db
        .list_names_in_namespace(MENS_SNAPSHOT_NAMESPACE)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let hash = bindings
        .into_iter()
        .find(|(name, _)| name == snapshot_id_or_uri)
        .map(|(_, h)| h)
        .ok_or_else(|| {
            anyhow::anyhow!("dataset snapshot `{snapshot_id_or_uri}` not found in VoxDb")
        })?;
    let bytes = db
        .get(&hash)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let env: StoredSnapshotEnvelope = serde_json::from_slice(&bytes)?;
    Ok((env.snapshot, env.records))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::training::{OracleReceipt, RecordProvenance, SpokeKind, TrainingRecordV2};

    fn sample_records() -> Vec<TrainingRecordV2> {
        vec![
            TrainingRecordV2::new(
                SpokeKind::VoxLang,
                "Write a Vox function `inc`",
                "sys",
                "fn inc(x: int) to int { return x + 1 }",
                OracleReceipt {
                    spoke: SpokeKind::VoxLang,
                    passed: true,
                    stage: "typeck+interp".to_string(),
                    diagnostics: Vec::new(),
                    exit_code: Some(0),
                },
                RecordProvenance {
                    origin: "repo_source".to_string(),
                    source_id: "inc.vox".to_string(),
                    task_family: "vox_fn".to_string(),
                    difficulty: 3,
                },
            ),
            TrainingRecordV2::new(
                SpokeKind::RustWorkspace,
                "Implement `inc` in Rust",
                "sys",
                "pub fn inc(x: i64) -> i64 { x + 1 }",
                OracleReceipt {
                    spoke: SpokeKind::RustWorkspace,
                    passed: true,
                    stage: "syn_ast+quality".to_string(),
                    diagnostics: Vec::new(),
                    exit_code: Some(0),
                },
                RecordProvenance {
                    origin: "repo_rust".to_string(),
                    source_id: "lib.rs".to_string(),
                    task_family: "rust_fn".to_string(),
                    difficulty: 3,
                },
            ),
        ]
    }

    #[test]
    fn build_trusty_snapshot_is_deterministic() {
        let recs = sample_records();
        let split = SplitPolicy {
            train_fraction: 0.9,
            eval_fraction: 0.1,
            seed: 42,
        };
        let (s1, j1) = build_trusty_snapshot("snap-1", &recs, split.clone());
        let (s2, j2) = build_trusty_snapshot("snap-1", &recs, split);
        assert_eq!(s1.trusty_uri, s2.trusty_uri);
        assert_eq!(s1.content_digest, s2.content_digest);
        assert_eq!(j1, j2);
        assert_eq!(s1.total_rows, 2);
        assert_eq!(s1.spoke_mixtures.len(), 5);
    }

    #[test]
    fn snapshot_jsonl_roundtrips_cleanly() {
        let recs = sample_records();
        let split = SplitPolicy {
            train_fraction: 0.9,
            eval_fraction: 0.1,
            seed: 42,
        };
        let (meta, jsonl) = build_trusty_snapshot("snap-db-1", &recs, split);
        assert!(meta.trusty_uri.starts_with("trusty://vox-dataset/"));
        let parsed: Vec<TrainingRecordV2> = jsonl
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).expect("valid TrainingRecordV2 row"))
            .collect();
        assert_eq!(parsed, recs);
    }
}
