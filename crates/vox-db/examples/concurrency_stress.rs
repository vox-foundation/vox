//! Concurrent write throughput + correctness stress harness comparing:
//!   - `shared`        : one `VoxDb` (one `turso::Connection`) cloned across all
//!     tasks, guarded by `GuardedConnection`'s mutex — mirrors
//!     `vox-gui`'s current `GuiDbPool` production pattern.
//!   - `pooled`        : `VoxDbPool` — one independent `turso::Connection` per task.
//!   - `pooled-mvcc`   : same as `pooled`, with `VOX_DB_MVCC=1` (Turso's beta
//!     row-level MVCC concurrent-writes journal mode).
//!   - `sqlite`        : `rusqlite`, one OS thread + one connection per task, WAL +
//!     busy_timeout=5000, all against the same on-disk file — the
//!     idiomatic SQLite pooling pattern. Genuinely contends SQLite's file lock.
//!   - `sqlite-shared` : `rusqlite`, one connection shared via `Arc<Mutex<_>>`
//!     across all worker threads — the like-for-like counterpart to `shared`
//!     (single connection, serialized in-process, no file-lock contention on
//!     either side).
//!
//! Cross-engine fairness: every mode's timed window starts *after* its one-time
//! schema/connection setup (Turso pays a full baseline migration, SQLite a single
//! `CREATE TABLE`) and stops as soon as the last writer finishes, and the rusqlite
//! connections set `synchronous=NORMAL` to match what `VoxDb::apply_pragmas`
//! applies to every Turso connection.
//!
//! Usage: `cargo run -p vox-db --example concurrency_stress -- \
//!             --mode pooled --tasks 64 --writes-per-task 25 [--file PATH]`
//!
//! Without `--file`, Turso modes use an in-memory database (one shared
//! `VoxDbPool`/`VoxDb`, so all tasks see the same data). `sqlite`/`sqlite-shared`
//! modes always use an on-disk file (`--file` or a temp path) since SQLite has no
//! equivalent to Turso's single-process shared in-memory `Database` handle across
//! independent connections. Pass `--file PATH` to force Turso modes onto a real
//! on-disk file too, matching production `.vox/store.db` I/O.
//!
//! `--file` is never deleted: every mode resets only its own `stress_probe` table
//! (`DROP TABLE IF EXISTS` + `CREATE TABLE`) at the start of each run, so it's
//! safe to point `--file` at a real `.vox/store.db`-shaped path and re-run
//! repeatedly without losing data or leaving stale rows from a prior run.

use std::time::Instant;

use vox_db::{DbConfig, VoxDb, VoxDbPool};

struct Args {
    mode: String,
    tasks: usize,
    writes_per_task: usize,
    file: Option<String>,
}

fn parse_args() -> Args {
    let mut mode = "pooled".to_string();
    let mut tasks = 64usize;
    let mut writes_per_task = 25usize;
    let mut file = None;
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--mode" => mode = it.next().expect("--mode needs a value"),
            "--tasks" => {
                tasks = it
                    .next()
                    .expect("--tasks needs a value")
                    .parse()
                    .expect("tasks must be an integer")
            }
            "--writes-per-task" => {
                writes_per_task = it
                    .next()
                    .expect("--writes-per-task needs a value")
                    .parse()
                    .expect("writes-per-task must be an integer")
            }
            "--file" => file = Some(it.next().expect("--file needs a value")),
            other => {
                panic!("unknown arg: {other} (expected --mode, --tasks, --writes-per-task, --file)")
            }
        }
    }
    Args {
        mode,
        tasks,
        writes_per_task,
        file,
    }
}

#[derive(Default)]
struct Stats {
    ok: usize,
    misuse_errors: usize,
    busy_errors: usize,
    other_errors: usize,
    rowid_mismatches: usize,
}

