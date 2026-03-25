//! # vox-db — High-level database facade for Vox
//!
//! Provides a unified API over Turso/libSQL for packages, code artifacts, agent memory, Codex
//! reactivity, and related tables. Prefer this crate over talking to `turso::Connection` directly
//! in application code.
//!
//! ## Nomenclature (avoid mixing layers)
//!
//! | Name | What it is |
//! |------|------------|
//! | **[`VoxDb`]** | Stable **Rust type** for this facade; use it in signatures and tests. |
//! | **[`Codex`]** | **Type alias** for `VoxDb` — same type, product-facing name in docs/UI. |
//! | **Arca** | Internal name for **schema + SQL** owned by this crate (`crates/vox-db/src/schema/`). |
//! | **`vox-pm`** | Package registry / artifacts — **not** the SQL schema SSOT. |
//!
//! Use [`VoxDb::store`] (async method) for content-addressed blob writes (`ops_cas`); it is not a getter.
//!
//! ## Connection modes
//!
//! - **Remote** (Turso cloud) — always available
//! - **Local** (file-based Turso) — `local` feature (default)
//! - **In-memory** — `DbConfig::Memory`, tests only (`local` feature)
//! - **Embedded replica** (local + cloud sync) — `replication` feature
//!
//! ## Turso batch SQL caveat
//!
//! Built-in and app-supplied migrations run through [`turso::Connection::execute_batch`], which uses
//! `execute` and **fails on statements that return rows** (e.g. bare `SELECT`, assignment `PRAGMA`
//! unless handled with `pragma_update`). [`VoxDb::connect`] / [`VoxDb::open`] apply pragmas via
//! `pragma_update` and skip empty migration bodies; see also [`VoxDb::apply_migrations`].
//!
//! ```no_run
//! use vox_db::{VoxDb, DbConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let db = VoxDb::connect(DbConfig::Remote {
//!         url: "turso://my-db.turso.io".to_string(),
//!         token: "my-token".to_string(),
//!     }).await?;
//!
//!     let hash = db.store("fn", b"fn hello(): ret 42").await?;
//!     println!("Stored: {hash}");
//!     Ok(())
//! }
//! ```

#![allow(clippy::collapsible_if)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::single_char_add_str)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::useless_vec)]

/// Compare live SQLite schema to `@table` / collection declarations; non-destructive migrations.
pub mod auto_migrate;
/// Benchmark observations stored in `research_metrics` (`bench:<repository_id>` sessions).
pub mod benchmark_telemetry;
pub mod build_hints;
/// Circuit breaker for write operations.
pub mod circuit_breaker;
/// User chat, tool calls, usage limits, topics (manifest chat/search slices).
mod codex_chat;
/// Research sessions, conversation versions/edges, topic evolution (manifest `v17`).
mod codex_conversation_graph;
pub mod schema;
/// Idempotent fixes after baseline `CREATE IF NOT EXISTS` (column adds, renames).
mod schema_cutover;
/// Legacy import/export planning and verification for greenfield Codex releases.
pub mod store;

pub mod codex_legacy;
/// Manifest-derived readiness (baseline digest, required tables).
pub mod codex_schema;
pub mod collection;
mod config;
pub mod data_flow;
pub mod ddl;
pub mod error_enrichment;
/// Parameters for [`VoxDb::record_eval_run`].
mod eval_params;
pub mod hash;
pub mod learning;
pub mod legacy_import_extras;
/// Parameters for [`VoxDb::store_memory`].
pub mod memory;
/// Declarative SQL migrations using the `schema_version` table (see `crate::schema`).
pub mod migration;
/// Data directory and per-user id helpers (delegates to `vox_config`).
pub mod paths;
/// Mens control-plane audit (`populi_control_event` in `research_metrics`).
pub mod populi_control_telemetry;
/// Opt-in mens local-registry publish rows (`VOX_MESH_CODEX_TELEMETRY`).
pub mod populi_registry_telemetry;
/// Registry-scoped user preferences (stored as JSON in the local config directory).
pub mod preferences;
pub mod project_store;
mod research;
/// Hybrid retrieval helpers (vector / full-text fusion) for RAG-style pipelines.
pub mod retrieval;
/// AST → [`crate::SchemaDigest`] for LLM context and codegen.
pub mod schema_digest;
/// OS keyring helpers for API tokens and similar secrets.
pub mod secrets;
mod socrates_telemetry;
mod sync_invocables;
pub mod toestub_store;
/// Mens QLoRA training run persistence (CRUD for `populi_training_run` table).
pub mod training_run;
/// Interpreted workflow journal (`workflow_journal_entry` in `research_metrics`).
pub mod workflow_journal;

