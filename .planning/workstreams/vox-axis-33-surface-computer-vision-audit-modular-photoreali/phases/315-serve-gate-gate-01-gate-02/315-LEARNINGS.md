---
phase: 315
phase_name: "serve-gate-gate-01-gate-02"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 18
  lessons: 3
  patterns: 6
  surprises: 2
missing_artifacts:
  - "UAT.md"
---

# Phase 315 Learnings: serve-gate-gate-01-gate-02

## Decisions

### Accuracy and think-initiation compared separately at 5% (strict >); exac...
Accuracy and think-initiation compared separately at 5% (strict >); exactly 5% passes

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 315-01-SUMMARY.md

---

### All six baseline keys required; any missing key is a hard error before s...
All six baseline keys required; any missing key is a hard error before scoring

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 315-01-SUMMARY.md

---

### NaN, infinity, pre <= 0, values outside [0, 1], n != 32, and unknown sli...
NaN, infinity, pre <= 0, values outside [0, 1], n != 32, and unknown slice names return Err, never 0.0

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 315-01-SUMMARY.md

---

### Stale collateral_damage_report.json removed as the first statement of th...
Stale collateral_damage_report.json removed as the first statement of the writer

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 315-01-SUMMARY.md

---

### failed_on: failing generic bench wins, else first failing reasoning key ...
failed_on: failing generic bench wins, else first failing reasoning key in SLICE_NAMES order, accuracy before _init

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 315-01-SUMMARY.md

---

### New pure module vox_eval::reasoning_slice
New pure module `vox_eval::reasoning_slice`: `reasoning_slice_of`, `require_reasoning_keys`, `eval_reasoning_slice`, `ReasoningSliceVerdict`, `reasoning_slices_json`, `REASONING_SLICE_SCOPE_NOTE`. It wraps `eval_collateral_damage` without editing it.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 315-01-SUMMARY.md

---

### Writer: BenchScore { value, n } from the scorer, six-key rule, stale-rep...
Writer: `BenchScore { value, n }` from the scorer, six-key rule, stale-report removal, strict comparison, `reasoning_slices` block, and `slice`/`n` on reasoning entries.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 315-01-SUMMARY.md

---

### ReasoningReport.n is now read by the writer; its dead-code allow is gone...
`ReasoningReport.n` is now read by the writer; its dead-code allow is gone.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 315-01-SUMMARY.md

---

### No model was run, no baseline number written, no env var or dependency a...
No model was run, no baseline number written, no env var or dependency added.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 315-01-SUMMARY.md

---

### Stored threshold must equal CollateralDamageConfig::default(); recompute...
Stored threshold must equal CollateralDamageConfig::default(); recompute always uses the default config

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 315-02-SUMMARY.md

---

### scope_note must equal REASONING_SLICE_SCOPE_NOTE exactly; slice_size and...
scope_note must equal REASONING_SLICE_SCOPE_NOTE exactly; slice_size and per-slice n must be 32

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 315-02-SUMMARY.md

---

### Marker condition manifest_path.exists() || adapter_path.exists() is unch...
Marker condition manifest_path.exists() || adapter_path.exists() is unchanged; only its body was replaced

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 315-02-SUMMARY.md

---

### No numeric equality between the collateral report and reasoning_report.j...
No numeric equality between the collateral report and reasoning_report.json (D-10): status scored and n 32 only

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 315-02-SUMMARY.md

---

### verify_reasoning_slices in vox-eval
`verify_reasoning_slices` in `vox-eval`: checks threshold, `scope_note`, `slice_size`, per-slice `n` and four numbers per slice, iterating `SLICE_NAMES`, then calls `eval_reasoning_slice` and refuses a recomputed drop naming `failed_key()`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 315-02-SUMMARY.md

---

### assert_collateral_gate in dispatch.rs
`assert_collateral_gate` in `dispatch.rs`: missing-report and FAILED messages kept verbatim, malformed JSON refused, per-slice report must be present, valid JSON, `status` `scored`, `n` 32. Reads only; no model, no write.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 315-02-SUMMARY.md

---

### Serve arm: the inline status check body is replaced by assert_collateral...
Serve arm: the inline status check body is replaced by `assert_collateral_gate(&model)?;` then the unchanged `assert_serve_preconditions(&model)?;`. The condition and marker files are untouched.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 315-02-SUMMARY.md

---

### How-to: names the six baseline keys, says a missing key is a hard error,...
How-to: names the six baseline keys, says a missing key is a hard error, and says serve re-checks the 32-item block and does not trust status alone. No numeric baseline written.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 315-02-SUMMARY.md

---

### No model was run, no score invented, no env var or dependency added.
No model was run, no score invented, no env var or dependency added.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 315-02-SUMMARY.md

## Lessons

### None blocking.
None blocking.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 315-01-SUMMARY.md

---

### The gate runs only when training_manifest.json or candle_qlora_adapter.s...
The gate runs only when `training_manifest.json` or `candle_qlora_adapter.safetensors` exists.

**Context:** Verified in phase acceptance checks.
**Source:** 315-VERIFICATION.md

---

### Serve checks slice status and n, not numeric equality with the collatera...
Serve checks slice `status` and `n`, not numeric equality with the collateral report. A consistent rewrite of both pre and post is a new baseline and is not detected as an edit (the how-to says so after WR-01).

**Context:** Verified in phase acceptance checks.
**Source:** 315-VERIFICATION.md

## Patterns

### Reasoning entries in reports[] carry slice true and n; generic entries c...
Reasoning entries in reports[] carry slice true and n; generic entries carry neither

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 315-01-SUMMARY.md

---

### Every report (pass and fail) carries reasoning_slices with scope_note, s...
Every report (pass and fail) carries reasoning_slices with scope_note, slice_size 32, and per-slice n

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 315-01-SUMMARY.md

---

### Strict wrapper around the unchanged generic eval_collateral_damage
Strict wrapper around the unchanged generic eval_collateral_damage: guards first, arithmetic once per metric

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 315-01-SUMMARY.md

---

### Reasoning keys classified by name against SLICE_NAMES, never by filesyst...
Reasoning keys classified by name against SLICE_NAMES, never by filesystem probe

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 315-01-SUMMARY.md

---

### Serve recomputes from stored pre/post numbers with the writer's own eval...
Serve recomputes from stored pre/post numbers with the writer's own eval_reasoning_slice; stored status, degradation_rate and exceeds_threshold are ignored

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 315-02-SUMMARY.md

---

### Slice paths built only from SLICE_NAMES, never from report content
Slice paths built only from SLICE_NAMES, never from report content

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 315-02-SUMMARY.md

## Surprises

### Estimate variance on 315-01
Estimate variance on 315-01: planned 70000 tokens vs 7000 actual (0.10x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 315-01-PLAN.md, 315-01-SUMMARY.md

---

### Estimate variance on 315-02
Estimate variance on 315-02: planned 55000 tokens vs 9000 actual (0.16x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 315-02-PLAN.md, 315-02-SUMMARY.md

