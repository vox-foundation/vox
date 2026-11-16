---
phase: 319-mesh-wide-selection-mesh-01
verified: 2026-10-30T04:03:56Z
status: passed
score: 12/12 must-haves verified
behavior_unverified: 0
overrides_applied: 0
decision_coverage:
  honored: 0
  total: 0
  not_honored: []
  skipped: true
  reason: "no trackable decisions"
covered_files:
  - .planning/phases/319-mesh-wide-selection-mesh-01/319-01-PLAN.md
  - .planning/phases/319-mesh-wide-selection-mesh-01/319-01-SUMMARY.md
  - .planning/phases/319-mesh-wide-selection-mesh-01/319-02-PLAN.md
  - .planning/phases/319-mesh-wide-selection-mesh-01/319-02-SUMMARY.md
  - .planning/phases/319-mesh-wide-selection-mesh-01/319-03-PLAN.md
  - .planning/phases/319-mesh-wide-selection-mesh-01/319-03-SUMMARY.md
  - crates/vox-orchestrator/src/orchestrator/tests/populi_single_owner.rs
  - crates/vox-plugin-populi-mesh/Cargo.toml
  - crates/vox-plugin-populi-mesh/src/transport/handlers/dispatch.rs
  - crates/vox-plugin-populi-mesh/src/transport/handlers/nodes.rs
  - crates/vox-populi-types/src/node_record.rs
  - crates/vox-populi/src/lib.rs
  - crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs
  - crates/vox-populi/src/transport/handlers/nodes.rs
covered_digest: "v3:sha256:aa224a4b2c92f54da6386a665fee53c22f95a2d5a5b15bd0027fa6c62fbd3799"
---

# Phase 319: Mesh-Wide Selection Verification Report

**Phase Goal:** Across a mesh, the selector picks the largest size that fits on any node's current free memory and routes the work there.
**Verified:** 2026-10-30T04:03:56Z
**Status:** passed
**Re-verification:** No — initial verification

