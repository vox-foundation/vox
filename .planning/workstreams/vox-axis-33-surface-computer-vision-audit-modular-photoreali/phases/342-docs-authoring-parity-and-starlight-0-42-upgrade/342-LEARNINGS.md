---
phase: 342
phase_name: "docs-authoring-parity-and-starlight-0-42-upgrade"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 8
  lessons: 8
  patterns: 4
  surprises: 4
missing_artifacts:
  - "UAT.md"
---

# Phase 342 Learnings: docs-authoring-parity-and-starlight-0-42-upgrade

## Decisions

### Fixed links, not the remark-doc-links outside-root guard (T-342-04)
Fixed links, not the remark-doc-links outside-root guard (T-342-04)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 342-01-SUMMARY.md

---

### crate_link_cell stays private; no new public API
crate_link_cell stays private; no new public API

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 342-01-SUMMARY.md

---

### Approved mature-only
Approved mature-only: no minimumReleaseAgeExclude lines; caret-minor floors

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 342-02-SUMMARY.md

---

### Bare @astrojs/starlight/components alias re-pointed via import.meta.reso...
Bare @astrojs/starlight/components alias re-pointed via import.meta.resolve; sub-path alias deleted

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 342-02-SUMMARY.md

---

### Default alert mapping kept
Default alert mapping kept: NOTE note, TIP tip, IMPORTANT caution, WARNING caution, CAUTION danger

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 342-03-SUMMARY.md

---

### Two list-nested alerts made top-level (GitHub does not render nested ale...
Two list-nested alerts made top-level (GitHub does not render nested alerts)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 342-03-SUMMARY.md

---

### Validator runs on defaults (failOnError true); no warn-only phase becaus...
Validator runs on defaults (failOnError true); no warn-only phase because baseline is zero

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 342-04-SUMMARY.md

---

### Same-page #hash is excluded only when the page itself is .mdx (closes T-...
Same-page #hash is excluded only when the page itself is .mdx (closes T-342-12 hole)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 342-04-SUMMARY.md

## Lessons

### 1. [Info] Hub regeneration also changed L2 and the index. The generator ...
**1. [Info] Hub regeneration also changed L2 and the index.** The generator output for the committed `layers.toml` includes `vox-python-bridge` (added by an earlier phase-325 commit) and bumps the L2 crate count 17 to 18 in `index.generated.md`. Plan said to keep every resulting hub diff; kept.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 342-01-SUMMARY.md

---

### Otherwise: None - plan executed as written.
Otherwise: None - plan executed as written.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 342-01-SUMMARY.md

---

### Version floors use the checkpoint's caret-minor floors (^7.4.0 / ^0.42.0...
Version floors use the checkpoint's caret-minor floors (^7.4.0 / ^0.42.0 / ^7.3.0) instead of the plan's ^7.4.7 / ^0.42.9 / ^7.3.2, per the human's "mature-only" answer. Acceptance greps on `package.json` text for the higher floors are therefore superseded by the checkpoint resolution; the lockfile is the source of truth (above).

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 342-02-SUMMARY.md

---

### Tracer commit contains only Task 2 files; Task 3's three files were comm...
Tracer commit contains only Task 2 files; Task 3's three files were committed separately (as planned).

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 342-02-SUMMARY.md

---

### Otherwise: plan executed as written.
Otherwise: plan executed as written.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 342-02-SUMMARY.md

---

### Process notes
Process notes: the Claude Code process restarted mid-run (scratchpad wiped, root-pin guard missing); I stopped, the coordinator restored the guard, and the plan resumed from the build step.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 342-04-SUMMARY.md

---

### None. (Manual-only item from VALIDATION, the package-legitimacy checkpoi...
None. (Manual-only item from VALIDATION, the package-legitimacy checkpoint, is recorded as approved in the 342-02 SUMMARY; lockfile CI-pnpm compatibility is observable only on the first PR docs-quality run.)

**Context:** Verified in phase acceptance checks.
**Source:** 342-VERIFICATION.md

---

### No gaps. The committed code delivers every roadmap success criterion: As...
No gaps. The committed code delivers every roadmap success criterion: Astro 7.4.5 / Starlight 0.42.9 builds 907 pages, alerts render as asides with a guard spec, the links validator is fatal by default and passes with zero findings, and both original `remark-doc-links` failures are fixed at their sources.

**Context:** Verified in phase acceptance checks.
**Source:** 342-VERIFICATION.md

## Patterns

### generator fix + regeneration instead of hand-editing *.generated.md
generator fix + regeneration instead of hand-editing *.generated.md

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 342-01-SUMMARY.md

---

### caret-minor floors under the release-age gate; no minimumReleaseAgeExclu...
caret-minor floors under the release-age gate; no minimumReleaseAgeExclude

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 342-02-SUMMARY.md

---

### plugin registered first in Starlight plugins, no options (default GitHub...
plugin registered first in Starlight plugins, no options (default GitHub-parity mapping)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 342-03-SUMMARY.md

---

### exclude predicate limited to .mdx pages/routes + documented 127.0.0.1:11...
exclude predicate limited to .mdx pages/routes + documented 127.0.0.1:11434 probe URL

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 342-04-SUMMARY.md

## Surprises

### Estimate variance on 342-01
Estimate variance on 342-01: planned 45000 tokens vs 3500 actual (0.08x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 342-01-PLAN.md, 342-01-SUMMARY.md

---

### Estimate variance on 342-02
Estimate variance on 342-02: planned 60000 tokens vs 9000 actual (0.15x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 342-02-PLAN.md, 342-02-SUMMARY.md

---

### Estimate variance on 342-03
Estimate variance on 342-03: planned 40000 tokens vs 6000 actual (0.15x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 342-03-PLAN.md, 342-03-SUMMARY.md

---

### Estimate variance on 342-04
Estimate variance on 342-04: planned 55000 tokens vs 6000 actual (0.11x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 342-04-PLAN.md, 342-04-SUMMARY.md

