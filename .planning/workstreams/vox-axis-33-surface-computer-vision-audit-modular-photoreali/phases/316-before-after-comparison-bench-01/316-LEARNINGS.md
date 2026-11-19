---
phase: 316
phase_name: "before-after-comparison-bench-01"
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

# Phase 316 Learnings: before-after-comparison-bench-01

## Decisions

### Detected::Ready carries the resolved BaseLabel so the compared report ca...
Detected::Ready carries the resolved BaseLabel so the compared report can name the pin without changing run_before_after_with's signature

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 316-01-SUMMARY.md

---

### A weight-index shard that is absolute or contains .
A weight-index shard that is absolute or contains .. is incomplete, so a hostile index cannot leave the snapshot directory

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 316-01-SUMMARY.md

---

### The how-to paragraph names the command and mens/runs/reasoning_compare/ ...
The how-to paragraph names the command and mens/runs/reasoning_compare/ and contains no digits

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 316-01-SUMMARY.md

---

### vox mens eval-reasoning-compare writes mens/runs/reasoning_compare/<size...
`vox mens eval-reasoning-compare` writes `mens/runs/reasoning_compare/<size>-<rev12>.json`. On this checkout the real run took the absent branch and wrote `Qwen3-8B-b968826d9c46.json` with `status` `not_compared`, `reason` `base_snapshot_absent`, and `gate_input` false. The file has no `accuracy` or `think_initiation` keys.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 316-01-SUMMARY.md

---

### A present pin and adapter are scored through score_loaded on the three c...
A present pin and adapter are scored through `score_loaded` on the three checked-in slices. An incomplete score makes the report `incomplete` and the command returns an error after the write.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 316-01-SUMMARY.md

---

### base Qwen3-0.6B resolves through the existing train_bases overlay to Qwe...
`--base Qwen3-0.6B` resolves through the existing `train_bases` overlay to `Qwen/Qwen3-0.6B@c1899de289a04d12100db370d81485cdf75e47ca`. No new rung was added to `mens/config/gpu-specs.yaml`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 316-01-SUMMARY.md

---

### Detected::Ready includes base
`Detected::Ready` includes `base: BaseLabel`. The plan's enum omitted it, and `compared_report_json` needs the label while `run_before_after_with` keeps the signature `(detected, bench_root, out, score_side)`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 316-01-SUMMARY.md

---

### Shard paths in model.safetensors.index.json must be relative and must no...
Shard paths in `model.safetensors.index.json` must be relative and must not contain `..`. Anything else is an incomplete snapshot.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 316-01-SUMMARY.md

## Lessons

### None that blocked the plan
None that blocked the plan. The three pre-existing `vox-eval` clippy warnings in `qwen_reasoning.rs` and `trial_flywheel.rs` remain, and those files were not edited.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 316-01-SUMMARY.md

---

### verify.key-links returned unverified because each from is a symbol descr...
`verify.key-links` returned unverified because each `from` is a symbol description, not a relative path. Wiring was checked in source.

**Context:** Verified in phase acceptance checks.
**Source:** 316-VERIFICATION.md

---

### N/A — infrastructure/tooling phase
N/A — infrastructure/tooling phase. Acceptance is the absence file and the unit tests above. No user-facing UI.

**Context:** Verified in phase acceptance checks.
**Source:** 316-VERIFICATION.md

## Patterns

### Absence records use status not_compared and never include accuracy or th...
Absence records use status not_compared and never include accuracy or think_initiation_rate

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 316-01-SUMMARY.md

---

### Hub root order is --hub-root, HF_HUB_CACHE, HF_HOME/hub, then the user c...
Hub root order is --hub-root, HF_HUB_CACHE, HF_HOME/hub, then the user cache

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 316-01-SUMMARY.md

---

### HF cache layout detection without opening weight files
HF cache layout detection without opening weight files

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 316-01-SUMMARY.md

---

### injected SideScorer so the absent branch never loads a model
injected SideScorer so the absent branch never loads a model

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 316-01-SUMMARY.md

## Surprises

### Estimate variance on 316-01
Estimate variance on 316-01: planned 95000 tokens vs 19087 actual (0.20x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 316-01-PLAN.md, 316-01-SUMMARY.md

