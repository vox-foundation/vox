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

## Phases 4–6 — Orchestrator splits (audit, deferred to follow-up) (2026-05-08)

Investigated the planned three-way split of vox-orchestrator (88K LoC →
mcp/queue/runtime/coordinator). Findings:

**mcp_tools/ (33K LoC, 237 cross-imports)** — surface-level coupling looks
manageable: 191 of 237 `use crate::*` imports are intra-mcp_tools. Only ~46
imports leave the subsystem, mostly `models`, `types`, `planning`, plus 2
references to the `Orchestrator` type itself.

**Why not extracted in this phase:**

1. **Cargo's incremental builds already amortize most of the cost.**
   Measured: `cargo check -p vox-orchestrator` after editing one mcp_tools
   file is **5.06s** (Phase 0 baseline) — already fast. The originally-estimated
   70% reduction would shave maybe 2-3s off this. The extraction itself
   takes 1-2 days of mechanical work.
2. **Coupling to `Orchestrator` type forces a trait-abstraction layer.**
   mcp_tools calls into the live Orchestrator (task dispatch, message bus,
   capability lookups). Extracting requires defining an `OrchestratorContext`
   trait covering ~20 method surfaces, then refactoring every call site.
3. **Same applies to queue/ and runtime/ subsystems** — they're more deeply
   coupled than mcp_tools. The "queue" subsystem touches messages.rs and
   tasks.rs (already noted as having circular deps with VictoryCondition/
   attention/observer in earlier conversation).

**What is achieved without physical extraction:**

- The `mcp-native` feature already gates compilation of mcp_tools (Phase 1
  daemon-binary fix; earlier session axum/rmcp gating). Slim builds skip
  it entirely.
- The 88K LoC sits in one crate but cargo's per-codegen-unit incremental
  rebuilds keep edit cycles tolerable.

**When to revisit:** if and only if `cargo check -p vox-orchestrator` after
a small edit climbs above ~10s on dev hardware. Until then the 5s status
quo doesn't justify weeks of refactor.

This is consistent with Phase 3's deferral logic: physical extractions are
deferred when the cost/benefit ratio doesn't pencil out at the current
crate sizes. The phase plan's original budget over-estimated wins from
crate splits relative to what feature-gating + leaf exclusion already
deliver.

## Phase 7 — vox-cli decoupling (audit, deferred to follow-up) (2026-05-08)

vox-cli (63K LoC, 156 deps) uses `vox_orchestrator` in 19 source files
across many command groups: `model/*`, `mcp_server/*`, `attention.rs`,
`dei.rs`, `safety.rs`, `status.rs`, `live.rs`, `generate.rs`, `ci/*`,
`ludus/hud.rs`, `visus/mod.rs`.

**Why deferred:** the cleanest decoupling — make `vox-orchestrator` an
optional dep behind a feature gate — requires `#[cfg(feature = "...")]`
gates on ~12 command files AND their clap subcommand routing in lib.rs.
Each gated subcommand needs a corresponding gate in the enum variant
that clap dispatches on. ~30+ touch points; mechanical but error-prone.

The simpler `OrchestratorClient` trait facade approach is even larger:
mapping the orchestrator's public API to a stable trait covers ~40
methods used across vox-cli, and would still require updating the same
~30 call sites.

**What's already in place:** `mcp-server`, `extras-ludus`, `ludus-hud`,
`mens-dei`, `coderabbit`, `scientia-social`, `dashboard` features
already gate orchestrator-using subsystems. The gap is the unconditional
inclusion of orchestrator-using commands like `vox status`, `vox attention`,
`vox safety`, `vox model *`, `vox dei`, `vox live` — these are user-facing
runtime/observability surfaces that don't fit naturally behind a feature.

**When to revisit:** if a scenario emerges where a build of vox-cli
specifically without orchestrator features is needed (e.g. an embedded
or cross-compiled CLI variant). Until then, the orchestrator dep is
load-bearing for the CLI experience and gating it is more disruption
than win.

The known inversion `vox-cli → vox-orchestrator` remains in `layers.toml`
under `[[known_inversions]]`. Phase 9 will keep it documented (as a
deliberate decision, not transitional debt) rather than removing it.

## Phase 8 — Plugin family flattening (audit, no action needed) (2026-05-08)

Audited all 13 live plugin crates (`crates/vox-plugin-*`):

- **No layer inversions.** L4 (concrete plugins) → L3 (vox-db) is allowed
  by the layer model. The layer-check passes clean.
- **vox-cli does NOT compile-time depend on any plugin.** Confirmed:
  `vox-cli/Cargo.toml` explicitly comments `vox-plugin-publication` is
  intentionally NOT a direct dep ("plugins are loaded at runtime").
- **vox-orchestrator does NOT compile-time depend on any plugin.**
  Confirmed via grep.
- **Three plugins use vox-db** (`mens-candle-cuda`, `populi-mesh`,
  `publication`). These are loaded as cdylib at runtime, so their
  vox-db compile cost doesn't affect vox-cli/orchestrator rebuild times.

**Conclusion:** the plugin family is structurally clean. The "flattening"
work was either already done in earlier sessions (cdylib delivery for
plugins) or is unnecessary at this layer. No code changes needed.

The 3 vox-db-using plugins COULD be refactored to use a `PluginStateBackend`
trait (same pattern as Phase 1), but the win is theoretical given runtime
loading already isolates compile costs.
