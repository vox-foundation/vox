---
phase: 05-unified-branch-worktree-convergence
plan: 04
subsystem: capability-registry
tags: [discoverability, capability-profiles, operations-catalog, mcp-registry, ssot-regen]
requires:
  - phase: 05-03
    provides: KB Phases 1-4 foundation (vox_kb_search/get/cite/status/reindex, baseline 96)
provides:
  - Merged claude/vox-discoverability-a72c8c (d3a93f756) Phases 06.1-06.3 (vox-capability-registry, vox explain, vox features, Universal Palette)
  - Merged feat/capability-profiles (77f5dfa9a) (Exposure, SideEffectClass, Subsystem boot gate, gui_ops transport, Axis remote mode)
  - Reconciled contracts/operations/catalog.v1.yaml with zero conflict markers and full capability-profile classification
  - Regenerated all SSOT artifacts (tool-registry.canonical.yaml, command-registry.yaml, capability-registry.yaml, capability-index.v1.json, capabilityIndex.generated.ts, gui-surface-coverage.v1.json, gui-surface-registry.v1.json, model-manifest.generated.json, reference/*.generated.md)
affects: [phase-12, phase-07, phase-25, phase-32, phase-06.4]
tech-stack:
  added: [vox-capability-registry]
  patterns: [capability-profile exposure classification, subsystem boot gate, gui_ops loopback/remote transport]
key-files:
  created:
    - .planning/workstreams/unified/phases/05-unified-branch-worktree-convergence/05-04-SUMMARY.md
  modified:
    - contracts/operations/catalog.v1.yaml
    - contracts/capabilities/capability-index.v1.json
    - contracts/mcp/tool-registry.canonical.yaml
    - crates/vox-cli/src/commands/ci/capability_index/mod.rs
    - crates/vox-orchestrator/src/models/leaderboard.rs
    - crates/vox-orchestrator-mcp/src/gui_ops/dto/models.rs
    - crates/vox-orchestrator-mcp/src/gui_ops/dto/routing_explain.rs
    - crates/vox-orchestrator-mcp/src/gui_ops/knowledge.rs
    - crates/vox-orchestrator-mcp/src/gui_ops/models.rs
    - crates/vox-gui/src/commands/models.rs
key-decisions:
  - "Exposed BenchRowDto, LeaderboardDto, bench_row_dto, and compute_model_leaderboard through vox_orchestrator_mcp::gui_ops::models so vox-gui retains zero direct dependency on vox-db or vox-db-types."
  - "Replaced retired legacy vox_kb_* CRUD rows in capability-profiles catalog and test fixtures with the 5 KB Library operations (vox_kb_search, vox_kb_get, vox_kb_cite, vox_kb_status, vox_kb_reindex) classified with exposure: public and requires: [search_index]."
  - "Updated sync_llms_regions in vox-cli capability-index to skip docs/src/.well-known/llms-full.txt when absent on disk, honoring docs-freshness Phase 24 TRUST-03 retirement."
patterns-established:
  - "All MCP operations in contracts/operations/catalog.v1.yaml carry side_effect_class, mcp.exposure, and mcp.requires."
  - "All CLI operations in contracts/operations/catalog.v1.yaml carry side_effect_class or an explicit ratchet entry."
requirements-completed: [CONV-01, CONV-04]
duration: 38min
completed: 2026-10-13
---

# Phase 05 Plan 04: Discoverability (`06.2`–`06.3`), Capability Profiles & SSOT Regeneration Summary

**Merged `claude/vox-discoverability-a72c8c` (`d3a93f756`) and `feat/capability-profiles` (`77f5dfa9a`) onto `feat/unified-2026-q4`, reconciled `contracts/operations/catalog.v1.yaml` across KB Library + Discoverability + Capability Profiles, and regenerated all capability/MCP/CLI/GUI SSOT registries.**

## Performance

- **Duration:** 38 min
- **Started:** 2026-10-13T21:20:00-07:00
- **Completed:** 2026-10-13T21:58:00-07:00
- **Tasks:** 3
- **Files modified:** 146

## Accomplishments

- **Task 1 (`b3b3cd687`):** Merged `claude/vox-discoverability-a72c8c` (`d3a93f756`, Phases `06.1`–`06.3`), bringing `crates/vox-capability-registry/`, `vox explain`, `vox features`, `vox_capability_explain`, `vox_capability_features`, and the Universal Palette (`⌘K` Omnibar) into `feat/unified-2026-q4`.
- **Task 2 (`9e11ee8ff`):** Merged `feat/capability-profiles` (`77f5dfa9a`), reconciling:
  - `contracts/operations/catalog.v1.yaml` (`capability.explain`, `capability.features`, and the 5 KB Library operations `kb.search`, `kb.get`, `kb.cite`, `kb.status`, `kb.reindex` with `side_effect_class`, `mcp.exposure`, and `mcp.requires`, plus `side_effect_class` classifications for `cli.serve`, `cli.kb`, `cli.kb.eval`, `cli.model.bench`, and `cli.model.eval-compare`).
  - `crates/vox-orchestrator-mcp/src/gui_ops/` (`knowledge.rs`, `models.rs`, `dto/models.rs`, `dto/routing_explain.rs`) and `crates/vox-gui/src/commands/models.rs` so `vox-gui` routes leaderboard and benchmark queries through `vox_orchestrator_mcp::gui_ops` without depending on `vox-db`.
- **Task 3 (`9e11ee8ff`):** Ran `vox ci operations-sync --target all --write`, `vox ci command-sync --write`, `vox ci capability-index --write`, `vox ci gui-surface-coverage --write`, `vox ci gui-surface-registry --write`, `vox ci capability-sync --write`, and `vox ci crate-edges`, and verified all unit and integration tests pass:
  - `cargo test -p vox-capability-registry --lib`: 49 passed, 0 failed
  - `cargo test -p vox-mcp-registry`: 84 passed (74 unit + 4 `exposure_classification` + 6 `tool_registry_smoke`), 0 failed
  - `cargo test -p vox-orchestrator-mcp --test legacy_voxkb_retired --test boot_requires_guard`: 4 passed, 0 failed
  - `vox ci crate-edges`: OK (699 live in-tree edges within baseline)

## Task Commits

1. **Task 1: Merge `claude/vox-discoverability-a72c8c` (`d3a93f756`)** - `b3b3cd687` (`merge(unified): merge claude/vox-discoverability-a72c8c (d3a93f756) Phases 06.2-06.3`)
2. **Tasks 2 & 3: Merge `feat/capability-profiles` (`77f5dfa9a`) & Regenerate SSOT Artifacts** - `9e11ee8ff` (`merge(unified): merge feat/capability-profiles (77f5dfa9a) and regenerate capability/MCP/CLI registries`)

## Files Created/Modified

- `contracts/operations/catalog.v1.yaml` - Reconciled `capability.*`, `kb.*`, `plan.status`, `skill.uninstall`, and 5 CLI `side_effect_class` rows.
- `crates/vox-cli/src/commands/ci/capability_index/mod.rs` - Skipped retired `docs/src/.well-known/llms-full.txt` when absent on disk (`TRUST-03`).
- `crates/vox-orchestrator/src/models/leaderboard.rs` - Derived `serde::Deserialize` on `ModeRoute` and converted `AgentTask` probe via `RoutingTask::from(&probe)`.
- `crates/vox-orchestrator-mcp/src/gui_ops/dto/routing_explain.rs` - Added `QualitySource::Measured` match arm to `quality_dto`.
- `crates/vox-orchestrator-mcp/src/gui_ops/models.rs` & `dto/models.rs` - Added `BenchRowDto`, `LeaderboardDto`, `bench_row_dto`, and `compute_model_leaderboard`.
- `crates/vox-gui/src/commands/models.rs` - Routed `get_model_leaderboard` and `list_benchmark_results` through `vox_orchestrator_mcp::gui_ops::models`.

## Decisions Made

- Kept `vox-gui` free of direct `vox-db` / `vox-db-types` edges by exposing `BenchRowDto` and `LeaderboardDto` from `vox_orchestrator_mcp::gui_ops::dto::models`.
- Updated `capability-profiles` test fixtures (`boot_requires_guard.rs`, `daemon_extra_tests.rs`, `profile.rs`, `exposure_classification.rs`, `profiles.rs`, `boot_gate.rs`, `dev-advertised-tools.default-features.txt`) from the retired `vox_kb_list`/`vox_kb_query` names to `vox_kb_search`/`vox_kb_status`, satisfying `legacy_voxkb_retired`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Resolved residual conflict marker in `crates/vox-research-shim/src/research/orchestrator/pipeline.rs`**
- **Found during:** Task 3 (`vox ci operations-sync --write` compilation)
- **Issue:** Line 132 of `crates/vox-research-shim/src/research/orchestrator/pipeline.rs` had an unresolved conflict marker between `run_session_label(config, session_id, session_key)` and inline session string formatting.
- **Fix:** Kept `run_session_label(config, session_id, session_key)` which encapsulates both `trial_run_label` and `research_session:{session_id}` fallback.
- **Files modified:** `crates/vox-research-shim/src/research/orchestrator/pipeline.rs`
- **Verification:** `vox-research-shim` compiled cleanly.
- **Committed in:** `9e11ee8ff`

**2. [Rule 1 - Bug] Reconciled `RoutingTask`, `ModeRoute` `Deserialize`, and `QualitySource::Measured` across `345542f4a` and `77f5dfa9a`**
- **Found during:** Task 3 (`vox-orchestrator` and `vox-orchestrator-mcp` compilation)
- **Issue:** `345542f4a` added `leaderboard.rs` (`ModeRoute` and `QualitySource::Measured`) while `77f5dfa9a` refactored `best_for_task_under_gate` to take `&RoutingTask` and moved `routing_explain.rs` into `vox-orchestrator-mcp`.
- **Fix:** Passed `&RoutingTask::from(&probe)` in `leaderboard.rs`, derived `serde::Deserialize` on `ModeRoute`, and added the `Some(QualitySource::Measured { .. })` arm to `quality_dto` in `crates/vox-orchestrator-mcp/src/gui_ops/dto/routing_explain.rs`.
- **Files modified:** `crates/vox-orchestrator/src/models/leaderboard.rs`, `crates/vox-orchestrator-mcp/src/gui_ops/dto/routing_explain.rs`
- **Verification:** `vox-orchestrator`, `vox-orchestrator-mcp`, and `vox-cli` compiled cleanly.
- **Committed in:** `9e11ee8ff`

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 cross-branch type signature reconciliation)
**Impact on plan:** Necessary cross-branch type and contract alignment; zero scope creep.

## Issues Encountered

None after resolving cross-branch contract and type signatures.

## Next Phase Readiness

- Wave 0 (`Phase 05: Unified Branch & Worktree Convergence`) is 100% complete (`05-01` .. `05-04`).
- Ready for Wave 1:
  - **Phase 12:** Autonomy Safety Floor & Lowering-Path Closures (`12-01`..`12-08`)
  - **Phase 07:** Trial Identity, Contracts & Tier A Persistence at DB Baseline 97 (`07-01`, `07-05`..`07-12`)
  - **Phase 25:** Unified Code & Architecture Graph Data Floor (`25-01`..`25-04`)
  - **Phase 32:** Docs Trust Signal Calibration & Ledger Sync (`32-01`..`32-04`)
