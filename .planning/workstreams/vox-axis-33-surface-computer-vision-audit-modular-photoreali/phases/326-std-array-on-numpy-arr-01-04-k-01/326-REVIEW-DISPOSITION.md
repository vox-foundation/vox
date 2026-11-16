---
phase: 326
review: 326-REVIEW.md
titles: json
findings:
  - id: CR-01
    severity: critical
    disposition: fixed
    title: "Binary plans drop the right-hand host"
  - id: CR-02
    severity: critical
    disposition: open
    title: "Typed dtypes materialize as float64"
  - id: CR-03
    severity: critical
    disposition: open
    title: "list[bool] typechecks, then the interpreter aborts"
  - id: WR-01
    severity: warning
    disposition: open
    title: "Interp-gap test still requires one to_list"
  - id: WR-02
    severity: warning
    disposition: open
    title: "Numeric roundtrip never enters the production reply arm"
  - id: WR-03
    severity: warning
    disposition: open
    title: "Arrow decode panics on a zero-column batch"
  - id: IN-01
    severity: info
    disposition: open
    title: "Plan-IR index still says seven nodes"
  - id: IN-02
    severity: info
    disposition: open
    title: "An empty list is always an f64 plan"
open: 7
total: 8
recorded: 2026-11-04T01:30:00Z
---

# Phase 326: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| CR-01 | critical | fixed | fix(326): keep each from_list host on binary array plans |
| CR-02 | critical | open | - |
| CR-03 | critical | open | - |
| WR-01 | warning | open | - |
| WR-02 | warning | open | - |
| WR-03 | warning | open | - |
| IN-01 | info | open | - |
| IN-02 | info | open | - |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
