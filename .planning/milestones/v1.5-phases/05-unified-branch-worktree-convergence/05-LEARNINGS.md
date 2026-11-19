---
phase: 5
phase_name: "unified-branch-worktree-convergence"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 17
  lessons: 1
  patterns: 11
  surprises: 0
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 5 Learnings: unified-branch-worktree-convergence

## Decisions

### Combined #610 Result<R, String> workflow signatures with ce9cbc67c VoxDb...
Combined #610 Result<R, String> workflow signatures with ce9cbc67c VoxDbTracker + args_json + parked workflow handling in durability_lower.rs

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 05-02-SUMMARY.md

---

### Enforced layout_only token stream verification in format() and try_forma...
Enforced layout_only token stream verification in format() and try_format() so comments and unprinted declarations are never silently dropped

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 05-02-SUMMARY.md

---

### Merged feat/true-workflow-durability (ce9cbc67c, commit 2da573423), reco...
Merged `feat/true-workflow-durability` (`ce9cbc67c`, commit `2da573423`), reconciling `Result<R, String>` workflow wrapper signatures from `#610` with `VoxDbTracker`, `args_json`, `workflow.wait`, `workflow.wait_signal`, `workflow.version`, and `workflow_patch_log` from `ce9cbc67c`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 05-02-SUMMARY.md

---

