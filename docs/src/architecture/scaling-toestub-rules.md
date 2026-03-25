# TOESTUB scaling rules (SSOT)

Detector id: **`scaling/surfaces`** (`crates/vox-toestub/src/detectors/scaling.rs`).

## Rule IDs (findings)

| Rule id | Severity | Meaning |
|---------|----------|---------|
| `scaling/blocking-in-async` | Info | `std::fs::*` in an `async` fn (use `tokio::fs` / `spawn_blocking`; allowlist in `contracts/scaling/policy.yaml`) |
| `scaling/thread-sleep-async` | Info | `thread::sleep` under async visitor |
| `scaling/path-literal` | Info | String literals matching SSOT path fragments (`mens/runs*`, etc.) — prefer `vox_scaling_policy` |
| `scaling/magic-limit` | Info | Integers in `magic_numeric_hints` from policy |
| `scaling/regex-new-hot` | Warning | `Regex::new(` without `LazyLock`/`OnceLock` on the line |
| `scaling/unbounded-read` | Info | `std::fs::read_to_string` heuristic |
| `scaling/lines-collect-vec` | Info | `.lines()` + `collect::<Vec` |
| `scaling/repeated-json-parse` | Info | `serde_json::from_str` near loop heuristic |
| `scaling/sql-no-limit` | Warning | SQL string with `SELECT` but no `LIMIT` (heuristic) |
| `scaling/http-client-no-timeout` | Warning | `Client::new()` heuristic |
| `scaling/nested-pairwise-loop` | Info | `(i+1)..` inner loop pattern |
| `scaling/cache-miss-hot-read` | Info | `read_to_string` / `fs::read` / `OpenOptions` shortly after a `for` loop header — batch or cache |
| `scaling/large-in-memory-accumulator` | Info | `Vec::with_capacity(N)` with very large `N` — confirm bound or stream |
| `scaling/env-default-duplication` | Info | Same string literal in `unwrap_or("…")` on multiple `std::env::var` lines — centralize |

## Suppressions

Same-line: `// toestub-ignore(scaling)` or `// toestub-ignore(scaling/<rule-suffix>)`.

## Policy

Thresholds and literals: [`contracts/scaling/policy.yaml`](../../../contracts/scaling/policy.yaml).  
Rust accessors: `vox-scaling-policy` crate.

**Severity note:** Scaling findings default to **Info** so `toestub --mode enforce-strict --rules scaling` can pass while audits still surface issues. Raise individual rules to `Warning` when tightening CI.
