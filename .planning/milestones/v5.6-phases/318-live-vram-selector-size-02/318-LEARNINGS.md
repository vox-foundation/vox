---
phase: 318
phase_name: "live-vram-selector-size-02"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 23
  lessons: 2
  patterns: 10
  surprises: 3
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 318 Learnings: live-vram-selector-size-02

## Decisions

### 1.7B and 4B stay ineligible because floor_measured is false
1.7B and 4B stay ineligible because floor_measured is false

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 318-01-SUMMARY.md

---

### CUDA free wins when present; Metal memory_free_mb is the fallback
CUDA free wins when present; Metal memory_free_mb is the fallback

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 318-01-SUMMARY.md

---

### Unit tests inject readings and do not call read_accelerator_free_mb
Unit tests inject readings and do not call read_accelerator_free_mb

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 318-01-SUMMARY.md

---

### select_qwen3_code_qlora_for_free_mb filters qwen3_code rows whose method...
`select_qwen3_code_qlora_for_free_mb` filters `qwen3_code` rows whose methods contain `qlora` and calls `pick_base`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 318-01-SUMMARY.md

---

### confirm_before_load keeps an automatic pick whose floor still fits and o...
`confirm_before_load` keeps an automatic pick whose floor still fits and otherwise steps down through that same picker.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 318-01-SUMMARY.md

---

### explicit_qwen3_size_for_free_mb errors for 1.7B, 4B, and any floor above...
`explicit_qwen3_size_for_free_mb` errors for 1.7B, 4B, and any floor above the reading.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 318-01-SUMMARY.md

---

### free_mb_from_readings uses CUDA free_gb when present, otherwise Metal me...
`free_mb_from_readings` uses CUDA `free_gb` when present, otherwise Metal `memory_free_mb`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 318-01-SUMMARY.md

---

### A missing free-memory reading is an error for an unnamed size and for an...
A missing free-memory reading is an error for an unnamed size and for an explicit Qwen3 spelling

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 318-02-SUMMARY.md

---

### small_code_default and strong_code_default stay on pick_base
small_code_default and strong_code_default stay on pick_base

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 318-02-SUMMARY.md

---

### Dogfood passes automatic true because its model is None
Dogfood passes automatic true because its model is None

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 318-02-SUMMARY.md

---

### Live train_arm probes stay behind the empty hw-probe-live-test feature
Live train_arm probes stay behind the empty hw-probe-live-test feature

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 318-02-SUMMARY.md

---

### resolve_training_selection uses the live selector when no model was name...
`resolve_training_selection` uses the live selector when no model was named and a reading is present.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 318-02-SUMMARY.md

---

### resolve_local_spoke_base calls read_accelerator_free_mb and no longer su...
`resolve_local_spoke_base` calls `read_accelerator_free_mb` and no longer substitutes `get_system_vram_gb` or `16_384`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 318-02-SUMMARY.md

---

### The train stage passes that reading only when the CLI model is absent or...
The train stage passes that reading only when the CLI model is absent or a `resolve_qwen3_size` spelling.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 318-02-SUMMARY.md

---

### apply_preload_free_mb steps an automatic pick down and refuses an explic...
`apply_preload_free_mb` steps an automatic pick down and refuses an explicit measured pin.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 318-02-SUMMARY.md

---

### Directory tiers and vox-mens-8b-v0.6 are gone from auto_select
Directory tiers and vox-mens-8b-v0.6 are gone from auto_select

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 318-03-SUMMARY.md

---

### is_apple_silicon stays on the signature and is not used to choose a size
is_apple_silicon stays on the signature and is not used to choose a size

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 318-03-SUMMARY.md

---

### No Cargo.toml edit and no new crate edge
No Cargo.toml edit and no new crate edge

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 318-03-SUMMARY.md

---

### select_from_free_mb calls select_qwen3_code_qlora_for_free_mb under the ...
`select_from_free_mb` calls `select_qwen3_code_qlora_for_free_mb` under the existing `runtime` feature.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 318-03-SUMMARY.md

