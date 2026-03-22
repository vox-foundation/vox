//! Async CRUD and analytics for `CodeStore`.

use crate::hash::content_hash;
use crate::store::CodeStore;
use crate::store::types::{
    AgentDefEntry, ArtifactEntry, BehaviorEventEntry, BuilderSessionEntry, CodexChangeLogEntry,
    CommandFrequencyEntry, ComponentEntry, EmbeddingEntry, ExecutionEntry, KnowledgeNodeSummary,
    LearnedPatternEntry, LogExecutionParams, LogInteractionParams, MemoryEntry,
    PackageSearchResult, PublishArtifactParams, RegisterAgentParams, ReviewEntry, SaveMemoryParams,
    SaveSnippetParams, ScheduledEntry, SessionTurnEntry, SkillManifestEntry, SnippetEntry,
    StoreError, TrainingPair, TypedStreamEventEntry, UserEntry,
};
use turso::params;

impl CodeStore {
    /// Borrow the underlying libSQL connection (`vox-db`, migrations, tests).
    #[inline]
    #[must_use]
    pub fn connection(&self) -> &turso::Connection {
        &self.conn
    }

    /// Run an async database operation from synchronous call sites (e.g. `std::thread` workers).
    ///
    /// If called from a Tokio worker, uses `block_in_place` + the current handle; otherwise builds
    /// a single-threaded runtime for the duration of the future.
    pub fn block_on<R: Send>(&self, fut: impl std::future::Future<Output = R> + Send) -> R {
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => tokio::task::block_in_place(|| handle.block_on(fut)),
            Err(_) => tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build Tokio runtime for CodeStore::block_on")
                .block_on(fut),
        }
    }

    // ── CAS ─────────────────────────────────────────────

    /// Insert content-addressed bytes into `objects`, returning the Base32Hex hash.
    pub async fn store(&self, kind: &str, data: &[u8]) -> Result<String, StoreError> {
        let hash = content_hash(data);
        self.conn
            .execute(
                "INSERT OR IGNORE INTO objects (hash, kind, data) VALUES (?1, ?2, ?3)",
                params![hash.clone(), kind, data],
            )
            .await?;
        Ok(hash)
    }

    /// Load blob bytes for a content hash.
    pub async fn get(&self, hash: &str) -> Result<Vec<u8>, StoreError> {
        let mut rows = self
            .conn
            .query("SELECT data FROM objects WHERE hash = ?1", params![hash])
            .await?;
        let row = rows
            .next()
            .await?
            .ok_or_else(|| StoreError::NotFound(hash.to_string()))?;
        let blob: Vec<u8> = row.get(0)?;
        Ok(blob)
    }

    // ── Names ───────────────────────────────────────────

    /// Map `(namespace, name)` to a content hash in `names`.
    pub async fn bind_name(
        &self,
        namespace: &str,
        name: &str,
        hash: &str,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO names (namespace, name, hash) VALUES (?1, ?2, ?3)",
                params![namespace, name, hash],
            )
            .await?;
        Ok(())
    }

    /// Resolve a logical name to its bound content hash.
    pub async fn lookup_name(&self, namespace: &str, name: &str) -> Result<String, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT hash FROM names WHERE namespace = ?1 AND name = ?2",
                params![namespace, name],
            )
            .await?;
        let row = rows
            .next()
            .await?
            .ok_or_else(|| StoreError::NotFound(format!("{namespace}.{name}")))?;
        Ok(row.get::<String>(0)?)
    }

    /// Move a binding from `old_name` to `new_name` within the same namespace.
    pub async fn rename(
        &self,
        namespace: &str,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), StoreError> {
        let hash = self.lookup_name(namespace, old_name).await?;
        self.bind_name(namespace, new_name, &hash).await?;
        self.conn
            .execute(
                "DELETE FROM names WHERE namespace = ?1 AND name = ?2",
                params![namespace, old_name],
            )
            .await?;
        Ok(())
    }

    /// List all `(name, hash)` pairs in a namespace, sorted by name.
    pub async fn list_names(&self, namespace: &str) -> Result<Vec<(String, String)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT name, hash FROM names WHERE namespace = ?1 ORDER BY name",
                params![namespace],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((row.get::<String>(0)?, row.get::<String>(1)?));
        }
        Ok(out)
    }

    /// Workspace / namespace string binding (UTF-8 object payload).
    pub async fn get_binding(&self, namespace: &str, name: &str) -> Result<String, StoreError> {
        let hash = self.lookup_name(namespace, name).await?;
        let bytes = self.get(&hash).await?;
        Ok(String::from_utf8(bytes)?)
    }

    /// Store UTF-8 text as a `binding` object and bind it to `(namespace, name)`.
    pub async fn set_binding(
        &self,
        namespace: &str,
        name: &str,
        value: &str,
    ) -> Result<(), StoreError> {
        let hash = self.store("binding", value.as_bytes()).await?;
        self.bind_name(namespace, name, &hash).await
    }

    // ── Object metadata (per content hash) ──────────────

    /// Upsert a `(key, value)` metadata row for a content hash.
    pub async fn set_object_metadata(
        &self,
        hash: &str,
        key: &str,
        value: &str,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO metadata (hash, key, value) VALUES (?1, ?2, ?3)",
                params![hash, key, value],
            )
            .await?;
        Ok(())
    }

    /// Read a single metadata value for `(hash, key)`.
    pub async fn get_object_metadata(&self, hash: &str, key: &str) -> Result<String, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT value FROM metadata WHERE hash = ?1 AND key = ?2",
                params![hash, key],
            )
            .await?;
        let row = rows
            .next()
            .await?
            .ok_or_else(|| StoreError::NotFound(format!("metadata {hash}.{key}")))?;
        Ok(row.get::<String>(0)?)
    }

    /// List all metadata key/value pairs for a hash, sorted by key.
    pub async fn get_all_object_metadata(
        &self,
        hash: &str,
    ) -> Result<Vec<(String, String)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT key, value FROM metadata WHERE hash = ?1 ORDER BY key",
                params![hash],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((row.get::<String>(0)?, row.get::<String>(1)?));
        }
        Ok(out)
    }

    // ── Causal ────────────────────────────────────────────

    /// Record that `hash` depends on `parent_hash` in `causal`.
    pub async fn add_dependency(&self, hash: &str, parent_hash: &str) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO causal (hash, parent_hash) VALUES (?1, ?2)",
                params![hash, parent_hash],
            )
            .await?;
        Ok(())
    }

    /// List parent hashes for a given object hash.
    pub async fn get_dependencies(&self, hash: &str) -> Result<Vec<String>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT parent_hash FROM causal WHERE hash = ?1",
                params![hash],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(row.get::<String>(0)?);
        }
        Ok(out)
    }

    // ── Packages ──────────────────────────────────────────

    /// Insert or replace a package version row in `packages`.
    pub async fn publish_package(
        &self,
        name: &str,
        version: &str,
        hash: &str,
        description: Option<&str>,
        author: Option<&str>,
        license: Option<&str>,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO packages (name, version, hash, description, author, license, yanked)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
                params![name, version, hash, description, author, license],
            )
            .await?;
        Ok(())
    }

    /// Declare a dependency edge for a published package version.
    pub async fn add_package_dep(
        &self,
        package_name: &str,
        package_version: &str,
        dep_name: &str,
        dep_version_req: &str,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO package_deps (package_name, package_version, dep_name, dep_version_req)
                 VALUES (?1, ?2, ?3, ?4)",
                params![package_name, package_version, dep_name, dep_version_req],
            )
            .await?;
        Ok(())
    }

    /// Non-yanked `(version, hash)` pairs for a package name, newest first.
    pub async fn get_package_versions(
        &self,
        name: &str,
    ) -> Result<Vec<(String, String)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT version, hash FROM packages WHERE name = ?1 AND IFNULL(yanked,0)=0 ORDER BY published_at DESC",
                params![name],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((row.get::<String>(0)?, row.get::<String>(1)?));
        }
        Ok(out)
    }

    /// Dependency requirements for a specific package version.
    pub async fn get_package_deps(
        &self,
        package_name: &str,
        package_version: &str,
    ) -> Result<Vec<(String, String)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT dep_name, dep_version_req FROM package_deps
                 WHERE package_name = ?1 AND package_version = ?2 ORDER BY dep_name",
                params![package_name, package_version],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((row.get::<String>(0)?, row.get::<String>(1)?));
        }
        Ok(out)
    }

    /// Fuzzy search over package name and description (non-yanked only).
    pub async fn search_packages(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<PackageSearchResult>, StoreError> {
        let q = format!("%{query}%");
        let mut rows = self
            .conn
            .query(
                "SELECT DISTINCT name, version, description, author, license FROM packages
                 WHERE IFNULL(yanked,0)=0 AND (name LIKE ?1 OR IFNULL(description,'') LIKE ?1)
                 ORDER BY published_at DESC LIMIT ?2",
                params![q, limit],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(PackageSearchResult {
                name: row.get::<String>(0)?,
                version: row.get::<String>(1)?,
                description: row.get::<Option<String>>(2)?,
                author: row.get::<Option<String>>(3)?,
                license: row.get::<Option<String>>(4)?,
            });
        }
        Ok(out)
    }

    /// Mark a package version as yanked (`yanked = 1`).
    pub async fn yank_package(&self, name: &str, version: &str) -> Result<u64, StoreError> {
        let n = self
            .conn
            .execute(
                "UPDATE packages SET yanked = 1 WHERE name = ?1 AND version = ?2",
                params![name, version],
            )
            .await?;
        Ok(n)
    }

    /// Remove a package version and its `package_deps` rows.
    pub async fn delete_package(&self, name: &str, version: &str) -> Result<u64, StoreError> {
        self.conn
            .execute(
                "DELETE FROM package_deps WHERE package_name = ?1 AND package_version = ?2",
                params![name, version],
            )
            .await?;
        let n = self
            .conn
            .execute(
                "DELETE FROM packages WHERE name = ?1 AND version = ?2",
                params![name, version],
            )
            .await?;
        Ok(n)
    }

    // ── Execution log ───────────────────────────────────

    /// Append one `execution_log` row; returns SQLite `rowid`.
    pub async fn log_execution<'a>(&self, p: LogExecutionParams<'a>) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO execution_log
                 (workflow_id, activity_name, status, attempt, input, output, error, options)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    p.workflow_id,
                    p.activity_name,
                    p.status,
                    p.attempt,
                    p.input,
                    p.output,
                    p.error,
                    p.options
                ],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Ordered execution steps for one workflow id.
    pub async fn get_execution_history(
        &self,
        workflow_id: &str,
    ) -> Result<Vec<ExecutionEntry>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, activity_name, status, attempt, error, created_at
                 FROM execution_log WHERE workflow_id = ?1 ORDER BY id ASC",
                params![workflow_id],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(ExecutionEntry {
                id: row.get::<i64>(0)?,
                activity_name: row.get::<String>(1)?,
                status: row.get::<String>(2)?,
                attempt: row.get::<i64>(3)? as u32,
                error: row.get::<Option<String>>(4)?,
                created_at: row.get::<String>(5)?,
            });
        }
        Ok(out)
    }

    // ── Scheduled ─────────────────────────────────────────

    /// Enqueue a function invocation at `run_at` (and optional cron).
    pub async fn schedule_function(
        &self,
        function_hash: &str,
        args: Option<&[u8]>,
        run_at: &str,
        cron_expr: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO scheduled (function_hash, args, run_at, cron_expr)
                 VALUES (?1, ?2, ?3, ?4)",
                params![function_hash, args, run_at, cron_expr],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Pending rows with `run_at <= now`, ordered soonest first.
    pub async fn get_due_scheduled(&self, now: &str) -> Result<Vec<ScheduledEntry>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, function_hash, args, run_at, cron_expr
                 FROM scheduled WHERE status = 'pending' AND run_at <= ?1
                 ORDER BY run_at ASC",
                params![now],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(ScheduledEntry {
                id: row.get::<i64>(0)?,
                function_hash: row.get::<String>(1)?,
                args: row.get::<Option<Vec<u8>>>(2)?,
                run_at: row.get::<String>(3)?,
                cron_expr: row.get::<Option<String>>(4)?,
            });
        }
        Ok(out)
    }

    /// Mark a `scheduled` row as completed so it is not returned by [`Self::get_due_scheduled`].
    pub async fn complete_scheduled(&self, id: i64) -> Result<(), StoreError> {
        self.conn
            .execute(
                "UPDATE scheduled SET status = 'completed' WHERE id = ?1",
                params![id],
            )
            .await?;
        Ok(())
    }

    // ── Components ──────────────────────────────────────

    /// Upsert component metadata in `components`.
    pub async fn register_component(
        &self,
        name: &str,
        namespace: &str,
        schema_hash: Option<&str>,
        description: Option<&str>,
        version: &str,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO components (name, namespace, schema_hash, description, version)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![name, namespace, schema_hash, description, version],
            )
            .await?;
        Ok(())
    }

    /// Components registered under a namespace, sorted by name.
    pub async fn list_components(
        &self,
        namespace: &str,
    ) -> Result<Vec<ComponentEntry>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT name, namespace, version, description FROM components
                 WHERE namespace = ?1 ORDER BY name",
                params![namespace],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(ComponentEntry {
                name: row.get::<String>(0)?,
                namespace: row.get::<String>(1)?,
                version: row.get::<String>(2)?,
                description: row.get::<Option<String>>(3)?,
            });
        }
        Ok(out)
    }

    /// Highest applied version from `schema_version` (baseline Codex uses **1** only).
    pub async fn schema_version(&self) -> Result<i64, StoreError> {
        let mut rows = self
            .conn
            .query("SELECT COALESCE(MAX(version), 0) FROM schema_version", ())
            .await?;
        let row = rows
            .next()
            .await?
            .ok_or_else(|| StoreError::Db("schema_version".into()))?;
        Ok(row.get::<i64>(0)?)
    }

    // ── Users / preferences ─────────────────────────────────

    /// Insert or replace a `users` row.
    pub async fn create_user(
        &self,
        id: &str,
        display_name: &str,
        email: Option<&str>,
        avatar_url: Option<&str>,
        role: &str,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO users (id, display_name, email, avatar_url, role)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, display_name, email, avatar_url, role],
            )
            .await?;
        Ok(())
    }

    /// Load one user by primary key.
    pub async fn get_user(&self, id: &str) -> Result<UserEntry, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, display_name, email, avatar_url, role FROM users WHERE id = ?1",
                params![id],
            )
            .await?;
        let row = rows
            .next()
            .await?
            .ok_or_else(|| StoreError::NotFound(id.to_string()))?;
        Ok(UserEntry {
            id: row.get::<String>(0)?,
            display_name: row.get::<String>(1)?,
            email: row.get::<Option<String>>(2)?,
            avatar_url: row.get::<Option<String>>(3)?,
            role: row.get::<String>(4)?,
        })
    }

    /// Upsert a string preference for a user.
    pub async fn set_user_preference(
        &self,
        user_id: &str,
        key: &str,
        value: &str,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO user_preferences (user_id, key, value) VALUES (?1, ?2, ?3)",
                params![user_id, key, value],
            )
            .await?;
        Ok(())
    }

    /// Read one preference key, or `None` if unset.
    pub async fn get_user_preference(
        &self,
        user_id: &str,
        key: &str,
    ) -> Result<Option<String>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT value FROM user_preferences WHERE user_id = ?1 AND key = ?2",
                params![user_id, key],
            )
            .await?;
        if let Some(row) = rows.next().await? {
            Ok(Some(row.get::<String>(0)?))
        } else {
            Ok(None)
        }
    }

    /// All `(key, value)` pairs for a user, sorted by key.
    pub async fn list_user_preferences(
        &self,
        user_id: &str,
    ) -> Result<Vec<(String, String)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT key, value FROM user_preferences WHERE user_id = ?1 ORDER BY key",
                params![user_id],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((row.get::<String>(0)?, row.get::<String>(1)?));
        }
        Ok(out)
    }

    // ── Memory ────────────────────────────────────────────

    /// Append a `memories` row; returns SQLite `rowid`.
    pub async fn save_memory(&self, p: SaveMemoryParams<'_>) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO memories (agent_id, session_id, memory_type, content, metadata, importance, vcs_snapshot_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    p.agent_id,
                    p.session_id,
                    p.memory_type,
                    p.content,
                    p.metadata,
                    p.importance,
                    p.vcs_snapshot_id
                ],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Recent memories for an agent, optionally filtered by `memory_type` (newest first).
    pub async fn recall_memory(
        &self,
        agent_id: &str,
        memory_type: Option<&str>,
        limit: i64,
        _vcs_snapshot_id: Option<&str>,
    ) -> Result<Vec<MemoryEntry>, StoreError> {
        let mut out = Vec::new();
        if let Some(mt) = memory_type {
            let mut rows = self
                .conn
                .query(
                    "SELECT id, agent_id, session_id, memory_type, content, metadata, importance, created_at
                     FROM memories WHERE agent_id = ?1 AND memory_type = ?2
                     ORDER BY created_at DESC LIMIT ?3",
                    params![agent_id, mt, limit],
                )
                .await?;
            while let Some(row) = rows.next().await? {
                out.push(memory_row(row)?);
            }
        } else {
            let mut rows = self
                .conn
                .query(
                    "SELECT id, agent_id, session_id, memory_type, content, metadata, importance, created_at
                     FROM memories WHERE agent_id = ?1
                     ORDER BY created_at DESC LIMIT ?2",
                    params![agent_id, limit],
                )
                .await?;
            while let Some(row) = rows.next().await? {
                out.push(memory_row(row)?);
            }
        }
        Ok(out)
    }

    /// Cross-agent listing: newest rows for a given `memory_type`.
    pub async fn list_memories_by_type(
        &self,
        memory_type: &str,
        limit: i64,
    ) -> Result<Vec<MemoryEntry>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, agent_id, session_id, memory_type, content, metadata, importance, created_at
                 FROM memories WHERE memory_type = ?1 ORDER BY created_at DESC LIMIT ?2",
                params![memory_type, limit],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(memory_row(row)?);
        }
        Ok(out)
    }

    /// Delete `memories` for `user_id` (agent id) older than `days`.
    pub async fn prune_memories(&self, user_id: &str, days: u32) -> Result<u64, StoreError> {
        let n = self
            .conn
            .execute(
                "DELETE FROM memories WHERE agent_id = ?1
                 AND datetime(created_at) < datetime('now', ?2)",
                params![user_id, format!("-{days} days")],
            )
            .await?;
        Ok(n)
    }

    // ── Knowledge & embeddings ────────────────────────────

    /// Search `knowledge_nodes` by label/content substring.
    pub async fn query_knowledge_nodes(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<(String, String, String)>, StoreError> {
        let q = format!("%{query}%");
        let mut rows = self
            .conn
            .query(
                "SELECT id, label, IFNULL(content,'') FROM knowledge_nodes
                 WHERE label LIKE ?1 OR IFNULL(content,'') LIKE ?1
                 LIMIT ?2",
                params![q, limit],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((
                row.get::<String>(0)?,
                row.get::<String>(1)?,
                row.get::<String>(2)?,
            ));
        }
        Ok(out)
    }

    /// Insert or update a knowledge graph node.
    pub async fn upsert_knowledge_node(
        &self,
        id: &str,
        node_type: &str,
        label: &str,
        content: Option<&str>,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT INTO knowledge_nodes (id, label, content, node_type) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO UPDATE SET
                   label = excluded.label,
                   content = excluded.content,
                   node_type = excluded.node_type",
                params![id, label, content, node_type],
            )
            .await?;
        Ok(())
    }

    /// Add or replace a weighted directed edge between nodes.
    pub async fn create_knowledge_edge(
        &self,
        src_id: &str,
        dst_id: &str,
        relation: &str,
        weight: f64,
        _metadata: Option<&str>,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO knowledge_edges (src_id, dst_id, relation, weight)
                 VALUES (?1, ?2, ?3, ?4)",
                params![src_id, dst_id, relation, weight],
            )
            .await?;
        Ok(())
    }

    /// Persist a float vector as little-endian blob in `embeddings`.
    pub async fn store_embedding(
        &self,
        source_type: &str,
        source_id: &str,
        _model: &str,
        vector: &[f32],
        metadata: Option<&str>,
        _vcs_snapshot_id: Option<&str>,
    ) -> Result<i64, StoreError> {
        let dim = vector.len() as i64;
        let mut blob = Vec::with_capacity(vector.len() * 4);
        for f in vector {
            blob.extend_from_slice(&f.to_le_bytes());
        }
        self.conn
            .execute(
                "INSERT INTO embeddings (source_type, source_id, dim, vector, metadata)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![source_type, source_id, dim, blob, metadata],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Outgoing edges from `node_id` joined to destination labels.
    pub async fn get_knowledge_neighbors(
        &self,
        node_id: &str,
    ) -> Result<Vec<(KnowledgeNodeSummary, String, f64)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT e.dst_id, e.relation, e.weight, n.label
                 FROM knowledge_edges e
                 JOIN knowledge_nodes n ON n.id = e.dst_id
                 WHERE e.src_id = ?1",
                params![node_id],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((
                KnowledgeNodeSummary {
                    id: row.get::<String>(0)?,
                    label: row.get::<String>(3)?,
                },
                row.get::<String>(1)?,
                row.get::<f64>(2)?,
            ));
        }
        Ok(out)
    }

    /// Brute-force cosine similarity over stored embeddings (optionally scoped by `source_type`).
    pub async fn search_similar_embeddings(
        &self,
        vector: &[f32],
        source_type: Option<&str>,
        limit: i64,
    ) -> Result<Vec<(EmbeddingEntry, f32)>, StoreError> {
        let q = if let Some(st) = source_type {
            self.conn
                .query(
                    "SELECT id, source_type, source_id, dim, vector, metadata FROM embeddings WHERE source_type = ?1",
                    params![st],
                )
                .await?
        } else {
            self.conn
                .query(
                    "SELECT id, source_type, source_id, dim, vector, metadata FROM embeddings",
                    (),
                )
                .await?
        };

        let mut rows = q;
        let mut scored: Vec<(EmbeddingEntry, f32)> = Vec::new();
        while let Some(row) = rows.next().await? {
            let dim: i64 = row.get::<i64>(3)?;
            let blob: Vec<u8> = row.get::<Vec<u8>>(4)?;
            let mut v = vec![0f32; dim as usize];
            if blob.len() >= v.len() * 4 {
                for (i, chunk) in blob.chunks_exact(4).enumerate() {
                    if i < v.len() {
                        v[i] = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    }
                }
            }
            let sim = cosine_similarity(vector, &v);
            scored.push((
                EmbeddingEntry {
                    id: row.get::<i64>(0)?,
                    source_type: row.get::<Option<String>>(1)?,
                    source_id: row.get::<String>(2)?,
                    dim,
                    metadata: row.get::<Option<String>>(5)?,
                },
                sim,
            ));
        }
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit as usize);
        Ok(scored)
    }

    // ── Behavior & learning ───────────────────────────────

    /// Append one analytics row to `behavior_events`.
    pub async fn record_behavior_event(
        &self,
        user_id: &str,
        event_type: &str,
        context: Option<&str>,
        metadata: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO behavior_events (user_id, event_type, context, metadata)
                 VALUES (?1, ?2, ?3, ?4)",
                params![user_id, event_type, context, metadata],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Per-event-type counts for a user.
    pub async fn get_behavior_summary(
        &self,
        user_id: &str,
    ) -> Result<Vec<(String, i64)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT event_type, COUNT(*) FROM behavior_events WHERE user_id = ?1 GROUP BY event_type",
                params![user_id],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((row.get::<String>(0)?, row.get::<i64>(1)?));
        }
        Ok(out)
    }

    /// Top CLI commands for a user with success heuristics from JSON metadata.
    pub async fn get_command_frequency(
        &self,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<CommandFrequencyEntry>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT IFNULL(context,''), COUNT(*),
                        SUM(CASE WHEN IFNULL(metadata,'') LIKE '%\"success\":true%' OR IFNULL(metadata,'') LIKE '%\"success\": true%' THEN 1 ELSE 0 END),
                        AVG(CAST(json_extract(metadata, '$.duration_ms') AS REAL))
                 FROM behavior_events
                 WHERE user_id = ?1 AND event_type = 'cli_command'
                 GROUP BY context ORDER BY COUNT(*) DESC LIMIT ?2",
                params![user_id, limit],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(CommandFrequencyEntry {
                command: row.get::<String>(0)?,
                count: row.get::<i64>(1)?,
                success_count: row.get::<i64>(2)?,
                avg_duration_ms: row.get::<Option<f64>>(3)?,
            });
        }
        Ok(out)
    }

    /// Recent behavior events, optionally filtered by `event_type`.
    pub async fn get_behavior_events(
        &self,
        user_id: &str,
        event_type: Option<&str>,
        limit: i64,
    ) -> Result<Vec<BehaviorEventEntry>, StoreError> {
        let mut out = Vec::new();
        if let Some(et) = event_type {
            let mut rows = self
                .conn
                .query(
                    "SELECT id, user_id, event_type, context, metadata, created_at
                     FROM behavior_events WHERE user_id = ?1 AND event_type = ?2
                     ORDER BY id DESC LIMIT ?3",
                    params![user_id, et, limit],
                )
                .await?;
            while let Some(row) = rows.next().await? {
                out.push(behavior_row(row)?);
            }
        } else {
            let mut rows = self
                .conn
                .query(
                    "SELECT id, user_id, event_type, context, metadata, created_at
                     FROM behavior_events WHERE user_id = ?1
                     ORDER BY id DESC LIMIT ?2",
                    params![user_id, limit],
                )
                .await?;
            while let Some(row) = rows.next().await? {
                out.push(behavior_row(row)?);
            }
        }
        Ok(out)
    }

    /// Learned patterns for a user ordered by confidence.
    pub async fn get_learned_patterns(
        &self,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<LearnedPatternEntry>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, user_id, pattern_type, category, description, confidence, vcs_snapshot_id
                 FROM learned_patterns WHERE user_id = ?1
                 ORDER BY confidence DESC, id DESC LIMIT ?2",
                params![user_id, limit],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(LearnedPatternEntry {
                id: row.get::<i64>(0)?,
                user_id: row.get::<String>(1)?,
                pattern_type: row.get::<String>(2)?,
                category: row.get::<String>(3)?,
                description: row.get::<String>(4)?,
                confidence: row.get::<f64>(5)?,
                vcs_snapshot_id: row.get::<Option<String>>(6)?,
            });
        }
        Ok(out)
    }

    /// Patterns for a user within one category.
    pub async fn get_patterns_by_category(
        &self,
        user_id: &str,
        category: &str,
    ) -> Result<Vec<LearnedPatternEntry>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, user_id, pattern_type, category, description, confidence, vcs_snapshot_id
                 FROM learned_patterns WHERE user_id = ?1 AND category = ?2
                 ORDER BY confidence DESC",
                params![user_id, category],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(LearnedPatternEntry {
                id: row.get::<i64>(0)?,
                user_id: row.get::<String>(1)?,
                pattern_type: row.get::<String>(2)?,
                category: row.get::<String>(3)?,
                description: row.get::<String>(4)?,
                confidence: row.get::<f64>(5)?,
                vcs_snapshot_id: row.get::<Option<String>>(6)?,
            });
        }
        Ok(out)
    }

    /// Insert a new learned pattern row.
    pub async fn store_learned_pattern(
        &self,
        user_id: &str,
        pattern_type: &str,
        category: &str,
        description: &str,
        confidence: f64,
        vcs_snapshot_id: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO learned_patterns (user_id, pattern_type, category, description, confidence, vcs_snapshot_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    user_id,
                    pattern_type,
                    category,
                    description,
                    confidence,
                    vcs_snapshot_id
                ],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Adjust confidence score for an existing pattern id.
    pub async fn update_pattern_confidence(
        &self,
        id: i64,
        confidence: f64,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "UPDATE learned_patterns SET confidence = ?1 WHERE id = ?2",
                params![confidence, id],
            )
            .await?;
        Ok(())
    }

    /// Recent LLM interactions joined with optional feedback rows.
    pub async fn get_training_data(&self, limit: i64) -> Result<Vec<TrainingPair>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT i.prompt, i.response, f.rating, f.correction_text, f.feedback_type
                 FROM llm_interactions i
                 LEFT JOIN llm_feedback f ON f.interaction_id = i.id
                 ORDER BY i.id DESC LIMIT ?1",
                params![limit],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(TrainingPair {
                prompt: row.get::<String>(0)?,
                response: row.get::<String>(1)?,
                rating: row.get::<Option<i64>>(2)?,
                correction: row.get::<Option<String>>(3)?,
                feedback_type: row
                    .get::<Option<String>>(4)?
                    .unwrap_or_else(|| "unknown".into()),
            });
        }
        Ok(out)
    }

    // ── LLM feedback ──────────────────────────────────────

    /// Insert one `llm_interactions` row; returns row id.
    pub async fn log_interaction(&self, p: LogInteractionParams<'_>) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO llm_interactions (session_id, user_id, prompt, response, model_version, latency_ms, token_count)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    p.session_id,
                    p.user_id,
                    p.prompt,
                    p.response,
                    p.model_version,
                    p.latency_ms,
                    p.token_count
                ],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Attach human feedback to an interaction id.
    pub async fn submit_feedback(
        &self,
        interaction_id: i64,
        user_id: Option<&str>,
        rating: Option<i64>,
        feedback_type: &str,
        correction_text: Option<&str>,
        preferred_response: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO llm_feedback (interaction_id, user_id, rating, feedback_type, correction_text, preferred_response)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    interaction_id,
                    user_id,
                    rating,
                    feedback_type,
                    correction_text,
                    preferred_response
                ],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    // ── Snippets & artifacts ──────────────────────────────

    /// Insert a code snippet row (`embedding_ref` reserved).
    pub async fn save_snippet(&self, p: SaveSnippetParams<'_>) -> Result<i64, StoreError> {
        let _ = p.embedding_ref;
        self.conn
            .execute(
                "INSERT INTO snippets (language, title, code, description, tags, author_id, source_ref)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    p.language,
                    p.title,
                    p.code,
                    p.description,
                    p.tags,
                    p.author_id,
                    p.source_ref
                ],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Text search across title, code, description, and tags.
    pub async fn search_snippets(
        &self,
        query: &str,
        _language: Option<&str>,
    ) -> Result<Vec<SnippetEntry>, StoreError> {
        let q = format!("%{query}%");
        let mut rows = self
            .conn
            .query(
                "SELECT id, language, title, code, description, tags FROM snippets
                 WHERE title LIKE ?1 OR code LIKE ?1 OR IFNULL(description,'') LIKE ?1 OR IFNULL(tags,'') LIKE ?1
                 ORDER BY id DESC LIMIT 500",
                params![q],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(SnippetEntry {
                id: row.get::<i64>(0)?,
                language: row.get::<String>(1)?,
                title: row.get::<String>(2)?,
                code: row.get::<String>(3)?,
                description: row.get::<Option<String>>(4)?,
                tags: row.get::<Option<String>>(5)?,
            });
        }
        Ok(out)
    }

    /// Upsert an artifact registry row.
    pub async fn publish_artifact(&self, p: PublishArtifactParams<'_>) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO artifacts (id, artifact_type, name, description, author_id, content_hash, version, tags, status)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    p.id,
                    p.artifact_type,
                    p.name,
                    p.description,
                    p.author_id,
                    p.content_hash,
                    p.version,
                    p.tags,
                    p.status
                ],
            )
            .await?;
        Ok(())
    }

    /// Search `artifacts` by name/description substring.
    pub async fn search_artifacts(&self, query: &str) -> Result<Vec<ArtifactEntry>, StoreError> {
        let q = format!("%{query}%");
        let mut rows = self
            .conn
            .query(
                "SELECT name, artifact_type, version, author_id, downloads, avg_rating, status, description
                 FROM artifacts WHERE name LIKE ?1 OR IFNULL(description,'') LIKE ?1 ORDER BY created_at DESC LIMIT 200",
                params![q],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(artifact_row(row)?);
        }
        Ok(out)
    }

    /// List artifacts of a given type, newest first.
    pub async fn list_artifacts(
        &self,
        artifact_type: &str,
    ) -> Result<Vec<ArtifactEntry>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT name, artifact_type, version, author_id, downloads, avg_rating, status, description
                 FROM artifacts WHERE artifact_type = ?1 ORDER BY created_at DESC LIMIT 500",
                params![artifact_type],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(artifact_row(row)?);
        }
        Ok(out)
    }

    /// Record an artifact review row.
    pub async fn submit_review(
        &self,
        artifact_id: &str,
        reviewer_id: &str,
        status: &str,
        comment: Option<&str>,
        rating: Option<i64>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO artifact_reviews (artifact_id, reviewer_id, status, comment, rating)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![artifact_id, reviewer_id, status, comment, rating],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    // ── Agents ────────────────────────────────────────────

    /// Upsert an agent definition in `agents`.
    pub async fn register_agent(&self, p: RegisterAgentParams<'_>) -> Result<(), StoreError> {
        let ip = if p.is_public { 1 } else { 0 };
        self.conn
            .execute(
                "INSERT OR REPLACE INTO agents (id, name, description, system_prompt, tools, model_config, owner_id, version, is_public)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    p.id,
                    p.name,
                    p.description,
                    p.system_prompt,
                    p.tools,
                    p.model_config,
                    p.owner_id,
                    p.version,
                    ip
                ],
            )
            .await?;
        Ok(())
    }

    /// All rows from `agents` sorted by name.
    pub async fn list_agents(&self) -> Result<Vec<AgentDefEntry>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT name, version, description, system_prompt, tools, model_config, is_public FROM agents ORDER BY name",
                (),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(AgentDefEntry {
                name: row.get::<String>(0)?,
                version: row.get::<String>(1)?,
                description: row.get::<Option<String>>(2)?,
                system_prompt: row.get::<Option<String>>(3)?,
                tools: row.get::<Option<String>>(4)?,
                model_config: row.get::<Option<String>>(5)?,
                is_public: row.get::<i64>(6)? != 0,
            });
        }
        Ok(out)
    }

    /// Load one agent by `id` or [`StoreError::NotFound`].
    pub async fn get_agent(&self, id: &str) -> Result<AgentDefEntry, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT name, version, description, system_prompt, tools, model_config, is_public FROM agents WHERE id = ?1",
                params![id],
            )
            .await?;
        let row = rows
            .next()
            .await?
            .ok_or_else(|| StoreError::NotFound(id.to_string()))?;
        Ok(AgentDefEntry {
            name: row.get::<String>(0)?,
            version: row.get::<String>(1)?,
            description: row.get::<Option<String>>(2)?,
            system_prompt: row.get::<Option<String>>(3)?,
            tools: row.get::<Option<String>>(4)?,
            model_config: row.get::<Option<String>>(5)?,
            is_public: row.get::<i64>(6)? != 0,
        })
    }

    // ── Skills ────────────────────────────────────────────

    /// Upsert `skill_manifests` (JSON manifest + markdown body).
    pub async fn publish_skill(
        &self,
        id: &str,
        version: &str,
        manifest_json: &str,
        skill_md: &str,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO skill_manifests (id, version, manifest_json, skill_md)
                 VALUES (?1, ?2, ?3, ?4)",
                params![id, version, manifest_json, skill_md],
            )
            .await?;
        Ok(())
    }

    /// Remove all manifest rows for a skill id.
    pub async fn unpublish_skill(&self, id: &str) -> Result<(), StoreError> {
        self.conn
            .execute("DELETE FROM skill_manifests WHERE id = ?1", params![id])
            .await?;
        Ok(())
    }

    /// Latest published manifest for `id`, if any.
    pub async fn get_skill_manifest(
        &self,
        id: &str,
    ) -> Result<Option<SkillManifestEntry>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, version, manifest_json, skill_md FROM skill_manifests WHERE id = ?1 ORDER BY published_at DESC LIMIT 1",
                params![id],
            )
            .await?;
        if let Some(row) = rows.next().await? {
            Ok(Some(SkillManifestEntry {
                id: row.get::<String>(0)?,
                version: row.get::<String>(1)?,
                manifest_json: row.get::<String>(2)?,
                skill_md: row.get::<String>(3)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// All skill manifests ordered by `published_at` descending.
    pub async fn list_skill_manifests(&self) -> Result<Vec<SkillManifestEntry>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, version, manifest_json, skill_md FROM skill_manifests ORDER BY published_at DESC",
                (),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(SkillManifestEntry {
                id: row.get::<String>(0)?,
                version: row.get::<String>(1)?,
                manifest_json: row.get::<String>(2)?,
                skill_md: row.get::<String>(3)?,
            });
        }
        Ok(out)
    }

    // ── Sessions (agent_sessions) ─────────────────────────

    /// Start or replace an `agent_sessions` row (`status = active`).
    pub async fn create_session(
        &self,
        session_id: &str,
        agent_id: &str,
        meta: Option<&str>,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO agent_sessions (id, agent_id, task_snapshot, status)
                 VALUES (?1, ?2, ?3, 'active')",
                params![session_id, agent_id, meta],
            )
            .await?;
        Ok(())
    }

    // ── DB snapshots (orchestrator undo/redo) ─────────────

    /// Serialize preferences, memories, and patterns for `agent_id` into `db_snapshots`.
    pub async fn take_db_snapshot(
        &self,
        snap_id: u64,
        agent_id: &str,
        description: &str,
    ) -> Result<(), StoreError> {
        let prefs = match self.list_user_preferences(agent_id).await {
            Ok(p) => p,
            Err(_) => Vec::new(),
        };
        let memories = match self.recall_memory(agent_id, None, 10_000, None).await {
            Ok(m) => m,
            Err(_) => Vec::new(),
        };
        let patterns = match self.get_learned_patterns(agent_id, 10_000).await {
            Ok(p) => p,
            Err(_) => Vec::new(),
        };
        let payload = serde_json::json!({
            "preferences": prefs,
            "memories": memories,
            "patterns": patterns,
        });
        let s = serde_json::to_string(&payload)
            .map_err(|e| StoreError::Serialization(e.to_string()))?;
        self.conn
            .execute(
                "INSERT OR REPLACE INTO db_snapshots (id, agent_id, description, payload) VALUES (?1, ?2, ?3, ?4)",
                params![snap_id as i64, agent_id, description, s],
            )
            .await?;
        Ok(())
    }

    /// Replace the agent’s preferences, memories, and patterns from snapshot `snap_id`.
    pub async fn restore_db_snapshot(&self, snap_id: u64) -> Result<(), StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT agent_id, payload FROM db_snapshots WHERE id = ?1",
                params![snap_id as i64],
            )
            .await?;
        let row = rows
            .next()
            .await?
            .ok_or_else(|| StoreError::NotFound(format!("snapshot {snap_id}")))?;
        let agent_id: String = row.get::<String>(0)?;
        let payload: String = row.get::<String>(1)?;
        let v: serde_json::Value =
            serde_json::from_str(&payload).map_err(|e| StoreError::Serialization(e.to_string()))?;

        self.conn
            .execute(
                "DELETE FROM user_preferences WHERE user_id = ?1",
                params![agent_id.clone()],
            )
            .await?;
        self.conn
            .execute(
                "DELETE FROM memories WHERE agent_id = ?1",
                params![agent_id.clone()],
            )
            .await?;
        self.conn
            .execute(
                "DELETE FROM learned_patterns WHERE user_id = ?1",
                params![agent_id.clone()],
            )
            .await?;

        if let Some(arr) = v.get("preferences").and_then(|x| x.as_array()) {
            for p in arr {
                if let Some(pair) = p.as_array()
                    && pair.len() >= 2
                    && let (Some(k), Some(val)) = (pair[0].as_str(), pair[1].as_str())
                {
                    self.set_user_preference(&agent_id, k, val).await?;
                }
            }
        }

        if let Some(arr) = v.get("memories").and_then(|x| x.as_array()) {
            for m in arr {
                let m: MemoryEntry = serde_json::from_value(m.clone())
                    .map_err(|e| StoreError::Serialization(e.to_string()))?;
                if m.content.is_empty() {
                    continue;
                }
                let _ = self
                    .save_memory(SaveMemoryParams {
                        agent_id: &agent_id,
                        session_id: &m.session_id,
                        memory_type: &m.memory_type,
                        content: &m.content,
                        metadata: m.metadata.as_deref(),
                        importance: m.importance,
                        vcs_snapshot_id: None,
                    })
                    .await;
            }
        }

        if let Some(arr) = v.get("patterns").and_then(|x| x.as_array()) {
            for p in arr {
                let p: LearnedPatternEntry = serde_json::from_value(p.clone())
                    .map_err(|e| StoreError::Serialization(e.to_string()))?;
                let _ = self
                    .store_learned_pattern(
                        &agent_id,
                        &p.pattern_type,
                        &p.category,
                        &p.description,
                        p.confidence,
                        p.vcs_snapshot_id.as_deref(),
                    )
                    .await;
            }
        }

        Ok(())
    }

    // ── Research sessions + conversation graph (V17) ────────

    /// Upsert `research_sessions` by `session_key`. Returns stable row `id`.
    pub async fn upsert_research_session(
        &self,
        session_key: &str,
        title: &str,
        status: &str,
        repository_id: &str,
        config_json: Option<&str>,
        summary_json: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO research_sessions (session_key, title, status, repository_id, config_json, summary_json, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))
                 ON CONFLICT(session_key) DO UPDATE SET
                   title = excluded.title,
                   status = excluded.status,
                   repository_id = excluded.repository_id,
                   config_json = excluded.config_json,
                   summary_json = excluded.summary_json,
                   updated_at = datetime('now')",
                params![
                    session_key,
                    title,
                    status,
                    repository_id,
                    config_json,
                    summary_json
                ],
            )
            .await?;
        let mut rows = self
            .conn
            .query(
                "SELECT id FROM research_sessions WHERE session_key = ?1",
                params![session_key],
            )
            .await?;
        let id: i64 = rows
            .next()
            .await?
            .ok_or_else(|| StoreError::Db("research_sessions upsert".into()))?
            .get(0)?;
        Ok(id)
    }

    /// Append one `conversation_versions` row (`UNIQUE(conversation_id, version_index)`).
    pub async fn append_conversation_version(
        &self,
        conversation_id: i64,
        version_index: i64,
        label: &str,
        snapshot_json: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO conversation_versions (conversation_id, version_index, label, snapshot_json)
                 VALUES (?1, ?2, ?3, ?4)",
                params![conversation_id, version_index, label, snapshot_json],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Directed edge between two distinct conversations (`CHECK` rejects self-loops).
    pub async fn insert_conversation_edge(
        &self,
        from_conversation_id: i64,
        to_conversation_id: i64,
        edge_kind: &str,
        weight: f64,
        metadata_json: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO conversation_edges (from_conversation_id, to_conversation_id, edge_kind, weight, metadata_json)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    from_conversation_id,
                    to_conversation_id,
                    edge_kind,
                    weight,
                    metadata_json
                ],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Append a `topic_evolution_events` audit row for `topics.id`.
    pub async fn append_topic_evolution_event(
        &self,
        topic_id: i64,
        event_kind: &str,
        prior_label: Option<&str>,
        new_label: Option<&str>,
        detail_json: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO topic_evolution_events (topic_id, event_kind, prior_label, new_label, detail_json)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![topic_id, event_kind, prior_label, new_label, detail_json],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    // ── Research / eval ───────────────────────────────────

    /// Append one `research_metrics` row (Socrates telemetry, retrieval fusion, research progress, etc.).
    pub async fn append_research_metric(
        &self,
        session_id: &str,
        metric_type: &str,
        metric_value: Option<f64>,
        metadata_json: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO research_metrics (session_id, metric_type, metric_value, metadata_json)
                 VALUES (?1, ?2, ?3, ?4)",
                params![session_id, metric_type, metric_value, metadata_json],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Recent rows for a `metric_type`, optionally filtered by `session_id` prefix (empty = all sessions).
    ///
    /// Returns `(session_id, metric_value, metadata_json)` newest first.
    pub async fn list_research_metrics_by_type(
        &self,
        metric_type: &str,
        session_id_prefix: &str,
        limit: i64,
    ) -> Result<Vec<(String, f64, Option<String>)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT session_id, IFNULL(metric_value,0.0), metadata_json FROM research_metrics
                 WHERE metric_type = ?1
                   AND (?2 = '' OR session_id LIKE (?2 || '%'))
                 ORDER BY id DESC
                 LIMIT ?3",
                params![metric_type, session_id_prefix, limit],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((
                row.get::<String>(0)?,
                row.get::<f64>(1)?,
                row.get::<Option<String>>(2)?,
            ));
        }
        Ok(out)
    }

    /// Metrics recorded for a session, optionally filtered by `metric_type` (empty = all types).
    pub async fn list_research_metrics(
        &self,
        session_id: &str,
        metric_type: &str,
    ) -> Result<Vec<(String, f64, Option<String>)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT metric_type, IFNULL(metric_value,0.0), metadata_json FROM research_metrics
                 WHERE session_id = ?1 AND (?2 = '' OR metric_type = ?2)
                 ORDER BY id ASC",
                params![session_id, metric_type],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((
                row.get::<String>(0)?,
                row.get::<f64>(1)?,
                row.get::<Option<String>>(2)?,
            ));
        }
        Ok(out)
    }

    /// Append one `eval_runs` benchmark row.
    pub async fn record_eval_run(
        &self,
        run_id: &str,
        model_path: Option<&str>,
        format_validity: Option<f64>,
        safety_rejection_rate: Option<f64>,
        quality_proxy: Option<f64>,
        metadata_json: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO eval_runs (run_id, model_path, format_validity, safety_rejection_rate, quality_proxy, metadata_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    run_id,
                    model_path,
                    format_validity,
                    safety_rejection_rate,
                    quality_proxy,
                    metadata_json
                ],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    // ── Maintenance (optional sync helpers) ───────────────

    /// Stub: Turso connections do not support reset from this API.
    pub fn reset(&self) -> Result<(), StoreError> {
        Err(StoreError::Db(
            "reset() is not supported on Turso connections — use `vox db` tooling or drop the database file"
                .into(),
        ))
    }

    /// **Unsupported** for async Turso `CodeStore` (use Codex sync path instead).
    pub fn sample_table(
        &self,
        _table: &str,
        _limit: i64,
    ) -> Result<(Vec<String>, Vec<Vec<String>>), StoreError> {
        Err(StoreError::Db(
            "sample_table requires synchronous Codex store".into(),
        ))
    }

    /// **Unsupported** here — run maintenance via `vox db` / Codex tooling.
    pub fn vacuum(&self) -> Result<(), StoreError> {
        Err(StoreError::Db(
            "VACUUM: use async maintenance or Codex".into(),
        ))
    }

    // ── Placeholder entity stores (for public API stability) ─

    /// All `builder_sessions` rows, newest first.
    pub async fn list_builder_sessions(&self) -> Result<Vec<BuilderSessionEntry>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, payload_json, created_at FROM builder_sessions ORDER BY created_at DESC",
                (),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(BuilderSessionEntry {
                id: row.get::<String>(0)?,
                payload_json: row.get::<String>(1)?,
                created_at: row.get::<String>(2)?,
            });
        }
        Ok(out)
    }

    /// Ordered turns for one `session_id`.
    pub async fn list_session_turns(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionTurnEntry>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, session_id, payload_json, created_at FROM session_turns WHERE session_id = ?1 ORDER BY id ASC",
                params![session_id],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(SessionTurnEntry {
                id: row.get::<i64>(0)?,
                session_id: row.get::<String>(1)?,
                payload_json: row.get::<String>(2)?,
                created_at: row.get::<String>(3)?,
            });
        }
        Ok(out)
    }

    /// SSE / stream events persisted under `stream_id`.
    pub async fn list_typed_stream_events(
        &self,
        stream_id: &str,
    ) -> Result<Vec<TypedStreamEventEntry>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, stream_id, payload_json, created_at FROM typed_stream_events WHERE stream_id = ?1 ORDER BY id ASC",
                params![stream_id],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(TypedStreamEventEntry {
                id: row.get::<i64>(0)?,
                stream_id: row.get::<String>(1)?,
                payload_json: row.get::<String>(2)?,
                created_at: row.get::<String>(3)?,
            });
        }
        Ok(out)
    }

    /// Populi reviews referencing `target_id`, newest first.
    pub async fn list_reviews_for_target(
        &self,
        target_id: &str,
    ) -> Result<Vec<ReviewEntry>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, target_id, review_kind, payload_json, created_at FROM populi_reviews WHERE target_id = ?1 ORDER BY id DESC",
                params![target_id],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(ReviewEntry {
                id: row.get::<i64>(0)?,
                target_id: row.get::<String>(1)?,
                review_kind: row.get::<String>(2)?,
                payload_json: row.get::<String>(3)?,
                created_at: row.get::<String>(4)?,
            });
        }
        Ok(out)
    }

    // ── Codex reactivity (manifest fragment v8) ───────────

    /// List `codex_change_log` rows with `id` strictly greater than `after_id`, optionally filtered by `topic`.
    pub async fn list_codex_changes_since(
        &self,
        topic: Option<&str>,
        after_id: i64,
        limit: i64,
    ) -> Result<Vec<CodexChangeLogEntry>, StoreError> {
        let limit = limit.clamp(1, 10_000);
        let mut rows = if let Some(t) = topic {
            self.conn
                .query(
                    "SELECT id, topic, entity_kind, entity_id, change_kind, payload_json, created_at
                     FROM codex_change_log WHERE topic = ?1 AND id > ?2 ORDER BY id ASC LIMIT ?3",
                    params![t, after_id, limit],
                )
                .await?
        } else {
            self.conn
                .query(
                    "SELECT id, topic, entity_kind, entity_id, change_kind, payload_json, created_at
                     FROM codex_change_log WHERE id > ?1 ORDER BY id ASC LIMIT ?2",
                    params![after_id, limit],
                )
                .await?
        };
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(CodexChangeLogEntry {
                id: row.get::<i64>(0)?,
                topic: row.get::<String>(1)?,
                entity_kind: row.get::<Option<String>>(2)?,
                entity_id: row.get::<Option<String>>(3)?,
                change_kind: row.get::<String>(4)?,
                payload_json: row.get::<Option<String>>(5)?,
                created_at: row.get::<String>(6)?,
            });
        }
        Ok(out)
    }

    /// Append one row to `codex_change_log` for reactive invalidation / SSE fanout.
    pub async fn append_codex_change(
        &self,
        topic: &str,
        entity_kind: Option<&str>,
        entity_id: Option<&str>,
        change_kind: &str,
        payload_json: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO codex_change_log (topic, entity_kind, entity_id, change_kind, payload_json)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![topic, entity_kind, entity_id, change_kind, payload_json],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Record a greenfield baseline / import provenance row in `codex_schema_lineage`.
    pub async fn record_codex_schema_lineage(
        &self,
        baseline_id: &str,
        schema_digest: &str,
        provenance: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO codex_schema_lineage (baseline_id, schema_digest, provenance)
                 VALUES (?1, ?2, ?3)",
                params![baseline_id, schema_digest, provenance],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Register a subscription topic (client delivery is out-of-band; SSOT is this table).
    pub async fn upsert_codex_subscription(
        &self,
        id: &str,
        topic: &str,
        filter_json: Option<&str>,
        client_hint: Option<&str>,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT INTO codex_subscriptions (id, topic, filter_json, client_hint)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO UPDATE SET
                   topic = excluded.topic,
                   filter_json = excluded.filter_json,
                   client_hint = excluded.client_hint",
                params![id, topic, filter_json, client_hint],
            )
            .await?;
        Ok(())
    }

    /// List `(orchestrator_agent_id, reliability)` rows for Socrates-style routing (manifest fragment v10).
    pub async fn list_agent_reliability(&self) -> Result<Vec<(u64, f64)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT agent_id, reliability FROM agent_reliability ORDER BY agent_id",
                (),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            let id: i64 = row.get(0)?;
            let r: f64 = row.get(1)?;
            out.push((id as u64, r));
        }
        Ok(out)
    }

    /// Update an agent reliability score with an EMA toward success (`1.0`) or failure (`0.0`).
    pub async fn record_task_reliability_observation(
        &self,
        agent_id: u64,
        success: bool,
    ) -> Result<(), StoreError> {
        let obs = if success { 1.0f64 } else { 0.0f64 };
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let aid = agent_id as i64;
        let mut rows = self
            .conn
            .query(
                "SELECT reliability FROM agent_reliability WHERE agent_id = ?1",
                params![aid],
            )
            .await?;
        let old: f64 = if let Some(row) = rows.next().await? {
            row.get(0)?
        } else {
            0.5
        };
        let new_rel = (old * 0.95 + obs * 0.05).clamp(0.02, 0.98);
        self.conn
            .execute(
                "INSERT INTO agent_reliability (agent_id, reliability, updated_at_ms) VALUES (?1, ?2, ?3)
                 ON CONFLICT(agent_id) DO UPDATE SET reliability = excluded.reliability, updated_at_ms = excluded.updated_at_ms",
                params![aid, new_rel, now_ms],
            )
            .await?;
        Ok(())
    }
}

