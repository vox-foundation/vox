# VoxDB Connection-Pooling & MVCC Benchmark Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prove out the cheaper alternative to a full Turso→SQLite engine swap — using `VoxDbPool`'s already-existing independent-connection design (and, separately, Turso's beta `VOX_DB_MVCC=1` mode) instead of `vox-gui`'s current single-shared-connection `GuiDbPool` pattern — with a correctness regression test, a latency micro-benchmark, and a concurrent-write throughput/safety stress harness that also exercises a real SQLite (`rusqlite`) baseline, then write a data-backed recommendation on whether either toggle should become the default.

**Architecture:** No production code path changes. This plan adds (1) one new regression test in `crates/vox-db/src/pool.rs` proving `VoxDbPool`-vended connections cannot race on `last_insert_rowid()` the way `vox-gui`'s shared-connection pattern is documented to; (2) a Criterion micro-benchmark comparing per-op latency across Turso-shared, Turso-pooled, and `rusqlite`; (3) a standalone stress-harness example that spawns many concurrent writers against Turso-shared, Turso-pooled, Turso-pooled+MVCC, and SQLite, measuring throughput, error classes, rowid correctness, and `PRAGMA integrity_check`; and (4) a written recommendation memo under `docs/src/architecture/`. Wiring the winning approach into `vox-gui`'s production `GuiDbPool` is explicitly out of scope — it is the follow-up this plan's recommendation gates.

**Tech Stack:** Rust, `tokio`, `turso` 0.6.1 (already a dependency), `criterion` 0.5 (already a workspace dependency, used elsewhere via `crates/vox-compiler/benches/`), `rusqlite` (new, dev-dependency only, `bundled` feature — compiles via `cc`, not `bindgen`/`clang-sys`, so it does not violate the build-toolchain invariant).

**Spec:** None — no separate spec document precedes this plan. The investigation's scope and the codebase facts it rests on (existing `VoxDbPool` in `crates/vox-db/src/pool.rs`, the `GuardedConnection` "Known residual gap" doc comment at `crates/vox-db/src/lib.rs:421-470`, `vox-gui`'s `GuiDbPool` sharing pattern at `crates/vox-gui/src/commands/gui_db_pool.rs`, and the existing `VOX_DB_MVCC` beta pragma at `crates/vox-db/src/store/open.rs:14-46`) were established directly against the code in the conversation that produced this plan.

<!-- AMENDED: #1 — pre-execution review found a pre-existing, non-archived doc covering the same defect with a conflicting recommendation; Task 6 must reconcile with it. -->
**Related prior work:** `docs/src/architecture/voxdb-turso-database-audit-and-benchmarks-research-2026.md` already documents the same `GuardedConnection`/`last_insert_rowid` defect at the same lines and recommends a full engine migration off Turso. This plan evaluates a narrower, cheaper alternative that document does not mention (`VoxDbPool`, staying on Turso) — see Task 6, which is now required to read and reconcile with it.

## Global Constraints

- Route every `cargo` invocation through the build broker: use plain `cargo` on `PATH`; never call `~/.cargo/bin/cargo` or `rustup run <toolchain> cargo` directly.
- Never run `cargo fmt --all`. Format only touched files: `cargo fmt -p vox-db` or `vox run scripts/fmt.vox`.
- `rusqlite` is added to `[dev-dependencies]` only — never to `[dependencies]`. It must not appear in the shipped `vox-db` library's dependency graph.
- No new dependency may add `cmake`, `nasm`, `Go`, `perl`, or `libclang` to the build. `rusqlite`'s `bundled` feature compiles SQLite via the `cc` crate (a plain C compiler), not `bindgen`, so it satisfies this.
- Any new `pub fn` added to `crates/vox-db/src/**` (excluding files under 30 non-blank lines) requires an adjacent `#[test]`/`#[tokio::test]` in the same file. This plan adds no new production `pub fn` — only a test module, a bench, and an example — so this rule is satisfied by construction; if a task deviates from that, add the test in the same step.
- Every new Markdown file under `docs/src/` must start with YAML frontmatter (`title`, `description`, `category`) written as part of creating the file, not appended later. Valid category here: `Architecture SSOTs`. Do not hand-add `last_updated`.
- `VOX_DB_MVCC` is an existing, already-wired BETA flag (`crates/vox-db/src/store/open.rs:28-33`, `crates/vox-config/src/config_registry.rs:1753`, `crates/vox-gui/src/config/generated_fields.rs:57`). This plan only measures it — do not rename, remove, or change its default.
- Never commit secrets. No `.env` changes are needed for this plan.
- Do not modify `crates/vox-gui/src/commands/gui_db_pool.rs` or any of its 6 call sites (`chat.rs`, `chat_turn.rs`, `harness_eval.rs`, `plan_panel.rs`, `research.rs`, `scientia.rs`). Adopting the winning approach in production GUI code is a separate, follow-up plan gated on this one's recommendation.

---

## File Structure

