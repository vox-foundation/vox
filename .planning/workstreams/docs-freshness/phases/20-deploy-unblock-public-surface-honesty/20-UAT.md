---
status: testing
phase: 20-deploy-unblock-public-surface-honesty
source: [20-VERIFICATION.md]
started: 2026-10-09T17:40:00Z
updated: 2026-10-09T17:40:00Z
---

## Current Test

number: 1
name: DEPLOY-02 live failure-escalation drill
expected: |
  `gh workflow run docs-deploy.yml --ref main -f simulate_failure=true`, run twice (wait for each).
  Exactly one open `docs-deploy-broken` issue (labels `docs-deploy-broken` + `nightly-failure`,
  assigned brbrainerd, pinged once); after the second failure its body shows `consecutive: 2` and
  the comment count is unchanged. The next green push-triggered deploy closes it with one
  "Recovered:" comment.
awaiting: user response

## Tests

### 1. DEPLOY-02 live failure-escalation drill
expected: Two simulated failures produce one issue edited in place (`consecutive: 2`, no new comments); the next green push deploy closes it with one "Recovered:" comment.
result: [pending]
note: Deferred by the user on 2026-10-09 when phase 20 closed. The liveness half of DEPLOY-02 was verified live (#636 opened and closed); the notify script passed locally against a stubbed `gh`.

## Summary

total: 1
passed: 0
issues: 0
pending: 1
skipped: 0
blocked: 0

## Gaps
