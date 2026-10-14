---
title: "Running Vox as a server"
description: "Run a scoped Vox capability profile as an HTTP/MCP server on loopback, on a LAN with TLS, or behind Tailscale or Cloudflare Tunnel, and discover what it serves with GET /v1/profile."
category: "How-To Guides"
status: "current"
---

# Running Vox as a server

`vox serve --profile <ids>` starts the Vox daemon with the MCP HTTP gateway and only the
capability slice you name (for example `models`, `knowledge`, `mesh`; see
[Capability profiles](../reference/capability-profiles.md) for the full list). This page covers
where it listens and who can call what.

## Threat model in one paragraph

A Vox server is **single tenant**: one operator, any number of clients, two shared bearer
tokens. The **write token** can call `public` and `operator` tools; the **read token** can call
only `public` tools marked read-eligible. Anyone holding a token acts as the operator — there are
no per-user identities, quotas, or audit separation. `internal` tools (agent gateways, approval
allowlists, emergency stop, secrets diagnostics) are served **only to loopback callers**, whatever
the profile says. "Loopback" is decided by the TCP peer address, never by a header.

## Create the tokens

`vox serve` refuses to start without at least one gateway token, and any network bind needs both.
Keep them in the Clavis vault (never in shell history or `.env`):

```bash
openssl rand -base64 32 | vox secrets set VOX_MCP_HTTP_BEARER_TOKEN --stdin
openssl rand -base64 32 | vox secrets set VOX_MCP_HTTP_READ_BEARER_TOKEN --stdin
```

## 1. Loopback (default)

```bash
vox serve --profile models
```

Binds `127.0.0.1:3921`. Nothing on the network can reach it.

## 2. LAN with TLS

Bind a LAN address with a PEM certificate and key:

```bash
vox serve --profile models,knowledge \
  --bind 0.0.0.0:3921 \
  --tls-cert /etc/vox/tls/cert.pem --tls-key /etc/vox/tls/key.pem
```

The gateway terminates TLS itself (rustls). Clients must trust the certificate.

## 3. LAN behind Tailscale or Cloudflare Tunnel

If a tunnel already encrypts traffic, attest it instead of configuring TLS:

```bash
# Tailscale: bind the tailnet address; WireGuard encrypts the hop.
vox serve --profile mesh --bind 100.101.102.103:3921 --behind-tunnel

# Cloudflare Tunnel, `tailscale serve`, or a same-host reverse proxy: stay on loopback.
vox serve --profile research --behind-tunnel
```

`--behind-tunnel` makes the gateway treat **every** caller as remote. A same-host tunnel
delivers remote traffic from `127.0.0.1`, so loopback trust would otherwise hand `internal`
tools to the internet. Both tokens are therefore required even on a loopback bind. If the proxy
sets `X-Forwarded-For`, also set `VOX_MCP_HTTP_TRUST_X_FORWARDED_FOR=1` so rate limits apply per
client rather than to the tunnel.

> **Do not** put a same-host reverse proxy in front of a loopback bind without `--behind-tunnel`:
> the gateway cannot tell proxied callers from local ones.

## What the gateway refuses

A bind that is not loopback (anything outside `127.0.0.0/8` and `::1`, including `0.0.0.0` and
`::`), or any bind with `--behind-tunnel`, refuses to start unless **all** of these hold:

| Requirement | Set with |
|---|---|
| Write token | `vox secrets set VOX_MCP_HTTP_BEARER_TOKEN --stdin` |
| Read token | `vox secrets set VOX_MCP_HTTP_READ_BEARER_TOKEN --stdin` |
| TLS or tunnel | `--tls-cert`/`--tls-key` (`VOX_MCP_HTTP_TLS_CERT`/`_KEY`) or `--behind-tunnel` (`VOX_MCP_HTTP_BEHIND_TUNNEL=1`) |
| No unauthenticated override | `VOX_MCP_HTTP_ALLOW_UNAUTHENTICATED` unset |