Evidence is `git show` / `git diff` of `a5d60a1e8..f2ab6c443` (implementation) and `79e56b6ff` (review docs). The working tree was not used.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Node heartbeats carry current free accelerator memory, not only total VRAM. | ✓ VERIFIED | `NodeRecord.accelerator_free_mb` (`Option<u64>`, serde default, skip if none) sits beside `gpu_vram_total_mb`. The mens block of `node_record_for_current_process` sets it from `read_accelerator_free_mb().ok()` via `accelerator_free_mb_from_reading` (`Some(14000)` → `Some(14000)`, `None` → `None`). Both heartbeat merges assign `target.accelerator_free_mb = src.accelerator_free_mb` on every merge, including `None`. `gpu_vram_total_mb` stays copy-if-present in vox-populi and is not copied by the plugin merge. `memory_free_bytes` is not assigned. Heartbeat handlers call the merge (`nodes.rs` populi line 250, plugin line 210). |
| 2 | The mesh pick chooses the largest size that fits on some node, routes the job to that node, and titrates down when no node fits the larger size. | ✓ VERIFIED | `select_qwen3_mesh` takes one `max()` of usable `accelerator_free_mb` readings and calls `select_qwen3_code_qlora_for_free_mb` once (that function calls `pick_base`). `route_pick_to_holder` keeps nodes whose free memory is at least `floor_mb` and sorts by lowest `cpu_usage_pct` (`unwrap_or(100.0)`). `mesh_pick_routes_largest_floor_to_the_node_that_holds_it` expects node `wide`, `32B`, `qwen3_96g`, floor `60000`, pin `Qwen/Qwen3-32B@9216db5781bf21249d130ec9da846c4624c16137`. Automatic pick on a 25000 node is `14B` / `qwen3_24g` / floor `20000`. |
| 3 | A stale or missing node reading is not treated as free memory, and the local fallback still works with no mesh. | ✓ VERIFIED | Candidates require `accelerator_free_mb.is_some()` and `now.saturating_sub(last_seen) <= window`. `mesh_pick_ignores_stale_and_missing_free_memory`: a 70000 reading at `last_seen` 6_399_999 and a node with only total VRAM / system RAM are not selected; the fresh 25000 node is `14B`. Empty candidates return `NoFreshReading` and do not call `pick_base`. `place_qwen3_code_qlora` uses `select_qwen3_code_qlora_for_free_mb` on the injected `local_free_mb` only for an empty registry or `NoFreshReading`. `local_free_mb: None` returns `NoFreshReading` and does not call `read_accelerator_free_mb`. |
| 4 | An injected pair of fresh nodes yields the 32B QLoRA pin on the only node whose `accelerator_free_mb` holds that floor, and policy exclusions plus the CPU tie-break are covered by the same function. | ✓ VERIFIED | Plan 01 `<success_criteria>`. Same test as truth 2 for the 32B pin on `wide`. `mesh_pick_tie_breaks_on_lowest_cpu`: both at 70000, CPU 80 vs 10, selects the CPU-10 node at 32B. `mesh_pick_skips_quarantine_maintenance_labels_and_donation`: quarantine, maintenance still ahead of the injected clock, a missing required label, and an empty donation slot list each drop the 70000 node; the 25000 node is selected at 14B. `node_passes_dispatch_policy` is shared with `select_best_node` and receives the injected clock from the mesh pick. |
| 5 | `gpu_vram_total_mb`, system `memory_free_bytes`, and `capabilities.min_vram_mb` do not choose the size. | ✓ VERIFIED | `mesh_pick_ignores_total_vram_system_ram_and_min_vram`: a node with those fields set and `accelerator_free_mb: None` loses to a 25000 node even when `req.min_vram_mb` is 70000. The mesh function never reads those fields for the floor. |
| 6 | An unset window uses 3_600_000 ms. Registry listing is unchanged when the secret is unset. | ✓ VERIFIED | `effective_stale_window_ms`: `Some(n)` if `n > 0`, otherwise `3_600_000` (`None` and `Some(0)`). `mesh_pick_unset_window_uses_dispatch_hour` covers both. `select_qwen3_mesh` does not call `server_stale_prune_ms`. `git diff a5d60a1e8..f2ab6c443` does not touch `list_nodes`. |
| 7 | An explicit `resolve_qwen3_size` spelling that no eligible node can hold is an error. The automatic pick on those same nodes steps down. | ✓ VERIFIED | Plan 02 `<success_criteria>` explicit clause. `explicit_qwen3_size_for_free_mb` bails when `floor_mb > free_mb` and does not call `pick_base`. `mesh_pick_explicit_size_that_no_node_holds_is_an_error`: `Some("32B")` on a 25000 node is `ExplicitDoesNotFit`; `explicit_size: None` on that node is 14B. `explicit_qwen3_code_qlora_for_free_mb_refuses_over_floor` asserts the same split at 25000 vs 70000. `mesh_pick_explicit_size_routes_to_a_node_that_holds_it` routes 32B to the 70000 node. |
| 8 | `select_best_node` does not call the QLoRA selector. It still returns the lowest `cpu_usage_pct` when no node has `accelerator_free_mb`. | ✓ VERIFIED | `select_best_node` filters with `node_passes_dispatch_policy` and `req.min_vram_mb`, then sorts CPU. It does not call `select_qwen3_mesh`, `select_qwen3_code_qlora_for_free_mb`, or `read_accelerator_free_mb`. `select_best_node_ignores_accelerator_free_mb`: idle node with no free reading and CPU 10 wins script dispatch; the mesh pick of the same nodes is the busy 70000 node at 32B. |
| 9 | A registry whose only node is fresh is a mesh pick of that node. An explicit size no eligible node can hold stays an error and does not become the local pick. | ✓ VERIFIED | Plan 03 `<success_criteria>`. `place_qwen3_only_this_node_is_a_mesh_pick`: node `local-1` at 70000 is mesh 32B even though `local_free_mb` is 25000. `place_qwen3_explicit_miss_does_not_fall_back_local`: fresh 25000 node, explicit `32B`, `local_free_mb` 70000 returns `ExplicitDoesNotFit`. `place_qwen3_code_qlora` propagates every mesh error other than `NoFreshReading`. |
| 10 | Tests build `NodeRecord` values, clocks, and free-memory numbers in process. They do not probe a GPU, contact Hugging Face, download weights, or start training. | ✓ VERIFIED | Mesh tests build records with `serde_json` and set `accelerator_free_mb` / `last_seen_unix_ms` in process. The new tests do not call `read_accelerator_free_mb`, `query_nvidia_smi_vram`, or `monitor_metal`. The only new production call to `read_accelerator_free_mb` is the mens block in `node_record_for_current_process`. Diff does not touch `small_code_default`, `strong_code_default`, or phase 322 paths. |
| 11 | An explicit size on the local fallback is refused, not stepped down. | ✓ VERIFIED | CR-01 `ceead59d7`, still present at `f2ab6c443`. When the mesh is empty or returns `NoFreshReading` and `explicit_size` is `Some`, `place_qwen3_code_qlora` calls `explicit_qwen3_code_qlora_for_free_mb` and maps the error to `ExplicitDoesNotFit`. `place_qwen3_empty_explicit_32b_does_not_step_down` and `place_qwen3_stale_or_missing_explicit_size_uses_local_rule` expect `ExplicitDoesNotFit` for `32B` on a 25000 local reading. `place_qwen3_empty_explicit_14b_does_not_step_up` keeps explicit `14B` on a 70000 local reading. |
| 12 | Overflowing `u32` readings are dropped before the max. | ✓ VERIFIED | WR-01 `f2ab6c443`. Usable candidates are fresh readings that pass `u32::try_from`. The max is taken over that subset. `ReadingExceedsU32` is returned only when every fresh reading overflows. `mesh_pick_drops_overflowing_free_memory_reading`: `u64::MAX` beside 25000 selects the 25000 node at 14B; a lone `u64::MAX` is `ReadingExceedsU32(u64::MAX)`. |

