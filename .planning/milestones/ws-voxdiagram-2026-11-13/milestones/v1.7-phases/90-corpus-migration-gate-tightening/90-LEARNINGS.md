---
phase: 90
phase_name: "corpus-migration-gate-tightening"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 5
  lessons: 2
  patterns: 4
  surprises: 6
missing_artifacts:
  - "UAT.md"
---

# Phase 90 Learnings: corpus-migration-gate-tightening

## Decisions

### A word of 29-48 cells keeps a line of its own and widens its box (Mermai...
A word of 29-48 cells keeps a line of its own and widens its box (Mermaid behaviour); only a longer word is hard-broken, at 48 cells

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 90-01-SUMMARY.md

---

### The 02-wrap golden case now exercises hard breaking through its CJK labe...
The 02-wrap golden case now exercises hard breaking through its CJK label only; its ASCII overlong word (40 chars) is whole

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 90-01-SUMMARY.md

---

### The figure id uniqueness check only flags a duplicate that involves an i...
The figure id uniqueness check only flags a duplicate that involves an id inside a figure; a page's own prose anchors are not the diagram's concern

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 90-02-SUMMARY.md

---

### A container fed only from outside is laid after its feeder along the flo...
A container fed only from outside is laid after its feeder along the flow (top-down and bottom-up); sideways layouts are unchanged (they already give titles no edges)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 90-03-SUMMARY.md

---

### Gantt reason lives in an HTML comment above the fence and one sentence i...
Gantt reason lives in an HTML comment above the fence and one sentence in the reference page; the generated baseline YAML has no reason field and was not touched (orchestrator decision)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 90-06-SUMMARY.md

## Lessons

### #### 1. Hosted CI on the PR
#### 1. Hosted CI on the PR

**Context:** Verified in phase acceptance checks.
**Source:** 90-VERIFICATION.md

---

### Test: Open the PR and let docs-quality, docs-deploy and the Linux fonts ...
**Test:** Open the PR and let docs-quality, docs-deploy and the Linux fonts lane run. **Expected:** All green. **Why human:** Deferred to the PR by decision; every local gate ran on macOS.

**Context:** Verified in phase acceptance checks.
**Source:** 90-VERIFICATION.md

## Patterns

### Fidelity = id-free canonical sets vs the oracle-proven golden, plus expo...
Fidelity = id-free canonical sets vs the oracle-proven golden, plus export/import round trip

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 90-01-SUMMARY.md

---

### Contact sheet per stem
Contact sheet per stem: pinned Mermaid render beside the built site's figure (light, dark) plus the as-shipped column PNG

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 90-01-SUMMARY.md

---

### Hand-converted block = frozen block minus backticks (tr -d) as an import...
Hand-converted block = frozen block minus backticks (tr -d) as an import case; HAND table maps the migrated fence to that golden

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 90-02-SUMMARY.md

---

### title_links also links each outside feeder of a container's entry nodes ...
title_links also links each outside feeder of a container's entry nodes to its title, so the title (and the frame) lies after the feeder in the flow

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 90-03-SUMMARY.md

## Surprises

### Estimate variance on 90-01
Estimate variance on 90-01: planned 110000 tokens vs 15200 actual (0.14x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 90-01-PLAN.md, 90-01-SUMMARY.md

---

### Estimate variance on 90-02
Estimate variance on 90-02: planned 100000 tokens vs 4800 actual (0.05x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 90-02-PLAN.md, 90-02-SUMMARY.md

---

### Estimate variance on 90-03
Estimate variance on 90-03: planned 100000 tokens vs 3500 actual (0.04x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 90-03-PLAN.md, 90-03-SUMMARY.md

---

### Estimate variance on 90-04
Estimate variance on 90-04: planned 110000 tokens vs 3800 actual (0.03x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 90-04-PLAN.md, 90-04-SUMMARY.md

---

### Estimate variance on 90-05
Estimate variance on 90-05: planned 110000 tokens vs 4200 actual (0.04x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 90-05-PLAN.md, 90-05-SUMMARY.md

---

### Estimate variance on 90-06
Estimate variance on 90-06: planned 70000 tokens vs 400 actual (0.01x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 90-06-PLAN.md, 90-06-SUMMARY.md