### Verified claude/warp-fork-axis-gui-b682cd (345542f4a) had zero diff agai...
Verified `claude/warp-fork-axis-gui-b682cd` (`345542f4a`) had zero diff against `origin/main` (`#641`) for `crates/vox-terminal-core` and recorded its merge commit (`7f4a21c8e`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 05-02-SUMMARY.md

---

### Merged wip/fmt-safety-2026-10-12 (499c75bce, commit 9829b0a16) and imple...
Merged `wip/fmt-safety-2026-10-12` (`499c75bce`, commit `9829b0a16`) and implemented `layout_only` token stream comparison in `crates/vox-compiler/src/fmt/mod.rs` plus exhaustive `Pattern` printing in `crates/vox-compiler/src/fmt/expr.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 05-02-SUMMARY.md

---

### Consolidated KB Phases 1-4 (#645), workflow durability (ce9cbc67c), and ...
Consolidated KB Phases 1-4 (#645), workflow durability (ce9cbc67c), and model_benchmark_results into a single Baseline 96 schema digest without extra integer bumps so Phase 07 (research-trial-flywheel) cleanly claims Baseline 97

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 05-03-SUMMARY.md

---

### Verified zero diff between HEAD (which included #645 via origin/main) an...
Verified zero diff between `HEAD` (which included `#645` via `origin/main`) and `e60182f6f` (`feat/kb-phase-1-foundation`), and recorded `e60182f6f` as a merged parent (`1801fb4b8`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 05-03-SUMMARY.md

---

### Updated contracts/db/baseline-version-policy.yaml repository_baseline_di...
Updated `contracts/db/baseline-version-policy.yaml` `repository_baseline_digest_hex` to `0x8e3a9f5c411b63aa622352ca005ae9ea27c18a4815d7557dc3dd9d6e81adef28` and verified `baseline_policy_matches_compiled_schema` passes.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 05-03-SUMMARY.md

---

### Added "workflow_patch_log" to LEGACY_EXPORT_TABLES in crates/vox-db/src/...
Added `"workflow_patch_log"` to `LEGACY_EXPORT_TABLES` in `crates/vox-db/src/codex_legacy.rs` and verified `legacy_export_covers_all_baseline_tables` (`--features legacy-import`) passes.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 05-03-SUMMARY.md

---

### Exposed BenchRowDto, LeaderboardDto, bench_row_dto, and compute_model_le...
Exposed BenchRowDto, LeaderboardDto, bench_row_dto, and compute_model_leaderboard through vox_orchestrator_mcp::gui_ops::models so vox-gui retains zero direct dependency on vox-db or vox-db-types.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 05-04-SUMMARY.md

---

### Replaced retired legacy vox_kb_* CRUD rows in capability-profiles catalo...
Replaced retired legacy vox_kb_* CRUD rows in capability-profiles catalog and test fixtures with the 5 KB Library operations (vox_kb_search, vox_kb_get, vox_kb_cite, vox_kb_status, vox_kb_reindex) classified with exposure: public and requires: [search_index].

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 05-04-SUMMARY.md

---

### Updated sync_llms_regions in vox-cli capability-index to skip docs/src/....
Updated sync_llms_regions in vox-cli capability-index to skip docs/src/.well-known/llms-full.txt when absent on disk, honoring docs-freshness Phase 24 TRUST-03 retirement.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 05-04-SUMMARY.md

---

### Task 1 (b3b3cd687)
**Task 1 (`b3b3cd687`):** Merged `claude/vox-discoverability-a72c8c` (`d3a93f756`, Phases `06.1`–`06.3`), bringing `crates/vox-capability-registry/`, `vox explain`, `vox features`, `vox_capability_explain`, `vox_capability_features`, and the Universal Palette (`⌘K` Omnibar) into `feat/unified-2026-q4`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 05-04-SUMMARY.md

---

### contracts/operations/catalog.v1.yaml (capability.explain, capability.fea...
`contracts/operations/catalog.v1.yaml` (`capability.explain`, `capability.features`, and the 5 KB Library operations `kb.search`, `kb.get`, `kb.cite`, `kb.status`, `kb.reindex` with `side_effect_class`, `mcp.exposure`, and `mcp.requires`, plus `side_effect_class` classifications for `cli.serve`, `cli.kb`, `cli.kb.eval`, `cli.model.bench`, and `cli.model.eval-compare`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 05-04-SUMMARY.md

---

### crates/vox-orchestrator-mcp/src/gui_ops/ (knowledge.rs, models.rs, dto/m...
`crates/vox-orchestrator-mcp/src/gui_ops/` (`knowledge.rs`, `models.rs`, `dto/models.rs`, `dto/routing_explain.rs`) and `crates/vox-gui/src/commands/models.rs` so `vox-gui` routes leaderboard and benchmark queries through `vox_orchestrator_mcp::gui_ops` without depending on `vox-db`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 05-04-SUMMARY.md

---

### cargo test -p vox-capability-registry --lib
`cargo test -p vox-capability-registry --lib`: 49 passed, 0 failed

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 05-04-SUMMARY.md

---

### cargo test -p vox-mcp-registry
`cargo test -p vox-mcp-registry`: 84 passed (74 unit + 4 `exposure_classification` + 6 `tool_registry_smoke`), 0 failed

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 05-04-SUMMARY.md

## Lessons

### None after resolving cross-branch contract and type signatures.
None after resolving cross-branch contract and type signatures.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 05-04-SUMMARY.md

## Patterns

### Fail-closed formatter
Fail-closed formatter: try_format returns an error and format returns the input unchanged whenever non-newline/non-EOF tokens differ after round-trip

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 05-02-SUMMARY.md

---

### layout_only token comparison in vox fmt
layout_only token comparison in vox fmt

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 05-02-SUMMARY.md

---

### VoxDbTracker durable workflow journal
VoxDbTracker durable workflow journal

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 05-02-SUMMARY.md

---

### Verify both baseline_policy_matches_compiled_schema and legacy_export_co...
Verify both baseline_policy_matches_compiled_schema and legacy_export_covers_all_baseline_tables (--features legacy-import) whenever merging branches that add DDL

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 05-03-SUMMARY.md

---

### Single Baseline 96 Keccak-256 schema digest
Single Baseline 96 Keccak-256 schema digest

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 05-03-SUMMARY.md

---

### sorted LEGACY_EXPORT_TABLES parity
sorted LEGACY_EXPORT_TABLES parity

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 05-03-SUMMARY.md

---

### All MCP operations in contracts/operations/catalog.v1.yaml carry side_ef...
All MCP operations in contracts/operations/catalog.v1.yaml carry side_effect_class, mcp.exposure, and mcp.requires.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 05-04-SUMMARY.md

---

### All CLI operations in contracts/operations/catalog.v1.yaml carry side_ef...
All CLI operations in contracts/operations/catalog.v1.yaml carry side_effect_class or an explicit ratchet entry.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 05-04-SUMMARY.md

---

### capability-profile exposure classification
capability-profile exposure classification

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 05-04-SUMMARY.md

---

### subsystem boot gate
subsystem boot gate

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 05-04-SUMMARY.md

---

### gui_ops loopback/remote transport
gui_ops loopback/remote transport

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 05-04-SUMMARY.md

## Surprises

