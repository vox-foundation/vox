# Coolify / Docker (Codex BaaS template)

Template for self-hosting **Codex**-style HTTP API workloads on **Turso** via Coolify. This is **not** the same stack as the repo-root [`docker-compose.yml`](../../docker-compose.yml) (which builds the **`vox` MCP** image from the root [`Dockerfile`](../../Dockerfile)).

## Which image?

| Goal | Image / build | Default command / port |
|------|----------------|-------------------------|
| **MCP HTTP** (this repo’s `Dockerfile`) | Build from repo root; optional `VOX_CLI_FEATURES` for mens | `CMD ["vox","mcp"]` — **3000** |
| **Codex API** (this template) | Set **`VOX_CODEX_IMAGE`** to your CI-built service image | Your service must listen on the port Coolify maps (template uses **8080**) |

If you point `VOX_CODEX_IMAGE` at the **`vox` MCP** image without changing `command`/`ports`, health checks and routing will not match. Either supply a dedicated Codex API image or replace this compose with the MCP-focused file from [`deployment-compose-ssot.md`](../../docs/src/architecture/deployment-compose-ssot.md).

## Environment

Set in Coolify or `.env` (never commit tokens). Coolify distinguishes **build-time** vs **runtime** variables and supports “literal” values to avoid `$` interpolation issues — see [Environment variables](https://coolify.io/docs/knowledge-base/environment-variables) and [Docker Compose in Coolify](https://coolify.io/docs/knowledge-base/docker/compose).

| Variable | Description |
|----------|-------------|
| `VOX_DB_URL` | Turso / libSQL HTTP URL |
| `VOX_DB_TOKEN` | Auth token |
| `VOX_DB_PATH` | Optional local path (dev or embedded replica local file) |
| `VOX_CODEX_IMAGE` | Image reference for the `codex-api` service (your build) |

Optional object storage (future R2 adapter): `R2_*` variables as in [Codex BaaS doc](../../docs/src/architecture/codex-baas.md).

## Compose

See [`docker-compose.yml`](docker-compose.yml). For **mens** alongside other services, merge env from [`docker/vox-compose-mens-environment.block.yaml`](../../docker/vox-compose-mens-environment.block.yaml) and follow [mens SSOT](../../docs/src/architecture/mens-ssot.md).

## Related

- [Deployment compose SSOT](../../docs/src/architecture/deployment-compose-ssot.md)
- [ADR 004: Codex / Turso](../../docs/src/adr/004-codex-arca-turso-ssot.md)
