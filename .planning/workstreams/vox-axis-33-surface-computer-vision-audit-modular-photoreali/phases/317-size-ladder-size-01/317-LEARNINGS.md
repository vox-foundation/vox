---
phase: 317
phase_name: "size-ladder-size-01"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 8
  lessons: 3
  patterns: 4
  surprises: 1
missing_artifacts:
  - "UAT.md"
---

# Phase 317 Learnings: size-ladder-size-01

## Decisions

### 1.7B and 4B are unmeasured qwen3_code QLoRA rows with no floor_mb; pick_...
1.7B and 4B are unmeasured qwen3_code QLoRA rows with no floor_mb; pick_base cannot select them

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 317-01-SUMMARY.md

---

### floor_measured defaults true so existing measured floors stay eligible
floor_measured defaults true so existing measured floors stay eligible

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 317-01-SUMMARY.md

---

### Capability tags such as qwen3_16g are not size names and do not enter re...
Capability tags such as qwen3_16g are not size names and do not enter resolve_qwen3_size

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 317-01-SUMMARY.md

---

### Added Qwen/Qwen3-1.7B@70d244cc86ccca08cf5af4e1e306ecf908b1ad5e and Qwen/...
Added `Qwen/Qwen3-1.7B@70d244cc86ccca08cf5af4e1e306ecf908b1ad5e` and `Qwen/Qwen3-4B@1cfa9a7208912126459214e8b04321603b3df60c` under `train_bases.qwen3_code` only, apache-2.0, `attribution_required: false`, `floor_measured: false`, with no `floor_mb`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 317-01-SUMMARY.md

---

### floor_measured serde-defaults to true
`floor_measured` serde-defaults to true. `pick_base` skips `floor_measured: false`. Existing measured floors and the 14B/32B LoRA rows stay. The new rows are not on `small_code_default` or `strong_code_default`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 317-01-SUMMARY.md

---

### resolve_qwen3_size accepts 0.6B..32B and the exact Qwen3-<size> / Qwen/Q...
`resolve_qwen3_size` accepts `0.6B`..`32B` and the exact `Qwen3-<size>` / `Qwen/Qwen3-<size>` spellings. Unknown sizes error. Phase 316's size-name path calls it. `qwen3_16g` stays on the memory picker.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 317-01-SUMMARY.md

---

### 1.7B (70d244cc86ccca08cf5af4e1e306ecf908b1ad5e) and 4B (1cfa9a7208912126...
1.7B (`70d244cc86ccca08cf5af4e1e306ecf908b1ad5e`) and 4B (`1cfa9a7208912126459214e8b04321603b3df60c`) are unmeasured `qwen3_code` QLoRA rows with no `floor_mb`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 317-01-SUMMARY.md

---

### floor_measured defaults true so measured rows stay eligible
`floor_measured` defaults true so measured rows stay eligible. `pick_base` requires both `floor_measured` and a `floor_mb` that fits.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 317-01-SUMMARY.md

## Lessons

### Edits to gpu-specs.yaml and spoke_base_resolver.rs were briefly overwrit...
Edits to `gpu-specs.yaml` and `spoke_base_resolver.rs` were briefly overwritten by a compressed file view. Both files were restored from git and the intended edits were reapplied. The committed yaml diff is the two new rows only. A later whole-file parse error was that corrupted yaml, not `Option<u32>` deserialization.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 317-01-SUMMARY.md

---

### Wiring: 3/3 connections verified
**Wiring:** 3/3 connections verified

**Context:** Verified in phase acceptance checks.
**Source:** 317-VERIFICATION.md

---

### verify.key-links reported all three unverified because from is a functio...
`verify.key-links` reported all three unverified because `from` is a function name rather than a file path (`Source file not found`). Manual read of the call sites shows each link is present. That tool result is not treated as NOT_WIRED.

**Context:** Verified in phase acceptance checks.
**Source:** 317-VERIFICATION.md

## Patterns

### A Qwen3 size name maps to exactly one licensed qwen3_code QLoRA pin
A Qwen3 size name maps to exactly one licensed qwen3_code QLoRA pin

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 317-01-SUMMARY.md

---

### An unmeasured rung records floor_measured false and omits floor_mb
An unmeasured rung records floor_measured false and omits floor_mb

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 317-01-SUMMARY.md

---

### floor_measured serde-defaults true; missing floor_mb is None
floor_measured serde-defaults true; missing floor_mb is None

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 317-01-SUMMARY.md

---

### Size spellings resolve only through resolve_qwen3_size on qwen3_code QLo...
Size spellings resolve only through resolve_qwen3_size on qwen3_code QLoRA rows

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 317-01-SUMMARY.md

## Surprises

### Estimate variance on 317-01
Estimate variance on 317-01: planned 42000 tokens vs 7455 actual (0.18x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 317-01-PLAN.md, 317-01-SUMMARY.md

