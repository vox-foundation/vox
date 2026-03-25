# Scaling CI enforcement rollout

## Modes

`toestub` / `vox ci toestub-scoped`:

| `--mode` | Exit behavior |
|----------|----------------|
| `legacy` (default) | Fail if any finding ≥ `Error` (unchanged historical behavior) |
| `audit` | Never fail; report `Info`+ (use with `--format json` for snapshots) |
| `enforce-warn` | Fail if any `Critical` |
| `enforce-strict` | Fail if any `Warning`+ |

## Recommended rollout

1. **Now:** `toestub-scoped` stays `legacy`; scaling findings are mostly `Warning`/`Info` so they surface without failing CI.
2. **After backlog burn-down:** run scoped paths with `enforce-strict` in optional workflows.
3. **Critical-only gate:** introduce targeted `Critical` rules (e.g. confirmed blocking HTTP without timeouts) and use `enforce-warn` on hot paths.

## Commands

- `vox ci scaling-audit verify` — schema + embedded policy parse.
- `vox ci scaling-audit emit-reports` — per-crate markdown + rollup + TOESTUB JSON snapshot under `contracts/reports/scaling-audit/`.

## SSOT

- Policy: `contracts/scaling/policy.yaml`
- Task templates: `contracts/scaling/task-templates.yaml`
- Contract index: `contracts/index.yaml` (`scaling-policy`, `scaling-policy-schema`)
