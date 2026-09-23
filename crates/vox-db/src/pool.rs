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

    /// Shared probe body for the two `last_insert_rowid()` independence tests
    /// below. Creates a scratch table, spawns `TASKS` tasks each doing
    /// `WRITES_PER_TASK` inserts against its own `pool.get()`-vended connection,
    /// reads each insert back by `last_insert_rowid()`, and counts markers that
    /// don't match. `yield_between` controls whether a `tokio::task::yield_now()`
    /// runs between `execute()` and `last_insert_rowid()` — see the doc comments
    /// on the two callers for why that matters and what it costs.
    async fn last_insert_rowid_probe(pool: VoxDbPool, yield_between: bool) -> usize {
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
                    if yield_between {
                        // Widens the unguarded execute -> last_insert_rowid window to
                        // a real scheduling point so a regression to a shared
                        // connection fails loudly instead of slipping through.
                        // Production has no such yield — see the caller's doc comment.
                        tokio::task::yield_now().await;
                    }
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
        total_mismatches
    }

    /// Default-run coverage for the "Known residual gap" documented on
    /// [`crate::GuardedConnection`] (`crates/vox-db/src/lib.rs:455-470`):
    /// `turso::Connection::last_insert_rowid()` is a synchronous, per-connection
    /// read that bypasses `ConcurrentGuard`, so two tasks sharing one *cloned*
    /// connection (exactly how `vox-gui`'s `GuiDbPool` shares one `Arc<VoxDb>`
    /// across all GUI commands) can race: task A's `execute()` + `last_insert_rowid()`
    /// pair can straddle task B's own `execute()` on the shared connection, and A
    /// silently reads back B's row id. `VoxDbPool::get()` hands each caller an
    /// independent `turso::Connection` (`db.connect()`, not a clone of one shared
    /// connection), so this race is structurally impossible.
    ///
    /// ## Weak detection power — read before trusting this as a race guard
    ///
    /// This test runs with `yield_between = false`, i.e. no scheduling point
    /// between `execute()` and `last_insert_rowid()`. That means it does **not**
    /// reliably catch a regression to a shared connection: the real unguarded
    /// window there is a few nanoseconds of straight-line synchronous code, far
    /// narrower than a contended `tokio::sync::Mutex` handoff, so a deliberately
    /// shared connection still reports 0 mismatches here almost every time
    /// (mutation-verified 2026-09-22: 0/1000 mismatches without the yield vs.
    /// 981/1000 with it — see the yielding test below for the full numbers). This
    /// is the pre-yield version of that test, which ran hundreds of times across
    /// the investigating session with zero corruption and zero false negatives
    /// from the race simply not reproducing.
    ///
    /// What this test DOES verify, reliably: `VoxDbPool::get()` runs to
    /// completion under realistic (non-adversarial) concurrent load and returns
    /// handles that behave independently, with zero database corruption. It
    /// exists so `cargo test -p vox-db --lib` has non-zero, always-green coverage
    /// of `VoxDbPool::get()`'s connection-vending path — it is not a substitute
    /// for the high-detection-power `#[ignore]`d reproducer below, which is the
    /// one to run when actually chasing a suspected sharing regression.
    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn pooled_connections_return_independent_handles() {
        let pool = VoxDbPool::new(DbConfig::Memory).await.expect("pool init");
        let total_mismatches = last_insert_rowid_probe(pool, false).await;
        assert_eq!(
            total_mismatches, 0,
            "pooled connections must never read back a different task's row id; a \
             nonzero count means VoxDbPool::get() is sharing a connection instead of \
             vending an independent one"
        );
    }

    /// High-detection-power variant of
    /// [`pooled_connections_return_independent_handles`], with a `yield_now()`
    /// between `execute()` and `last_insert_rowid()` to widen the race window
    /// into something the scheduler can actually land in.
    ///
    /// ## Why there is a `yield_now()` in the write loop
    ///
    /// The yield is a deliberate testing technique, not a model of production
    /// timing. **Production code has no yield point there.** Do not read it as a
    /// claim that production tasks interleave at this rate; they do not, which is
    /// precisely why the defect is so hard to observe in the wild.
    ///
    /// The yield exists because without it this test cannot fail. Mutation-verified
    /// 2026-09-22: substituting a shared connection (one `pool.get()` reused across
    /// all tasks — i.e. the exact regression this test guards against) reports **0**
    /// mismatches with no yield, because a contended `tokio::Mutex` handoff costs
    /// microseconds while the window it would have to land in is nanoseconds, so the
    /// interleaving effectively never happens. With the yield in place, that same
    /// shared-connection mutation reports **981/1000** mismatches and fails, while
    /// this test — independent per-task connections, identical yield — reports 0 and
    /// passes. The yield is therefore what converts this from a near-always-passing
    /// empirical probe into a regression gate that actually catches the regression,
    /// and it costs nothing in correctness: pooled connections are independent
    /// regardless of scheduling, so no legitimate implementation can be made to fail
    /// by adding a scheduling point here.
    ///
    /// ## `#[ignore]`d 2026-09-22: the widened window also hits an unrelated,
    /// ## pre-existing Turso `:memory:` concurrency bug — do not remove `#[ignore]`
    /// ## without first re-litigating this
    ///
    /// The same `yield_now()` that makes this test a reliable gate for the
    /// rowid race *also* makes roughly half of runs panic with
    /// `Corrupt("Invalid page type: 0")` from inside `turso`'s own B-tree code,
    /// not from this test's assertion (10/20 runs, 2026-09-22, this machine — 8
    /// as the `Corrupt` panic directly, 2 as a `.expect("row present")` panic
    /// from a `SELECT` that found zero rows for an id `last_insert_rowid()` just
    /// returned; both are the same underlying corruption surfacing at different
    /// points). This is a distinct, more serious finding than the rowid race:
    /// `VoxDbPool::new(DbConfig::Memory)` backs every `pool.get()`'d connection
    /// with the same underlying `Arc<turso::Database>` (see `VoxDbPool::get`,
    /// above) — for `:memory:`, that means every "independent" connection shares
    /// one in-process B-tree.
    ///
    /// **Root cause isolated 2026-09-22.** Two competing hypotheses were open:
    /// (a) the concurrent INSERT/SELECT traffic itself corrupts the shared
    /// B-tree, or (b) `VoxDbPool::get()` calling `VoxDb::apply_pragmas` (6 async
    /// round-trips: `journal_mode`, `busy_timeout`, `synchronous`,
    /// `foreign_keys`, `cache_size`, then a `journal_mode` read-back) on every
    /// single call — with all 100 tasks calling `pool.get()` concurrently while
    /// other tasks are already executing inserts — is the actual trigger. To
    /// distinguish them, a temporary test variant acquired all 100 connections
    /// serially (`pool.get().await` in a plain loop, so every `apply_pragmas`
    /// call completed before any concurrent write traffic began) and then spawned
    /// the identical concurrent write/yield/read-back workload against the
    /// pre-acquired connections. Result: 11/20 runs still corrupted (55%, all
    /// `Corrupt("Invalid page type: 0")`), statistically indistinguishable from
    /// the 10/20 (50%) baseline with `apply_pragmas` running concurrently with
    /// writes. **This rules out hypothesis (b):** serializing pragma application
    /// does not reduce the corruption rate, so concurrent `apply_pragmas` calls
    /// are not the cause. The corruption is caused by genuine concurrent
    /// read/write traffic against Turso's shared in-memory B-tree across
    /// independent connections, confirming the original hypothesis (a).
    ///
    /// Whether this also affects `DbConfig::Local` (file-backed, what production
    /// and this plan's other benchmarks actually use) is **not established** —
    /// untested as of this writing, and out of scope for this plan, which makes
    /// no production code changes. Until someone characterizes and either fixes
    /// or documents this properly (a real bug report against `turso`, or a
    /// documented constraint on `VoxDbPool`'s `:memory:` mode), this test stays
    /// `#[ignore]`d so it does not nondeterministically fail unrelated CI runs on
    /// this crate — run it deliberately with `cargo test -p vox-db --lib --
    /// --ignored pooled_connections_never_race_on_last_insert_rowid` to reproduce
    /// either finding.
    #[ignore = "yield_now() (added to make this a reliable rowid-race gate) also \
                triggers an unrelated, unfixed Turso :memory: concurrency bug \
                (Corrupt(\"Invalid page type: 0\")) in ~50% of runs — see doc \
                comment above before re-enabling"]
    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn pooled_connections_never_race_on_last_insert_rowid() {
        let pool = VoxDbPool::new(DbConfig::Memory).await.expect("pool init");
        let total_mismatches = last_insert_rowid_probe(pool, true).await;
        assert_eq!(
            total_mismatches, 0,
            "pooled connections must never read back a different task's row id; a \
             nonzero count means VoxDbPool::get() is sharing a connection instead of \
             vending an independent one"
        );
    }
}