The error names every missing item. `vox doctor` shows the same verdict under
**MCP HTTP gateway bind policy** without starting anything.

Off loopback, `POST /api/v2/mesh/nodes/{id}/kill` is refused with `exposure_denied` even with the
write token.

## Discover what a server serves

```bash
curl -s https://vox.lan:3921/v1/profile -H "Authorization: Bearer <read-or-write-token>"
```

For example, a `models` server answering a write-token caller:

```json
{
  "profiles": ["models"],
  "max_exposure": "operator",
  "subsystems": { "booted": ["db", "llm_facade", "model_registry"], "failed": [] },
  "tools": ["vox_list_models"],
  "axis_surfaces": ["chat", "models"],
  "server_version": "0.6.0"
}
```

`tools` is what **your token from your address** may call — the same list `GET /v1/tools`
returns. A tool missing from it fails with `exposure_denied` (403), `insufficient_role` (403),
`tool_not_in_profile` (404), or `subsystem_not_booted` (503).

## Known limitations

- The gateway's Origin guard rejects browser requests whose `Origin` is not a loopback host, and
  WebSocket upgrades without an `Origin` header. Non-browser HTTP clients (`curl`, SDKs) are
  unaffected; LAN browser pages such as `/v1/mobile` and remote WebSocket clients are not yet
  supported.
- There is no per-IP limit on TLS handshakes; for a public deployment, put the server behind a
  tunnel.

## Related

- [Environment variables](../reference/env-vars.md) — `VOX_MCP_HTTP_*`
- [MCP HTTP gateway contract](../reference/mcp-http-gateway-contract.md)

## Remote Axis

Axis can attach to a `vox serve` gateway instead of the daemon on your own machine.

1. Start the server with TLS (or `--behind-tunnel`) and bearer tokens, for example:
   `vox serve --profile mesh --bind 0.0.0.0:3921 --tls-cert cert.pem --tls-key key.pem`.
2. On the server, print the certificate fingerprint Axis will show:

   ```bash
   openssl x509 -in cert.pem -outform DER | openssl dgst -sha3-256
   ```

3. In Axis, open **Settings → Server**, enter `https://<host>:<port>` and the read or write token,
   and press **Connect**. Compare the SHA3-256 fingerprint in the dialog with step 2, then
   **Trust and connect**. Axis pins that certificate; if it ever changes, Axis asks again and shows
   both fingerprints.
4. **Disconnect** returns Axis to the local daemon.

Plain `http://` is accepted only for loopback addresses. Axis and the server must share the same
`major.minor` version, otherwise the connection is refused with `version_mismatch`.

**Where things are stored.** The token is kept in the OS keyring through Clavis (registry token
`axis-remote:<origin>`); if the keyring is unavailable Clavis falls back to `~/.vox/auth.json` and
prints a warning. The server URL and pinned fingerprint are the `axis_remote_url` and
`axis_remote_tls_sha3_256` keys in `~/.vox/config.toml`.

**What you see.** A surface is shown only if the server's profile lists it in `axis_surfaces` and
every command module it uses can run remotely (`contracts/gui/command-backends.v1.yaml`). Otherwise
it shows *Not available on this server* with one of three reasons:

| Reason | Meaning |
|---|---|
| not in server profile | The profile does not include the surface. |
| needs this machine | The surface uses files, processes, or settings on the local machine. |
| not remote-capable yet | The surface reads the workspace database directly; it moves behind the gateway per profile (plan 07). |

Remotely usable today: Mesh, Repository (VCS isolation), Search (Vox graph), plus Settings and
Coverage, which configure Axis itself. In Settings, sections that configure the local Vox install
are locked while connected.

**Known limits.** The orchestrator status stream is not relayed from the server (agent events are);
the GUI permission mode is not forwarded, so dangerous tools follow the server's approval default.