fn memory_row(row: turso::Row) -> Result<MemoryEntry, StoreError> {
    Ok(MemoryEntry {
        id: row.get::<i64>(0)?,
        agent_id: row.get::<String>(1)?,
        session_id: row.get::<String>(2)?,
        memory_type: row.get::<String>(3)?,
        content: row.get::<String>(4)?,
        metadata: row.get::<Option<String>>(5)?,
        importance: row.get::<f64>(6)?,
        created_at: row.get::<String>(7)?,
    })
}

fn behavior_row(row: turso::Row) -> Result<BehaviorEventEntry, StoreError> {
    Ok(BehaviorEventEntry {
        id: row.get::<i64>(0)?,
        user_id: row.get::<String>(1)?,
        event_type: row.get::<String>(2)?,
        context: row.get::<Option<String>>(3)?,
        metadata: row.get::<Option<String>>(4)?,
        created_at: row.get::<String>(5)?,
    })
}

fn artifact_row(row: turso::Row) -> Result<ArtifactEntry, StoreError> {
    Ok(ArtifactEntry {
        name: row.get::<String>(0)?,
        artifact_type: row.get::<String>(1)?,
        version: row.get::<String>(2)?,
        author_id: row.get::<String>(3)?,
        downloads: row.get::<i64>(4)?,
        avg_rating: row.get::<f64>(5)?,
        status: row.get::<String>(6)?,
        description: row.get::<Option<String>>(7)?,
    })
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0f32;
    let mut na = 0f32;
    let mut nb = 0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    let d = (na.sqrt() * nb.sqrt()).max(1e-8);
    dot / d
}
