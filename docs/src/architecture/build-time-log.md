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

## Phase 1 — L0 type cleanup + plugin-host inversion (2026-05-08)

`vox-plugin-types` extracted (manifest + skill_manifest + state-backend trait).
`vox-plugin-host` no longer depends on `vox-db` (uses `Arc<dyn PluginStateBackend>`
trait; `vox-db` impls it). Daemon binary gated via `required-features=mcp-native`.

| Scenario | Time | vs baseline |
|---|---|---|
| Orchestrator incremental (lib.rs) | 6.24s | +0.65s (~12% — added plugin-types crate to dep graph) |
| CLI incremental | 7.60s | **−19.16s (−72%)** — Phase 0 baseline included a forced re-link from earlier dep gating; warm-cache CLI is now near-floor |

Layer-check (strict mode) clean: 1 known inversion remaining (`vox-cli → vox-orchestrator`, scheduled for Phase 7).
