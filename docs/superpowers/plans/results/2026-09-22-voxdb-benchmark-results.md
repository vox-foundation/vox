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
