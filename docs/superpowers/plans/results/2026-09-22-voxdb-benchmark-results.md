# VoxDB Turso/SQLite Benchmark Results — 2026-09-22

Machine: Darwin Bertrands-MacBook-Pro.local 25.6.0 Darwin Kernel Version 25.6.0: Fri Jul 31 19:19:08 PDT 2026; root:xnu-12377.161.14~5/RELEASE_ARM64_T6050 arm64
Rust toolchain: rustc 1.98.1 (48a229cea 2026-09-01)

## Point-insert latency (`cargo bench -p vox-db --bench concurrency_bench`)

| Benchmark | Mean time |
|---|---|
| `turso_shared_connection_insert` | 11.359 µs |
| `turso_pooled_connection_insert` | 30.821 µs |
| `rusqlite_insert` | 1.6965 µs |

HTML report: `target/criterion/report/index.html`
