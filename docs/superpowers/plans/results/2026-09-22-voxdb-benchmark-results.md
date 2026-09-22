# VoxDB Turso/SQLite Benchmark Results — 2026-09-22

Machine: Darwin Bertrands-MacBook-Pro.local 25.6.0 Darwin Kernel Version 25.6.0: Fri Jul 31 19:19:08 PDT 2026; root:xnu-12377.161.14~5/RELEASE_ARM64_T6050 arm64
Rust toolchain: rustc 1.98.1 (48a229cea 2026-09-01)

## Point-insert latency (`cargo bench -p vox-db --bench concurrency_bench`)

| Benchmark | Mean time |
|---|---|
| `turso_shared_connection_insert` | 11.359 µs |
| `turso_pooled_connection_insert` | 30.821 µs |
| `rusqlite_insert` | 1.6965 µs |

*Table values are Criterion's reported mean across 100 samples.*

**Note on pooled vs. shared latency gap:** `turso_pooled_connection_insert` (30.821 µs) measures connection acquisition (`VoxDbPool::get()` invoking `connect()` + `apply_pragmas()`) plus the insert operation on every iteration, whereas `turso_shared_connection_insert` (11.359 µs) times only the insert operation on an already-open connection. The ~2.7× gap reflects connection-acquisition overhead, not a difference in insert speed. Task 5's concurrent-write throughput harness acquires a pooled connection once per task and reuses it across multiple writes, so its numbers do not pay this per-operation cost and are not directly comparable to this latency table.

HTML report: `target/criterion/report/index.html`

## Concurrent write stress (`cargo run -p vox-db --example concurrency_stress`)

20 writes/task, on-disk file, `busy_timeout=5000` (Turso and SQLite both).

Re-measured after correcting for two confounds a final review identified: SQLite
connections now set `synchronous=NORMAL` (matching Turso's default) instead of
the compiled default `FULL`, and each mode's one-time schema-setup cost (Turso's
~219-table baseline migration vs. SQLite's single `CREATE TABLE`) is excluded
from the timed window. The numbers below are **not** comparable to any earlier
version of this table; the earlier numbers measured fsync policy and setup cost
as much as write throughput.

| mode | tasks | elapsed | throughput (writes/sec) | misuse_errors | busy_errors | rowid_mismatches | row_count_ok | integrity_check |
|---|---|---|---|---|---|---|---|---|
| shared | 32 | 15.83 ms | 40430 | 0 | 0 | 0 | true | ok |
| shared | 128 | 65.47 ms | 39104 | 0 | 0 | 0 | true | ok |
| pooled | 32 | 55.87 ms | 11456 | 0 | 0 | 0 | true | ok |
| pooled | 128 | 432.87 ms | 5914 | 0 | 0 | 0 | true | ok |
| pooled-mvcc | 32 | N/A (setup incompatible) | N/A | N/A | N/A | N/A | N/A | N/A |
| pooled-mvcc | 128 | N/A (setup incompatible) | N/A | N/A | N/A | N/A | N/A | N/A |
| sqlite | 32 | 130.30 ms | 4912 | 0 | 0 | 0 | true | ok |
| sqlite | 128 | 679.11 ms | 3770 | 0 | 0 | 0 | true | ok |

`pooled-mvcc` was not re-run: it fails during schema setup, before any timer or
pragma is reached, so neither correction can affect it. Task 5's clean re-run of
that mode remains valid (see the note below).

### Run-to-run variance (read the table above with this in hand)

A single sample of this harness is not a stable number. Three further repetitions
of the same six commands, immediately after the run tabulated above:

| mode | tasks | run 1 (table above) | run 2 | run 3 | run 4 | median |
|---|---|---|---|---|---|---|
| shared | 32 | 40430 | 36060 | 31356 | 26064 | 33708 |
| shared | 128 | 39104 | 37694 | 28401 | 43583 | 38399 |
| pooled | 32 | 11456 | 11558 | 11326 | 11437 | 11447 |
| pooled | 128 | 5914 | 10991 | 14054 | 11093 | 11042 |
| sqlite | 32 | 4912 | 4139 | 4986 | 5206 | 4949 |
| sqlite | 128 | 3770 | 2594 | 4449 | 3777 | 3774 |

Spread is wide (`pooled`/128 ranges 5914–14054, a 2.4× swing; `sqlite`/128 ranges
2594–4449), so **any single-run percentage derived from this harness is false
precision.** What *is* stable is the ordering: `shared` > `pooled` > `sqlite` in
all four repetitions at both concurrency levels, with no overlap between the
three modes' ranges. Downstream claims should be stated from the medians and as
ranges, not from one run. `pooled`/32 is the one tightly-clustered cell
(11326–11558), which is consistent with its window being dominated by a fixed
per-task `pool.get()` cost rather than by contention.

**Effect of the `synchronous` correction alone:** SQLite's measured throughput
rose roughly 2.4–2.9× against the pre-fix numbers (1686 → 4949 median at 32
tasks; 1587 → 3774 at 128). It remains the slowest of the three working modes at
both levels, but the size of that gap was substantially an artifact of running
`rusqlite` at `synchronous=FULL` against Turso's `NORMAL`.

**Note on rowid_mismatches:** shared mode's `rowid_mismatches` was 0 at both 32 and 128 tasks. The documented `last_insert_rowid` race in `GuardedConnection` (see its doc comment in `crates/vox-db/src/lib.rs`) did not reproduce in this run. This is probabilistic/timing-dependent and does not disprove the race; it means this specific test dataset and environment did not independently demonstrate it.

A follow-up mutation experiment on `crates/vox-db/src/pool.rs`'s regression test explains *why* it never reproduces here. Forcing a genuinely shared connection (one `pool.get()` reused by all 100 tasks) still yields 0 mismatches, because the unguarded window between `execute()` and `last_insert_rowid()` is a few nanoseconds of synchronous code while a contended `tokio::Mutex` handoff takes microseconds — the interleaving effectively never lands. Inserting a `yield_now().await` into that window makes the shared-connection variant report **981/1000** mismatches, while the independent-per-task-connection variant still reports 0 under the identical widened window. The race is real and the detection logic works; the natural window is simply far too narrow to hit at this scale. Neither variant was committed.

**Note on pooled-mvcc:** Turso's MVCC journal mode rejects the AUTOINCREMENT keyword. VoxDB's baseline production schema (automatically applied by `VoxDbPool::new()`'s migration step) uses AUTOINCREMENT in multiple tables. The initial 128-task run appeared to fail with a different error (`MVCC logical log file exists...`), but this was due to Turso's MVCC mode creating a `.db-log` sidecar file that the cleanup command (`rm -f /tmp/vox-stress-test.db /tmp/vox-stress-test.db-wal /tmp/vox-stress-test.db-shm`) does not remove. A clean re-run with wildcard cleanup (`rm -f /tmp/vox-stress-test.db*`) confirmed that both 32-task and 128-task pooled-mvcc runs fail with the same `Parse error: AUTOINCREMENT is not supported in MVCC mode` error. This is a real and consistent compatibility finding: the production schema cannot currently run against MVCC-enabled Turso databases. The rows above are marked N/A rather than representing zero throughput or errors; there is no throughput data to report because the harness never completed its initialization phase.
