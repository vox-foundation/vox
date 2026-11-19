---
phase: 319
phase_name: "mesh-wide-selection-mesh-01"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 21
  lessons: 4
  patterns: 9
  surprises: 3
missing_artifacts:
  - "UAT.md"
---

# Phase 319 Learnings: mesh-wide-selection-mesh-01

## Decisions

### Missing cpu_usage_pct sorts as 100.0 so a node that reported CPU wins th...
Missing cpu_usage_pct sorts as 100.0 so a node that reported CPU wins the tie.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 319-01-SUMMARY.md

---

### A free-memory reading that does not fit u32 is ReadingExceedsU32
A free-memory reading that does not fit u32 is ReadingExceedsU32. It is not truncated.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 319-01-SUMMARY.md

---

### NodeRecord.accelerator_free_mb is optional megabytes beside gpu_vram_tot...
`NodeRecord.accelerator_free_mb` is optional megabytes beside `gpu_vram_total_mb`. Missing JSON stays `None`. `memory_free_bytes` is still system RAM.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 319-01-SUMMARY.md

---

### select_qwen3_mesh calls select_qwen3_code_qlora_for_free_mb once on the ...
`select_qwen3_mesh` calls `select_qwen3_code_qlora_for_free_mb` once on the largest eligible reading. That function calls `pick_base`. The ladder is not copied.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 319-01-SUMMARY.md

---

### Two holders of the same floor break on the lower cpu_usage_pct
Two holders of the same floor break on the lower `cpu_usage_pct`. Quarantine, maintenance, required labels, and donation policy exclude a node before the pick.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 319-01-SUMMARY.md

---

### The mesh pick ignores gpu_vram_total_mb, memory_free_bytes, and capabili...
The mesh pick ignores `gpu_vram_total_mb`, `memory_free_bytes`, and `capabilities.min_vram_mb`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 319-01-SUMMARY.md

---

### A missing CPU sample sorts as 100.0, so a node that reported CPU wins th...
A missing CPU sample sorts as 100.0, so a node that reported CPU wins the tie.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 319-01-SUMMARY.md

---

### An explicit size that does not fit returns ExplicitDoesNotFit and does n...
An explicit size that does not fit returns ExplicitDoesNotFit and does not call the automatic picker.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 319-02-SUMMARY.md

---

### A heartbeat that omits accelerator_free_mb assigns None
A heartbeat that omits accelerator_free_mb assigns None. gpu_vram_total_mb and memory_free_bytes are not cleared by that assignment.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 319-02-SUMMARY.md

---

### A node is a size candidate only when accelerator_free_mb is present and ...
A node is a size candidate only when `accelerator_free_mb` is present and `last_seen_unix_ms` is inside the injected window. `None` and `Some(0)` use 3_600_000 ms.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 319-02-SUMMARY.md

---

### explicit_qwen3_code_qlora_for_free_mb loads the same overlay as the auto...
`explicit_qwen3_code_qlora_for_free_mb` loads the same overlay as the automatic picker and refuses a floor above the reading. `select_qwen3_mesh` maps that failure to `ExplicitDoesNotFit` and does not step down.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 319-02-SUMMARY.md

---

### accelerator_free_mb_from_reading maps the phase 318 reading
`accelerator_free_mb_from_reading` maps the phase 318 reading. `None` stays omitted. Both heartbeat merges assign the field unconditionally, so a later omission clears it. Total VRAM still copies only when present. System RAM is untouched.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 319-02-SUMMARY.md

---

### The caller passes the window
The caller passes the window. `select_qwen3_mesh` does not call `server_stale_prune_ms`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 319-02-SUMMARY.md

---

### Some(0) is the unset window, matching the locked 3_600_000 ms default
`Some(0)` is the unset window, matching the locked 3_600_000 ms default. No env var was added.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 319-02-SUMMARY.md

---

### A registry of only this node with a fresh reading is a mesh pick of this...
A registry of only this node with a fresh reading is a mesh pick of this node, even when a smaller local reading was also supplied.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 319-03-SUMMARY.md

---

### The automatic picker failure is NothingFits, so a too-small fresh readin...
The automatic picker failure is NothingFits, so a too-small fresh reading is not NoFreshReading and does not use the local reading.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 319-03-SUMMARY.md

---

