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
//! unless handled with `pragma_update`). The `vox-pm` open path applies pragmas via
//! `pragma_update` and skips empty migration bodies; see also [`VoxDb::apply_migrations`].
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
use crate::paths::local_user_id;

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
    PackageSearchResult, PlanNodeRow, PlanSessionRow, PlanVersionRow, PublicationManifestParams,
    PublicationManifestRow, PublishArtifactParams, QuestionRow, RegisterAgentParams, RegressionRow,
    ReviewEntry, SaveMemoryParams, SaveSnippetParams, ScheduledEntry, ScholarlySubmissionRow,
    SessionEventRow, SessionRow, SessionTurnEntry, SkillExecutionParams, SkillExecutionRow,
    SkillManifestEntry, SkillReliabilityReport, SnippetEntry, StoreError, ThroughputProfileRow,
    TrainingPair, TypedStreamEventEntry, UserEntry, WarningRow, WorkflowExecutionRow,
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

/// Default maximum number of connection retry attempts.
const DEFAULT_MAX_RETRIES: u64 = 3;
/// Default base delay between retries in milliseconds.
const DEFAULT_RETRY_BASE_MS: u64 = 500;

impl VoxDb {
    /// Wrap an already-open [`VoxDb`] (e.g. after custom Turso setup).
    pub fn from_store(conn: turso::Connection, sync_db: Option<turso::sync::Database>) -> Self {
        Self {
            conn,
            sync_db,
            breaker: std::sync::Arc::new(DbCircuitBreaker::from_env()),
        }
    }

    /// Connect to a database using the given configuration, with retry logic.
    pub async fn connect(config: DbConfig) -> Result<Self, StoreError> {
        Self::connect_with_retries(config, DEFAULT_MAX_RETRIES, DEFAULT_RETRY_BASE_MS).await
    }

    /// Connect using the platform-aware default local path.
    ///
    /// Uses `paths::default_db_path()` to determine the DB file location.
    /// Falls back to `DbConfig::from_env()` if the platform path cannot be
    /// determined.
    #[cfg(feature = "local")]
    pub async fn connect_default() -> Result<Self, StoreError> {
        let config = if let Some(path) = paths::default_db_path() {
            DbConfig::Local {
                path: path.to_string_lossy().to_string(),
            }
        } else {
            DbConfig::from_env().map_err(|e| StoreError::NotFound(e))?
        };
        Self::connect(config).await
    }

    /// Like [`Self::connect_default`], but if the primary DB reports [`StoreError::LegacySchemaChain`],
    /// opens (or creates) [`paths::training_telemetry_db_path`] so training tools can persist runs
    /// without migrating the main Codex database first.
    #[cfg(feature = "local")]
    pub async fn connect_default_with_training_fallback() -> Result<Self, StoreError> {
        match Self::connect_default().await {
            Ok(db) => Ok(db),
            Err(StoreError::LegacySchemaChain { max_version }) => {
                let Some(sidecar) = paths::training_telemetry_db_path() else {
                    return Err(StoreError::LegacySchemaChain { max_version });
                };
                tracing::info!(
                    sidecar = %sidecar.display(),
                    primary_schema_max = max_version,
                    "Primary VoxDB uses a legacy schema; using training telemetry sidecar. \
                     Migrate the main DB with `vox codex export-legacy`, fresh init, and `vox codex import-legacy` when ready."
                );
                Self::connect(DbConfig::Local {
                    path: sidecar.to_string_lossy().into_owned(),
                })
                .await
            }
            Err(e) => Err(e),
        }
    }

