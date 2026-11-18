# Phase 476 Verification

## Observed Benchmark Output (`contracts/reports/terminal-bench.v1.json`, `macos/aarch64`, debug profile under host 9B Metal training load)
- `input_to_frame_p50_ms`: `0.006125 ms` (`p99`: `0.033875 ms`)
- `pty_scroll_throughput_mb_s`: `4.453456970144112 MiB/s` (debug unoptimized threshold: `>= 2.0 MiB/s`)
- `alt_screen_switch_ms`: `0.048334 ms` (SLA `< 12.0 ms`)
- `agent_exec_roundtrip_p50_ms`: `0.055417 ms` (SLA `< 8.0 ms`)
- `tokio_worker_stall_max_ms`: `0.0300334 ms` (SLA `< 2.0 ms`)
- `osc633_block_accuracy_pct`: `100.0%` (SLA `>= 99.9%`)
- `session_rss_kb_at_1k_blocks`: `271 KiB` (SLA `<= 46080 KiB`)
- `all_slas_met`: `true`

## Commands Observed Passing
- `cargo test -p vox-terminal-core --features vox-lang`: `bench::tests::terminal_benchmark_suite_meets_all_slas_and_writes_report ... ok` (`89/89` unit + `11` integration test suites).
- `vox check scripts/quality/terminal-bench-flywheel.vox`: `exit 0` (`Check passed with 0 warning(s)`).
- `vox run --interp scripts/quality/terminal-bench-flywheel.vox`: `exit 0` (`PASS: schema_version=1 all_slas_met=true ...`).
- `cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/terminal-surfaces-audit-and-benchmarks-research-2026.md`: `exit 0`.
- `pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console`: `23/23` files, `141/141` tests passed.

## Scope & Environment Caveats
- Benchmark throughput numbers above were collected in Cargo's unoptimized `[unoptimized + debuginfo]` test profile during active background Metal training contention (`PID 6385`). Release-mode WebGL2 throughput and live 4-theme Tauri/Playwright visual CV screenshots were NOT executed in this headless CLI session (Console UI rendering, `OSC 8`/`OSC 52`/Kitty `CSI u`, `rusage` chips, and `std.frame`/`std.plot` SVG cards were verified via Vitest JSDOM component tests).