| File | Responsibility |
|---|---|
| `crates/vox-db/src/pool.rs` (modify, append `#[cfg(test)] mod tests`) | Regression test proving `VoxDbPool` connections never race on `last_insert_rowid()`. |
| `crates/vox-db/Cargo.toml` (modify) | Add `rusqlite` dev-dependency and register the `concurrency_bench` Criterion target. |
| `crates/vox-db/benches/concurrency_bench.rs` (create) | Point-latency micro-benchmark: Turso shared vs Turso pooled vs `rusqlite`, one op at a time, no concurrency. |
| `crates/vox-db/examples/concurrency_stress.rs` (create) | CLI harness: N concurrent writers × M writes each, across `shared` / `pooled` / `pooled-mvcc` / `sqlite` modes. Reports throughput, error counts by class, rowid-mismatch count, row-count durability, and `PRAGMA integrity_check`. |
| `docs/superpowers/plans/results/2026-09-22-voxdb-benchmark-results.md` (create, Tasks 4-5 output) | Raw captured output from the bench and stress-harness runs. |
| `docs/src/architecture/2026-09-22-voxdb-turso-pooling-and-mvcc-recommendation.md` (create, Task 6) | Frontmatter'd recommendation memo synthesizing the above into a default-on/default-off decision for pooling and for `VOX_DB_MVCC`. |

---

### Task 1: Prove `VoxDbPool` connections don't race on `last_insert_rowid()`

**Files:**
- Modify: `crates/vox-db/src/pool.rs`
- Test: same file (`#[cfg(all(test, feature = "local"))] mod tests` appended at end of file)

**Interfaces:**
- Consumes: `crate::VoxDbPool::new(config: DbConfig) -> Result<Self, StoreError>`, `VoxDbPool::get(&self) -> Result<VoxDb, StoreError>` (both already exist, unmodified), `VoxDb::connection(&self) -> &GuardedConnection` (existing), `GuardedConnection::{execute, query, last_insert_rowid}` (existing; `last_insert_rowid` reaches `turso::Connection` via `Deref`).
- Produces: nothing new consumed by later tasks — this is a standalone regression test.

- [ ] **Step 1: Write the test**

Append to `crates/vox-db/src/pool.rs`:

```rust
#[cfg(all(test, feature = "local"))]
mod tests {
    use super::*;
    use crate::DbConfig;

    /// Regression test for the "Known residual gap" documented on
    /// [`crate::GuardedConnection`] (`crates/vox-db/src/lib.rs:455-470`):
    /// `turso::Connection::last_insert_rowid()` is a synchronous, per-connection
    /// read that bypasses `ConcurrentGuard`, so two tasks sharing one *cloned*
    /// connection (exactly how `vox-gui`'s `GuiDbPool` shares one `Arc<VoxDb>`
    /// across all GUI commands) can race: task A's `execute()` + `last_insert_rowid()`
    /// pair can straddle task B's own `execute()` on the shared connection, and A
    /// silently reads back B's row id.
    ///
    /// `VoxDbPool::get()` hands each caller an independent `turso::Connection`
    /// (`db.connect()`, not a clone of one shared connection), so this race is
    /// structurally impossible: `last_insert_rowid()` can only ever reflect that
    /// connection's own last write. This test asserts that invariant holds under
    /// real concurrent load, verifying every task's returned id by reading the row
    /// back and comparing an embedded per-task marker.
    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn pooled_connections_never_race_on_last_insert_rowid() {
        let pool = VoxDbPool::new(DbConfig::Memory).await.expect("pool init");

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
```

- [ ] **Step 2: Run the test**

Run: `cargo test -p vox-db pooled_connections_never_race_on_last_insert_rowid -- --nocapture`

Expected: PASS. Unlike a typical red-green TDD cycle, this is not new production behavior — `VoxDbPool` already vends independent connections. This step's purpose is to lock in that already-correct invariant as a permanent CI regression guard, since no existing test in `lib.rs`'s `guarded_connection_tests` module (which only asserts no `Misuse` errors and no dropped rows) currently covers rowid correctness.

- [ ] **Step 3: Commit**

```bash
git add crates/vox-db/src/pool.rs
git commit -m "test(vox-db): regression-guard VoxDbPool against the documented rowid race"
```

---

### Task 2: Point-latency micro-benchmark (Turso shared vs pooled vs SQLite)

**Files:**
- Modify: `crates/vox-db/Cargo.toml`
- Create: `crates/vox-db/benches/concurrency_bench.rs`

**Interfaces:**
- Consumes: `VoxDb::connect`, `VoxDb::connection()`, `VoxDbPool::new`, `VoxDbPool::get`, `GuardedConnection::{execute, execute_batch}` (all existing/unmodified); `rusqlite::Connection::{open_in_memory, execute, pragma_update}` (new dev-dependency).
- Produces: `cargo bench -p vox-db --bench concurrency_bench` — HTML report at `target/criterion/report/index.html`, consumed by Task 4.

- [ ] **Step 1: Add the `rusqlite` dev-dependency and register the bench**

Edit `crates/vox-db/Cargo.toml`. In `[dev-dependencies]`, add:

```toml
rusqlite = { version = "0.32", default-features = false, features = ["bundled"] }
criterion = { workspace = true }
```

After the existing `[[test]]` block, add:

```toml
[[bench]]
name = "concurrency_bench"
harness = false
```

