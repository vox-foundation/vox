---
phase: 61
phase_name: "external-adoption-git-rev-codegen-binary-release"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 1
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 61 Learnings: external-adoption-git-rev-codegen-binary-release

## Decisions

### [MODEL: ARGON-4] [PARALLEL-SAFE] Eliminate the external adopter blocker ...
`[MODEL: ARGON-4]` `[PARALLEL-SAFE]` Eliminate the external adopter blocker where `vox build` emitted relative `../../crates/` paths when run from a standalone `voxup` binary on a machine without a local Vox monorepo clone, by adding a deterministic `git` + `rev`/`tag` dependency fallback tier in `crates/vox-codegen/src/codegen_rust/emit/mod.rs`.

**Rationale:** Primary phase objective established in plan.
**Source:** 61-01-PLAN.md

## Lessons

## Patterns

## Surprises

