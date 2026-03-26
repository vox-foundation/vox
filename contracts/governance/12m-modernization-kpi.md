# 12-month modernization — KPI & cadence (J01–J04)

This note implements governance tasks **J01–J04** from the deep-modernization program.

## J01 — Weighted dashboard (template)

| Workstream | Weight | Metric (example) | Source |
|------------|--------|------------------|--------|
| Parser/HIR | 20% | `legacy_ast_nodes` count on golden corpus; parse error class coverage | tests + inventory docs |
| Type/diagnostics | 15% | Diagnostics with `category` + docs parity | `diagnostic-taxonomy.md` |
| Docker/runtime | 15% | `doctor --probe` green in images; compose config CI | CI + `docker_healthcheck_contract` |
| Populi | 20% | HTTP tests green; body limit / A2A cap configured | `vox-populi` transport tests |
| Mens | 15% | `mens-gate` profile; docs-code alignment | `gates.yaml`, `mens-training.md` |
| Learnability | 15% | Broken-link checks; CONTRIBUTING path | doc gates |

**Program % complete** = completed tasks / 200 (from program backlog). Reconcile monthly.

## J02 — Monthly architecture review

- **When:** first Monday of the month (30 min).
- **Inputs:** failing gates, open ADRs, HIR graduation list, Populi threat model notes.
- **Output:** 5-bullet risks + owners; update `contracts/hir/legacy-baseline.toml` if graduates changed.

## J03 — RFC-lite for breaking changes

For user-visible language or wire-format breaks:

1. Issue or short ADR stub with **motivation**, **migration**, **rollback**.
2. Dual-write or feature flag window when feasible.
3. `vox ci command-compliance` + OpenAPI / registry updates before merge.

## J04 — Quarterly debt burndown

Link K-complexity proxy metrics (parse ambiguity reports, `legacy_ast_nodes`, control-plane incidents) to **one** concrete reduction objective per quarter (e.g. “remove N AST wrappers” or “close M doc SSOT gaps”). Track in the same table as J01.
