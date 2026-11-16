---
name: gsd-code-review
description: "Review source files changed during a phase for correctness, security, layer architecture, and code quality."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Verification"
---

# GSD Code Review

> **Workflow Stage:** Verification  
> **CLI:** `vox audit arch`  
> **MCP:** `vox_gsd_list_documents`

## Objective

Review source files changed during a phase for correctness, security, layer architecture, and code quality.

## Artifacts Managed

- `.planning/phases/<phase>/REVIEW.md`

## Execution Protocol

1. Inspect changed files against `layers.toml`, file/block LoC limits, secret/crypto SSOT rules, and test coverage.
2. Verify behavior assertions rather than shape-only checks.
3. Record review verdict and actionable findings in `REVIEW.md`.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-verify-work`
