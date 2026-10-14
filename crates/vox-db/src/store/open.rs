//! Connection helpers and built-in schema migration.
//!
//! # Turso `execute_batch` constraint
//!
//! [`turso::Connection::execute_batch`] executes each statement with `execute`, which errors if a
//! statement returns rows. Assignment `PRAGMA` values are applied with `pragma_update` in
//! [`VoxDb::apply_pragmas`]. Migration bodies must be DDL/DML only; empty strings in a fragment
//! are skipped.

use crate::schema::{BASELINE_VERSION, baseline_sql};
use crate::store::types::StoreError;

impl crate::VoxDb {
    pub(crate) async fn apply_pragmas(conn: &turso::Connection) -> Result<(), StoreError> {
        // `execute_batch` runs each statement via `execute()`, which errors on any row-producing
        // statement. Assignment-style `PRAGMA x = y` returns the new value as a row; use
        // `pragma_update` so those rows are consumed (see turso `Connection::prepare_execute_batch`).
        let _ = conn.pragma_update("journal_mode", "WAL").await?;
        let _ = conn.pragma_update("busy_timeout", 5000).await?;
        // Do not set `wal_autocheckpoint` via `pragma_update`: libsql rejects assignment forms for
        // this pragma (parse → unsupported). SQLite default is 1000 pages, which matches intent.
        let _ = conn.pragma_update("synchronous", "NORMAL").await?;
        let _ = conn.pragma_update("foreign_keys", "ON").await?;
        let _ = conn.pragma_update("cache_size", -65536).await?;
        // `temp_store` / `mmap_size` are valid in stock SQLite but not in Turso/libSQL's supported
        // `PragmaName` set (`turso_parser`); `pragma_update` then fails with "Not a valid pragma name".

        if std::env::var("VOX_DB_MVCC")
            .map(|s| s == "1")
            .unwrap_or(false)
        {
            tracing::warn!("BETA: MVCC concurrent writes enabled for Turso.");
            let _ = conn.pragma_update("journal_mode", "mvcc").await?;
        }

        let mut rows = conn.query("PRAGMA journal_mode", ()).await?;
        if let Some(row) = rows.next().await? {
            let mode: String = row.get(0).unwrap_or_default();
            if mode.to_lowercase() != "wal"
                && mode.to_lowercase() != "mvcc"
                && mode.to_lowercase() != "memory"
            {
                tracing::warn!(mode = %mode, "Database connection did not apply WAL/MVCC journal_mode (likely remote Turso)");
            }
        }

        Ok(())
    }

    /// Drop every user-defined table (not `sqlite_%`). Caller usually follows with `Self::migrate`.
    #[cfg(feature = "local")]
    pub async fn drop_all_user_tables(conn: &turso::Connection) -> Result<(), StoreError> {
        conn.execute("PRAGMA foreign_keys = OFF", ())
            .await
            .map_err(StoreError::from)?;
        let mut rows = conn
            .query(
                "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
                (),
            )
            .await?;
        let mut names = Vec::new();
        while let Some(row) = rows.next().await? {
            names.push(row.get::<String>(0)?);
        }
        for name in names {
            let quoted = sqlite_quote_ident(&name);
            conn.execute(&format!("DROP TABLE IF EXISTS {quoted}"), ())
                .await
                .map_err(StoreError::from)?;
        }
        conn.execute("PRAGMA foreign_keys = ON", ())
            .await
            .map_err(StoreError::from)?;
        Ok(())
    }

    /// Open a local file, wipe all user tables, and apply baseline + cutover (same as fresh migrate).
    #[cfg(feature = "local")]
    pub async fn open_local_reset_to_baseline(path: &str) -> Result<Self, StoreError> {
        let db = turso::Builder::new_local(path).build().await?;
        let conn = db.connect()?;
        Self::apply_pragmas(&conn).await?;
        Self::drop_all_user_tables(&conn).await?;
        Self::migrate(&conn).await?;
        Ok(Self::assembled(conn, None, Some(std::sync::Arc::new(db))))
    }

