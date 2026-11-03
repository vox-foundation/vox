---
title: "Populi Quickstart"
description: "Run a local Vox mesh control plane and pair a worker node in minutes — no environment variables required."
category: "How-To Guides"
training_eligible: true
---

# Populi Quickstart

This guide walks you through running a local Vox mesh control plane — no environment variables required.

## Prerequisites

- `vox-ml-cli` built with the `populi` feature: `cargo build -p vox-ml-cli --features populi`
- A writable `~/.vox/` directory (created automatically on first run)

## Step 1 — Start the control plane

```sh
vox populi serve --enable
```

On the very first run, Vox generates a random bearer token and stores it in the Clavis vault (`SecretId::VoxMeshToken`) — it is never written to `~/.vox/config.toml` and never printed in plaintext:

```text
vox populi: generated a mesh bearer token and stored it in the Clavis vault (fingerprint a3f7c2b91045e6d8).
  Any local `vox populi`/orchestrator process resolves it automatically (`vox secrets get VOX_MESH_TOKEN` confirms it's set, redacted).
  Keep it secret — it authenticates all control-plane requests.
vox populi: listening on http://127.0.0.1:PORT
```

If stdin is a TTY, you're asked to confirm before the token is generated and stored; pass `--yes` (or run non-interactively, e.g. under systemd/Docker) to skip the prompt.

The OS assigns a free port automatically (you can override with `--bind 127.0.0.1:9847`).

Subsequent runs reuse the stored token — no output unless it has changed. `vox secrets get VOX_MESH_TOKEN` only shows redacted status, never the plaintext — that's intentional, and it's the same for every managed secret.

## Step 2 — Verify the server is running

Any other Vox process on this machine (the orchestrator, a second `vox populi` invocation) resolves the token from the vault automatically — no manual export needed for those.

For manual `curl` testing in a second terminal, you need a value you can type. Set your own token as an env var *before* the first `--enable` run, so it takes precedence over auto-generation and you keep a copy:

```sh
export VOX_MESH_TOKEN=$(openssl rand -hex 24)
vox populi serve --enable
```

Then, in a second terminal, reuse the same value:

```sh
curl http://127.0.0.1:PORT/health
# {"status":"ok"}

curl -H "Authorization: Bearer $VOX_MESH_TOKEN" \
     http://127.0.0.1:PORT/v1/populi/nodes
# {"nodes":[]}
```

Replace `PORT` with the port printed in Step 1.

## Step 3 — Register a worker node

```sh
curl -s -X POST \
  -H "Authorization: Bearer $VOX_MESH_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"id":"my-node","scope_id":null,"capabilities":{"cpu_cores":8},"labels":{}}' \
  http://127.0.0.1:PORT/v1/populi/join | jq .
```

List nodes again to confirm:

```sh
curl -H "Authorization: Bearer $VOX_MESH_TOKEN" \
     http://127.0.0.1:PORT/v1/populi/nodes | jq .nodes[].id
# "my-node"
```

## Inspecting configuration

```sh
# Print all resolved config values and their sources
vox populi config show

# Validate the config and flag any missing or conflicting values
vox populi config check
```

## Pinning the port

Add `--bind 127.0.0.1:9847` to use a fixed port:

```sh
vox populi serve --enable --bind 127.0.0.1:9847
```

## Using the durable store (optional)

When the canonical VoxDb database is reachable, the control plane automatically uses it as a durable backing store for the A2A inbox, exec leases, and dispatch results.  The in-memory cache is warmed from the DB at startup.  No extra flags are needed — it just works.

## Token management

| Source | How |
|--------|-----|
| Auto-generated (default) | Stored in the Clavis vault as `VOX_MESH_TOKEN` on first `--enable` run |
| Environment override | Set `VOX_MESH_TOKEN=<value>` before starting; takes precedence over the vault |
| Manual set | `vox secrets set VOX_MESH_TOKEN --stdin` |
| Legacy `~/.vox/config.toml` `mesh.token` | Still read if present (older installs), but deprecated: the next `vox populi serve --enable` migrates it into the vault and deletes it from the file. `vox populi config check` flags it while it's there. |

To rotate the token, run `vox secrets set VOX_MESH_TOKEN --stdin` with a new value and restart `vox populi serve --enable`. There's no `vox secrets delete` yet — overwriting is the supported way to change it.

## Connecting the orchestrator

The orchestrator resolves `VOX_MESH_TOKEN` from the vault automatically once it's stored — no manual export needed on the same machine:

```sh
export VOX_MESH_CONTROL_ADDR=http://127.0.0.1:PORT
vox orchestrate ...
```

## Troubleshooting

**Port already in use** — omit `--bind` to let the OS assign a free port, or choose a different port with `--bind 127.0.0.1:<PORT>`.

**401 Unauthorized** — the `Authorization: Bearer` header is missing or the token doesn't match the one in the vault. Run `vox populi config show` to check the token source, and `vox secrets get VOX_MESH_TOKEN` to confirm one is set (redacted; it won't show the plaintext).

**Mesh store warm-up warning** — `mesh store warm-up failed; continuing with empty cache` is printed when VoxDb is unavailable.  The server still starts and operates fully in-memory.