pub use auto_migrate::AutoMigrator;
pub use circuit_breaker::{CircuitBreakerError, CircuitState, DbCircuitBreaker};
pub use codex_schema::{
    CodexApiReadiness, evaluate_codex_api_readiness, missing_codex_reactivity_tables,
};
pub use collection::Collection;
pub use config::DbConfig;
pub use data_flow::{DataFlowMap, build_data_flow};
pub use ddl::{SchemaDiff, diff_schemas, table_to_ddl, tables_to_ddl};
pub use error_enrichment::{EnrichedDbError, enrich_error};
pub use eval_params::EvalRunParams;
pub use memory::MemoryParams;
pub use migration::{Migration, builtin_migrations, validate_migrations};
pub use project_store::open_project_db;
pub use research::{
    CapabilityMapRecord, ExternalResearchPacket, ResearchIngestRequest, ResearchIngestResult,
    RetrievalDiagnostics, retrieval_diagnostics,
};
pub use retrieval::{
    RetrievalEvidenceSource, RetrievalMode, RetrievalQuery, RetrievalResult, fuse_hybrid_results,
};
pub use schema_digest::{SchemaDigest, digest_to_json, format_llm_context, generate_schema_digest};
pub use socrates_telemetry::{
    SocratesSurfaceAggregate, SocratesSurfaceTelemetry, hallucination_risk_proxy,
};
pub use store::{
    A2AMessageRow, AgentDefEntry, AgentEventRow, ArtifactEntry, BehaviorEventEntry,
    BenchmarkEventRow, BuildHealthSummary, BuildRunRow, BuilderSessionEntry, CloudCostSummary,
    CloudDispatchRow, CodexChangeLogEntry, CommandFrequencyEntry, ComponentEntry, CrateSample,
    CrateSampleRow, EmbeddingEntry, EndpointReliabilityEntry, ExecutionEntry, KnowledgeNodeSummary,
    LearnedPatternEntry, LocalTrainRow, LogExecutionParams, LogInteractionParams, MemoryEntry,
    PackageSearchResult, PlanNodeRow, PlanSessionRow, PlanVersionRow, PublicationAttemptRow,
    PublicationManifestParams, PublicationManifestRow, PublicationMediaAssetParams,
    PublicationMediaAssetRow, PublicationStatusEventRow, PublishArtifactParams, QuestionRow,
    RegisterAgentParams, RegressionRow, ReviewEntry, SaveMemoryParams, SaveSnippetParams,
    ScheduledEntry, ScholarlySubmissionRow, SessionEventRow, SessionRow, SessionTurnEntry,
    SkillExecutionParams, SkillExecutionRow, SkillManifestEntry, SkillReliabilityReport,
    SnippetEntry, StoreError, ThroughputProfileRow, TrainingPair, TypedStreamEventEntry, UserEntry,
    WarningRow, WorkflowExecutionRow,
};
pub use sync_invocables::InvocableSyncEngine;
pub use toestub_store::{
    add_suppression, get_file_cache_blocking, list_suppressions_blocking, load_baseline,
    load_latest_task_queue, save_baseline, save_task_queue, set_file_cache_blocking,
};

/// Public product name for the unified database facade (**Codex** over Arca/Turso).
///
/// `VoxDb` remains the stable Rust type name; new documentation should prefer **Codex**.
pub type Codex = VoxDb;

/// High-level database facade for the Vox ecosystem (**Codex**).
///
/// Owns a single [`VoxDb`] (one Turso connection). Higher-level helpers (memory, learner,
/// schema sync) delegate to that store; advanced callers use [`Self::store`] for direct access.
///
/// **Concurrency:** one connection per `VoxDb` handle; not `Sync` across concurrent writers unless
/// the underlying Turso client serializes access (typical for one handle per task).
#[derive(Clone)]
pub struct VoxDb {
    pub(crate) conn: turso::Connection,
    pub(crate) sync_db: Option<turso::sync::Database>,
    pub(crate) breaker: std::sync::Arc<DbCircuitBreaker>,
}

