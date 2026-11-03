---
title: MCP HTTP gateway contract
description: OpenAPI contract and operational guardrails for the optional MCP HTTP and WebSocket gateway.
category: "Language Reference"

schema_type: "TechArticle"
---

# MCP HTTP gateway contract

Machine-readable contract for the optional MCP HTTP/WebSocket gateway lives at:

**[`contracts/mcp/http-gateway.openapi.yaml`](../../../contracts/mcp/http-gateway.openapi.yaml)** (from repo root)

This surface is emitted by `vox-mcp` only when `VOX_MCP_HTTP_ENABLED=1` and is intentionally bounded for remote/mobile operations.

## Guardrails

- Auth: bearer token unless explicitly bypassed for local testing (`Write` via `VOX_MCP_HTTP_BEARER_TOKEN`, optional `Read` via `VOX_MCP_HTTP_READ_BEARER_TOKEN`). Cloudless hard-cut target is Clavis-managed token resolution with env retained only for compatibility in non-strict profiles.
- Tool calls: allowlisted (`VOX_MCP_HTTP_ALLOWED_TOOLS`)
- Read-role tool scope: canonical MCP registry metadata (`http_read_role_eligible`) intersected with `VOX_MCP_HTTP_ALLOWED_TOOLS`; optional `VOX_MCP_HTTP_READ_ROLE_ALLOWED_TOOLS` narrows further
- Policy observability: `GET /v1/info` includes `allowed_tools` and effective `read_role_allowed_tools`
- Rate limiting: per-client identity budget (`VOX_MCP_HTTP_RATE_LIMIT_PER_MINUTE`)
- Optional reverse-proxy requirement: `X-Forwarded-Proto: https`

## Reverse proxy / TLS termination

- Keep gateway bind local/private (`VOX_MCP_HTTP_HOST`) and expose public ingress through a trusted TLS terminator.
- If strict forwarded-HTTPS enforcement is desired, set `VOX_MCP_HTTP_REQUIRE_FORWARDED_HTTPS=1` and ensure proxy injects `X-Forwarded-Proto: https`.
- Only enable `VOX_MCP_HTTP_TRUST_X_FORWARDED_FOR=1` when requests cannot bypass the trusted proxy layer.
- Configure proxy WebSocket pass-through for `/v1/ws` upgrade traffic.

## Network bind policy and discovery

- Loopback (`127.0.0.0/8`, `::1`) is the default bind. Any other host, or
  `VOX_MCP_HTTP_BEHIND_TUNNEL=1`, requires `VOX_MCP_HTTP_BEARER_TOKEN`,
  `VOX_MCP_HTTP_READ_BEARER_TOKEN`, and TLS (`VOX_MCP_HTTP_TLS_CERT`/`_KEY`) or the tunnel
  attestation; `VOX_MCP_HTTP_ALLOW_UNAUTHENTICATED` is rejected there. Startup refuses otherwise.
- Caller loopback is the TCP peer address; behind a tunnel no caller is loopback. Off loopback,
  `internal` tools are never served and `POST /api/v2/mesh/nodes/{id}/kill` is refused.
- `GET /v1/profile` returns the active profile, booted/failed subsystems, `axis_surfaces`, server
  version, and the tools the calling token may use. See
  [Running Vox as a server](../how-to/how-to-run-vox-as-a-server.md).

## Related

- [Crate API: vox-mcp](../reference/cli.md)
- [MCP tool registry (contract SSOT)](mcp-tool-registry-contract.md)
- [MCP HTTP read-role governance contract](mcp-http-read-role-governance-contract.md)
- [Environment variables (SSOT)](env-vars.md)
- [`contracts/README.md`](../../../contracts/README.md)
