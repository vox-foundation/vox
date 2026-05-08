# Workspace Reorg Outcome (2026-05-08)

Companion to [2026-05-08-workspace-reorg-design.md](./2026-05-08-workspace-reorg-design.md).
Records what was actually done across the 10 phases, what was deferred, and why.

## Phases delivered (3 of 10)

### Phase 0 — Baseline & guards ✓
- Created `crates/vox-layer-check/` (Rust binary): parses `cargo metadata`,
  validates each workspace dep edge against `docs/src/architecture/layers.toml`.
- Created `layers.toml` assigning each of 76 crates to layers L0–L5 with
  three explicit `[[known_inversions]]` entries.
- Established `build-time-baseline.md` with measured incrementals.
- Wired CI to run the layer-check (warn-only at first; flipped to strict in Phase 9).

### Phase 1 — L0 type cleanup + plugin-host inversion fix ✓
- New `vox-plugin-types` (L1 leaf): plugin manifest types, skill manifest
  types, `PluginStateBackend` trait + `PluginStateSkillEntry` /
  `PluginStateError` types.
- `vox-plugin-api/manifest.rs` and `vox-plugin-host/skill_manifest.rs`
  now re-export shims pointing at `vox-plugin-types`.
- `vox-plugin-host` no longer depends on `vox-db` — uses
  `Arc<dyn PluginStateBackend>` instead. `vox-db` impls the trait.
- Daemon binary `[[bin]] required-features = ["mcp-native"]` so slim
  builds skip it cleanly.
- Drive-by fixes: pre-existing `mesh_driver_compile` test (rev 1 → 2);
  pre-existing `vox-mens` `gpu`-feature broken `crate::training::native`
  reference (replaced with stable bail-out).
- Build-time win: vox-cli incremental 26.76s → 7.6s (warm cache).

### Phase 2 — workspace-hack leaf exclusion ✓
- Configured hakari's `[traversal-excludes]` and `[final-excludes]` so
  L0 leaf crates (`vox-orchestrator-types`, `vox-db-types`, `vox-protocol`,
  `vox-mesh-types`, `vox-primitives`, `vox-plugin-types`, `vox-layer-check`)
  are NOT scanned for unification AND NOT given a workspace-hack dep.
- Originally planned 5-way physical split (vox-hack-core/async/net/codegen/ml).
  Pivoted because hakari's auto-management is the simpler intervention point;
  candle-* family was already excluded.
- Build-time win: L0 leaf incremental check at 0.5s (truly leaf — no hack
  compile in the dep graph).

### Phase 9 — Harden CI guards + final docs ✓
- Layer-check flipped from `--warn-only` to strict in CI: any unknown
  inversion fails the build.
- `layers.toml` updated: `vox-cli → vox-orchestrator` reframed from
  "transitional" to "permitted exception" (Phase 7 audit determined the
  decoupling cost outweighs the benefit).
- This outcome doc + the per-phase entries in `build-time-log.md`.

## Phases deferred (6 of 10)

Phases 3, 4, 5, 6, 7, 8 were each audited in full and deferred with
documented rationale (see `build-time-log.md`). Common theme: **the
originally-budgeted physical extractions all turned out to require
architectural untangling** (extension traits for the orphan rule, large
trait facades for cross-crate calls, ~30+ cfg-gates for clap routing) that
exceeds single-session scope without delivering proportional build-time wins.

| Phase | Target | Why deferred |
|---|---|---|
| 3 | vox-db → vox-db-stores | 67 `impl VoxDb` blocks need extension-trait migration; ~50 callers; vox-compiler dep is structural |
| 4 | vox-orchestrator-mcp | 33K LoC subsystem with deep coupling to `Orchestrator` type; cargo's incremental build already gives 5s edits — extraction win is marginal |
| 5 | vox-orchestrator-queue | Coupled with VictoryCondition/attention/observer (known from earlier session) |
| 6 | vox-orchestrator-runtime | Same as Phase 5 |
| 7 | vox-cli-thin | 19 source files use orchestrator across many command groups; ~30+ cfg-gates needed; clean trait facade is even larger |
| 8 | Plugin family flatten | Plugin family is already structurally clean (cdylib delivery; no compile-time pulls from vox-cli/orchestrator). Three plugins still use vox-db directly but L4 → L3 is allowed by the layer model. |

## Build-time outcome

Headline numbers (warm cache, incremental check after touching one file):

| Scenario | Baseline | After | Win |
|---|---|---|---|
| L0 leaf (vox-plugin-types) | n/a (didn't exist) | 0.53s | new — true leaf |
| L0 leaf (vox-orchestrator-types) | 0.36s | 0.68s | ~unchanged |
| Orchestrator (touch lib.rs) | 5.59s | 6.24s | +0.65s (added plugin-types edge) |
| Orchestrator (touch mcp_tools/) | 5.06s | ~5s | unchanged (no extraction) |
| CLI | 26.76s | 7.60s | **−72%** |

The CLI win is the biggest tangible result. The orchestrator number is roughly
flat — the 70%-class wins from the originally-planned subsystem split don't
materialize until that extraction is actually done. Cargo's incremental
compilation already amortizes much of the in-crate edit cost.

## What's enforced going forward

1. **Layer-check runs strict in CI.** Any new dep edge that violates the
   layer model fails the build. Adding a new crate requires an entry in
   `layers.toml`.
2. **L0 leaf crates stay leaves.** Hakari's exclusion list prevents
   `workspace-hack` from re-poisoning them on regeneration.
3. **The plugin-host inversion stays fixed.** `PluginStateBackend` is
   the only path; reintroducing a direct `vox-db` dep on `vox-plugin-host`
   triggers the layer-check.
4. **Three known inversions are documented** with rationale: vox-cli →
   vox-orchestrator (deliberate), vox-pm → vox-compiler/db (transitional —
   future re-tier).

## Recommended next step

If/when build-time pain on `vox-orchestrator` or `vox-cli` justifies it,
the deferred extractions can be picked up incrementally — one subsystem
at a time, with a clear cost/benefit measurement before each. The
infrastructure (layer-check, layers.toml, build-time-log.md, hakari
exclusions) is in place to support that work. The 88K orchestrator and
63K cli are not going to be split in single sessions; treat each future
extraction as its own multi-day project.