mod facade;

#[cfg(test)]
mod codex_contract {
    use super::{Codex, VoxDb};

    #[test]
    fn codex_alias_same_layout_as_voxdb() {
        assert_eq!(std::mem::size_of::<Codex>(), std::mem::size_of::<VoxDb>());
        assert_eq!(std::mem::align_of::<Codex>(), std::mem::align_of::<VoxDb>());
    }
}

#[cfg(all(test, feature = "local"))]
mod tests {
    use super::*;
    use crate::codex_legacy::{
        LEGACY_EXPORT_SKIP_TABLES, LEGACY_EXPORT_TABLES, export_legacy_jsonl, import_legacy_jsonl,
        list_sqlite_user_tables, verify_legacy_store,
    };
    use crate::codex_schema::missing_codex_reactivity_tables;
    use crate::schema::{BASELINE_VERSION, CODEX_CHAT_TABLES};

    #[tokio::test]
    async fn cas_store_and_load_is_idempotent() {
        let db = VoxDb::connect(DbConfig::Memory).await.expect("db");
        let hash = db.store("test_kind", b"test_data").await.expect("store");
        let data = db.get(&hash).await.expect("get");
        assert_eq!(data, b"test_data");
    }

    #[tokio::test]
    async fn schema_init_v7_is_ok() {
        let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
        let v = db.schema_version().await.expect("version");
        assert_eq!(v, BASELINE_VERSION);
    }

    #[tokio::test]
    async fn append_codex_change_is_ok() {
        let db = VoxDb::connect(DbConfig::Memory).await.expect("db");
        let id = db
            .append_codex_change("test.topic", None, None, "upsert", None)
            .await
            .expect("append");
        assert!(id > 0);
    }

    #[tokio::test]
    async fn test_connect_memory() {
        let db = VoxDb::connect(DbConfig::Memory)
            .await
            .expect("Failed to connect to memory DB");
        let hash = db
            .store("test_kind", b"test_data")
            .await
            .expect("Store failed");
        assert!(!hash.is_empty());
    }

    #[tokio::test]
    async fn codex_reactivity_schema_and_legacy_verify() {
        let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
        let v = db.schema_version().await.expect("version");
        assert_eq!(v, BASELINE_VERSION);
        assert!(
            missing_codex_reactivity_tables(&db)
                .await
                .expect("cap")
                .is_empty()
        );
        let leg = verify_legacy_store(&db).await.expect("verify");
        assert!(leg.has_codex_reactivity);
        assert!(!leg.is_legacy_schema_chain);
        let id = db
            .append_codex_change("test.topic", None, None, "upsert", None)
            .await
            .expect("change log");
        assert!(id > 0);
    }

    #[tokio::test]
    async fn codex_alias_connects() {
        let db: Codex = VoxDb::connect(DbConfig::Memory).await.expect("db");
        assert_eq!(db.schema_version().await.expect("v"), BASELINE_VERSION);
    }

    #[tokio::test]
    async fn baseline_schema_includes_chat_and_search_tables() {
        let db = VoxDb::connect(DbConfig::Memory).await.expect("db");
        assert_eq!(
            db.schema_version().await.expect("schema_version"),
            BASELINE_VERSION
        );
        for t in CODEX_CHAT_TABLES {
            let rows = db
                .query_all(
                    "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
                    turso::params![t.to_string()],
                )
                .await
                .expect("sqlite_master");
            assert!(!rows.is_empty(), "missing table {t}");
        }
        for t in [
            "search_documents",
            "search_document_chunks",
            "search_indexing_jobs",
        ] {
            let rows = db
                .query_all(
                    "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
                    turso::params![t.to_string()],
                )
                .await
                .expect("search table");
            assert!(!rows.is_empty(), "missing search table {t}");
        }
        for t in ["processing_runs", "processing_run_steps", "audit_log"] {
            let rows = db
                .query_all(
                    "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
                    (t.to_string(),),
                )
                .await
                .expect("sqlite_master");
            assert!(!rows.is_empty(), "missing V16 table {t}");
        }
        for t in [
            "research_sessions",
            "conversation_versions",
            "conversation_edges",
            "topic_evolution_events",
        ] {
            let rows = db
                .query_all(
                    "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
                    (t.to_string(),),
                )
                .await
                .expect("sqlite_master");
            assert!(!rows.is_empty(), "missing V17 table {t}");
        }
    }

