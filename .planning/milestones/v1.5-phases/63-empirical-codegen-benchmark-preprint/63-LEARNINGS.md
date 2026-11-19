---
phase: 63
phase_name: "empirical-codegen-benchmark-preprint"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 1
  lessons: 2
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 63 Learnings: empirical-codegen-benchmark-preprint

## Decisions

### [MODEL: ARGON-4] [SEQUENTIAL] Execute the first reproducible cross-model...
`[MODEL: ARGON-4]` `[SEQUENTIAL]` Execute the first reproducible cross-model comparative trial (`vox model eval-compare`) testing Vox's central hypothesis (`.vox` vs. TypeScript under multi-turn compiler feedback), wire the `CR-L8` diagnostic→repair→corpus feedback flywheel (`vox.lint.*` + `vox.repair.*` → `vox-corpus/src/flywheel.rs`) and `CR-L2`/`CR-L3` panel measurements from [`vox-as-llm-target-audit-and-plan-2026.md`](../../../../docs/src/architecture/vox-as-llm-target-audit-and-plan-2026.md), and publish the signed provenance report and SCIENTIA manuscript so grant applications and community announcements cite measured empirical data.

**Rationale:** Primary phase objective established in plan.
**Source:** 63-01-PLAN.md

## Lessons

### vox.lint.* and vox.repair.* telemetry outcomes are wired into vox-corpus...
`vox.lint.*` and `vox.repair.*` telemetry outcomes are wired into `vox-corpus::flywheel` (`LintFeedbackRecord`, `RepairFeedbackRecord`) and event sink `contracts/reports/corpus-feedback-events/`, enabling live CR-L8 feedback.

**Context:** Verified in phase acceptance checks.
**Source:** 63-VERIFICATION.md

---

### RepairCorpusSubcommand is implemented and registered in vox-audit for CR...
`RepairCorpusSubcommand` is implemented and registered in `vox-audit` for CR-L3, evaluating 50 project fixtures.

**Context:** Verified in phase acceptance checks.
**Source:** 63-VERIFICATION.md

## Patterns

## Surprises