impl Stats {
    fn merge(&mut self, other: Stats) {
        self.ok += other.ok;
        self.misuse_errors += other.misuse_errors;
        self.busy_errors += other.busy_errors;
        self.other_errors += other.other_errors;
        self.rowid_mismatches += other.rowid_mismatches;
    }
}

fn classify_error(e: &str, stats: &mut Stats) {
    if e.contains("concurrent use forbidden") {
        stats.misuse_errors += 1;
    } else if e.contains("SQLITE_BUSY") || e.contains("database is locked") || e.contains("Busy") {
        stats.busy_errors += 1;
    } else {
        stats.other_errors += 1;
    }
}

/// Drops and recreates `stress_probe` so every run starts from a clean table
/// without ever touching (or deleting) the file it lives in. Works against
/// both Turso's `execute_batch` and rusqlite's `execute_batch`.
const RESET_PROBE: &str = "DROP TABLE IF EXISTS stress_probe;
CREATE TABLE stress_probe (
    id INTEGER PRIMARY KEY,
    marker TEXT NOT NULL
)";

#[tokio::main]
async fn main() {
    let args = parse_args();

    let (stats, expected_rows, actual_rows, elapsed, integrity_check) = match args.mode.as_str() {
        "shared" => run_turso_shared(&args).await,
        "pooled" => run_turso_pooled(&args, false).await,
        "pooled-mvcc" => run_turso_pooled(&args, true).await,
        "sqlite" => run_sqlite(&args),
        "sqlite-shared" => run_sqlite_shared(&args),
        other => panic!(
            "unknown --mode {other} (expected shared|pooled|pooled-mvcc|sqlite|sqlite-shared)"
        ),
    };

    // Print every stat before asserting on integrity below — if the assert
    // panics, this run's diagnostics are still on stdout instead of lost.
    println!("mode              = {}", args.mode);
    println!("tasks             = {}", args.tasks);
    println!("writes_per_task   = {}", args.writes_per_task);
    println!("elapsed           = {elapsed:?}");
    let elapsed_secs = elapsed.as_secs_f64().max(f64::MIN_POSITIVE);
    println!(
        "throughput        = {:.0} writes/sec",
        stats.ok as f64 / elapsed_secs
    );
    println!("expected_rows     = {expected_rows}");
    println!("actual_rows       = {actual_rows}");
    println!("row_count_ok      = {}", expected_rows == actual_rows);
    println!("ok_writes         = {}", stats.ok);
    println!("misuse_errors     = {}", stats.misuse_errors);
    println!("busy_errors       = {}", stats.busy_errors);
    println!("other_errors      = {}", stats.other_errors);
    println!("rowid_mismatches  = {}", stats.rowid_mismatches);
    println!("integrity_check   = {integrity_check}");

    assert_eq!(
        integrity_check, "ok",
        "PRAGMA integrity_check must report ok after the stress run"
    );
}

async fn verify_and_count(db: &VoxDb, marker: &str, stats: &mut Stats) {
    let id = db.connection().last_insert_rowid();
    match db
        .connection()
        .query(
            "SELECT marker FROM stress_probe WHERE id = ?1",
            turso::params![id],
        )
        .await
    {
        Ok(mut rows) => match rows.next().await {
            Ok(Some(row)) => {
                let actual: String = row.get(0).unwrap_or_default();
                if actual != marker {
                    stats.rowid_mismatches += 1;
                }
            }
            Ok(None) => classify_error("no row returned for last_insert_rowid", stats),
            Err(e) => classify_error(&e.to_string(), stats),
        },
        Err(e) => classify_error(&e.to_string(), stats),
    }
}