- [ ] **Step 2: Write the benchmark**

Create `crates/vox-db/benches/concurrency_bench.rs`:

```rust
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
```

- [ ] **Step 3: Verify it compiles and runs (fast smoke pass, not the full sampled run)**

Run: `cargo bench -p vox-db --bench concurrency_bench -- --test`

Expected: all three benchmark functions execute once each without panicking (criterion's `--test` mode skips full statistical sampling).

- [ ] **Step 4: Commit**

```bash
git add crates/vox-db/Cargo.toml crates/vox-db/benches/concurrency_bench.rs
git commit -m "bench(vox-db): add Turso shared/pooled vs SQLite point-insert latency benchmark"
```

---

### Task 3: Concurrent throughput + correctness + integrity stress harness

**Files:**
- Create: `crates/vox-db/examples/concurrency_stress.rs`

**Interfaces:**
- Consumes: `DbConfig::{Memory, Local}`, `VoxDb::connect`, `VoxDb::connection()`, `VoxDbPool::{new, get}`, `GuardedConnection::{execute, query, execute_batch, last_insert_rowid}` (all existing); `rusqlite::Connection` (dev-dependency, added in Task 2).
- Produces: a runnable CLI (`cargo run -p vox-db --example concurrency_stress -- --mode <shared|pooled|pooled-mvcc|sqlite> --tasks N --writes-per-task M [--file PATH]`), consumed by Task 5.

- [ ] **Step 1: Write the harness**

Create `crates/vox-db/examples/concurrency_stress.rs`:

```rust
//! Concurrent write throughput + correctness stress harness comparing:
//!   - `shared`      : one `VoxDb` (one `turso::Connection`) cloned across all
//!                     tasks, guarded by `GuardedConnection`'s mutex — mirrors
//!                     `vox-gui`'s current `GuiDbPool` production pattern.
//!   - `pooled`      : `VoxDbPool` — one independent `turso::Connection` per task.
//!   - `pooled-mvcc` : same as `pooled`, with `VOX_DB_MVCC=1` (Turso's beta
//!                     row-level MVCC concurrent-writes journal mode).
//!   - `sqlite`      : `rusqlite`, one OS thread + one connection per task, WAL +
//!                     busy_timeout=5000, all against the same on-disk file — the
//!                     idiomatic SQLite pooling pattern.
//!
//! Usage: `cargo run -p vox-db --example concurrency_stress -- \
//!             --mode pooled --tasks 64 --writes-per-task 25 [--file PATH]`
//!
//! Without `--file`, Turso modes use an in-memory database (one shared
//! `VoxDbPool`/`VoxDb`, so all tasks see the same data). `sqlite` mode always uses
//! an on-disk file (`--file` or a temp path) since SQLite has no equivalent to
//! Turso's single-process shared in-memory `Database` handle across independent
//! connections. Pass `--file PATH` to force Turso modes onto a real on-disk file
//! too, matching production `.vox/store.db` I/O.

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
            other => panic!(
                "unknown arg: {other} (expected --mode, --tasks, --writes-per-task, --file)"
            ),
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
    } else if e.contains("SQLITE_BUSY") || e.contains("database is locked") || e.contains("Busy")
    {
        stats.busy_errors += 1;
    } else {
        stats.other_errors += 1;
    }
}

const CREATE_PROBE: &str = "CREATE TABLE IF NOT EXISTS stress_probe (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    marker TEXT NOT NULL
)";

#[tokio::main]
async fn main() {
    let args = parse_args();
    let start = Instant::now();

    let (stats, expected_rows, actual_rows) = match args.mode.as_str() {
        "shared" => run_turso_shared(&args).await,
        "pooled" => run_turso_pooled(&args, false).await,
        "pooled-mvcc" => run_turso_pooled(&args, true).await,
        "sqlite" => run_sqlite(&args),
        other => panic!("unknown --mode {other} (expected shared|pooled|pooled-mvcc|sqlite)"),
    };
    let elapsed = start.elapsed();

    println!("mode              = {}", args.mode);
    println!("tasks             = {}", args.tasks);
    println!("writes_per_task   = {}", args.writes_per_task);
    println!("elapsed           = {elapsed:?}");
    println!(
        "throughput        = {:.0} writes/sec",
        expected_rows as f64 / elapsed.as_secs_f64()
    );
    println!("expected_rows     = {expected_rows}");
    println!("actual_rows       = {actual_rows}");
    println!("row_count_ok      = {}", expected_rows == actual_rows);
    println!("ok_writes         = {}", stats.ok);
    println!("misuse_errors     = {}", stats.misuse_errors);
    println!("busy_errors       = {}", stats.busy_errors);
    println!("other_errors      = {}", stats.other_errors);
    println!("rowid_mismatches  = {}", stats.rowid_mismatches);
}

async fn verify_and_count(
    db: &VoxDb,
    marker: &str,
    stats: &mut Stats,
) {
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
            _ => stats.rowid_mismatches += 1,
        },
        Err(_) => stats.rowid_mismatches += 1,
    }
}

