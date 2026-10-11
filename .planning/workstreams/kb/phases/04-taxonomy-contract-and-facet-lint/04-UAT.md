---
status: testing
phase: 04-taxonomy-contract-and-facet-lint
source: [04-VERIFICATION.md]
started: 2026-10-11T00:00:00Z
updated: 2026-10-11T00:00:00Z
---

## Current Test

number: 1
name: Hosted CI is green on the phase PR
expected: |
  `linux` job (fast tier including the facet backlog tighten-only gate, ssot-drift including kb_facets_ts,
  clippy on affected crates), `tests` shards and `ui` all pass. The backlog gate was exercised locally only by
  unit tests plus a real-git test, never against a real base ref. Requires pushing the branch / opening a
  draft PR (the plan 04-05..04-08 push rule is now satisfied: the seeded backlog is committed).
awaiting: user response

## Tests

### 1. Hosted CI is green on the phase PR
expected: All gates above pass
result: [pending]

### 2. GUI vitest and typecheck
expected: `pnpm --dir crates/vox-gui/ui test` (kbFacets.generated.test.ts) and `pnpm --dir crates/vox-gui/ui typecheck` pass; kbFacets.generated.ts enum/key values equal contracts/documentation/kb-facets.v1.schema.json
result: [pending]

### 3. Decide TAX-07 `sources[]` scope
expected: ROADMAP success criterion 1 reads "sources[] entry without url/date/hash" (unscoped), while REQUIREMENTS TAX-07 and CONTEXT scope the required url/date/hash check to `origin: web-promoted` pages. The code follows the requirement (missing fields fail only on promoted pages; malformed date or hash fail on any page). Keep promoted-only, or also hard-fail any page with an incomplete sources[] entry?
result: [pending]

## Summary

total: 3
passed: 0
issues: 0
pending: 3
skipped: 0
blocked: 0

## Gaps
