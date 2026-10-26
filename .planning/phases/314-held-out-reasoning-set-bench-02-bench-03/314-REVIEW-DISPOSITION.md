---
phase: 314
review: 314-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: open
    title: "Token cap is not recorded and a bad value silently falls back to 2048"
  - id: WR-02
    severity: warning
    disposition: deferred
    title: "Unclosed think is graded as a wrong answer with no report marker"
  - id: WR-03
    severity: warning
    disposition: deferred
    title: "MATH-500 golds are format-fragile under the locked string grader"
  - id: IN-01
    severity: info
    disposition: skipped
    title: "Bench name from the baseline file is joined into paths unsanitised"
  - id: IN-02
    severity: info
    disposition: skipped
    title: "A failed report write can leave an older scored report on disk"
  - id: IN-03
    severity: info
    disposition: skipped
    title: "Comma stripping makes gold 1,-2 equal 1-2"
  - id: IN-04
    severity: info
    disposition: skipped
    title: "Reasoning-bench routing depends on the process working directory"
open: 1
total: 7
recorded: 2026-10-26T23:55:00Z
---

# Phase 314: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | open | Fix in progress: a set but unparseable `VOX_REASONING_EVAL_MAX_TOKENS` must not silently become 2048. Unset still defaults to 2048. Values below 1 stay raised to 1, matching the registry text. |
| WR-02 | warning | deferred | The reviewer asked for a report marker, not a grading change. Phase 314 reuses `grade_final_answer` unchanged. An unclosed think already yields no answer. A truncation count belongs with the phase 316 comparison, not this slice check-in. |
| WR-03 | warning | deferred | MATH-500 content and the grader are locked. Equivalent surface forms are a known ceiling on the 32-item slice, not a fail-closed hole. |
| IN-01 | info | skipped | Operator-controlled path. The non-reasoning branch already joins the bench name. Below the fix scope. |
| IN-02 | info | skipped | A write error still returns `Err`, so the collateral gate does not treat the run as a score. Same note as the security audit. |
| IN-03 | info | skipped | Comma stripping is the locked thousands-separator rule from phase 313. Replacing the gold would edit the locked MATH-500 slice. |
| IN-04 | info | skipped | Relative `mens/data` matches the existing eval-local root. A missing file still fails closed. |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
