# Contracts

| Artifact | Purpose |
| ---------- | --------- |
| [`codex-api.openapi.yaml`](./codex-api.openapi.yaml) | HTTP surface for `vox-codex-api` / `run_dashboard` (guarded by **`vox ci check-codex-ssot`**). |
| [`cli/command-registry.yaml`](./cli/command-registry.yaml) | Shipped CLI surface; validate with **`vox ci command-compliance`** from the repo root (canonical); shell delegates are optional. |
| [`communication/protocol-catalog.yaml`](./communication/protocol-catalog.yaml) | Communication protocol family inventory: delivery planes, owners, contract/doc paths, and coexistence decisions. |
| [`communication/context-envelope.schema.json`](./communication/context-envelope.schema.json) | Canonical context envelope contract for MCP, orchestrator, search, and Populi handoffs. |
| [`mcp/http-gateway.openapi.yaml`](./mcp/http-gateway.openapi.yaml) | Optional bounded HTTP/WebSocket surface exposed by `vox-mcp` (`VOX_MCP_HTTP_*`). |
| [`mcp/tool-registry.canonical.yaml`](./mcp/tool-registry.canonical.yaml) | MCP tool names + descriptions; compile-time via `crates/vox-mcp-registry`. |
| [`mcp/http-read-role-governance.yaml`](./mcp/http-read-role-governance.yaml) | Canonical MCP HTTP read-role tool profile enforced by `vox ci command-compliance`. |
| [`orchestration/agent-harness.schema.json`](./orchestration/agent-harness.schema.json) | Portable contract-first harness specification for roles, stages, gates, durable state, and failure taxonomy. |
| [`orchestration/context-work-item.schema.json`](./orchestration/context-work-item.schema.json) | Work-item schema for context-management epics, capabilities, and tasks. |
| [`populi/control-plane.openapi.yaml`](./populi/control-plane.openapi.yaml) | Populi mesh control-plane + A2A relay OpenAPI contract. |
| [`workflow/workflow-journal.v1.schema.json`](./workflow/workflow-journal.v1.schema.json) | Interpreted workflow journal v1 JSON Schema contract (indexed as `workflow-journal-v1-schema`; validated via `vox ci contracts-index`). |
| [`rust/ecosystem-support.yaml`](./rust/ecosystem-support.yaml) | Rust crate-family support matrix (tier/boundary/value/debt) for bell-curve lanes. |
| [`index.yaml`](./index.yaml) | Machine-readable list of contract artifacts (`vox ci contracts-index`). |

See [Codex HTTP API reference](../docs/src/reference/codex-http-api.md). SSOT guard: **`vox ci check-codex-ssot`** (the canonical cross-platform guard — shell delegate scripts `scripts/check_codex_ssot.*` have been removed; use `vox ci` directly).

**CI entrypoint:** prefer **`vox ci …`** over ad-hoc `scripts/*.sh` where a `vox ci` subcommand exists — see [runner contract](../docs/src/ci/runner-contract.md) and [command compliance](../docs/src/ci/command-compliance-ssot.md).
