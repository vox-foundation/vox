---
title: "Privileged preview trust-split"
description: "Policy for workflows that deploy PR code with secrets vs trusted teardown that never executes PR code (t3code relay-preview pattern)."
category: "CI & Quality"
status: "current"
topic: "ci-guards"
domain: "platform"
authorship: "llm-generated"
authorship_model: "unknown"
---

# Privileged preview trust-split

<!-- kb:summary -->
Policy for GitHub workflows that deploy pull-request code with secrets. Deploying may run the
PR head only behind a human-approved environment and a same-repository label. Cleanup and
teardown must run from the default branch through `pull_request_target` or a trusted
`workflow_run`, and must never execute scripts from the PR tree while holding secrets.
`vox ci preview-trust-audit` scans the workflows for the risky combination of
`pull_request_target`, a checkout of the PR head and secret use, and fails unless an entry with
an owner and reason exists in the preview-trust allowlist.
<!-- /kb:summary -->

Vox adopts the t3code `relay-preview` / `relay-preview-cleanup` split:

1. **Deploy / privileged mutate** may check out and run **PR head** code only when gated by a GitHub Environment requiring reviewers (or an equivalent human gate) and a same-repo label.
2. **Cleanup / teardown** must use `pull_request_target` (or `workflow_run` from a trusted publisher), check out the **default branch only**, and **never** execute scripts from the PR tree while holding secrets.

## Audit

`vox ci preview-trust-audit` scans `.github/workflows/*.yml` for high-risk combinations:

- `pull_request_target` **and** checkout of `pull_request.head` / PR ref **and** secret usage → fail unless allowlisted.
- Missing documentation pointer for any new privileged preview workflow.

Allowlist entries live in `contracts/ci/preview-trust-allowlist.v1.yaml` with `owner` + `reason`.
