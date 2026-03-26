---
title: "Contributing — Populi control plane"
description: "Onboarding for vox-populi HTTP transport and operators"
category: "how-to"
last_updated: 2026-03-25
---

# Contributing — Populi / mens HTTP

## Read first

- [Mens / Populi SSOT](../reference/populi.md)
- [OpenAPI](../../../schemas/populi-control-plane.openapi.yaml)
- [Deployment compose](../reference/deployment-compose.md)

## Key paths

| Path | Role |
|------|------|
| `crates/vox-populi/src/transport/router.rs` | Axum router, auth, body limits |
| `crates/vox-populi/src/transport/handlers.rs` | Join, heartbeat, A2A, bootstrap |
| `crates/vox-populi/tests/http_control_plane.rs` | Integration tests (`transport` feature) |

## Commands

```bash
cargo test -p vox-populi --features transport --test http_control_plane
cargo test -p vox-populi --features transport openapi_paths
```

## Security defaults

- **`GET /health`** stays unauthenticated even when `VOX_MESH_TOKEN` is set.
- Never log bearer tokens or bootstrap secrets.
- Prefer **machine-readable** probes (`vox doctor --probe`) in OCI `HEALTHCHECK`.
