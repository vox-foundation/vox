---
phase: 315
review: 315-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: open
    title: "How-to overclaims that every hand-edited pass is refused"
  - id: WR-02
    severity: warning
    disposition: deferred
    title: "Token cap is not recorded on the reasoning report"
  - id: WR-03
    severity: warning
    disposition: deferred
    title: "scope_note is compared as exact text"
  - id: WR-04
    severity: warning
    disposition: open
    title: "Out-of-range serve test trips on JSON parse, not the range guard"
  - id: IN-01
    severity: info
    disposition: skipped
    title: "Bad reasoning baselines are rejected after scoring"
  - id: IN-02
    severity: info
    disposition: skipped
    title: "Refusal text says re-run even for a real drop"
  - id: IN-03
    severity: info
    disposition: skipped
    title: "_init suffix applies to any bench name"
open: 2
total: 7
recorded: 2026-10-26T18:20:00Z
---

# Phase 315: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | open | Narrow the how-to. A flipped status, a missing slice block, or numbers that recompute past 5% are refused. A consistent rewrite of pre and post is a new baseline and is not detected as an edit. |
| WR-02 | warning | deferred | Phase 314 kept the cap off the report. Recording it, and comparing caps at serve, belongs with the phase 316 before/after. Values below 1 stay raised to 1, matching the registry. |
| WR-03 | warning | deferred | Exact `scope_note` is the plan contract. Rewording the constant is a future change, and old reports would need a new eval. The match stays exact. |
| WR-04 | warning | open | The serve test must use a JSON number outside [0, 1], such as 1.5, so deleting the range guard fails the test. `1e999` is a parse error and does not prove the guard. |
| IN-01 | info | skipped | A bad baseline still writes no report, because the stale file is removed first and the error returns before the new report. Scoring first is wasted work, not a pass. |
| IN-02 | info | skipped | Below the fix scope. The operator still cannot serve. |
| IN-03 | info | skipped | The `_init` suffix is the existing score_bench rule. The six required keys are still checked by name. |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