---

### select_tier_for_vram and the mens/runs/qwen3_27b_metal_check/* and vox-m...
`select_tier_for_vram` and the `mens/runs/qwen3_27b_metal_check/*` and `vox-mens-8b-v0.6` map are deleted, including the re-exports.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 318-03-SUMMARY.md

---

### select_optimal_local_model returns Result, calls read_accelerator_free_m...
`select_optimal_local_model` returns `Result`, calls `read_accelerator_free_mb`, then `select_from_free_mb`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 318-03-SUMMARY.md

---

### resolve_chat_config_with_auto_pick uses an injected result for auto-loca...
`resolve_chat_config_with_auto_pick` uses an injected result for `auto-local`. `None` still calls the live function.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 318-03-SUMMARY.md

---

### The GUI command maps that Result
The GUI command maps that `Result`. The unit test injects Ok and Err and does not call the live command.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 318-03-SUMMARY.md

## Lessons

### No gap. The committed range implements one pick_base selector over measu...
No gap. The committed range implements one `pick_base` selector over measured `qwen3_code` QLoRA rows, reads CUDA free memory then Metal `memory_free_mb`, uses that pick for every unnamed spoke, re-reads before schola selects a train backend, refuses an explicit size that does not fit, and routes chat auto-local and the GUI host through the same selector. The directory-tier function is gone.

**Context:** Verified in phase acceptance checks.
**Source:** 318-VERIFICATION.md

---

### _Verified: 2026-10-29T22:03:14Z_ _Verifier: Claude (gsd-verifier)_
_Verified: 2026-10-29T22:03:14Z_ _Verifier: Claude (gsd-verifier)_

**Context:** Verified in phase acceptance checks.
**Source:** 318-VERIFICATION.md

## Patterns

### select_qwen3_code_qlora_for_free_mb is the only max-by-floor loop
select_qwen3_code_qlora_for_free_mb is the only max-by-floor loop

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 318-01-SUMMARY.md

---

### Largest measured QLoRA floor goes through pick_base
Largest measured QLoRA floor goes through pick_base

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 318-01-SUMMARY.md

---

### Explicit sizes are refused, never substituted
Explicit sizes are refused, never substituted

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 318-01-SUMMARY.md

---

### automatic_qwen3 is the last argument of schola run_train
automatic_qwen3 is the last argument of schola run_train

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 318-02-SUMMARY.md

---

### Unnamed size uses select_qwen3_code_qlora_for_free_mb
Unnamed size uses select_qwen3_code_qlora_for_free_mb

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 318-02-SUMMARY.md

---

### Named capability tags still pass None into resolve_base_model
Named capability tags still pass None into resolve_base_model

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 318-02-SUMMARY.md

---

### apply_preload_free_mb does not read the accelerator
apply_preload_free_mb does not read the accelerator

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 318-02-SUMMARY.md

---

### resolve_chat_config_with_auto_pick takes an injected Result
resolve_chat_config_with_auto_pick takes an injected Result

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 318-03-SUMMARY.md

---

### Injected free_mb selects; the live function only reads then calls that s...
Injected free_mb selects; the live function only reads then calls that selector

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 318-03-SUMMARY.md

---

### Chat tests pass Some(Ok) or Some(Err) and do not probe
Chat tests pass Some(Ok) or Some(Err) and do not probe

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 318-03-SUMMARY.md

## Surprises

### Estimate variance on 318-01
Estimate variance on 318-01: planned 48000 tokens vs 3984 actual (0.08x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 318-01-PLAN.md, 318-01-SUMMARY.md

---

### Estimate variance on 318-02
Estimate variance on 318-02: planned 52000 tokens vs 7579 actual (0.15x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 318-02-PLAN.md, 318-02-SUMMARY.md

---

### Estimate variance on 318-03
Estimate variance on 318-03: planned 42000 tokens vs 11296 actual (0.27x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 318-03-PLAN.md, 318-03-SUMMARY.md

