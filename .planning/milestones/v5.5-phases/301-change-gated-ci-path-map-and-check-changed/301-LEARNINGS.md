---
phase: 301
phase_name: "change-gated-ci-path-map-and-check-changed"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 1
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "VERIFICATION.md"
  - "UAT.md"
  - "STATE.md"
---

# Phase 301 Learnings: change-gated-ci-path-map-and-check-changed

## Decisions

### Ship contracts/ci/changed-path-owners.v1.yaml and vox ci check-changed s...
Ship `contracts/ci/changed-path-owners.v1.yaml` and `vox ci check-changed` so inert diffs exit 0 without cargo, and non-inert diffs emit an owner/gate plan (and optionally run a minimal gate set).

**Rationale:** Primary phase objective established in plan.
**Source:** 301-01-PLAN.md

## Lessons

## Patterns

## Surprises

