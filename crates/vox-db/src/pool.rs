use crate::store::types::StoreError;
use crate::{DbCircuitBreaker, DbConfig, VoxDb};
use std::sync::Arc;

/// Underlying database factory for parallel execution.
pub enum DbBackend {
    #[cfg(feature = "local")]
    Local(Arc<turso::Database>),
    Sync(Arc<turso::sync::Database>),
}

/// A lightweight pool/factory for [`VoxDb`] connections.
///
/// `VoxDbPool` holds the core libSQL `Database` engine and can vend multiple independent
/// `Connection` handles for parallel task execution. This avoids blocking connections
/// in highly concurrent systems (like the orchestrator MCP loops).
#[derive(Clone)]
pub struct VoxDbPool {
    backend: Arc<DbBackend>,
    breaker: Arc<DbCircuitBreaker>,
    // Shared sqlite probe cache across all handles from this pool.
    sqlite_probe_cache: Arc<tokio::sync::RwLock<Option<crate::capabilities::SqliteProbeSnapshot>>>,
    writer: Arc<tokio::sync::OnceCell<crate::VoxWriteHandle>>,
}

impl VoxDbPool {
    /// Create a new pool using the project's canonical store configuration.
    #[cfg(all(feature = "local", feature = "host-integration"))]
    pub async fn new_canonical() -> Result<Self, StoreError> {
        let config = DbConfig::resolve_canonical().map_err(StoreError::NotFound)?;
        Self::new(config).await
    }

    /// Instantiate the database pool from the specified config.
    /// This runs migrations automatically on the first connection established.
    pub async fn new(config: DbConfig) -> Result<Self, StoreError> {
        let backend = match &config {
            #[cfg(feature = "local")]
            DbConfig::Local { path } => {
                let db = turso::Builder::new_local(path)
                    .build()
                    .await
                    .map_err(StoreError::from)?;
                Arc::new(DbBackend::Local(Arc::new(db)))
            }
            #[cfg(feature = "local")]
            DbConfig::Memory => {
                let db = turso::Builder::new_local(":memory:")
                    .build()
                    .await
                    .map_err(StoreError::from)?;
                Arc::new(DbBackend::Local(Arc::new(db)))
            }
            DbConfig::Remote { url, token } => {
                let db = turso::sync::Builder::new_remote(":memory:")
                    .with_remote_url(url)
                    .with_auth_token(token)
                    .build()
                    .await
                    .map_err(StoreError::from)?;
                Arc::new(DbBackend::Sync(Arc::new(db)))
            }
            #[cfg(feature = "replication")]
            DbConfig::EmbeddedReplica {
                local_path,
                url,
                token,
            } => {
                let db = turso::sync::Builder::new_remote(local_path.as_str())
                    .with_remote_url(url)
                    .with_auth_token(token)
                    .build()
                    .await
                    .map_err(StoreError::from)?;
                Arc::new(DbBackend::Sync(Arc::new(db)))
            }
        };

        let pool = Self {
            backend,
            breaker: Arc::new(DbCircuitBreaker::from_env()),
            sqlite_probe_cache: Arc::new(tokio::sync::RwLock::new(None)),
            writer: Arc::new(tokio::sync::OnceCell::new()),
        };

        // Bootstrap: perform schema check / migrations via a temporary connection
        let conn = pool.get().await?;
        crate::VoxDb::apply_pragmas(&conn.conn).await?;
        crate::VoxDb::migrate(&conn.conn).await?;
        Ok(pool)
    }

    /// Obtain a new, independent [`VoxDb`] handle.
    /// Connections are very cheap to construct from an initialized local database.
    pub async fn get(&self) -> Result<VoxDb, StoreError> {
        let conn = match &*self.backend {
            #[cfg(feature = "local")]
            DbBackend::Local(db) => db.connect().map_err(StoreError::from)?,
            DbBackend::Sync(db) => db.connect().await.map_err(StoreError::from)?,
        };

        crate::VoxDb::apply_pragmas(&conn).await?;

        // Extract raw turso sync database if any.
        // VoxDb expects an Option<turso::sync::Database>, but we hold Arc<turso::sync::Database>.
        // Since turso doesn't allow extracting it cheaply if it doesn't implement Clone,
        // we might actually have a slight discrepancy, but VoxDb sync calls can use it.
        // Wait, turso::sync::Database doesn't implement Clone?
        // Let's just create the exact type `VoxDb` expects.

        let sync_db_out = match &*self.backend {
            DbBackend::Sync(sdb) => Some((**sdb).clone()),
            #[cfg(feature = "local")]
            _ => None,
        };

        Ok(VoxDb {
            conn: crate::GuardedConnection::new(conn),
            sync_db: sync_db_out,
            local_db: None,
            writer: self.writer.get().cloned(),
            breaker: Arc::clone(&self.breaker),
            sqlite_probe_cache: Arc::clone(&self.sqlite_probe_cache),
        })
    }

