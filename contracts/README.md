# Contracts

| Artifact | Purpose |
|----------|---------|
| [`codex-api.openapi.yaml`](./codex-api.openapi.yaml) | HTTP surface for `vox-codex-api` / `run_dashboard` (guarded by `scripts/check_codex_ssot.sh`). |
| [`cli/command-registry.yaml`](./cli/command-registry.yaml) | Shipped CLI surface; validate with **`vox ci command-compliance`** from the repo root (canonical); shell delegates are optional. |
| [`mcp/tool-registry.canonical.yaml`](./mcp/tool-registry.canonical.yaml) | MCP tool names + descriptions; compile-time via `crates/vox-mcp-registry`. |
| [`index.yaml`](./index.yaml) | Machine-readable list of contract artifacts (`vox ci contracts-index`). |

See [Codex HTTP API reference](../docs/src/reference/codex-http-api.md). SSOT guards: `scripts/check_codex_ssot.sh` (Linux/macOS CI) and `scripts/check_codex_ssot.ps1` (Windows).

**CI entrypoint:** prefer **`vox ci …`** over ad-hoc `scripts/*.sh` where a `vox ci` subcommand exists — see [runner contract](../docs/src/ci/runner-contract.md) and [command compliance](../docs/src/ci/command-compliance-ssot.md).