async fn run_turso_shared(args: &Args) -> (Stats, usize, usize, std::time::Duration, String) {
    let config = match &args.file {
        Some(path) => DbConfig::Local { path: path.clone() },
        None => DbConfig::Memory,
    };
    let db = VoxDb::connect(config).await.expect("connect");
    db.connection()
        .execute_batch(RESET_PROBE)
        .await
        .expect("reset table");
    let db = std::sync::Arc::new(db);

    // Timed window starts only after one-time schema/connection setup: the
    // Turso modes pay `VoxDb`'s full baseline migration here, `sqlite` pays a
    // single `CREATE TABLE`, and including that asymmetric fixed cost would
    // make the reported throughput a function of setup rather than of writes.
    let start = Instant::now();
    let mut handles = Vec::with_capacity(args.tasks);
    for t in 0..args.tasks {
        let db = db.clone();
        let writes = args.writes_per_task;
        handles.push(tokio::spawn(async move {
            let mut stats = Stats::default();
            for w in 0..writes {
                let marker = format!("shared-{t}-{w}");
                match db
                    .connection()
                    .execute(
                        "INSERT INTO stress_probe (marker) VALUES (?1)",
                        turso::params![marker.clone()],
                    )
                    .await
                {
                    Ok(_) => {
                        stats.ok += 1;
                        verify_and_count(&db, &marker, &mut stats).await;
                    }
                    Err(e) => classify_error(&e.to_string(), &mut stats),
                }
            }
            stats
        }));
    }

    let mut total = Stats::default();
    for h in handles {
        total.merge(h.await.expect("task panicked"));
    }
    let elapsed = start.elapsed();

    let actual_rows = count_rows(&db).await;
    let integrity = check_integrity(&db).await;
    (
        total,
        args.tasks * args.writes_per_task,
        actual_rows,
        elapsed,
        integrity,
    )
}

async fn run_turso_pooled(
    args: &Args,
    mvcc: bool,
) -> (Stats, usize, usize, std::time::Duration, String) {
    if mvcc {
        // SAFETY: nothing else reads or writes VOX_DB_MVCC concurrently — this
        // runs before any connection opens in this process, and each invocation
        // handles exactly one --mode.
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var("VOX_DB_MVCC", "1")
        };
    }
    let config = match &args.file {
        Some(path) => DbConfig::Local { path: path.clone() },
        None => DbConfig::Memory,
    };
    let pool = match VoxDbPool::new(config).await {
        Ok(pool) => pool,
        Err(e) if mvcc => {
            let msg = e.to_string();
            // Only the known AUTOINCREMENT/MVCC incompatibility gets a graceful,
            // zero-exit report. Anything else (bad --file path, permissions,
            // future schema change) falls through to the same loud panic the
            // non-mvcc path uses below.
            if msg.contains("AUTOINCREMENT") || msg.contains("MVCC") {
                println!("mode              = pooled-mvcc (setup failed)");
                println!("setup_error       = {msg}");
                println!(
                    "note              = Turso's MVCC journal mode rejects AUTOINCREMENT; \
                     vox-db's baseline schema (run automatically by VoxDbPool::new()'s \
                     migration step) uses AUTOINCREMENT in multiple tables, so VOX_DB_MVCC \
                     cannot currently be enabled against the production schema at all. This \
                     is a real compatibility finding, not a harness bug."
                );
                std::process::exit(0);
            }
            panic!("pool init: {msg}");
        }
        Err(e) => panic!("pool init: {e}"),
    };
    {
        let db = pool.get().await.expect("get conn for schema");
        db.connection()
            .execute_batch(RESET_PROBE)
            .await
            .expect("reset table");
    }

    // See `run_turso_shared`: setup (pool init + baseline migration + probe
    // table) is deliberately outside the timed window.
    let start = Instant::now();
    let mut handles = Vec::with_capacity(args.tasks);
    for t in 0..args.tasks {
        let pool = pool.clone();
        let writes = args.writes_per_task;
        handles.push(tokio::spawn(async move {
            let mut stats = Stats::default();
            let db = match pool.get().await {
                Ok(db) => db,
                Err(e) => {
                    classify_error(&e.to_string(), &mut stats);
                    return stats;
                }
            };
            for w in 0..writes {
                let marker = format!("pooled-{t}-{w}");
                match db
                    .connection()
                    .execute(
                        "INSERT INTO stress_probe (marker) VALUES (?1)",
                        turso::params![marker.clone()],
                    )
                    .await
                {
                    Ok(_) => {
                        stats.ok += 1;
                        verify_and_count(&db, &marker, &mut stats).await;
                    }
                    Err(e) => classify_error(&e.to_string(), &mut stats),
                }
            }
            stats
        }));
    }

    let mut total = Stats::default();
    for h in handles {
        total.merge(h.await.expect("task panicked"));
    }
    let elapsed = start.elapsed();

    let db = pool.get().await.expect("get conn for count");
    let actual_rows = count_rows(&db).await;
    let integrity = check_integrity(&db).await;
    (
        total,
        args.tasks * args.writes_per_task,
        actual_rows,
        elapsed,
        integrity,
    )
}