    /// Obtain a handle to the dedicated database writer task.
    /// Spawns the actor on first access if not already running.
    pub async fn writer(&self) -> Result<crate::VoxWriteHandle, StoreError> {
        self.writer
            .get_or_try_init(|| async {
                let db = self.get().await?;
                Ok(crate::writer_actor::spawn_writer(db))
            })
            .await
            .cloned()
    }
}

#[cfg(all(test, feature = "local"))]
mod tests {
    use super::*;
    use crate::DbConfig;

    /// Guards the `last_insert_rowid()` bypass documented on
    /// [`crate::GuardedConnection`]: the read is synchronous and skips
    /// `ConcurrentGuard`, so tasks sharing one cloned connection (as `vox-gui`'s
    /// `GuiDbPool` does) can read back another task's row id. `VoxDbPool::get()`
    /// must vend independent connections, so this must report 0 mismatches.
    ///
    /// The `yield_now()` between insert and `last_insert_rowid()` has no
    /// production counterpart; it widens the window so a sharing regression
    /// fails. Mutation-verified 2026-09-22: a shared connection gave 981/1000
    /// mismatches with the yield and 0/1000 without it.
    ///
    /// It uses a temp file, not `:memory:`: Turso 0.6.1 `:memory:` corrupts in
    /// ~50% of runs under this workload (reproduces with bare turso; file-backed
    /// 0/300, 2026-09-25). See `crates/vox-db/tests/pool_corruption_probe.rs` and
    /// `docs/src/architecture/2026-09-22-voxdb-turso-pooling-and-mvcc-recommendation.md`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn pooled_connections_never_race_on_last_insert_rowid() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir
            .path()
            .join("rowid_race.db")
            .to_string_lossy()
            .into_owned();
        let pool = VoxDbPool::new(DbConfig::Local { path })
            .await
            .expect("pool init");
        {
            let db = pool.get().await.expect("get conn for schema");
            db.connection()
                .execute_batch(
                    "CREATE TABLE IF NOT EXISTS rowid_race_probe (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        marker TEXT NOT NULL
                    )",
                )
                .await
                .expect("create probe table");
        }

        const TASKS: usize = 100;
        const WRITES_PER_TASK: usize = 10;
        let mut handles = Vec::with_capacity(TASKS);
        for t in 0..TASKS {
            let pool = pool.clone();
            handles.push(tokio::spawn(async move {
                let db = pool.get().await.expect("get pooled conn");
                let mut mismatches = 0usize;
                for w in 0..WRITES_PER_TASK {
                    let marker = format!("task-{t}-write-{w}");
                    db.connection()
                        .execute(
                            "INSERT INTO rowid_race_probe (marker) VALUES (?1)",
                            turso::params![marker.clone()],
                        )
                        .await
                        .expect("insert");
                    tokio::task::yield_now().await;
                    let returned_id = db.connection().last_insert_rowid();

                    let mut rows = db
                        .connection()
                        .query(
                            "SELECT marker FROM rowid_race_probe WHERE id = ?1",
                            turso::params![returned_id],
                        )
                        .await
                        .expect("select back");
                    let row = rows.next().await.expect("row query").expect("row present");
                    let actual_marker: String = row.get(0).expect("marker column");
                    if actual_marker != marker {
                        mismatches += 1;
                    }
                }
                mismatches
            }));
        }

        let mut total_mismatches = 0usize;
        for h in handles {
            total_mismatches += h.await.expect("task panicked");
        }
        assert_eq!(
            total_mismatches, 0,
            "pooled connections must never read back a different task's row id; a \
             nonzero count means VoxDbPool::get() is sharing a connection instead of \
             vending an independent one"
        );
    }
}
