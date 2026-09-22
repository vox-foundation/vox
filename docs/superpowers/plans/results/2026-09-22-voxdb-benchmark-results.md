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

| mode | tasks | throughput (writes/sec) | misuse_errors | busy_errors | rowid_mismatches | row_count_ok | integrity_check |
|---|---|---|---|---|---|---|---|
| shared | 32 | 4095 | 0 | 0 | 0 | true | ok |
| shared | 128 | 12045 | 0 | 0 | 0 | true | ok |
| pooled | 32 | 2712 | 0 | 0 | 0 | true | ok |
| pooled | 128 | 7965 | 0 | 0 | 0 | true | ok |
| pooled-mvcc | 32 | N/A (setup incompatible) | N/A | N/A | N/A | N/A | N/A |
| pooled-mvcc | 128 | N/A (setup incompatible) | N/A | N/A | N/A | N/A | N/A |
| sqlite | 32 | 1686 | 0 | 0 | 0 | true | ok |
| sqlite | 128 | 1587 | 0 | 0 | 0 | true | ok |

**Note on rowid_mismatches:** shared mode's `rowid_mismatches` was 0 at both 32 and 128 tasks. The documented `last_insert_rowid` race in `GuardedConnection` (see its doc comment in `crates/vox-db/src/lib.rs`) did not reproduce in this run. This is probabilistic/timing-dependent and does not disprove the race; it means this specific test dataset and environment did not independently demonstrate it.

**Note on pooled-mvcc:** Turso's MVCC journal mode rejects the AUTOINCREMENT keyword. VoxDB's baseline production schema (automatically applied by `VoxDbPool::new()`'s migration step) uses AUTOINCREMENT in multiple tables. The initial 128-task run appeared to fail with a different error (`MVCC logical log file exists...`), but this was due to Turso's MVCC mode creating a `.db-log` sidecar file that the cleanup command (`rm -f /tmp/vox-stress-test.db /tmp/vox-stress-test.db-wal /tmp/vox-stress-test.db-shm`) does not remove. A clean re-run with wildcard cleanup (`rm -f /tmp/vox-stress-test.db*`) confirmed that both 32-task and 128-task pooled-mvcc runs fail with the same `Parse error: AUTOINCREMENT is not supported in MVCC mode` error. This is a real and consistent compatibility finding: the production schema cannot currently run against MVCC-enabled Turso databases. The rows above are marked N/A rather than representing zero throughput or errors; there is no throughput data to report because the harness never completed its initialization phase.
