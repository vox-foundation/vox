---
phase: 258
phase_name: "router-benchmark-priors-and-quality-dpo-flywheel"
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

# Phase 258 Learnings: router-benchmark-priors-and-quality-dpo-flywheel

## Decisions

### Execute Phase 258 (T3-ROUTER-DPO-FLY-01..04)
Execute Phase 258 (`T3-ROUTER-DPO-FLY-01..04`): Wire `ExternalBenchmarkPrior` (`SweBench`/`LiveBench`/`SkateBench`/`SnitchBench`) and `ResponseQualityAnalyzer` (`0..=100` Flesch-Kincaid + 19-connector score) into `ModelRegistry` (`registry.rs`), `ModelSelectionEngine` (`scoring.rs`), and live Auto-DPO preference pair harvesting (`agent_loop.rs`).

**Rationale:** Primary phase objective established in plan.
**Source:** 258-01-PLAN.md

## Lessons

## Patterns

## Surprises

