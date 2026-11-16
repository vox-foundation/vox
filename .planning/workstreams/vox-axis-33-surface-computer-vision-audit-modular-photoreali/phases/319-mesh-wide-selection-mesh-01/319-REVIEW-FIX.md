---
phase: 319-mesh-wide-selection-mesh-01
fixed_at: 2026-10-30T03:56:24Z
review_path: .planning/phases/319-mesh-wide-selection-mesh-01/319-REVIEW.md
iteration: 1
findings_in_scope: 2
fixed: 2
skipped: 0
status: all_fixed
---

# Phase 319: Code Review Fix Report

**Fixed at:** 2026-10-30T03:56:24Z
**Source review:** `.planning/phases/319-mesh-wide-selection-mesh-01/319-REVIEW.md`
**Iteration:** 1

**Summary:**
- Findings in scope: 2
- Fixed: 2
- Skipped: 0

## Fixed Issues

### CR-01: Local fallback ignores an explicit size

**Files modified:** `crates/vox-plugin-populi-mesh/src/transport/handlers/dispatch.rs`
**Commit:** `ceead59d7`
**Applied fix:** On an empty registry or `NoFreshReading`, an `explicit_size` now calls `explicit_qwen3_code_qlora_for_free_mb` and maps a failure to `ExplicitDoesNotFit`. Only a missing explicit size calls `select_qwen3_code_qlora_for_free_mb`. Tests cover an empty registry (`32B` at 25000 MB, `14B` at 70000 MB) and the same rule when every reading is stale or missing.

### WR-01: One overflowing free-memory reading drops every other node

**Files modified:** `crates/vox-plugin-populi-mesh/src/transport/handlers/dispatch.rs`
**Commit:** `f2ab6c443`
**Applied fix:** Fresh readings that do not fit in `u32` are dropped before the max. Usable nodes are what `route_pick_to_holder` sees. `ReadingExceedsU32` is returned only when every fresh reading overflows. A test with one `u64::MAX` reading and one fresh 25000 MB node picks that node's `14B`.

## Verification

Ran in the main checkout (`workflow.use_worktrees` is false). No isolated worktree.

- `cargo test -p vox-plugin-populi-mesh --lib place_qwen3_ -- --test-threads=8` — 8 passed, 0 failed, 27 filtered out.
- `cargo test -p vox-plugin-populi-mesh --lib mesh_pick_ -- --test-threads=8` — 9 passed, 0 failed, 27 filtered out.

Re-review of `ceead59d7` and `f2ab6c443` against `319-CONTEXT.md` found no new critical or warning issues. `319-REVIEW.md` status is `clean`.

---

_Fixed: 2026-10-30T03:56:24Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
