# Phase 302: Transfer-Budget Artifacts, PR Comments & Weighted Test Sharding - Context

## Phase Boundary
Promote `vox-orchestrator-mcp` transfer-budget tests into a machine-readable CI artifact, add a trusted-default-branch PR comment publisher (t3code `thread-transfer-report` pattern), and wire a committed shard-weights file into `partition_crates` for nextest sharding.

## Requirements
- T3-CI-BUDGET-01..04

## Implementation Decisions
- Emit artifact via `vox ci transfer-budget-report` (Rust in `vox-cli-ci`) rather than a new `.sh`/`.ps1` — VoxScript-first for future regen scripts; CLI is the existing CI surface.
- Trusted publisher workflow checks out **default branch only** and never executes PR code when posting comments.
- Shard weights live in `contracts/ci/shard-weights.v1.json`; `crate_weight` prefers the file, falls back to `.rs` file count.
- Ratchet report: `contracts/reports/shard-balance-ratchet.v1.json` + budget ceilings in `contracts/reports/transfer-budget-ci.v1.json`.

## Existing Code Insights
- Budgets already pinned in `crates/vox-orchestrator-mcp/tests/transfer_budget.rs`.
- `partition_crates` + `--shards N` already exist in `vox-cli-ci/src/affected_cmd.rs`.

## Deferred Ideas
- Full delta comment formatting parity with t3code's Node reporter (Vox publisher posts a short markdown summary).
- GUI e2e duration weights (nextest crate weights first).
