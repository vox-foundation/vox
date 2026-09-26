//! Reproducer for the Turso 0.6.1 B-tree corruption (`Corrupt("Invalid page type: 0")`)
//! hit by concurrent INSERT + `last_insert_rowid()` + SELECT traffic across independent
//! connections. See `docs/src/architecture/2026-09-22-voxdb-turso-pooling-and-mvcc-recommendation.md`
//! (2026-09-25 section) for rates and conclusions.
//!
//! Every test here is `#[ignore]`d: they fail nondeterministically by design (that is the
//! bug being characterized). Run one per process so each trial gets a fresh database:
//!
//! ```text
//! cargo test -p vox-db --test pool_corruption_probe -- --ignored --exact <name>
//! ```

use std::sync::Arc;

use vox_db::{DbConfig, VoxDb, VoxDbPool};

const TASKS: usize = 100;
const WRITES_PER_TASK: usize = 10;
const CREATE: &str = "CREATE TABLE IF NOT EXISTS rowid_race_probe (id INTEGER PRIMARY KEY AUTOINCREMENT, marker TEXT NOT NULL)";

/// Same body as `pool.rs::tests::pooled_connections_never_race_on_last_insert_rowid`:
/// insert, yield, `last_insert_rowid()`, select back. Works for both
/// `GuardedConnection` and bare `turso::Connection` (identical method shapes).
macro_rules! write_loop {
    ($conn:expr, $t:expr) => {{
        let mut mismatches = 0usize;
        for w in 0..WRITES_PER_TASK {
            let marker = format!("task-{}-write-{w}", $t);
            $conn
                .execute(
                    "INSERT INTO rowid_race_probe (marker) VALUES (?1)",
                    turso::params![marker.clone()],
                )
                .await
                .expect("insert");
            tokio::task::yield_now().await;
            let id = $conn.last_insert_rowid();
            let mut rows = $conn
                .query(
                    "SELECT marker FROM rowid_race_probe WHERE id = ?1",
                    turso::params![id],
                )
                .await
                .expect("select back");
            let row = rows.next().await.expect("row query").expect("row present");
            let actual: String = row.get(0).expect("marker column");
            if actual != marker {
                mismatches += 1;
            }
        }
        mismatches
    }};
}

/// Fresh on-disk path per process; `-wal`/`-shm` siblings are removed with it.
struct TempDb(String);
impl TempDb {
    fn new() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let p = std::env::temp_dir().join(format!(
            "vox-corruption-probe-{}-{nanos}.db",
            std::process::id()
        ));
        Self(p.to_string_lossy().into_owned())
    }
}
impl Drop for TempDb {
    fn drop(&mut self) {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", self.0));
        }
    }
}

/// Joins every task, prints one greppable summary line, and fails if any task panicked or
/// `PRAGMA integrity_check` is not `ok`. Rowid mismatches are reported, not asserted (the
/// `shared` variant is expected to have them — that is the separate rowid race).
async fn finish(
    label: &str,
    handles: Vec<tokio::task::JoinHandle<usize>>,
    integrity: impl std::future::Future<Output = String>,
) {
    let mut mismatches = 0usize;
    let mut panics = Vec::new();
    for h in handles {
        match h.await {
            Ok(m) => mismatches += m,
            Err(e) => panics.push(e.to_string()),
        }
    }
    let integrity = integrity.await;
    println!(
        "PROBE {label}: panics={} mismatches={mismatches} integrity={integrity:?} first_panic={:?}",
        panics.len(),
        panics.first()
    );
    assert!(
        panics.is_empty() && integrity == "ok",
        "corruption: {label}"
    );
}

async fn integrity_guarded(db: &VoxDb) -> String {
    match db.connection().query("PRAGMA integrity_check", ()).await {
        Ok(mut rows) => match rows.next().await {
            Ok(Some(r)) => r.get::<String>(0).unwrap_or_default(),
            other => format!("{other:?}"),
        },
        Err(e) => format!("error: {e}"),
    }
}

async fn integrity_raw(conn: &turso::Connection) -> String {
    match conn.query("PRAGMA integrity_check", ()).await {
        Ok(mut rows) => match rows.next().await {
            Ok(Some(r)) => r.get::<String>(0).unwrap_or_default(),
            other => format!("{other:?}"),
        },
        Err(e) => format!("error: {e}"),
    }
}

async fn run_pooled(label: &str, config: DbConfig) {
    let pool = VoxDbPool::new(config).await.expect("pool init");
    let setup = pool.get().await.expect("setup conn");
    setup
        .connection()
        .execute_batch(CREATE)
        .await
        .expect("create");
    let mut handles = Vec::with_capacity(TASKS);
    for t in 0..TASKS {
        let pool = pool.clone();
        handles.push(tokio::spawn(async move {
            let db = pool.get().await.expect("get pooled conn");
            write_loop!(db.connection(), t)
        }));
    }
    finish(label, handles, integrity_guarded(&setup)).await;
}

async fn run_shared(label: &str, config: DbConfig) {
    let db = Arc::new(VoxDb::connect(config).await.expect("connect"));
    db.connection().execute_batch(CREATE).await.expect("create");
    let mut handles = Vec::with_capacity(TASKS);
    for t in 0..TASKS {
        let db = db.clone();
        handles.push(tokio::spawn(async move { write_loop!(db.connection(), t) }));
    }
    finish(label, handles, integrity_guarded(&db)).await;
}

/// Plain `turso::Builder`, one `db.connect()` per task. No vox-db code at all: no
/// migrations, no `GuardedConnection`; only the two pragmas without which the workload
/// cannot run (`busy_timeout`, else every contended insert is `Busy`) or would differ in
/// journaling from `VoxDb` (`journal_mode=WAL`).
async fn raw_connect(db: &turso::Database) -> turso::Connection {
    let conn = db.connect().expect("connect");
    conn.pragma_update("journal_mode", "WAL")
        .await
        .expect("wal");
    conn.pragma_update("busy_timeout", 5000)
        .await
        .expect("busy_timeout");
    conn
}

async fn run_raw(label: &str, path: &str) {
    let db = Arc::new(
        turso::Builder::new_local(path)
            .build()
            .await
            .expect("build"),
    );
    let setup = raw_connect(&db).await;
    setup.execute_batch(CREATE).await.expect("create");
    let mut handles = Vec::with_capacity(TASKS);
    for t in 0..TASKS {
        let db = db.clone();
        handles.push(tokio::spawn(async move {
            let conn = raw_connect(&db).await;
            write_loop!(conn, t)
        }));
    }
    finish(label, handles, integrity_raw(&setup)).await;
}

#[ignore = "nondeterministic Turso corruption reproducer; run deliberately"]
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn pooled_memory() {
    run_pooled("pooled_memory", DbConfig::Memory).await;
}

#[ignore = "nondeterministic Turso corruption reproducer; run deliberately"]
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn pooled_file() {
    let f = TempDb::new();
    run_pooled("pooled_file", DbConfig::Local { path: f.0.clone() }).await;
}

#[ignore = "nondeterministic Turso corruption reproducer; run deliberately"]
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn shared_file() {
    let f = TempDb::new();
    run_shared("shared_file", DbConfig::Local { path: f.0.clone() }).await;
}

#[ignore = "nondeterministic Turso corruption reproducer; run deliberately"]
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn raw_turso_memory() {
    run_raw("raw_turso_memory", ":memory:").await;
}

#[ignore = "nondeterministic Turso corruption reproducer; run deliberately"]
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn raw_turso_file() {
    let f = TempDb::new();
    run_raw("raw_turso_file", &f.0).await;
}
