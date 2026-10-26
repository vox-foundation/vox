---
title: "Suppression / toestub-ignore human review"
description: "PRs that only add lint suppressions require human review label or CODEOWNERS approval."
category: "contributor"
status: "current"
---

# Suppression human review

## Policy

- Every `toestub-ignore(...)`, `#[allow(...)]`, `@ts-expect-error`, and eslint/oxlint disable must carry an inline reason (enforced by `vox/lint/unreasoned-suppression`).
- PRs whose **only** substantive changes are suppressions / `contracts/toestub/suppressions.v1.json` entries require:
  1. Label `needs-human-review`, **or**
  2. Approval from CODEOWNERS on `contracts/toestub/**`

## CI advisory

`vox ci suppression-review-gate` exits 0 with a warning when the changed set looks suppression-only and neither label env nor acknowledgement is present. Merge bots may treat the warning as blocking later; today it is advisory.

## CODEOWNERS

`contracts/toestub/` is owned for review routing in `.github/CODEOWNERS`.
