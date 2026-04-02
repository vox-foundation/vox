//! Shared DDL fragments and derived schema digests for cross-crate SSOT.
//!
//! - Relational DDL [`POPULI_TRAINING_RUN_DDL`] / [`CODEX_CAPABILITY_MAP_DDL`] are appended to the
//!   Arca `agents` domain fragment so baseline migration creates them (no lazy DDL).
//! - [`orchestrator_schema_digest`] is the single definition for orchestrator `SchemaDigest` sync:
//!   document collections use the SQLite `_id`/`_data` layout (see [`crate::collection::Collection`]).

use crate::schema_digest::{CollectionInfo, FieldInfo, SchemaDigest};

/// Mens training run tracking (was lazy-created in `training_run.rs`).
pub const POPULI_TRAINING_RUN_DDL: &str = r"
CREATE TABLE IF NOT EXISTS populi_training_run (
    run_id               TEXT    NOT NULL PRIMARY KEY,
    adapter_tag          TEXT,
    model_name           TEXT,
    output_dir           TEXT    NOT NULL,
    data_dir             TEXT    NOT NULL,
    status               TEXT    NOT NULL DEFAULT 'running',
    epoch                INTEGER NOT NULL DEFAULT 0,
    global_step          INTEGER NOT NULL DEFAULT 0,
    planned_steps        INTEGER,
    last_loss            REAL,
    last_checkpoint_path TEXT,
    created_at           INTEGER NOT NULL,
    updated_at           INTEGER NOT NULL
);";

/// Competitive capability map (was lazy-created in `research.rs`).
pub const CODEX_CAPABILITY_MAP_DDL: &str = r"
CREATE TABLE IF NOT EXISTS codex_capability_map (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    topic TEXT NOT NULL,
    vendor TEXT NOT NULL,
    area TEXT NOT NULL,
    openclaw_capability TEXT NOT NULL,
    vox_evidence TEXT NOT NULL,
    status TEXT NOT NULL,
    advantage_direction TEXT NOT NULL,
    recommended_action TEXT NOT NULL,
    linked_paths_json TEXT NOT NULL,
    metadata_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_codex_cap_vendor_topic ON codex_capability_map (vendor, topic);";

fn usage_field_docs() -> Vec<FieldInfo> {
    vec![
        FieldInfo {
            name: "user_id".into(),
            type_str: "str".into(),
            is_optional: false,
            references_table: None,
        },
        FieldInfo {
            name: "provider".into(),
            type_str: "str".into(),
            is_optional: false,
            references_table: None,
        },
        FieldInfo {
            name: "model".into(),
            type_str: "str".into(),
            is_optional: false,
            references_table: None,
        },
        FieldInfo {
            name: "date".into(),
            type_str: "str".into(),
            is_optional: false,
            references_table: None,
        },
        FieldInfo {
            name: "calls".into(),
            type_str: "int".into(),
            is_optional: false,
            references_table: None,
        },
        FieldInfo {
            name: "tokens_in".into(),
            type_str: "int".into(),
            is_optional: false,
            references_table: None,
        },
        FieldInfo {
            name: "tokens_out".into(),
            type_str: "int".into(),
            is_optional: false,
            references_table: None,
        },
        FieldInfo {
            name: "cost_usd".into(),
            type_str: "float".into(),
            is_optional: false,
            references_table: None,
        },
        FieldInfo {
            name: "is_rate_limited".into(),
            type_str: "bool".into(),
            is_optional: false,
            references_table: None,
        },
        FieldInfo {
            name: "last_429".into(),
            type_str: "int".into(),
            is_optional: true,
            references_table: None,
        },
    ]
}

/// [`SchemaDigest`] for `vox-orchestrator` init: **collections** match [`crate::collection::Collection`].
///
/// `provider_usage` rows are JSON documents (`_data`), not flat SQL columns — the orchestrator
/// writes them via [`crate::collection::Collection`].
#[must_use]
pub fn orchestrator_schema_digest() -> SchemaDigest {
    SchemaDigest {
        tables: vec![],
        collections: vec![
            CollectionInfo {
                name: "provider_usage".into(),
                fields: usage_field_docs(),
                description: Some(
                    "Schemaless JSON docs: daily LLM usage per user/provider/model (orchestrator)"
                        .into(),
                ),
                is_public: false,
                sample_data: vec![],
            },
            CollectionInfo {
                name: "handoff_payloads".into(),
                fields: vec![],
                description: Some("Schemaless storage for agent handoff documents".into()),
                is_public: false,
                sample_data: vec![],
            },
            CollectionInfo {
                name: "attention_events".into(),
                fields: vec![],
                description: Some("Append-only pilot attention event log (Phase 15)".into()),
                is_public: false,
                sample_data: vec![],
            },
            CollectionInfo {
                name: "agent_trust_scores".into(),
                fields: vec![],
                description: Some(
                    "Per-agent EWMA trust scores for attention-aware routing (Phase 15)".into(),
                ),
                is_public: false,
                sample_data: vec![],
            },
        ],
        relationships: vec![],
        indexes: vec![],
        queries: vec![],
        mutations: vec![],
        actions: vec![],
        summary: "Vox Orchestrator Core Schema (collections SSOT in vox-db::schema::spec)".into(),
        vcs_snapshot_id: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VoxDb;

    #[tokio::test]
    async fn orchestrator_digest_sync_makes_collection_layout_for_provider_usage() {
        let db = VoxDb::open_memory().await.expect("mem");
        db.sync_schema_from_digest(&orchestrator_schema_digest())
            .await
            .expect("sync");
        db.collection("provider_usage")
            .ensure_table()
            .await
            .expect("ensure");
        let mut rows = db
            .connection()
            .query("PRAGMA table_info(provider_usage)", ())
            .await
            .expect("pragma");
        let mut cols = Vec::new();
        while let Some(r) = rows.next().await.expect("row") {
            let name: String = r.get(1).expect("name");
            cols.push(name);
        }
        assert!(
            cols.contains(&"_id".to_string()) && cols.contains(&"_data".to_string()),
            "expected collection columns, got {cols:?}"
        );
    }
}
