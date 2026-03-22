---
title: "Codex HTTP API (reference)"
category: reference
last_updated: 2026-03-21
---

# Codex HTTP API

Rust implementation: [`vox-codex-api`](../../../crates/vox-codex-api/src/lib.rs) (`codex_router`, `run_dashboard`).

## SSOT

- **OpenAPI 3** — [`contracts/codex-api.openapi.yaml`](../../../contracts/codex-api.openapi.yaml) (validated by [`scripts/check_codex_ssot.sh`](../../../scripts/check_codex_ssot.sh) / [`scripts/check_codex_ssot.ps1`](../../../scripts/check_codex_ssot.ps1)).

## Tests

- `cargo test -p vox-codex-api` — Tower `oneshot` integration tests in [`crates/vox-codex-api/tests/http_smoke.rs`](../../../crates/vox-codex-api/tests/http_smoke.rs).

## Defaults

| Item | Value |
|------|--------|
| Bind | `VOX_DASH_HOST` (default `127.0.0.1`) + `VOX_DASH_PORT` (default `3847`) |
| Readiness | `GET /ready` uses [`vox_db::evaluate_codex_api_readiness`](../../../crates/vox-db/src/codex_schema.rs) (baseline `schema_version` **1** + required tables + manifest digest) |

## Related

- [Environment variables (SSOT)](env-vars-ssot.md) — `VOX_DASH_*`, Codex DB envs
- [Codex BaaS scaffolding](../architecture/codex-baas.md)
- [Codex vNext schema](../architecture/codex-vnext-schema.md)
