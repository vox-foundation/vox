---
title: "Privileged preview trust-split"
description: "Policy for workflows that deploy PR code with secrets vs trusted teardown that never executes PR code (t3code relay-preview pattern)."
category: "CI & Quality"
status: "current"
topic: "ci-guards"
domain: "platform"
authorship: "llm-assisted"
authorship_model: "claude-opus-4-6"
---

# Privileged preview trust-split

<!-- kb:summary -->
Documents the privileged preview trust split for CI and deployment: which surfaces may accept elevated trust, how preview credentials are scoped, and the guardrails that keep privileged paths out of untrusted PR previews. Used by operators configuring CI trust boundaries and by agents auditing preview exposure.
<!-- /kb:summary -->


Vox adopts the t3code `relay-preview` / `relay-preview-cleanup` split:

1. **Deploy / privileged mutate** may check out and run **PR head** code only when gated by a GitHub Environment requiring reviewers (or an equivalent human gate) and a same-repo label.
2. **Cleanup / teardown** must use `pull_request_target` (or `workflow_run` from a trusted publisher), check out the **default branch only**, and **never** execute scripts from the PR tree while holding secrets.

## Audit

`vox ci preview-trust-audit` scans `.github/workflows/*.yml` for high-risk combinations:

- `pull_request_target` **and** checkout of `pull_request.head` / PR ref **and** secret usage → fail unless allowlisted.
- Missing documentation pointer for any new privileged preview workflow.

Allowlist entries live in `contracts/ci/preview-trust-allowlist.v1.yaml` with `owner` + `reason`.
