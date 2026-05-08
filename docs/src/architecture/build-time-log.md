# Build-Time Log

Per-phase measurements for the workspace reorg. Compare against
[build-time-baseline.md](./build-time-baseline.md).

Append a row at the end of each phase. Format: phase | scenario | time | delta vs baseline.

## Phase 0 — Baseline established (2026-05-08)

See [build-time-baseline.md](./build-time-baseline.md). Layer-check tool live
in warn-only mode; no architectural changes yet — same numbers as baseline.

| Scenario | Time | vs baseline |
|---|---|---|
| Orchestrator incremental (lib.rs) | 5.59s | — |
| Orchestrator incremental (mcp_tools/) | 5.06s | — |
| CLI incremental | 26.76s | — |
