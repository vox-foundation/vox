# Phase 476 Review Fixes

- Changed `run_terminal_benchmark_suite` in `crates/vox-terminal-core/src/bench.rs` to return `anyhow::Result<TerminalBenchReportV1>` with `?` propagation (removing `.expect(...)` calls from production path).
- Gated writing `contracts/reports/terminal-bench.v1.json` in tests behind `VOX_BENCH_WRITE` env var or missing report file so repeated test runs do not churn tracked report bytes.
- Verified: `cargo test -p vox-terminal-core --features vox-lang` passed (`bench::tests::terminal_benchmark_suite_meets_all_slas_and_writes_report ... ok`).