async fn count_rows(db: &VoxDb) -> usize {
    let mut rows = db
        .connection()
        .query("SELECT COUNT(*) FROM stress_probe", ())
        .await
        .expect("count query");
    let row = rows.next().await.expect("count row").expect("row present");
    row.get::<i64>(0).expect("count value") as usize
}

/// Returns the `PRAGMA integrity_check` result without asserting on it — the
/// caller prints every other stat first, then asserts, so a corruption finding
/// doesn't panic the process before its diagnostics ever reach stdout.
async fn check_integrity(db: &VoxDb) -> String {
    let mut rows = db
        .connection()
        .query("PRAGMA integrity_check", ())
        .await
        .expect("integrity_check query");
    let row = rows
        .next()
        .await
        .expect("integrity row")
        .expect("row present");
    row.get(0).unwrap_or_default()
}

fn run_sqlite(args: &Args) -> (Stats, usize, usize, std::time::Duration, String) {
    let path = args.file.clone().unwrap_or_else(|| {
        std::env::temp_dir()
            .join(format!("vox-sqlite-stress-{}.db", std::process::id()))
            .to_string_lossy()
            .into_owned()
    });
    // Never delete a user-supplied (or temp) file: a `--file` pointed at a real
    // `.vox/store.db`-shaped path must survive this run. Only the probe table
    // itself is reset, below.

    {
        let conn = rusqlite::Connection::open(&path).expect("open");
        conn.pragma_update(None, "journal_mode", "WAL")
            .expect("wal");
        conn.pragma_update(None, "busy_timeout", 5000i64)
            .expect("busy_timeout");
        // Durability parity with Turso: `VoxDb::apply_pragmas` sets
        // `synchronous=NORMAL` on every Turso connection, while rusqlite would
        // otherwise run at SQLite's compiled default of FULL (fsync per commit).
        // Without this the cross-engine comparison measures fsync policy, not
        // engine concurrency behavior.
        conn.pragma_update(None, "synchronous", "NORMAL")
            .expect("synchronous");
        conn.execute_batch(RESET_PROBE).expect("reset table");
    }

    // See `run_turso_shared`: setup is deliberately outside the timed window.
    let start = Instant::now();
    let mut handles = Vec::with_capacity(args.tasks);
    for t in 0..args.tasks {
        let path = path.clone();
        let writes = args.writes_per_task;
        handles.push(std::thread::spawn(move || {
            let conn = rusqlite::Connection::open(&path).expect("open");
            conn.pragma_update(None, "busy_timeout", 5000i64)
                .expect("busy_timeout");
            conn.pragma_update(None, "synchronous", "NORMAL")
                .expect("synchronous");
            let mut stats = Stats::default();
            for w in 0..writes {
                let marker = format!("sqlite-{t}-{w}");
                match conn.execute(
                    "INSERT INTO stress_probe (marker) VALUES (?1)",
                    rusqlite::params![marker.clone()],
                ) {
                    Ok(_) => {
                        stats.ok += 1;
                        let id = conn.last_insert_rowid();
                        let actual: Result<String, _> = conn.query_row(
                            "SELECT marker FROM stress_probe WHERE id = ?1",
                            rusqlite::params![id],
                            |r| r.get(0),
                        );
                        match actual {
                            Ok(actual) if actual == marker => {}
                            Ok(_) => stats.rowid_mismatches += 1,
                            Err(e) => classify_error(&e.to_string(), &mut stats),
                        }
                    }
                    Err(e) => classify_error(&e.to_string(), &mut stats),
                }
            }
            stats
        }));
    }

    let mut total = Stats::default();
    for h in handles {
        total.merge(h.join().expect("thread panicked"));
    }
    let elapsed = start.elapsed();

    let conn = rusqlite::Connection::open(&path).expect("open for count");
    let actual_rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM stress_probe", [], |r| r.get(0))
        .expect("count");
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .expect("integrity_check");

    (
        total,
        args.tasks * args.writes_per_task,
        actual_rows as usize,
        elapsed,
        integrity,
    )
}

