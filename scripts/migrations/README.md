# scripts/migrations/

One `.vox` file per **numbered migration item** from
[data-storage-migration-backlog-2026.md](../../docs/src/architecture/data-storage-migration-backlog-2026.md).

## Naming

`YYYY-phase{N}-{slug}.vox`

## Conventions

- Every script is idempotent. A second run on a completed migration is a no-op.
- Every script supports `--check` / `--dry-run` (prints what would change, exits non-zero on divergence).
- Every script has a header comment block naming the migration item ID, SSOT finding IDs, and the CI check(s) that verify its post-condition.
- Scripts are executed via `vox run scripts/migrations/<name>.vox` and committed to VCS before an agent invokes them (per AGENTS.md §VoxScript-First Glue Code).

## Not for

- Runtime data movement during normal agent sessions — use `vox db` subcommands instead.
- Anything that modifies production data outside a migration window.

## See also

- [AGENTS.md §VoxScript-First Glue Code](../../AGENTS.md)
- [data-storage-ssot-2026.md](../../docs/src/architecture/data-storage-ssot-2026.md)
