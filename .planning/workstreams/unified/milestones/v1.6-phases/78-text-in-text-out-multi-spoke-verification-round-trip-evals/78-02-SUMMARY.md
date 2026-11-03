---
phase: 78-text-in-text-out-multi-spoke-verification-round-trip-evals
plan: 02
subsystem: corpus
tags: [mens, eval, roundtrip, mcp, agent-harness, scorecard]
requires: [78-01]
provides:
  - McpToolSpec, verify_mcp_roundtrip
  - AgentHarnessSpec, verify_agent_harness_roundtrip
  - MensSpokeScorecardV1, SpokeScorecardEntry, SpokeEvalTask, evaluate_multi_spoke_suite, canonical_five_spoke_golden_suite
  - contracts/reports/mens-spoke-scorecard.v1.json
affects: [crates/vox-corpus, contracts/reports]
key-files:
  modified:
    - crates/vox-corpus/src/training/spoke_roundtrip.rs
    - contracts/reports/mens-spoke-scorecard.v1.json
key-decisions:
  - "Standardized MensSpokeScorecardV1 with schema_version mens_spoke_scorecard_v1 and both JSON and Markdown table outputs."
requirements-completed: [SPOKE-04, SPOKE-05, SPOKE-06]
completed: 2026-10-16
---

# Phase 78 Plan 02: MCP Tool-Calling, Agent Harness & Unified Multi-Spoke Scorecard Summary

## Accomplishments
- Implemented `McpToolSpec` and `verify_mcp_roundtrip` (`SPOKE-04`) validating single or multi-step MCP tool invocations and goal completion.
- Implemented `AgentHarnessSpec` and `verify_agent_harness_roundtrip` (`SPOKE-05`) validating `TaskPlan` JSON structure, file-scope boundaries, token/step budgets, and verification gates.
- Implemented `MensSpokeScorecardV1`, `evaluate_multi_spoke_suite`, `canonical_five_spoke_golden_suite`, and `contracts/reports/mens-spoke-scorecard.v1.json` (`SPOKE-06`).