    #[tokio::test]
    async fn raw_sqlite_gamify_profiles_integer_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("raw.db");
        let db = VoxDb::connect(DbConfig::Local {
            path: p.to_string_lossy().into_owned(),
        })
        .await
        .expect("db");
        db.connection()
            .execute(
                "INSERT INTO gamify_profiles (user_id, level, xp) VALUES (?1, ?2, ?3)",
                turso::params!["u1", 3i64, 900i64],
            )
            .await
            .expect("insert");
        let mut q = db
            .connection()
            .query(
                "SELECT xp FROM gamify_profiles WHERE user_id = ?1",
                turso::params!["u1"],
            )
            .await
            .expect("sel");
        let row = q.next().await.expect("r").expect("row");
        assert_eq!(row.get::<i64>(0).expect("xp"), 900);
    }

    #[tokio::test]
    async fn legacy_export_covers_all_baseline_tables() {
        let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
        let mut live = list_sqlite_user_tables(db.connection())
            .await
            .expect("list tables");
        live.retain(|n| !LEGACY_EXPORT_SKIP_TABLES.contains(&n.as_str()));
        live.sort();
        let mut expected: Vec<&str> = LEGACY_EXPORT_TABLES.to_vec();
        expected.sort();
        assert_eq!(
            live,
            expected.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            "LEGACY_EXPORT_TABLES must match sqlite_master after migrate (minus skip list)"
        );
    }

    /// Gamification + coordination rows survive JSONL export/import on baseline DBs.
    #[tokio::test]
    async fn legacy_jsonl_roundtrips_gamification_and_coordination() {
        use std::io::Cursor;
        use tempfile::tempdir;

        let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
        db.connection()
            .execute(
                "INSERT INTO gamify_profiles (user_id, level, xp) VALUES ('u1', 3, 900)",
                (),
            )
            .await
            .expect("insert profile");
        db.connection()
            .execute(
                "INSERT INTO gamify_companions (id, user_id, name, language) VALUES ('c1', 'u1', 'Ada', 'vox')",
                (),
            )
            .await
            .expect("insert companion");
        db.connection()
            .execute(
                "INSERT INTO distributed_locks (lock_key, holder_node, holder_agent, fence_token, expires_at) VALUES ('lk', 'node-a', 'owner', 1, '2099-01-01')",
                (),
            )
            .await
            .expect("insert lock");

        let mut jsonl = Vec::<u8>::new();
        let n = export_legacy_jsonl(&db, &mut jsonl).await.expect("export");
        assert!(n >= 3, "expected ≥3 rows, got {n}");
        let profile_lines = String::from_utf8_lossy(&jsonl)
            .lines()
            .filter(|l| l.contains("\"table\":\"gamify_profiles\""))
            .count();
        assert_eq!(
            profile_lines, 1,
            "export must emit exactly one gamify_profiles row"
        );
        let prof_json: serde_json::Value = String::from_utf8_lossy(&jsonl)
            .lines()
            .find(|l| l.contains("\"table\":\"gamify_profiles\""))
            .and_then(|l| serde_json::from_str(l).ok())
            .expect("parse profile jsonl");
        assert_eq!(
            prof_json["row"]["xp"].as_i64(),
            Some(900),
            "exported JSON must preserve xp: {}",
            prof_json["row"]
        );

        let dir = tempdir().expect("tempdir");
        let fresh_path = dir.path().join("roundtrip.db");
        let fresh_str = fresh_path.to_string_lossy().to_string();
        let db2 = VoxDb::connect(DbConfig::Local {
            path: fresh_str.clone(),
        })
        .await
        .expect("fresh file db");
        let imported = import_legacy_jsonl(&db2, Cursor::new(&jsonl))
            .await
            .expect("import");
        assert!(imported >= 3);

        let mut q = db2
            .connection()
            .query(
                "SELECT xp, level FROM gamify_profiles WHERE user_id = ?1",
                turso::params!["u1"],
            )
            .await
            .expect("q");
        let row = q.next().await.expect("row").expect("has row");
        assert_eq!(row.get::<i64>(0).expect("xp"), 900);
        assert_eq!(row.get::<i64>(1).expect("level"), 3);

        let mut q2 = db2
            .connection()
            .query(
                "SELECT name FROM gamify_companions WHERE id = ?1",
                turso::params!["c1"],
            )
            .await
            .expect("q2");
        let row2 = q2.next().await.expect("row").expect("r2");
        assert_eq!(row2.get::<String>(0).expect("name"), "Ada");

        let mut q3 = db2
            .connection()
            .query(
                "SELECT holder_agent FROM distributed_locks WHERE lock_key = ?1",
                turso::params!["lk"],
            )
            .await
            .expect("q3");
        let row3 = q3.next().await.expect("row").expect("r3");
        assert_eq!(row3.get::<String>(0).expect("holder"), "owner");
    }

    /// Simulates `vox codex export-legacy` → new file → `vox codex import-legacy` without the CLI.
    #[tokio::test]
    async fn legacy_chain_db_export_then_import_into_baseline_roundtrips_objects() {
        use crate::StoreError;
        use crate::schema::BASELINE_VERSION;
        use std::io::Cursor;
        use tempfile::tempdir;
        use turso::Builder;

        let dir = tempdir().expect("tempdir");
        let legacy_path = dir.path().join("legacy.db");
        let legacy_str = legacy_path.to_string_lossy().to_string();
        let fresh_path = dir.path().join("fresh.db");
        let fresh_str = fresh_path.to_string_lossy().to_string();

        let built = Builder::new_local(&legacy_str)
            .build()
            .await
            .expect("legacy build");
        let conn = built.connect().expect("legacy conn");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )
        .await
        .expect("schema_version ddl");
        conn.execute("INSERT INTO schema_version (version) VALUES (17)", ())
            .await
            .expect("insert v17");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS objects (
                hash TEXT PRIMARY KEY,
                kind TEXT NOT NULL,
                data BLOB NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )
        .await
        .expect("objects ddl");
        conn.execute(
            "INSERT INTO objects (hash, kind, data) VALUES ('legacy_row_h', 'legacy_kind', X'01020304')",
            (),
        )
        .await
        .expect("insert object");
        drop(conn);

        let err = match VoxDb::connect(DbConfig::local(&legacy_str)).await {
            Ok(_) => panic!("normal open must reject legacy chain"),
            Err(e) => e,
        };
        assert!(
            matches!(err, StoreError::LegacySchemaChain { max_version: 17 }),
            "expected LegacySchemaChain {{ max_version: 17 }}, got {err:?}"
        );

        let export_db = VoxDb::connect_legacy_export_only(DbConfig::local(&legacy_str))
            .await
            .expect("legacy export open");
        let mut jsonl = Vec::<u8>::new();
        let n = export_legacy_jsonl(&export_db, &mut jsonl)
            .await
            .expect("export");
        assert!(n >= 1, "expected at least one exported row");

        let fresh = VoxDb::connect(DbConfig::local(&fresh_str))
            .await
            .expect("fresh baseline");
        assert_eq!(fresh.schema_version().await.expect("sv"), BASELINE_VERSION);
        let imported = import_legacy_jsonl(&fresh, Cursor::new(&jsonl))
            .await
            .expect("import");
        assert!(imported >= 1);

        let mut q = fresh
            .conn
            .query(
                "SELECT kind, hex(data) FROM objects WHERE hash = ?1",
                turso::params!["legacy_row_h"],
            )
            .await
            .expect("select");
        let row = q.next().await.expect("row").expect("has row");
        let kind: String = row.get(0).expect("kind");
        let hex_data: String = row.get(1).expect("hex");
        assert_eq!(kind, "legacy_kind");
        assert_eq!(hex_data.to_uppercase(), "01020304");

        let leg = verify_legacy_store(&fresh).await.expect("verify");
        assert_eq!(leg.schema_version, BASELINE_VERSION);
        assert!(!leg.is_legacy_schema_chain);
    }
}