### select_best_node still returns the lowest CPU and does not call the qwen...
`select_best_node` still returns the lowest CPU and does not call the qwen3 selector. A node with no free-memory reading can win that path while `select_qwen3_mesh` picks the node that has free memory.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 319-03-SUMMARY.md

---

### An empty registry, or NoFreshReading, with local_free_mb
An empty registry, or `NoFreshReading`, with `local_free_mb: Some` returns the local variant from `select_qwen3_code_qlora_for_free_mb`. `local_free_mb: None` returns `NoFreshReading` and does not probe.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 319-03-SUMMARY.md

---

### One fresh node is a mesh pick of that node
One fresh node is a mesh pick of that node. A smaller local reading does not replace it.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 319-03-SUMMARY.md

---

### ExplicitDoesNotFit and NothingFits propagate
`ExplicitDoesNotFit` and `NothingFits` propagate. A local reading that would have fit is not used.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 319-03-SUMMARY.md

---

### The automatic picker error is NothingFits
The automatic picker error is `NothingFits`. `PickFailed` remains for a local selector failure.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 319-03-SUMMARY.md

## Lessons

### None in the shipped commits.
None in the shipped commits.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 319-01-SUMMARY.md

---

### The populi explicit-size test needs --features mens because mens is not ...
The populi explicit-size test needs `--features mens` because `mens` is not a default feature. That command was run with the feature and passed 1 test.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 319-02-SUMMARY.md

---

### verify.key-links returned verified
`verify.key-links` returned `verified: false` for every link because plan `from:` values are symbols, not file paths (`Source file not found`). The calls are present at `f2ab6c443`.

**Context:** Verified in phase acceptance checks.
**Source:** 319-VERIFICATION.md

---

### Wiring: 7/7 connections verified
**Wiring:** 7/7 connections verified

**Context:** Verified in phase acceptance checks.
**Source:** 319-VERIFICATION.md

## Patterns

### Mesh tests inject NodeRecord values, a clock, and a stale window
Mesh tests inject NodeRecord values, a clock, and a stale window. They do not probe a GPU.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 319-01-SUMMARY.md

---

### Size comes from accelerator_free_mb through select_qwen3_code_qlora_for_...
Size comes from accelerator_free_mb through select_qwen3_code_qlora_for_free_mb. Total VRAM, system RAM, and min_vram_mb do not choose the size.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 319-01-SUMMARY.md

---

### vox-plugin-populi-mesh enables the existing mens feature on vox-populi
vox-plugin-populi-mesh enables the existing mens feature on vox-populi. No new workspace crate edge.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 319-01-SUMMARY.md

---

### Stale means now.saturating_sub(last_seen) > window
Stale means now.saturating_sub(last_seen) > window. The boundary age of 3_600_000 ms is still fresh.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 319-02-SUMMARY.md

---

### The caller passes the stale window
The caller passes the stale window. select_qwen3_mesh does not read server_stale_prune_ms.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 319-02-SUMMARY.md

---

### None and Some(0) both use 3_600_000 ms
None and Some(0) both use 3_600_000 ms. No new env var.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 319-02-SUMMARY.md

---

### place_qwen3_code_qlora never calls read_accelerator_free_mb
place_qwen3_code_qlora never calls read_accelerator_free_mb. The local number is an argument.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 319-03-SUMMARY.md

---

### Empty registry or NoFreshReading uses the caller-supplied local_free_mb
Empty registry or NoFreshReading uses the caller-supplied local_free_mb. local_free_mb None does not probe.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 319-03-SUMMARY.md

---

### ExplicitDoesNotFit and NothingFits propagate
ExplicitDoesNotFit and NothingFits propagate. They do not become a local pick.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 319-03-SUMMARY.md

## Surprises

### Estimate variance on 319-01
Estimate variance on 319-01: planned 48000 tokens vs 3908 actual (0.08x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 319-01-PLAN.md, 319-01-SUMMARY.md

---

### Estimate variance on 319-02
Estimate variance on 319-02: planned 52000 tokens vs 3212 actual (0.06x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 319-02-PLAN.md, 319-02-SUMMARY.md

---

### Estimate variance on 319-03
Estimate variance on 319-03: planned 36000 tokens vs 1411 actual (0.04x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 319-03-PLAN.md, 319-03-SUMMARY.md