    /// Apply the manifest **baseline** DDL and advance `schema_version` to [`BASELINE_VERSION`].
    ///
    /// DDL in each fragment is run through Turso `execute_batch`. Existing databases with
    /// version ≤ [`BASELINE_VERSION`] are automatically advanced (DDL is idempotent).
    pub(crate) async fn migrate(conn: &turso::Connection) -> Result<(), StoreError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )
        .await?;

        let mut rows = conn
            .query("SELECT COALESCE(MAX(version), 0) FROM schema_version", ())
            .await?;
        let current_version: i64 = if let Some(row) = rows.next().await? {
            row.get(0)?
        } else {
            0
        };
        drop(rows);

        // Single maintained baseline: only `BASELINE_VERSION` (or 0 pre-bootstrap) is valid.
        // `MAX(schema_version)` from ad-hoc `apply_migrations` rows also trips this guard.
        if current_version > BASELINE_VERSION {
            tracing::warn!(
                target: "vox_db::legacy_schema",
                schema_max = current_version,
                baseline = BASELINE_VERSION,
                "database schema is newer than this binary; upgrade vox for compatibility."
            );
            return Err(StoreError::LegacySchemaChain {
                max_version: current_version,
            });
        }

        if current_version < BASELINE_VERSION {
            let sql = baseline_sql().trim();
            if !sql.is_empty() {
                conn.execute_batch(sql).await?;
            }

            // Note: In v51, the legacy reliability tables (agent_reliability, skill_reliability,
            // workflow_reliability, repository_reliability) were retired and consolidated into
            // the `reliability_scores` table in the baseline schema. Since we no longer support
            // upgrading pre-v77 database files, the dynamic migration path here is retired.

            // `baseline_sql()`'s CREATE TABLE IF NOT EXISTS is a no-op against a table that
            // already exists, so a column added to the DDL string above only takes effect on a
            // brand-new database. Existing databases need an explicit, idempotent ALTER TABLE —
            // checked via PRAGMA table_info first, since blindly running ALTER TABLE ADD COLUMN
            // would fail with "duplicate column name" on a fresh database (where CREATE TABLE
            // just created the column already).
            let has_archived_at = {
                let mut cols = conn.query("PRAGMA table_info(conversations)", ()).await?;
                let mut found = false;
                while let Some(row) = cols.next().await? {
                    let name: String = row.get(1)?;
                    if name == "archived_at" {
                        found = true;
                        break;
                    }
                }
                found
            };
            if !has_archived_at {
                conn.execute_batch("ALTER TABLE conversations ADD COLUMN archived_at TEXT;")
                    .await?;
            }
            conn.execute_batch(
                "CREATE INDEX IF NOT EXISTS idx_conversations_archived ON conversations(archived_at);",
            )
            .await?;

            let has_misguidance_status = {
                let mut cols = conn
                    .query("PRAGMA table_info(research_misguidance_events)", ())
                    .await?;
                let mut found = false;
                while let Some(row) = cols.next().await? {
                    let name: String = row.get(1)?;
                    if name == "status" {
                        found = true;
                        break;
                    }
                }
                found
            };
            if !has_misguidance_status {
                let _ = conn
                    .execute_batch(
                        "ALTER TABLE research_misguidance_events ADD COLUMN status TEXT NOT NULL DEFAULT 'open'; \
                         ALTER TABLE research_misguidance_events ADD COLUMN resolved_at_ms INTEGER;",
                    )
                    .await;
            }

            // Schema baseline 94 (Phase 5 D-08): hopper_inbox.resource_id for resource-locked hopper tasks.
            let has_hopper_resource_id = {
                let mut cols = conn.query("PRAGMA table_info(hopper_inbox)", ()).await?;
                let mut found = false;
                while let Some(row) = cols.next().await? {
                    let name: String = row.get(1)?;
                    if name == "resource_id" {
                        found = true;
                        break;
                    }
                }
                found
            };
            if !has_hopper_resource_id {
                conn.execute_batch("ALTER TABLE hopper_inbox ADD COLUMN resource_id TEXT;")
                    .await?;
            }

            // Schema baseline 95 (KB Phase 2): library-index columns. Same PRAGMA-guarded shape as
            // above; the indexes are created AFTER the ALTERs because a v94 database lacks these
            // columns when `baseline_sql()` runs.
            Self::ensure_kb_library_columns(conn).await?;
            Self::backfill_search_document_trust_tiers(conn).await?;
            Self::retier_agent_knowledge_nodes(conn).await?;
            conn.execute_batch(
                "CREATE INDEX IF NOT EXISTS idx_search_documents_corpus ON search_documents(corpus);
                 CREATE UNIQUE INDEX IF NOT EXISTS idx_search_chunks_doc_hash ON search_document_chunks(document_id, chunk_hash);
                 CREATE INDEX IF NOT EXISTS idx_search_chunks_hash ON search_document_chunks(chunk_hash);",
            )
            .await?;

            // Same idempotent-ALTER pattern as `archived_at` above: `workflow_run_log`
            // predates `args_json` (G27, added in Task 1.4 of true-workflow-durability),
            // so existing databases need it added explicitly.
            let has_args_json = {
                let mut cols = conn
                    .query("PRAGMA table_info(workflow_run_log)", ())
                    .await?;
                let mut found = false;
                while let Some(row) = cols.next().await? {
                    let name: String = row.get(1)?;
                    if name == "args_json" {
                        found = true;
                        break;
                    }
                }
                found
            };
            if !has_args_json {
                conn.execute_batch("ALTER TABLE workflow_run_log ADD COLUMN args_json TEXT;")
                    .await?;
            }

            let mut has_wake_at_ms = false;
            let mut has_parked_reason = false;
            let mut cols = conn
                .query("PRAGMA table_info(workflow_run_log)", ())
                .await?;
            while let Some(row) = cols.next().await? {
                let name: String = row.get(1)?;
                has_wake_at_ms |= name == "wake_at_ms";
                has_parked_reason |= name == "parked_reason";
            }
            drop(cols);
            if !has_wake_at_ms {
                conn.execute_batch("ALTER TABLE workflow_run_log ADD COLUMN wake_at_ms INTEGER;")
                    .await?;
            }
            if !has_parked_reason {
                conn.execute_batch("ALTER TABLE workflow_run_log ADD COLUMN parked_reason TEXT;")
                    .await?;
            }

            crate::schema_extensions::apply_schema_extensions(conn).await?;

            conn.execute(
                "INSERT INTO schema_version (version) VALUES (?1)
                 ON CONFLICT(version) DO UPDATE SET applied_at = datetime('now')",
                (BASELINE_VERSION,),
            )
            .await?;
        }

        Self::ensure_latency_columns(conn).await?;

        Ok(())
    }

    /// Baseline 95: add the KB Library columns to a pre-95 `search_documents` /
    /// `search_document_chunks`. Idempotent (PRAGMA-guarded); a fresh database already has them.
    async fn ensure_kb_library_columns(conn: &turso::Connection) -> Result<(), StoreError> {
        const COLUMNS: [(&str, &str, &str); 11] = [
            ("search_documents", "corpus", "TEXT NOT NULL DEFAULT ''"),
            ("search_documents", "rel_path", "TEXT"),
            ("search_documents", "frontmatter_json", "TEXT"),
            ("search_documents", "indexed_commit", "TEXT"),
            ("search_documents", "index_error", "TEXT"),
            // Baseline 96 (KB Phase 3): stored, fail-closed tier on every corpus row.
            (
                "search_documents",
                "trust_tier",
                "TEXT NOT NULL DEFAULT 'untrusted'",
            ),
            ("search_document_chunks", "chunk_hash", "TEXT"),
            (
                "search_document_chunks",
                "chunk_kind",
                "TEXT NOT NULL DEFAULT 'section'",
            ),
            (
                "search_document_chunks",
                "heading_path",
                "TEXT NOT NULL DEFAULT ''",
            ),
            ("search_document_chunks", "anchor", "TEXT"),
            ("search_document_chunks", "token_estimate", "INTEGER"),
        ];
        for (table, column, decl) in COLUMNS {
            let mut rows = conn
                .query(&format!("PRAGMA table_info({table})"), ())
                .await?;
            let mut has_column = false;
            while let Some(row) = rows.next().await? {
                if row.get::<String>(1)? == column {
                    has_column = true;
                }
            }
            drop(rows);
            if !has_column {
                conn.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {column} {decl};"))
                    .await?;
            }
        }
        Ok(())
    }

    /// Baseline 96 (D-01, D-03): one-time re-tiering of existing `search_documents` rows, run
    /// inside the version gate. A corpus row takes its tier from `kb_index_state.trust_tier`
    /// (the single corpus-level source; an unrecognised string canonicalizes to `untrusted`),
    /// `vox://publication/*` rows become `advisory`, and everything else keeps the fail-closed
    /// column default. Rows are re-tiered, never deleted.
    async fn backfill_search_document_trust_tiers(
        conn: &turso::Connection,
    ) -> Result<(), StoreError> {
        let mut rows = conn
            .query("SELECT corpus, trust_tier FROM kb_index_state", ())
            .await?;
        let mut corpora = Vec::new();
        while let Some(row) = rows.next().await? {
            corpora.push((row.get::<String>(0)?, row.get::<String>(1)?));
        }
        drop(rows);
        for (corpus, tier) in corpora {
            // The empty corpus holds web/legacy rows, which must stay fail-closed.
            if corpus.is_empty() {
                continue;
            }
            let tier = crate::ContextTrustTier::from_db_str(&tier).as_db_str();
            conn.execute(
                "UPDATE search_documents SET trust_tier = ?1 WHERE corpus = ?2",
                (tier, corpus.as_str()),
            )
            .await?;
        }
        conn.execute(
            "UPDATE search_documents SET trust_tier = 'advisory' \
             WHERE source_uri LIKE 'vox://publication/%'",
            (),
        )
        .await?;
        Ok(())
    }

    /// Baseline 96 (D-03, D-07): one-time re-type of legacy agent-written knowledge nodes, run
    /// inside the version gate. Before baseline 96, `MemoryManager::persist_fact` was the only
    /// producer of the `fact` and `concept` node types (graphify ingest ids are prefixed
    /// `graphify:` and are left alone), so every such row is model-originated and becomes
    /// `agent_fact` / `agent_concept` (Untrusted). Rows are re-typed, never deleted.
    async fn retier_agent_knowledge_nodes(conn: &turso::Connection) -> Result<(), StoreError> {
        use vox_orchestrator_types::trust::{AGENT_CONCEPT_NODE_TYPE, AGENT_FACT_NODE_TYPE};
        for (from, to) in [
            ("fact", AGENT_FACT_NODE_TYPE),
            ("concept", AGENT_CONCEPT_NODE_TYPE),
        ] {
            conn.execute(
                "UPDATE knowledge_nodes SET node_type = ?1 \
                 WHERE node_type = ?2 AND id NOT LIKE 'graphify:%'",
                (to, from),
            )
            .await?;
        }
        Ok(())
    }

    /// Task M3 added `ttft_ms`/`tpot_ms` (and the scoreboard's p95/goodput columns) to the
    /// version-94 baseline DDL without bumping the version, so a database already recorded at
    /// [`BASELINE_VERSION`] never received them: the telemetry `INSERT` into `llm_interactions`
    /// failed and the scoreboard rollup died on `no such column: ttft_ms`. This runs on every
    /// open, outside the version gate; it is a cheap `PRAGMA` per table and a no-op on a fresh
    /// database or one that already has the columns. A table that does not exist is skipped.
    async fn ensure_latency_columns(conn: &turso::Connection) -> Result<(), StoreError> {
        const COLUMNS: [(&str, &str, &str); 5] = [
            ("llm_interactions", "ttft_ms", "INTEGER"),
            ("llm_interactions", "tpot_ms", "REAL"),
            ("model_scoreboard", "p95_ttft_ms", "INTEGER"),
            ("model_scoreboard", "p95_tpot_ms", "REAL"),
            ("model_scoreboard", "goodput_tokens_per_sec", "REAL"),
        ];
        for (table, column, decl) in COLUMNS {
            let mut rows = conn
                .query(&format!("PRAGMA table_info({table})"), ())
                .await?;
            let (mut table_exists, mut has_column) = (false, false);
            while let Some(row) = rows.next().await? {
                table_exists = true;
                if row.get::<String>(1)? == column {
                    has_column = true;
                }
            }
            drop(rows);
            if table_exists && !has_column {
                conn.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {column} {decl};"))
                    .await?;
            }
        }
        Ok(())
    }

    /// Open local file **without** running `migrate` (export tools only — reads legacy multi-version DBs).
    #[cfg(feature = "local")]
    pub async fn open_local_legacy_export(path: &str) -> Result<Self, StoreError> {
        let db = turso::Builder::new_local(path).build().await?;
        let conn = db.connect()?;
        Self::apply_pragmas(&conn).await?;
        Ok(Self::assembled(conn, None, Some(std::sync::Arc::new(db))))
    }

    /// Open or create a local database file (requires `local` feature on `vox-db`).
    #[cfg(feature = "local")]
    pub async fn open(path: &str) -> Result<Self, StoreError> {
        let db = turso::Builder::new_local(path).build().await?;
        let conn = db.connect()?;
        Self::apply_pragmas(&conn).await?;
        Self::migrate(&conn).await?;
        Ok(Self::assembled(conn, None, Some(std::sync::Arc::new(db))))
    }

    /// In-memory store (requires `local` feature on `vox-db`).
    #[cfg(feature = "local")]
    pub async fn open_memory() -> Result<Self, StoreError> {
        Self::open(":memory:").await
    }

    /// Blocking constructor for in-memory store (tests and `std::thread` call sites).
    #[cfg(feature = "local")]
    pub fn new_memory() -> Result<Self, StoreError> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| StoreError::Db(format!("VoxDb::new_memory tokio runtime: {e}")))?
            .block_on(Self::open_memory())
    }

    /// Remote Turso / libSQL (sync client).
    pub async fn open_remote(url: &str, auth_token: &str) -> Result<Self, StoreError> {
        let db = turso::sync::Builder::new_remote(":memory:")
            .with_remote_url(url)
            .with_auth_token(auth_token)
            .build()
            .await?;
        let conn = db.connect().await?;
        Self::apply_pragmas(&conn).await?;
        Self::migrate(&conn).await?;
        Ok(Self::assembled(conn, Some(db), None))
    }

    /// Embedded replica: local path with remote sync.
    #[cfg(feature = "replication")]
    pub async fn open_embedded_replica(
        local_path: &str,
        url: &str,
        auth_token: &str,
    ) -> Result<Self, StoreError> {
        let db = turso::sync::Builder::new_remote(local_path)
            .with_remote_url(url)
            .with_auth_token(auth_token)
            .build()
            .await?;
        let conn = db.connect().await?;
        Self::apply_pragmas(&conn).await?;
        Self::migrate(&conn).await?;
        Ok(Self::assembled(conn, Some(db), None))
    }

    /// Default local project store at [`super::DEFAULT_PROJECT_STORE_PATH`].
    #[cfg(feature = "local")]
    pub async fn open_default() -> Result<Self, StoreError> {
        let path = super::DEFAULT_PROJECT_STORE_PATH;
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent).map_err(|e| StoreError::Db(e.to_string()))?;
        }
        Self::open(path).await
    }

    /// Pull remote changes when using a sync-backed store; no-op for pure local files.
    pub async fn sync(&self) -> Result<(), StoreError> {
        if let Some(db) = &self.sync_db {
            let _ = db.pull().await?;
        }
        Ok(())
    }

    /// Explicit read boundary for replica-backed handles (see [`crate::ReadConsistency`]).
    pub async fn sync_for(&self, consistency: crate::ReadConsistency) -> Result<(), StoreError> {
        match consistency {
            crate::ReadConsistency::Local => Ok(()),
            crate::ReadConsistency::ReplicaLatest => self.sync().await,
        }
    }
}

