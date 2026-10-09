---
status: testing
phase: 03-provenance-trust-tiers-and-leak-closure
source: [03-VERIFICATION.md]
started: 2026-10-09T00:00:00Z
updated: 2026-10-09T00:00:00Z
---

## Current Test

number: 1
name: Hosted CI is green on this branch
expected: |
  nextest for vox-db-types, vox-db, vox-search, vox-orchestrator-types, vox-orchestrator,
  vox-orchestrator-mcp, vox-gui, vox-ml-cli; clippy -D warnings on touched crates;
  vox ci ssot-drift; vox ci crate-edges; schema digest/baseline test for BASELINE_VERSION 96.
  The vox-orchestrator goal-search tests were not run locally and are affected by the
  WR-08 change (goal.rs now drops Untrusted lines).
awaiting: user response

## Tests

### 1. Hosted CI is green on this branch
expected: All gates above pass (requires pushing the branch / draft PR)
result: [pending]

### 2. GUI suite: Playwright kb-inbox-side-doors.spec.ts, vitest (MemoryView/DocPublishModal/ResearchView), pnpm typecheck
expected: Pass; screenshots land in crates/vox-gui/ui/review-bundle/latest/
result: [pending]

### 3. Decide upgrade-path residuals WR-01 and WR-02 (03-REVIEW.md)
expected: Either accept as pre-release (baseline 96 is on no remote branch), or schedule the fixes — run retier_agent_knowledge_nodes on every open outside the version gate, and make the baseline-96 backfill stamp Library rows untrusted so only a git-clean reindex promotes them
result: [pending]

## Summary

total: 3
passed: 0
issues: 0
pending: 3
skipped: 0
blocked: 0

## Gaps