async fn run_turso_shared(args: &Args) -> (Stats, usize, usize) {
    let config = match &args.file {
        Some(path) => DbConfig::Local { path: path.clone() },
        None => DbConfig::Memory,
    };
    let db = VoxDb::connect(config).await.expect("connect");
    db.connection()
        .execute_batch(CREATE_PROBE)
        .await
        .expect("create table");
    let db = std::sync::Arc::new(db);

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

    let actual_rows = count_rows(&db).await;
    check_integrity(&db).await;
    (total, args.tasks * args.writes_per_task, actual_rows)
}

async fn run_turso_pooled(args: &Args, mvcc: bool) -> (Stats, usize, usize) {
    if mvcc {
        // SAFETY: single-threaded at this point in `main`, before any connection opens.
        unsafe { std::env::set_var("VOX_DB_MVCC", "1") };
    }
    let config = match &args.file {
        Some(path) => DbConfig::Local { path: path.clone() },
        None => DbConfig::Memory,
    };
    let pool = VoxDbPool::new(config).await.expect("pool init");
    {
        let db = pool.get().await.expect("get conn for schema");
        db.connection()
            .execute_batch(CREATE_PROBE)
            .await
            .expect("create table");
    }

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

    let db = pool.get().await.expect("get conn for count");
    let actual_rows = count_rows(&db).await;
    check_integrity(&db).await;
    (total, args.tasks * args.writes_per_task, actual_rows)
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

async fn check_integrity(db: &VoxDb) {
    let mut rows = db
        .connection()
        .query("PRAGMA integrity_check", ())
        .await
        .expect("integrity_check query");
    let row = rows.next().await.expect("integrity row").expect("row present");
    let result: String = row.get(0).unwrap_or_default();
    println!("integrity_check   = {result}");
    assert_eq!(
        result, "ok",
        "PRAGMA integrity_check must report ok after the stress run"
    );
}

fn run_sqlite(args: &Args) -> (Stats, usize, usize) {
    let path = args.file.clone().unwrap_or_else(|| {
        std::env::temp_dir()
            .join(format!("vox-sqlite-stress-{}.db", std::process::id()))
            .to_string_lossy()
            .into_owned()
    });
    let _ = std::fs::remove_file(&path);

    {
        let conn = rusqlite::Connection::open(&path).expect("open");
        conn.pragma_update(None, "journal_mode", "WAL")
            .expect("wal");
        conn.pragma_update(None, "busy_timeout", 5000i64)
            .expect("busy_timeout");
        conn.execute(CREATE_PROBE, []).expect("create table");
    }

    let mut handles = Vec::with_capacity(args.tasks);
    for t in 0..args.tasks {
        let path = path.clone();
        let writes = args.writes_per_task;
        handles.push(std::thread::spawn(move || {
            let conn = rusqlite::Connection::open(&path).expect("open");
            conn.pragma_update(None, "busy_timeout", 5000i64)
                .expect("busy_timeout");
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
                            _ => stats.rowid_mismatches += 1,
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

    let conn = rusqlite::Connection::open(&path).expect("open for count");
    let actual_rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM stress_probe", [], |r| r.get(0))
        .expect("count");
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .expect("integrity_check");
    println!("integrity_check   = {integrity}");
    assert_eq!(
        integrity, "ok",
        "PRAGMA integrity_check must report ok after the stress run"
    );

    (total, args.tasks * args.writes_per_task, actual_rows as usize)
}
```

- [ ] **Step 2: Verify it compiles and runs for every mode**

Run each of the following and confirm exit code 0 and `row_count_ok = true`, `rowid_mismatches = 0`, `integrity_check = ok`:

```bash
cargo run -p vox-db --example concurrency_stress -- --mode shared --tasks 16 --writes-per-task 5
cargo run -p vox-db --example concurrency_stress -- --mode pooled --tasks 16 --writes-per-task 5
cargo run -p vox-db --example concurrency_stress -- --mode pooled-mvcc --tasks 16 --writes-per-task 5
cargo run -p vox-db --example concurrency_stress -- --mode sqlite --tasks 16 --writes-per-task 5
```

Note: `shared` mode may print `rowid_mismatches > 0` at this small scale — that is expected evidence of the documented race, not a bug in the harness (see Task 5, which runs this deliberately at higher concurrency to make the effect legible).

<!-- AMENDED: #2 — new step; pre-execution review found no task ran clippy, and AGENTS.md names this exact gap ("Pre-push clippy gap") as a recurring, repo-acknowledged source of shipped Rust defects. -->
- [ ] **Step 3: Run clippy across all new Rust code from Tasks 1-3**

Run: `cargo clippy -p vox-db --all-targets -- -D warnings`

This is one consolidated gate covering Task 1's new test module, Task 2's bench, and Task 3's example together (all three now exist on disk, so `--all-targets` reaches all of them in a single invocation). AGENTS.md's "Perennial Bug Patterns" section names the fast/default `vox ci pre-push` tier's lack of clippy as a recurring source of shipped defects — do not defer this to push time.

Expected: no warnings. Fix any that appear before committing.

- [ ] **Step 4: Commit**

```bash
git add crates/vox-db/examples/concurrency_stress.rs
git commit -m "test(vox-db): add concurrent write throughput/correctness/integrity stress harness"
```

---

### Task 4: Run the latency micro-benchmark and capture results

**Files:**
- Create: `docs/superpowers/plans/results/2026-09-22-voxdb-benchmark-results.md`

**Interfaces:**
- Consumes: the `concurrency_bench` target from Task 2.
- Produces: a results file consumed by Task 6's recommendation memo.

- [ ] **Step 1: Run the full benchmark**

Run: `cargo bench -p vox-db --bench concurrency_bench`

This takes several minutes (Criterion's default sampling). Note the three reported mean times (`turso_shared_connection_insert`, `turso_pooled_connection_insert`, `rusqlite_insert`) from the terminal summary, and the HTML report path (`target/criterion/report/index.html`).

- [ ] **Step 2: Write the results file (latency section)**

Create `docs/superpowers/plans/results/2026-09-22-voxdb-benchmark-results.md` with:

```markdown
# VoxDB Turso/SQLite Benchmark Results — 2026-09-22

Machine: <FILL: `uname -a` output>
Rust toolchain: <FILL: `rustc --version` output>

## Point-insert latency (`cargo bench -p vox-db --bench concurrency_bench`)

| Benchmark | Mean time |
|---|---|
| `turso_shared_connection_insert` | <FILL from criterion output> |
| `turso_pooled_connection_insert` | <FILL from criterion output> |
| `rusqlite_insert` | <FILL from criterion output> |

HTML report: `target/criterion/report/index.html`
```

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/plans/results/2026-09-22-voxdb-benchmark-results.md
git commit -m "docs: capture VoxDB point-insert latency benchmark results"
```

---

### Task 5: Run the concurrency stress harness across all four modes and capture results

**Files:**
- Modify: `docs/superpowers/plans/results/2026-09-22-voxdb-benchmark-results.md`

**Interfaces:**
- Consumes: the `concurrency_stress` example from Task 3.
- Produces: the completed results file consumed by Task 6.

- [ ] **Step 1: Run every mode at two concurrency levels, on-disk (not `:memory:`) to match production I/O**

```bash
for MODE in shared pooled pooled-mvcc sqlite; do
  for TASKS in 32 128; do
    rm -f /tmp/vox-stress-test.db /tmp/vox-stress-test.db-wal /tmp/vox-stress-test.db-shm
    echo "=== mode=$MODE tasks=$TASKS ==="
    cargo run -q -p vox-db --example concurrency_stress -- \
      --mode "$MODE" --tasks "$TASKS" --writes-per-task 20 --file /tmp/vox-stress-test.db
  done
done
```

Capture the full stdout of each of the 8 runs (mode × concurrency).

- [ ] **Step 2: Append the throughput/correctness/safety section to the results file**

Append to `docs/superpowers/plans/results/2026-09-22-voxdb-benchmark-results.md`:

```markdown
## Concurrent write stress (`cargo run -p vox-db --example concurrency_stress`)

20 writes/task, on-disk file, `busy_timeout=5000` (Turso and SQLite both).

| mode | tasks | throughput (writes/sec) | misuse_errors | busy_errors | rowid_mismatches | row_count_ok | integrity_check |
|---|---|---|---|---|---|---|---|
| shared | 32 | <FILL> | <FILL> | <FILL> | <FILL> | <FILL> | <FILL> |
| shared | 128 | <FILL> | <FILL> | <FILL> | <FILL> | <FILL> | <FILL> |
| pooled | 32 | <FILL> | <FILL> | <FILL> | <FILL> | <FILL> | <FILL> |
| pooled | 128 | <FILL> | <FILL> | <FILL> | <FILL> | <FILL> | <FILL> |
| pooled-mvcc | 32 | <FILL> | <FILL> | <FILL> | <FILL> | <FILL> | <FILL> |
| pooled-mvcc | 128 | <FILL> | <FILL> | <FILL> | <FILL> | <FILL> | <FILL> |
| sqlite | 32 | <FILL> | <FILL> | <FILL> | <FILL> | <FILL> | <FILL> |
| sqlite | 128 | <FILL> | <FILL> | <FILL> | <FILL> | <FILL> | <FILL> |
```

Fill every `<FILL>` cell directly from the 8 captured stdout blocks in Step 1 — do not estimate or round from memory.

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/plans/results/2026-09-22-voxdb-benchmark-results.md
git commit -m "docs: capture VoxDB concurrent-write stress benchmark results"
```

---

### Task 6: Write the recommendation memo

**Files:**
- Create: `docs/src/architecture/2026-09-22-voxdb-turso-pooling-and-mvcc-recommendation.md`

**Interfaces:**
- Consumes: `docs/superpowers/plans/results/2026-09-22-voxdb-benchmark-results.md` (Tasks 4-5).
- Produces: a decision other engineers and future plans read before touching `vox-gui`'s `GuiDbPool` or `VOX_DB_MVCC`'s default.

<!-- AMENDED: #1 — new step; a pre-existing doc covers the same defect with a conflicting recommendation and must be reconciled, not silently contradicted. -->
- [ ] **Step 1: Read the prior full-migration audit before writing the memo**

Read `docs/src/architecture/voxdb-turso-database-audit-and-benchmarks-research-2026.md` in full. It documents the same `GuardedConnection`/`last_insert_rowid` defect at the same `crates/vox-db/src/lib.rs` lines this plan cites, and recommends a full engine migration off Turso (`sqlx::sqlite::SqlitePool`, Turso demoted behind `feature = "cloud-sync"`). It does not mention `VoxDbPool` or `GuiDbPool` anywhere. Note where this plan's measured results agree, disagree, or are complementary with that doc's recommendation — you will state this in Step 2's memo.

- [ ] **Step 2: Write the memo**

Create `docs/src/architecture/2026-09-22-voxdb-turso-pooling-and-mvcc-recommendation.md`:

```markdown
---
title: "VoxDB pooling & MVCC benchmark recommendation"
description: "Measured throughput, correctness, and safety comparison of VoxDb's shared-connection pattern vs VoxDbPool vs VoxDbPool+MVCC vs SQLite, with a default-on/default-off recommendation."
category: "Architecture SSOTs"
---

# VoxDB pooling & MVCC benchmark recommendation

## Summary

<FILL: one paragraph — which of `shared` / `pooled` / `pooled-mvcc` won on throughput,
whether any mode produced rowid mismatches, misuse/busy errors, or a failed
`integrity_check`, and whether SQLite's numbers beat Turso's in this workload
shape. Pull directly from `docs/superpowers/plans/results/2026-09-22-voxdb-benchmark-results.md`.>

## Results

<FILL: paste both tables from the results file verbatim.>

## Decision: real per-task pooling (`VoxDbPool` vs `GuiDbPool`'s shared connection)

Apply this rubric to the captured numbers:

- If `pooled` throughput ≥ `shared` throughput **and** `pooled` has zero
  `rowid_mismatches` at every concurrency level **and** `pooled`'s
  `integrity_check` is `ok` at every concurrency level →
  **recommend**: file a follow-up plan to switch `vox-gui`'s `GuiDbPool` from
  one shared `Arc<VoxDb>` to `VoxDbPool`-vended per-command connections,
  default-on for all users. Note in that follow-up plan that
  `GuiDbPool::handle()` will need to become `async` (a pooled connection is
  acquired per call, not cloned from a stored `Arc`), which touches all 6
  current call sites (`chat.rs`, `chat_turn.rs`, `harness_eval.rs`,
  `plan_panel.rs`, `research.rs`, `scientia.rs`).
- If `pooled` shows nonzero `busy_errors` or a failed `integrity_check` at
  128 tasks → **do not** default-switch; `vox-gui`'s original
  SQLITE_BUSY/"os error 33" failure mode (documented in
  `crates/vox-gui/src/commands/gui_db_pool.rs`'s own doc comment, from
  before `GuiDbPool` was introduced) has reproduced. Recommend instead
  adopting `VoxDbPool::writer()`'s existing dedicated single-writer actor
  (`crates/vox-db/src/writer_actor.rs`) for writes, paired with `VoxDbPool`
  only for reads.
- State the actual outcome reached: <FILL>.

## Decision: `VOX_DB_MVCC=1` default

Apply this rubric:

- If `pooled-mvcc` throughput is not meaningfully higher than `pooled` at
  128 tasks in this workload → **do not** flip the default; Turso's own
  docs describe MVCC concurrent writes as beta/early-access with known gaps
  (no `CREATE INDEX` support under MVCC, memory-inefficient row versioning),
  and this measurement gives no throughput reason to accept that risk yet.
  Keep `VOX_DB_MVCC` opt-in.
- If `pooled-mvcc` throughput is meaningfully higher **and** shows zero
  `rowid_mismatches`/`busy_errors` and a passing `integrity_check` →
  recommend a follow-up spike specifically scoped to MVCC's documented
  gaps (schema migrations that `CREATE INDEX`, concurrent-conflict retry
  logic) before proposing default-on, since "beta" status is an upstream
  claim this benchmark does not by itself clear.
- State the actual outcome reached: <FILL>.

## Relationship to the prior full-migration audit

<!-- AMENDED: #1 — reconciles this memo with docs/src/architecture/voxdb-turso-database-audit-and-benchmarks-research-2026.md, found during pre-execution review to cover the same defect with a different recommendation. -->
`docs/src/architecture/voxdb-turso-database-audit-and-benchmarks-research-2026.md`
already documents the same `GuardedConnection`/`last_insert_rowid` defect at the
same lines and recommends a full engine migration to `sqlx::sqlite::SqlitePool`
(Turso demoted behind `feature = "cloud-sync"`). This memo evaluates a narrower,
cheaper alternative that prior audit does not mention — real per-task pooling via
the already-existing `VoxDbPool`, staying on Turso. <FILL: state whether this
memo's measured data supports pooling as sufficient on its own, supports the
prior audit's full-migration recommendation instead (e.g. because pooling's
numbers don't clear the bar), or treats the two as complementary — pooling as the
low-risk near-term fix, full migration kept open as a later option.>

## Out of scope / follow-ups

- Wiring either winning approach into `vox-gui`'s production `GuiDbPool` —
  separate plan, gated on the decisions above.
- A full Turso→SQLite engine migration was **not** evaluated here beyond
  this workload-shaped latency/throughput comparison; it remains a larger,
  separately-scoped decision (loses `DbConfig::EmbeddedReplica`/`Remote`,
  used today by `vox-orchestrator`'s mesh sync) — see the reconciliation
  section above for how this memo's data bears on that prior recommendation.
```

- [ ] **Step 3: Fill in every `<FILL>` from the Task 4/5 results file, per the rubrics above, and the reconciliation `<FILL>` from Step 1**

- [ ] **Step 4: Lint the new doc**

Run: `cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/architecture/2026-09-22-voxdb-turso-pooling-and-mvcc-recommendation.md`

Expected: lint passes (valid frontmatter, no broken links).

- [ ] **Step 5: Commit**

```bash
git add docs/src/architecture/2026-09-22-voxdb-turso-pooling-and-mvcc-recommendation.md
git commit -m "docs(architecture): recommend VoxDB pooling/MVCC defaults from benchmark data"
```

---

## Self-Review

**Spec coverage:**
- "Try that cheaper alternative" → Task 1 (correctness) + Tasks 2-3 (measurement) exercise `VoxDbPool`, the already-existing real-pool alternative to `GuiDbPool`'s shared connection.
- "Before/after measure to see if it works" → Task 2 (latency) + Task 5 (throughput), both compare `shared` (before) against `pooled`/`pooled-mvcc` (after).
- "Make sure it doesn't break our DB" → Task 1's rowid-correctness assertion, plus every mode in Task 3's harness runs `PRAGMA integrity_check` and a row-count durability check, captured for every run in Task 5.
- "Should it be on by default for all users" → Task 6's two explicit decision rubrics (pooling, and separately `VOX_DB_MVCC`), each conditioned on the measured data.
- "Test Turso vs SQLite with examples from this codebase" → all three benchmark/harness targets use a table shaped after the real `agent_events` hot-write path (`facade/writer_raw.rs::insert_agent_event_raw`), and every mode (including `sqlite`) is exercised identically.
- "Multiple relevant benchmarks, compare, advise" → Task 2 (point latency), Task 3/5 (concurrent throughput + correctness + safety) are two distinct, complementary benchmarks; Task 6 is the advisory synthesis.

**Placeholder scan:** All code blocks are complete, runnable Rust/TOML/bash — no `TODO`/`fill in details`/"similar to Task N" in any task's steps. The only `<FILL>` markers are in Tasks 4-6's *data* templates, which cannot be populated before the experiments in Tasks 1-5 run; each is paired with an exact source (a specific command's stdout) and, in Task 6, an explicit decision rubric — not a vague instruction.

**Type consistency:** `Stats` (Task 3) is defined once with fields `{ok, misuse_errors, busy_errors, other_errors, rowid_mismatches}` and a `merge` method; every call site in `run_turso_shared`, `run_turso_pooled`, and `run_sqlite` uses that exact shape. `verify_and_count(db: &VoxDb, marker: &str, stats: &mut Stats)` is defined once and called identically from both Turso paths. `DbConfig::Local { path }` and `DbConfig::Memory` match the real enum shape confirmed in `crates/vox-db/src/pool.rs`. `VoxDbPool::{new, get}` and `GuardedConnection::{execute, query, execute_batch, last_insert_rowid}` signatures match the real code read from `crates/vox-db/src/{pool.rs,lib.rs}`.

---

## Deferred Minor Issues

Found by the three-track pre-execution review (2026-09-22). None block execution; pick these up opportunistically during the task they reference, or skip them — none affect correctness of the plan's stated goal.

1. **Task 3 SAFETY comment is imprecisely worded.** `// SAFETY: single-threaded at this point in \`main\`, before any connection opens.` — the tokio multi-thread runtime's worker pool already exists at that point, so "single-threaded" overstates the guarantee. What actually makes the `unsafe { std::env::set_var(...) }` call safe is that nothing else reads/writes `VOX_DB_MVCC` concurrently at that point in the one `--mode` this process invocation runs (confirmed true by Track A/B). Reword to something like `// SAFETY: nothing else reads or writes VOX_DB_MVCC concurrently — this runs before any connection opens in this process, and each invocation handles exactly one --mode.` (Track A)
2. **`rusqlite` version pinned inline in `crates/vox-db/Cargo.toml` rather than via `[workspace.dependencies]`.** Not a rule violation (nothing mandates workspace-pinning a single-crate dev-only dependency), but inconsistent with how `turso` and other deps are pinned at the workspace level. Consider moving to `[workspace.dependencies]` if `rusqlite` is ever needed by a second crate. (Track A)
3. **Throughput printf in `concurrency_stress.rs`'s `main` can print `inf`/`NaN`** for degenerate `--tasks 0` / `--writes-per-task 0` values (`expected_rows as f64 / elapsed.as_secs_f64()`, with `elapsed` near zero). Never panics (float division doesn't), purely cosmetic in a benchmark CLI's own output. Fix if noticed during Task 3/5: `let secs = elapsed.as_secs_f64().max(f64::MIN_POSITIVE);`. (Track B)
4. **Task 3's dependency on Task 2, and Task 5's dependency on Task 4, are stated only via prose "Interfaces → Consumes" lines, not a dedicated "Depends on:" field.** File-path-based conflict detection alone catches the real collision (Task 4 creates / Task 5 modifies the same results file), but a scheduler that only reads Consumes/Produces prose could miss that Task 3 needs Task 2's `rusqlite` dev-dependency to compile, or that Task 5 needs Task 4's results file to already have its latency section. Both are already called out in the Execution Order section below, which is now the authoritative source for task sequencing. (Track C)
5. **`run_turso_shared` and `run_turso_pooled` in `concurrency_stress.rs` duplicate their per-task write-loop body** (~15 lines: format marker, execute, match on Ok/Err, call `verify_and_count`/`classify_error`) — the only real difference is how `db: &VoxDb` is obtained before the loop. Could collapse into one `async fn write_loop(db: &VoxDb, prefix: &str, task_id: usize, writes: usize) -> Stats` called from both. Estimated savings: ~15 lines net. Not done inline here since the two call sites obtain `db` differently enough (`Arc<VoxDb>` clone vs. `pool.get().await` per spawn) that forcing the abstraction now would cost review time disproportionate to a one-time benchmark tool; revisit if this harness grows a fifth mode. (Track C)

---

## Execution Order

Derived from the Shared-File Conflict Map (Track C) plus the compile-order dependencies Tracks A/B/C confirmed by reading the actual code.

**Sequential Constraints (cannot parallelize):**

- Task 2 → Task 3: Task 3's `crates/vox-db/examples/concurrency_stress.rs` depends on `crates/vox-db`'s `rusqlite` dev-dependency, added in Task 2's `Cargo.toml` edit. No shared file, but a real compile-order dependency.
- Task 3 → Task 4: Task 4 runs the `concurrency_bench` target, which only needs Task 2 — but Task 4 is listed after Task 3 in the plan and there is no reason to run it earlier; keep in order for simplicity of the single-operator execution flow described in Task 5.
- Task 3 → Task 5: Task 5 runs the `concurrency_stress` binary built in Task 3.
- Task 4 → Task 5: **Shared-file collision.** Both declare `docs/superpowers/plans/results/2026-09-22-voxdb-benchmark-results.md` (Task 4 Create, Task 5 Modify/append). Task 5 also needs Task 4's latency section already written, since Task 6 consumes both sections from one file.
- Tasks 4+5 → Task 6: Task 6 (memo) consumes the completed results file from both.
- Task 1 has no dependency on or from any other task and could run first, last, or in parallel with Tasks 2-3 if the executor supports it — it touches a different file (`pool.rs`) than every other task.

**Pre-Flight Checklist:**

- [ ] Worktree isolated via `superpowers:using-git-worktrees` (recommended — this plan adds a new dev-dependency and two new compilation targets to `vox-db`; isolate from any other concurrent work on that crate).
- [ ] Target git history confirmed: branch off `main` (per house rules, `main` is the default branch; do not commit directly to it).
- [ ] Next migration sequence: N/A — this plan never calls `VoxDb::migrate` or touches `crates/vox-db/src/schema/manifest.rs`/`BASELINE_VERSION` (confirmed by Track C).
- [ ] Local test database: N/A — no PGlite in this stack. Task 1's test and Task 2's bench use `DbConfig::Memory` (Turso in-memory); Task 3/5's stress harness uses a disposable `/tmp` file, recreated (`rm -f`) before every run (confirmed enforced by Track C, not just asserted).

**Recommended Task Sequence:**

Task 1 → Task 2 → Task 3 → Task 4 → Task 5 → Task 6

(Batch candidate: Task 1 can run in parallel with Tasks 2-3 if the executor supports true parallel task execution — it shares no file and no compile-order dependency with them. Tasks 2→3→4→5→6 must stay sequential per the constraints above.)

**SDD Ledger Pre-Population** (copy into `progress.md` if using `superpowers:subagent-driven-development`):

```
Conflict on docs/superpowers/plans/results/2026-09-22-voxdb-benchmark-results.md (Task 4 creates, Task 5 appends): sequential execution enforced — ruling: settled
Task 3 → Task 2 compile-order dependency (rusqlite dev-dependency): sequential execution enforced — ruling: settled
[Important #1 — Task 6 must reconcile with docs/src/architecture/voxdb-turso-database-audit-and-benchmarks-research-2026.md, which covers the same GuardedConnection/last_insert_rowid defect and recommends a full engine migration this plan does not evaluate]: fix instruction = read it in Task 6 Step 1, fill the "Relationship to the prior full-migration audit" section in Task 6 Step 2's memo template — ruling: settled, amended into plan
[Important #2 — no task ran cargo clippy, a repo-documented perennial gap]: fix instruction = run `cargo clippy -p vox-db --all-targets -- -D warnings` as Task 3 Step 3, before Task 3's commit — ruling: settled, amended into plan
[Track A/B/C: all API signatures, move/clone correctness, Send/'static bounds, SQL parameterization, and Rust type consistency verified against real source — no code-correctness defects found]: ruling: settled, no fix needed
```