    /// Blocking [`Self::connect_default`] for `std::thread` workers without a Tokio handle.
    #[cfg(feature = "local")]
    pub fn connect_default_sync() -> Result<Self, StoreError> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| StoreError::Db(format!("tokio runtime: {e}")))?
            .block_on(Self::connect_default())
    }

    /// Optional hook before dropping a database handle (flush/sync); currently a no-op.
    pub fn shutdown_for_drop(&self) {}

    /// Connect with configurable retry parameters.
    ///
    /// Logs retries to stderr with the product name **Codex** (alias for this type).
    pub async fn connect_with_retries(
        config: DbConfig,
        max_retries: u64,
        retry_base_ms: u64,
    ) -> Result<Self, StoreError> {
        let mut attempts = 0u64;
        loop {
            attempts += 1;
            let result = match &config {
                #[cfg(feature = "local")]
                DbConfig::Local { path } => {
                    let db = turso::Builder::new_local(path)
                        .build()
                        .await
                        .map_err(StoreError::from)?;
                    let conn = db.connect().map_err(StoreError::from)?;
                    Ok((conn, None))
                }
                #[cfg(feature = "local")]
                DbConfig::Memory => {
                    let db = turso::Builder::new_local(":memory:")
                        .build()
                        .await
                        .map_err(StoreError::from)?;
                    let conn = db.connect().map_err(StoreError::from)?;
                    Ok((conn, None))
                }
                DbConfig::Remote { url, token } => {
                    let db = turso::sync::Builder::new_remote(":memory:")
                        .with_remote_url(url)
                        .with_auth_token(token)
                        .build()
                        .await
                        .map_err(StoreError::from)?;
                    let conn = db.connect().await.map_err(StoreError::from)?;
                    Ok((conn, Some(db)))
                }
                #[cfg(feature = "replication")]
                DbConfig::EmbeddedReplica {
                    local_path,
                    url,
                    token,
                } => {
                    let db = turso::sync::Builder::new_remote(local_path)
                        .with_remote_url(url)
                        .with_auth_token(token)
                        .build()
                        .await
                        .map_err(StoreError::from)?;
                    let conn = db.connect().await.map_err(StoreError::from)?;
                    Ok((conn, Some(db)))
                }
            };

            match result {
                Ok((conn, sync_db)) => {
                    Self::apply_pragmas(&conn).await?;
                    Self::migrate(&conn).await?;
                    return Ok(Self {
                        conn,
                        sync_db,
                        breaker: std::sync::Arc::new(DbCircuitBreaker::from_env()),
                    });
                }
                Err(e) if attempts < max_retries => {
                    eprintln!(
                        "Failed to connect to Codex, retrying ({}/{})... Error: {}",
                        attempts, max_retries, e
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(retry_base_ms * attempts))
                        .await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Open the configured store **without** running the Arca baseline migration.
    ///
    /// Used by `vox codex export-legacy` so databases that still have the historical multi-version
    /// `schema_version` chain can be read. Normal apps should use [`Self::connect`].
    pub async fn connect_legacy_export_only(config: DbConfig) -> Result<Self, StoreError> {
        let conn = match config {
            DbConfig::Remote { url, token } => {
                turso::sync::Builder::new_remote(":memory:")
                    .with_remote_url(url)
                    .with_auth_token(token)
                    .build()
                    .await?
                    .connect()
                    .await?
            }
            #[cfg(feature = "local")]
            DbConfig::Local { path } => {
                let db = turso::Builder::new_local(&path).build().await?;
                db.connect()?
            }
            #[cfg(feature = "local")]
            DbConfig::Memory => {
                return Err(StoreError::NotFound(
                    "legacy export requires VOX_DB_PATH or remote URL (not memory)".into(),
                ));
            }
            #[cfg(feature = "replication")]
            DbConfig::EmbeddedReplica { url, token, .. } => {
                turso::Connection::open_remote(url, token).await?
            }
        };
        Ok(Self {
            conn,
            sync_db: None,
            breaker: std::sync::Arc::new(DbCircuitBreaker::from_env()),
        })
    }

    /// Access the circuit breaker for this database.
    pub fn breaker(&self) -> &DbCircuitBreaker {
        &self.breaker
    }

    /// Apply a [`SchemaDigest`]-driven plan: create missing tables/columns/indexes, never drop.
    pub async fn sync_schema_from_digest(&self, digest: &SchemaDigest) -> Result<(), StoreError> {
        let migrator = AutoMigrator::new(&self.conn);
        migrator.sync_from_digest(digest).await?;
        Ok(())
    }

    /// Return the platform-specific data directory (if resolvable).
    pub fn data_dir() -> Option<std::path::PathBuf> {
        paths::data_dir()
    }

    // ── Collection & Schema Methods ─────────────────────

    /// Get a handle to a schemaless document collection.
    ///
    /// The collection stores JSON documents in a SQLite table with `json_extract`
    /// based querying. Call `ensure_table()` on the returned handle to create the
    /// backing table if it doesn't exist.
    pub fn collection(&self, name: impl Into<String>) -> collection::Collection<'_> {
        collection::Collection::new(name, &self.conn)
    }

    /// Create an auto-migrator for schema synchronization.
    ///
    /// Use this to introspect the live database schema and diff it against your
    /// desired `@table` declarations, then apply non-destructive migrations.
    pub fn auto_migrator(&self) -> auto_migrate::AutoMigrator<'_> {
        auto_migrate::AutoMigrator::new(&self.conn)
    }

    /// Automatically sync the database schema derived from AST declarations.
    pub async fn sync_schema_ast(
        &self,
        tables: &[&vox_compiler::ast::decl::TableDecl],
        collections: &[&vox_compiler::ast::decl::CollectionDecl],
        indexes: &[&vox_compiler::ast::decl::IndexDecl],
    ) -> Result<auto_migrate::MigrationPlan, StoreError> {
        let plan = self
            .auto_migrator()
            .sync_schema(tables, collections, indexes)
            .await?;
        Ok(plan)
    }

    // ── Memory Convenience Methods ──────────────────────

    /// Persist an agent memory row (`memories` table). See [`MemoryParams`] for fields.
    pub async fn store_memory(&self, params: MemoryParams<'_>) -> Result<i64, StoreError> {
        self.save_memory(params).await
    }

    /// Full-text-ish search over knowledge nodes (delegates to `VoxDb::query_knowledge_nodes`).
    ///
    /// Returns `(id, title, snippet)` tuples as produced by the store layer.
    pub async fn search_memories(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<(String, String, String)>, StoreError> {
        self.query_knowledge_nodes(query, limit).await
    }

    /// Vector similarity search in `embeddings` (optional `source_type` filter).
    pub async fn search_embeddings(
        &self,
        vector: &[f32],
        source_type: Option<&str>,
        limit: i64,
    ) -> Result<Vec<(EmbeddingEntry, f32)>, StoreError> {
        self.search_similar_embeddings(vector, source_type, limit)
            .await
    }

    /// Return a behavioral learner for this database.
    pub fn learner(&self) -> learning::BehavioralLearner<'_> {
        learning::BehavioralLearner::new(self)
    }

    /// Run a parameterized `SELECT` and collect all rows (for small result sets).
    pub async fn query_all(
        &self,
        sql: &str,
        params: impl turso::IntoParams + Send,
    ) -> Result<Vec<turso::Row>, StoreError> {
        let mut cursor = self.conn.query(sql, params).await?;
        let mut rows = Vec::new();
        while let Some(row) = cursor.next().await? {
            rows.push(row);
        }
        Ok(rows)
    }

    /// Apply ordered migrations that have not yet been executed (same `schema_version` table as Arca).
    ///
    /// Returns versions that were newly applied.
    ///
    /// # SQL constraints
    ///
    /// Each [`Migration::up_sql`] is run with [`turso::Connection::execute_batch`]. It must **not**
    /// contain row-returning statements (no standalone `SELECT`; use DDL/DML only). See crate-level
    /// docs.
    pub async fn apply_migrations(&self, migrations: &[Migration]) -> Result<Vec<i64>, StoreError> {
        validate_migrations(migrations)?;
        self.connection()
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS schema_version (
                    version INTEGER PRIMARY KEY,
                    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
                );",
            )
            .await?;

        let current = self.schema_version().await?;
        let mut applied = Vec::new();
        for migration in migrations {
            if migration.version <= current {
                continue;
            }
            self.connection().execute_batch(&migration.up_sql).await?;
            self.connection()
                .execute(
                    "INSERT INTO schema_version (version) VALUES (?1)",
                    (migration.version,),
                )
                .await?;
            applied.push(migration.version);
        }
        Ok(applied)
    }

    /// Append a training telemetry event to `agent_events` for orchestrator visibility.
    ///
    /// `event_kind` matches telemetry_schema constants (e.g. `"train_start"`, `"train_step"`, `"train_complete"`).
    /// `payload` is a JSON string of the event body.
    pub async fn record_training_event(
        &self,
        run_id: &str,
        event_kind: &str,
        payload: serde_json::Value,
    ) -> Result<(), store::StoreError> {
        let store = self;
        store
            .record_agent_event(
                &format!("populi_train:{run_id}"),
                event_kind,
                &payload.to_string(),
                env!("CARGO_PKG_VERSION"),
            )
            .await?;
        Ok(())
    }

    /// Record a checkpoint write event (adapter path, step, epoch) in `agent_events`.
    pub async fn record_training_checkpoint(
        &self,
        run_id: &str,
        epoch: u32,
        global_step: u32,
        adapter_path: &str,
    ) -> Result<(), store::StoreError> {
        self.record_training_event(
            run_id,
            "checkpoint_saved",
            serde_json::json!({
                "run_id": run_id,
                "epoch": epoch,
                "global_step": global_step,
                "adapter_path": adapter_path,
            }),
        )
        .await
    }

    /// Run `f` between `BEGIN` and `COMMIT` on this connection; `ROLLBACK` on error.
    ///
    /// **Caveat:** `f` is `.await`ed without holding a guard; avoid spawning work that uses the
    /// same `VoxDb` concurrently inside `f`. Prefer short, sequential async blocks.
    pub async fn transaction<F, T>(&self, f: F) -> Result<T, StoreError>
    where
        F: std::future::Future<Output = Result<T, StoreError>>,
    {
        let _ = self.conn.execute("BEGIN", ()).await?;
        match f.await {
            Ok(val) => {
                let _ = self.conn.execute("COMMIT", ()).await?;
                Ok(val)
            }
            Err(e) => {
                let _ = self.conn.execute("ROLLBACK", ()).await.ok();
                Err(e)
            }
        }
    }

    /// Register the current machine directory as a known Vox project (`components` + path key).
    ///
    /// The `user_preferences` path write is **best-effort**: failures are ignored so component
    /// registration still succeeds (check logs if paths do not persist).
    pub async fn register_local_project(
        &self,
        name: &str,
        path: &std::path::Path,
    ) -> Result<(), StoreError> {
        let abs_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let path_str = abs_path.to_string_lossy();

        self.register_component(
            name,
            "local", // namespace for local projects
            None,    // schema_hash not needed for projects
            Some(&format!("Local project at {}", path_str)),
            "1.0.0",
        )
        .await?;

        // Also store the path in user_preferences as a 'known_project'
        let _ = self
            .conn
            .execute(
                "INSERT OR REPLACE INTO user_preferences (user_id, key, value) VALUES (?1, ?2, ?3)",
                (
                    local_user_id(),
                    format!("project.{}.path", name),
                    path_str.to_string(),
                ),
            )
            .await;

        Ok(())
    }

    /// Return true if the given activity was completed in the specified workflow run.
    pub async fn is_workflow_activity_completed(
        &self,
        run_id: &str,
        workflow_name: &str,
        activity_id: &str,
    ) -> Result<bool, StoreError> {
        let row = self.query_all(
            "SELECT 1 FROM workflow_activity_log WHERE run_id = ?1 AND workflow_name = ?2 AND activity_id = ?3 AND status = 'completed'",
            (run_id.to_string(), workflow_name.to_string(), activity_id.to_string())
        ).await?;
        Ok(!row.is_empty())
    }

    /// Record that an activity has started in the durable journal.
    pub async fn record_workflow_activity_started(
        &self,
        run_id: &str,
        workflow_name: &str,
        activity_name: &str,
        activity_id: &str,
    ) -> Result<(), StoreError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        self.conn.execute(
            "INSERT OR IGNORE INTO workflow_activity_log (run_id, workflow_name, activity_name, activity_id, status, recorded_at_ms) VALUES (?1, ?2, ?3, ?4, 'started', ?5)",
            (run_id.to_string(), workflow_name.to_string(), activity_name.to_string(), activity_id.to_string(), now)
        ).await?;
        Ok(())
    }

    /// Record that an activity has successfully completed in the durable journal.
    pub async fn record_workflow_activity_completed(
        &self,
        run_id: &str,
        workflow_name: &str,
        activity_name: &str,
        activity_id: &str,
    ) -> Result<(), StoreError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        self.conn.execute(
            "INSERT OR REPLACE INTO workflow_activity_log (run_id, workflow_name, activity_name, activity_id, status, recorded_at_ms) VALUES (?1, ?2, ?3, ?4, 'completed', ?5)",
            (run_id.to_string(), workflow_name.to_string(), activity_name.to_string(), activity_id.to_string(), now)
        ).await?;
        Ok(())
    }
}

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
    use crate::codex_legacy::verify_legacy_store;
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

    /// Simulates `vox codex export-legacy` → new file → `vox codex import-legacy` without the CLI.
    #[tokio::test]
    async fn legacy_chain_db_export_then_import_into_baseline_roundtrips_objects() {
        use crate::StoreError;
        use crate::codex_legacy::{export_legacy_jsonl, import_legacy_jsonl};
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