fn run_sqlite_shared(args: &Args) -> (Stats, usize, usize, std::time::Duration, String) {
    let path = args.file.clone().unwrap_or_else(|| {
        std::env::temp_dir()
            .join(format!(
                "vox-sqlite-shared-stress-{}.db",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned()
    });
    // Never delete a user-supplied (or temp) file — see `run_sqlite`.
    let conn = rusqlite::Connection::open(&path).expect("open");
    conn.pragma_update(None, "journal_mode", "WAL")
        .expect("wal");
    conn.pragma_update(None, "busy_timeout", 5000i64)
        .expect("busy_timeout");
    conn.pragma_update(None, "synchronous", "NORMAL")
        .expect("synchronous");
    conn.execute_batch(RESET_PROBE).expect("reset table");
    let conn = std::sync::Arc::new(std::sync::Mutex::new(conn));

    // See `run_turso_shared`: setup is deliberately outside the timed window.
    let start = Instant::now();
    let mut handles = Vec::with_capacity(args.tasks);
    for t in 0..args.tasks {
        let conn = conn.clone();
        let writes = args.writes_per_task;
        handles.push(std::thread::spawn(move || {
            let mut stats = Stats::default();
            for w in 0..writes {
                let marker = format!("sqlite-shared-{t}-{w}");
                let guard = conn.lock().expect("mutex poisoned");
                match guard.execute(
                    "INSERT INTO stress_probe (marker) VALUES (?1)",
                    rusqlite::params![marker.clone()],
                ) {
                    Ok(_) => {
                        stats.ok += 1;
                        let id = guard.last_insert_rowid();
                        let actual: Result<String, _> = guard.query_row(
                            "SELECT marker FROM stress_probe WHERE id = ?1",
                            rusqlite::params![id],
                            |r| r.get(0),
                        );
                        match actual {
                            Ok(actual) if actual == marker => {}
                            Ok(_) => stats.rowid_mismatches += 1,
                            Err(e) => classify_error(&e.to_string(), &mut stats),
                        }
                    }
                    Err(e) => classify_error(&e.to_string(), &mut stats),
                }
            }
            stats
        }));
    }

    let mut total = Stats::default();
    for h in handles {
        total.merge(h.join().expect("thread panicked"));
    }
    let elapsed = start.elapsed();

    // Drop the mutex/Arc wrapper so the final count/integrity queries run
    // against a plain `Connection`, not fighting a (by-then-idle) lock.
    let conn = std::sync::Arc::try_unwrap(conn)
        .unwrap_or_else(|_| panic!("connection still shared after all workers joined"))
        .into_inner()
        .expect("mutex poisoned");
    let actual_rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM stress_probe", [], |r| r.get(0))
        .expect("count");
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .expect("integrity_check");

    (
        total,
        args.tasks * args.writes_per_task,
        actual_rows as usize,
        elapsed,
        integrity,
    )
}