**Score:** 12/12 truths verified (0 present, behavior-unverified)

### Plan success criteria

| Plan | Criterion | Status |
|------|-----------|--------|
| 319-01 | An injected pair of fresh nodes yields the 32B QLoRA pin on the only node whose `accelerator_free_mb` holds that floor, and policy exclusions plus the CPU tie-break are covered by the same function. | ✓ Met (truths 2, 4) |
| 319-02 | A stale 70000 reading cannot outrank a fresh 25000 reading, explicit 32B on that fresh node errors, and a heartbeat that omits `accelerator_free_mb` clears it. | ✓ Met (truths 1, 3, 7) |
| 319-03 | No fresh mesh reading resolves on this machine from an injected number, one fresh node stays a mesh pick, and an explicit size nobody holds does not fall back. | ✓ Met (truths 3, 9, 11) |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/vox-populi-types/src/node_record.rs` | `accelerator_free_mb` on `NodeRecord` | ✓ VERIFIED | Field present, documented as free accelerator MB, serde default. `verify.artifacts` passed. |
| `crates/vox-plugin-populi-mesh/src/transport/handlers/dispatch.rs` | `select_qwen3_mesh` and `place_qwen3_code_qlora` | ✓ VERIFIED | Both functions are substantive and called by the in-module tests. `verify.artifacts` passed for plans 01 and 03. |
| `crates/vox-plugin-populi-mesh/Cargo.toml` | existing `vox-populi` dependency with `mens` | ✓ VERIFIED | Diff changes `vox-populi = { workspace = true }` to `features = ["mens"]`. No new crate. |
| `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs` | `explicit_qwen3_code_qlora_for_free_mb` | ✓ VERIFIED | Public function loads the overlay and calls `explicit_qwen3_size_for_free_mb`. |
| `crates/vox-populi/src/transport/handlers/nodes.rs` | heartbeat merge clears `accelerator_free_mb` on omission | ✓ VERIFIED | Unconditional assignment; total VRAM stays inside `is_some`. |
| `crates/vox-plugin-populi-mesh/src/transport/handlers/nodes.rs` | plugin heartbeat merge clears the field | ✓ VERIFIED | Same unconditional assignment. Plugin merge still does not copy `gpu_vram_total_mb`. |
| `crates/vox-populi/src/lib.rs` | fill from the phase 318 reading | ✓ VERIFIED | Helper plus the mens-gated assignment. Field stays `None` when mens is off. |

**Artifacts:** 7/7 verified

### Key Link Verification

`verify.key-links` returned `verified: false` for every link because plan `from:` values are symbols, not file paths (`Source file not found`). The calls are present at `f2ab6c443`.

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `select_qwen3_mesh` | `select_qwen3_code_qlora_for_free_mb` | one call on the max usable `accelerator_free_mb` | ✓ WIRED | Automatic arm only. That function calls `pick_base`. |
| `select_qwen3_mesh` | `explicit_qwen3_code_qlora_for_free_mb` | explicit spelling, no step-down | ✓ WIRED | `Some(size)` arm. Error becomes `ExplicitDoesNotFit`. |
| `select_qwen3_mesh` | `NodeRecord.accelerator_free_mb` | floor compared with each candidate | ✓ WIRED | `route_pick_to_holder` keeps `free >= floor`. |
| `node_record_for_current_process` | `read_accelerator_free_mb` | mens block maps `Ok` and leaves `None` on `Err` | ✓ WIRED | `.ok()` then `accelerator_free_mb_from_reading`. |
| `merge_optional_node_fields` | `NodeRecord.accelerator_free_mb` | assignment runs when the incoming field is `None` | ✓ WIRED | Both copies. Heartbeat handlers call the merge. |
| `place_qwen3_code_qlora` | `select_qwen3_mesh` | mesh result other than empty registry or `NoFreshReading` | ✓ WIRED | `Ok` returns `Qwen3Placement::Mesh`. Other errors propagate. |
| `place_qwen3_code_qlora` | `select_qwen3_code_qlora_for_free_mb` | empty registry or `NoFreshReading`, caller-supplied `local_free_mb` | ✓ WIRED | Automatic local arm only. Explicit local arm uses `explicit_qwen3_code_qlora_for_free_mb`. |

**Wiring:** 7/7 connections verified

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `node_record_for_current_process` | `accelerator_free_mb` | `read_accelerator_free_mb` (CUDA free, else Metal) | Yes, when mens is on and the probe returns `Ok` | ✓ FLOWING |
| heartbeat `merge_optional_node_fields` | `accelerator_free_mb` | incoming `NodeRecord` JSON | Yes. Omission deserializes as `None` and clears the stored value | ✓ FLOWING |
| `select_qwen3_mesh` / `place_qwen3_code_qlora` | `size_token`, `preset`, `floor_mb`, `node_id` | `pick_base` or `resolve_qwen3_size` on the measured overlay, then the holder node | Yes. The mesh function does not hardcode 32B or 14B | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `mesh_pick_` inventory | count `fn mesh_pick_` at `f2ab6c443` | 9 tests. Reported run: 9 passed. No warm test binary in this process; cargo was not re-run (spot-check budget). | ✓ PASS |
| `place_qwen3_` inventory | count `fn place_qwen3_` at `f2ab6c443` | 8 tests. Reported run: 8 passed. Same constraint. | ✓ PASS |
| Explicit over-floor refusal | `explicit_qwen3_code_qlora_for_free_mb_refuses_over_floor` source | Asserts error at 25000 and `32B` / `qwen3_96g` / floor `60000` at 70000. | ✓ PASS |
| Heartbeat clear | both `merge_clears_accelerator_free_mb_when_later_heartbeat_omits_it` | Omit clears free memory; total VRAM and `memory_free_bytes` stay; a later `Some(8192)` sets the field. | ✓ PASS |

### Probe Execution

No phase-declared probes. Success criteria are cargo tests, not `scripts/*/tests/probe-*.sh`.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| MESH-01 | 319-01, 319-02, 319-03 | Heartbeats carry current free accelerator memory. The selector picks the largest size that fits on some node, routes the job there, and titrates down when no node fits. A stale or missing reading is not treated as free memory. | ✓ SATISFIED | Truths 1–12. The REQUIREMENTS.md checkbox is still open; this verification does not call `phase.complete`. |

No requirement mapped to phase 319 is missing from the plans.

### Decision Coverage

`check.decision-coverage-verify` skipped: `No trackable decisions in CONTEXT.md` (`honored: 0`, `total: 0`). The `<decisions>` block is prose, not id-tagged entries, so the tool had nothing to match. Status impact: none.

Manual check of that prose against `f2ab6c443`: free-memory field and heartbeat clear, one `pick_base` call on the max usable reading, CPU tie-break, quarantine / maintenance / labels / donation, fit test ignores total VRAM and system RAM and `min_vram_mb`, explicit sizes error, stale window defaults to `3_600_000` with no new env var, missing readings cannot win, all-stale returns an error, empty or no-fresh uses the injected local reading, `select_best_node` does not call that selector, a single fresh node stays a mesh pick, tests inject records. All of those are in the committed code.

### Test Quality Audit

| Test File | Linked Req | Active | Skipped | Circular | Assertion Level | Verdict |
|-----------|------------|--------|---------|----------|-----------------|---------|
| `crates/vox-plugin-populi-mesh/src/transport/handlers/dispatch.rs` | MESH-01 | 18 (`9` `mesh_pick_`, `8` `place_qwen3_`, `1` `select_best_node_ignores_`) | 0 | No. Literals (`32B`, `14B`, floors, error variants) are the oracle. One test also compares to `select_qwen3_code_qlora_for_free_mb(70000)` after asserting those literals. | Value | PASS |
| `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs` | MESH-01 | `explicit_qwen3_code_qlora_for_free_mb_refuses_over_floor` | 0 | No | Value | PASS |
| `crates/vox-populi/src/lib.rs` | MESH-01 | `accelerator_free_mb_from_reading_omits_on_failure` | 0 | No | Value | PASS |
| `crates/vox-populi/src/transport/handlers/nodes.rs` | MESH-01 | `merge_clears_accelerator_free_mb_when_later_heartbeat_omits_it` | 0 | No | Value | PASS |
| `crates/vox-plugin-populi-mesh/src/transport/handlers/nodes.rs` | MESH-01 | same merge test | 0 | No | Value | PASS |

**Disabled tests on requirements:** 0
**Circular patterns detected:** 0
**Insufficient assertions:** 0

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/vox-populi/src/lib.rs` | mens block context | pre-existing `TODO: cuda_driver_version from precision layer if needed` | ℹ️ Info | Context line in the diff, not added by this phase. Not `TBD` / `FIXME` / `XXX`. |
| `dispatch.rs` `place_qwen3_code_qlora` | `NothingFits` arm | no test injects a fresh reading below every measured floor | ℹ️ Info | The code propagates `NothingFits` and does not use `local_free_mb`. Not a plan success criterion. |
| `dispatch.rs` | `place_qwen3_code_qlora` | not called from `dispatch_script` | ℹ️ Info | Plan 03 locks script forwarding as unchanged. The function is the caller. |

**Anti-patterns:** 3 info, 0 blockers, 0 warnings

### Human Verification Required

N/A — infrastructure phase. The goal is a library selector and heartbeat field. Acceptance criteria are the committed tests above. No user-facing surface was added.

### Gaps Summary

None. Roadmap success criteria and all three plan `<success_criteria>` blocks hold at `f2ab6c443`. CR-01 refuses an explicit size on the local fallback. WR-01 drops overflowing readings before the max.

---

_Verified: 2026-10-30T04:03:56Z_
_Verifier: Claude (gsd-verifier)_
