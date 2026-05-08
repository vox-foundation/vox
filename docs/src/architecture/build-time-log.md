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

## Phase 2 — workspace-hack leaf exclusion (2026-05-08)

Originally planned as a 5-way split. After auditing hakari's design (auto-generated
unification crate; physical split would lose auto-management) the better intervention
is to **exclude L0 leaf crates from workspace-hack entirely** via hakari's
`[traversal-excludes].workspace-members` and `[final-excludes].workspace-members`.

Excluded crates (no longer pay the unified-feature floor):
- vox-orchestrator-types, vox-db-types, vox-protocol, vox-mesh-types
- vox-primitives, vox-plugin-types, vox-layer-check

| Scenario | Time | vs Phase 1 |
|---|---|---|
| L0 leaf incremental (vox-orchestrator-types) | 0.68s | unchanged (already minimal) |
| L0 leaf incremental (vox-plugin-types) | 0.53s | new measurement; truly leaf now |

The candle-* family was already excluded by the existing `[final-excludes].third-party`,
so a separate `vox-hack-ml` was unnecessary.

For heavy crates (orchestrator/cli/db) workspace-hack is retained — feature unification
there genuinely avoids duplicate compiles.

## Phase 3 — vox-db split (audit only, deferred) (2026-05-08)

Investigated extracting `vox-db-stores` from `vox-db` (32K LoC, 67 `impl VoxDb`
blocks). Conclusion: the physical split has high cost and modest benefit given
the current architecture.

**Why deferred:**
1. **Orphan rule** forces extension traits for each of the 67 method blocks.
   Each call site `db.method()` then requires a `use vox_db_stores::SomeStoreTrait`
   import. ~50 callers in vox-orchestrator alone, plus vox-cli, vox-search, etc.
2. **No unconditional fat deps to gate**: `vox-compiler` + `vox-compiler-emit`
   are used structurally (`@table` schema parsing in `auto_migrate`/`ddl`/
   `schema_digest`), not by accident.
3. **Incremental edit cost is bounded**: editing one ops file in vox-db
   triggers a 9.5s recompile of the whole crate. Splitting would reduce this
   to ~3-5s per edit, but we'd amortize the migration work over hundreds of
   call-site changes — net negative on this phase's budget.

The bigger wins live in Phase 4 (orchestrator MCP split — already feature-gated,
clean seam) and Phase 7 (CLI decoupling). Those phases proceed first; Phase 3
can be revisited as a follow-up if vox-db edit-cycle pain materializes.

| Scenario | Time | vs baseline |
|---|---|---|
| vox-db incremental (touch oratio_eval.rs) | 9.54s | unchanged — no split done |
