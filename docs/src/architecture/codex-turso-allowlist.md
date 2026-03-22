---
title: "Direct Turso usage allowlist"
category: architecture
last_updated: 2026-03-20
---

# Direct `turso::` usage allowlist

ADR 004 discourages direct `turso::` usage outside the data-plane crates. In practice, the workspace still contains direct calls in CLI helpers, tests, and integration code. For the full API/env contract, see [Codex / Arca compatibility boundaries](codex-arca-compatibility-boundaries.md).

## Allowed (by design)

| Area | Rationale |
|------|-----------|
| **`vox-pm`** | Owns `CodeStore` and SQL connection lifecycle. |
| **`vox-db`** | Facade over `CodeStore`; may use Turso types in public helpers. |
| **`vox-cli`** | Sample/diagnostic SQL and params (`turso::params!`, `Value`) against the user DB. |
| **Tests / `vox-integration-tests`** | Fixture and contract tests. |

## Goal

Reduce new direct `turso::` surface: application features should call **`VoxDb`** / **`CodeStore`** APIs. When adding a new direct call, document the exception in this file or add a narrow helper on `vox-db` / `vox-pm`.

## Verification

Periodically run `rg "turso::" crates/` and reconcile with this policy. CI may add a guard listing unexpected crates in the future.
