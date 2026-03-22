---
title: "GitHub-hosted runner exceptions"
category: ci
last_updated: 2026-03-21
---

# GitHub-hosted runner exceptions

The repository defaults to **self-hosted** runners for main Rust CI (see [runner contract](runner-contract.md)). The following workflows intentionally use **GitHub-hosted** runners:

| Workflow | Runner | Reason |
|----------|--------|--------|
| `docs-deploy.yml` | `ubuntu-latest` | GitHub Pages deploy + mdBook; portable Pages API. |
| `link_checker.yml` | `ubuntu-latest` | External link checks; no secrets to self-hosted pool. |

Any new workflow using `ubuntu-latest` must add a row here or switch to the self-hosted tuple.

**Not GitHub-hosted (self-hosted only):** [`ci.yml`](../../../.github/workflows/ci.yml) and [`ml_data_extraction.yml`](../../../.github/workflows/ml_data_extraction.yml) use **`[self-hosted, linux, x64]`** (plus **`docker`** / CUDA lanes per [runner contract](runner-contract.md)). They are listed here so agents do not mistake them for missing exceptions — see [workflow enumeration](workflow-enumeration.md) for step-level detail.