fn sqlite_quote_ident(name: &str) -> String {
    let mut s = String::with_capacity(name.len() + 2);
    s.push('"');
    for c in name.chars() {
        if c == '"' {
            s.push_str("\"\"");
        } else {
            s.push(c);
        }
    }
    s.push('"');
    s
}

#[cfg(all(test, feature = "local"))]
mod tests {
    use crate::{ReadConsistency, VoxDb};
    use turso::Builder;

    #[tokio::test]
    async fn sync_for_local_skips_pull() {
        let db = VoxDb::open_memory().await.expect("open");
        db.sync_for(ReadConsistency::Local).await.expect("sync");
    }

    #[tokio::test]
    async fn fresh_db_schema_version_is_baseline_latest() {
        let db = Builder::new_local(":memory:").build().await.expect("mem");
        let conn = db.connect().expect("conn");
        VoxDb::apply_pragmas(&conn).await.expect("pragmas");
        VoxDb::migrate(&conn).await.expect("migrate");
        let mut rows = conn
            .query("SELECT MAX(version) FROM schema_version", ())
            .await
            .expect("q");
        let v: i64 = rows
            .next()
            .await
            .expect("next")
            .expect("row")
            .get(0)
            .expect("v");
        assert_eq!(v, crate::schema::BASELINE_VERSION);
    }
}
