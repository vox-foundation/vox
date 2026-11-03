//! Point insert latency: Turso (shared vs pooled connection) vs SQLite (rusqlite),
//! against a table shaped like the `agent_events` hot-write path (see
//! `crates/vox-db/src/facade/writer_raw.rs::insert_agent_event_raw`).
//!
//! Run: `cargo bench -p vox-db --bench concurrency_bench`
//! HTML report: `target/criterion/report/index.html`

use criterion::{Criterion, criterion_group, criterion_main};
use vox_db::{DbConfig, VoxDb, VoxDbPool};

const CREATE_PROBE: &str = "CREATE TABLE IF NOT EXISTS bench_probe (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    cli_version TEXT NOT NULL
)";

fn tokio_rt() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().expect("tokio runtime")
}

fn bench_turso_shared_insert(c: &mut Criterion) {
    let rt = tokio_rt();
    let db = rt.block_on(async {
        let db = VoxDb::connect(DbConfig::Memory).await.expect("connect");
        db.connection()
            .execute_batch(CREATE_PROBE)
            .await
            .expect("create table");
        db
    });

    c.bench_function("turso_shared_connection_insert", |b| {
        b.iter(|| {
            rt.block_on(async {
                db.connection()
                    .execute(
                        "INSERT INTO bench_probe (agent_id, event_type, payload_json, cli_version)
                         VALUES (?1, ?2, ?3, ?4)",
                        turso::params!["agent-x", "tool_call", "{}", "1.0.0"],
                    )
                    .await
                    .expect("insert")
            })
        })
    });
}

fn bench_turso_pooled_insert(c: &mut Criterion) {
    let rt = tokio_rt();
    let pool = rt.block_on(async {
        let pool = VoxDbPool::new(DbConfig::Memory).await.expect("pool");
        let db = pool.get().await.expect("get");
        db.connection()
            .execute_batch(CREATE_PROBE)
            .await
            .expect("create table");
        pool
    });

    c.bench_function("turso_pooled_connection_insert", |b| {
        b.iter(|| {
            rt.block_on(async {
                let db = pool.get().await.expect("get");
                db.connection()
                    .execute(
                        "INSERT INTO bench_probe (agent_id, event_type, payload_json, cli_version)
                         VALUES (?1, ?2, ?3, ?4)",
                        turso::params!["agent-x", "tool_call", "{}", "1.0.0"],
                    )
                    .await
                    .expect("insert")
            })
        })
    });
}

fn bench_sqlite_insert(c: &mut Criterion) {
    let conn = rusqlite::Connection::open_in_memory().expect("open");
    // WAL/busy_timeout are no-ops on `:memory:` (kept to mirror the pragmas a real
    // on-disk connection would carry; this benchmark measures single-connection
    // point-write overhead, not WAL contention).
    let _ = conn.pragma_update(None, "journal_mode", "WAL");
    conn.pragma_update(None, "busy_timeout", 5000i64)
        .expect("busy_timeout");
    conn.execute(
        "CREATE TABLE bench_probe (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            agent_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            cli_version TEXT NOT NULL
        )",
        [],
    )
    .expect("create table");

    c.bench_function("rusqlite_insert", |b| {
        b.iter(|| {
            conn.execute(
                "INSERT INTO bench_probe (agent_id, event_type, payload_json, cli_version)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params!["agent-x", "tool_call", "{}", "1.0.0"],
            )
            .expect("insert")
        })
    });
}

criterion_group!(
    benches,
    bench_turso_shared_insert,
    bench_turso_pooled_insert,
    bench_sqlite_insert
);
criterion_main!(benches);
