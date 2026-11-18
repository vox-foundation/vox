# Phase 476 Context: `vox bench terminal` Competitor Benchmark Suite (`terminal-bench.v1.json`) & Verification Flywheel

**Workstream:** `axis-terminal-sota-v80`
**Phase:** 476
**Requirements:** `TERM-BENCH-01`, `TERM-BENCH-02`, `TERM-BENCH-03`, `TERM-BENCH-04`
**SSOT:** [`terminal-surfaces-audit-and-benchmarks-research-2026.md`](../../../../docs/src/architecture/terminal-surfaces-audit-and-benchmarks-research-2026.md)

## Decisions
1. Implement deterministic self-calibrating microbenchmark + SLA harness directly in `crates/vox-terminal-core/src/bench.rs` exporting `contracts/reports/terminal-bench.v1.json` with Nov 2026 competitor baselines (Ghostty 1.1+, Warp Nov 2026, Kitty 0.39+, WezTerm 2026, Alacritty 0.15+, Zed/VS Code).
2. Calibrate `min_pty_scroll_throughput_mb_s` cleanly for dev debug builds (`2.0 MiB/s` unoptimized debug vs `25.0+ MiB/s` release VT) while holding tight latency SLAs (`tokio_worker_stall_max_ms < 2.0`, `alt_screen_switch_ms < 12.0`, `agent_exec_roundtrip_p50_ms < 8.0`, `osc633_block_accuracy_pct >= 99.9`).
3. Gate every run in VoxLang via `scripts/quality/terminal-bench-flywheel.vox` (`vox check` + `vox run --interp`).
