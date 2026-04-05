---
title: "Crate API: vox-mcp"
description: "Official documentation for Crate API: vox-mcp for the Vox language. Detailed technical reference, architecture guides, and implementation"
category: "reference"
last_updated: 2026-03-29
training_eligible: true
---
# Crate API: vox-mcp

**Architecture SSOT (language decorator vs this crate):** [`MCP exposure from the Vox language (SSOT)`](../architecture/mcp-vox-language-exposure.md) — explains generated `mcp_server.rs` vs `vox-mcp`, WebSocket/VoxDb boundaries, and roadmap for zero-wiring.

## MCP tool and Rust type naming

| Layer | Convention |
| ----- | ---------- |
| **Tool names** (wire / `tools/list`) | Lowercase **`vox_*`** snake_case per [`tools/mod.rs`](../../../crates/vox-mcp/src/tools/mod.rs) `TOOL_REGISTRY`. Do not repeat the word `vox` twice in one tool id (server is already Vox). |
| **Rust param structs** | Legacy: `VoxScientia*`, `VoxNews*`. **New code:** prefer shorter aliases when added (`Scientia*`, `News*`) — same `serde` + `schemars` shape, **no JSON change**. |
| **CLI** | See [CLI reference](../reference/cli.md); Scientia publication commands live under `vox scientia …` (English canonical). |

## Authoritative sources

| Topic | Location |
| ----- | -------- |
| **Telemetry trust, optional Codex rows, mesh/cost flags** | [Telemetry trust SSOT](../architecture/telemetry-trust-ssot.md) — pair with [env-vars](../reference/env-vars.md) (`VOX_MESH_CODEX_TELEMETRY`, `VOX_MCP_LLM_COST_EVENTS`, benchmark flags). |
| **@mcp.tool / @mcp.resource codegen vs vox-mcp (two surfaces)** | [`docs/src/architecture/mcp-vox-language-exposure.md`](../architecture/mcp-vox-language-exposure.md) |
| **Generated app MCP surface in `app_contract.json`** | `schema_version` **2** + `mcp_tools` / `mcp_resources` ([`app_contract.rs`](../../../crates/vox-compiler/src/app_contract.rs)) |
| **Tool names + descriptions (SSOT)** | [`contracts/mcp/tool-registry.canonical.yaml`](../../../contracts/mcp/tool-registry.canonical.yaml) → `crates/vox-mcp-registry` → `TOOL_REGISTRY` |
| **Per-tool JSON input schemas (MCP `tools/list`)** | [`crates/vox-mcp/src/tools/input_schemas.rs`](../../../crates/vox-mcp/src/tools/input_schemas.rs) → `tool_input_schema` |
| **Session → agent binding** | `vox_map_agent_session` (canonical in `TOOL_REGISTRY`); **`vox_map_opencode_session`** / **`vox_map_vscode_session`** are **wire aliases** only (same JSON args), defined in `crates/vox-mcp/src/tools/tool_aliases.rs`. |
| Orchestrator integration & agent flows | [`agents/orchestrator.md`](../../agents/orchestrator.md), [`crates/vox-orchestrator/`](../../../crates/vox-orchestrator) |
| MCP tool wiring & params | [`crates/vox-mcp/src/lib.rs`](../../../crates/vox-mcp/src/lib.rs), [`crates/vox-mcp/src/params.rs`](../../../crates/vox-mcp/src/params.rs) |
| LLM batch doc playbook | [`agents/llm-documentation-playbook.md`](../../../AGENTS.md) |

## LLM model routing (`models.toml`)

MCP chat, inline edit, and ghost-text tools resolve models through [`vox-orchestrator`](../../../crates/vox-orchestrator)’s [`ModelRegistry`](../../../crates/vox-orchestrator/src/models/mod.rs). On first run, a default registry is written to **`models.toml`** under the Vox config directory (same discovery as Codex paths).

| Operator action | Where |
| --------------- | ----- |
| Add/remove models, tune `strengths` / `cost_per_1k` | Edit `models.toml` and restart MCP (or reload config if your host supports it). |
| Pin paid “performance” routing per task bucket (`codegen`, `review`, …) without recompiling | Set `[premium_alias]` entries in `models.toml` (e.g. `codegen = "anthropic/claude-sonnet-4.5"`). An empty map falls back to built-in defaults, then ranked paid models by cost when `cost_preference` is **performance**. |
| OpenRouter free daily caps | Usage rows aggregate under provider **`openrouter`** and model **`:free`** (see `ModelSpec::llm_usage_key`). |
| Local fallback | Default registry includes an **Ollama-compatible** local model (`llama3.2`); MCP probes `GET /api/tags` before chat (cached briefly per process) and calls `/api/chat`. Base URL precedence is **`POPULI_URL`** → **`OLLAMA_URL`** (or `OLLAMA_HOST` for raw Ollama clients) → `http://localhost:11434`. `vox-schola serve` now exposes both OpenAI (`/v1/chat/completions`) and Ollama-compatible (`/api/tags`, `/api/chat`) endpoints, so local Mens serving works without extra protocol shims. **Desktop-oriented:** phones do not run Ollama on loopback; use **`VOX_INFERENCE_PROFILE`** / cloud or on-device runtimes per [mobile-edge-ai.md](../reference/mobile-edge-ai.md). |
| Cloud → local when `allow_cloud_ollama_fallback` | Same as above, but **only** when **`VOX_INFERENCE_PROFILE`** is **`desktop_ollama`** or **`lan_gateway`** (otherwise Ollama probes, direct `ProviderType::Ollama`, and fallback are skipped). Chat and inline tools: persisted **daily cap**, in-memory **budget exceeded**, **rate limit**, and many **HTTP errors** retry once via the best **Ollama** candidate (largest `max_tokens`, stable id). Ghost-text uses the same path with **free-tier-only** resolution. |
| **`VOX_MCP_LLM_COST_EVENTS`** | `1` / `true` always emits [`AgentEventKind::CostIncurred`](../../../crates/vox-orchestrator/src/events.rs) on successful MCP LLM calls; `0` / `false` never. **Default:** emit only when Codex is **not** attached (avoid double-counting with `provider_usage`); with Codex, set `1` if your consumer needs bus events as well as DB. |

Codex-attached deployments pair MCP LLM calls with [`BudgetGate`](../../../crates/vox-orchestrator/src/gate.rs) + [`UsageTracker`](../../../crates/vox-orchestrator/src/usage.rs); HTTP **429** marks rate limits on the usage key.

### Mens + MCP startup

When **`VOX_MESH_ENABLED=1`**, the **`vox-mcp`** binary calls `vox_populi::publish_local_registry_best_effort()` after DB connect (same pattern as **`vox run`**), then best-effort **`POST /v1/populi/join`** when **`VOX_ORCHESTRATOR_MESH_CONTROL_URL`** or **`VOX_MESH_CONTROL_ADDR`** normalizes to a non-bind-all HTTP(S) base ([`normalize_http_control_base`](../../../crates/vox-populi/src/lib.rs)), plus periodic **`POST /v1/populi/heartbeat`** (see **`VOX_MESH_HTTP_JOIN`**, **`VOX_MESH_HTTP_HEARTBEAT_SECS`** on [mens SSOT](../reference/populi.md)). Optional Codex rows: **`VOX_MESH_CODEX_TELEMETRY`**, **`mesh_http_join_ok` / `mesh_http_join_err`** (trust framing: [telemetry-trust-ssot](../architecture/telemetry-trust-ssot.md)). Docker: [mens SSOT](../reference/populi.md) (`VOX_MESH_MESH_SIDECAR`, `infra/containers/entrypoints/vox-entrypoint.sh`).

## Process model

`vox-mcp` is normally a **stdio MCP server**: one process per editor/CLI session, embedding `Orchestrator` and optional Turso `CodeStore`. It is not, by default, a standalone network daemon; long-running behavior is whatever the host keeps alive.

Optional sidecar mode (ADR 022 Phase B): set **`VOX_ORCHESTRATOR_DAEMON_SOCKET`** to a TCP peer and run **`vox-orchestrator-d`** as a separate owner process. MCP performs startup `orch.ping` repo-id alignment checks and can opt into read/write RPC pilots via **`VOX_MCP_ORCHESTRATOR_*`** env flags ([env SSOT](../reference/env-vars.md)). Current transition policy is split-plane: daemon-aligned task/agent pilots are supported, while VCS/context/event/session tool surfaces still default to embedded stores until full daemon contracts land.

### Optional HTTP + WebSocket gateway (bounded remote/mobile control)

`vox-mcp` now supports an explicit opt-in network gateway for remote/mobile clients. This is **off by default** and intended for bounded remote control of a host that already owns the repo/toolchain.

- Enable: `VOX_MCP_HTTP_ENABLED=1`
- Bind defaults: `VOX_MCP_HTTP_HOST=127.0.0.1`, `VOX_MCP_HTTP_PORT=3921`
- Auth: set `VOX_MCP_HTTP_BEARER_TOKEN` (write role) and/or `VOX_MCP_HTTP_READ_BEARER_TOKEN` (read role), unless `VOX_MCP_HTTP_ALLOW_UNAUTHENTICATED=1`
- Tool safety: allowlist via `VOX_MCP_HTTP_ALLOWED_TOOLS` (CSV); defaults to a safe subset
- Read-role scope: tool-level registry metadata (`http_read_role_eligible`) intersected with `VOX_MCP_HTTP_ALLOWED_TOOLS`; optional `VOX_MCP_HTTP_READ_ROLE_ALLOWED_TOOLS` narrows further; governance profile in `contracts/mcp/http-read-role-governance.yaml`
- Request budget: `VOX_MCP_HTTP_RATE_LIMIT_PER_MINUTE` (default `120`)
- Reverse-proxy hardening: `VOX_MCP_HTTP_REQUIRE_FORWARDED_HTTPS=1` requires `X-Forwarded-Proto: https`
- Optional health auth: `VOX_MCP_HTTP_HEALTH_AUTH=1`
- Optional forwarded client identity for rate limit keys: `VOX_MCP_HTTP_TRUST_X_FORWARDED_FOR=1`
- `GET /v1/info` exposes both `allowed_tools` and effective `read_role_allowed_tools` after metadata/env intersections

Bounded endpoints:

- `GET /health`
- `GET /v1/info`
- `GET /v1/tools`
- `POST /v1/tools/call`
- `GET /v1/ws` (WebSocket: `list_tools`, `call_tool`)
- `GET /v1/mobile` (minimal mobile workspace UI)
- `GET /v1/mobile/status` (git/orchestrator status bundle)

Contract SSOT: [MCP HTTP gateway contract](../reference/mcp-http-gateway-contract.md) and [OpenAPI](../../../contracts/mcp/http-gateway.openapi.yaml).

Security note: this gateway is for phone/browser **remote operations** against a non-phone host, not for turning stock phones into full `vox` toolchain hosts.

### Reverse proxy and TLS boundary

- Terminate TLS at a trusted edge proxy (nginx/caddy/envoy) and keep `vox-mcp` bound on private/local interfaces.
- For strict HTTPS signaling behind proxy hops, set `VOX_MCP_HTTP_REQUIRE_FORWARDED_HTTPS=1` and forward `X-Forwarded-Proto: https`.
- Enable `VOX_MCP_HTTP_TRUST_X_FORWARDED_FOR=1` only when the gateway is reachable exclusively through a trusted proxy tier.
- Ensure WebSocket upgrade support for `GET /v1/ws` at the proxy (`Connection: upgrade`, `Upgrade: websocket`).

## Module: `vox-mcp\src\a2a.rs`

A2A (Agent-to-Agent) MCP tools — send, inbox, ack, broadcast, history.

### `fn a2a_send`

Send a targeted A2A message from one agent to another.

Current contract details:

- **Sender binding:** If the sender agent has a **mapped** orchestrator session (`vox_map_agent_session` / wire alias), `sender_session_id` is **required** and must equal that session id; otherwise the tool errors. If the sender has **no** mapped session, the call still succeeds for compatibility, but the response includes `sender_identity_binding: "unbound"` and a non-null `binding_advisory` string (not log-only).
- `route` explicitly selects the wire token for a canonical delivery plane:
  - `local`: in-process orchestrator message bus
  - `db`: durable `a2a_messages` inbox (requires orchestrator DB)
  - `mesh`: Populi HTTP relay
- The response also reports canonical `delivery_plane` values:
  - `local` -> `local_ephemeral`
  - `db` -> `local_durable`
  - `mesh` -> `remote_mesh`
- **Mesh idempotency (MCP):** `vox_a2a_send` always forwards a **non-empty** idempotency key to Populi: trimmed `correlation_id` when set, otherwise a deterministic default derived from sender, receiver, message type, and payload. The **Populi HTTP** `POST /v1/populi/a2a/deliver` handler does **not** synthesize that default; direct HTTP clients must send `idempotency_key` themselves when they need deduplication (see [populi SSOT](../reference/populi.md)).

### `fn a2a_inbox`

Read unacknowledged messages in an agent's inbox.

Current contract details:

- `source` controls inbox plane selection:
  - `merged` (default): merge local bus and Populi mesh inbox
  - `local`: read only in-process orchestrator bus
  - `mesh`: read only Populi relay inbox
- Responses surface canonical `delivery_planes` so callers can reason about semantics without depending on wire-token names.
- `agent_session_id` is required when this agent has a mapped orchestrator session.
- For direct Populi HTTP clients (outside MCP), non-claimer inbox reads support cursor paging with `max_messages` + `before_message_id`. The Rust client provides `A2AInboxPager` and `relay_a2a_inbox_all_paged` in `vox_populi::http_client`.
- `vox_a2a_inbox` also forwards optional `max_messages` and `before_message_id` to the mesh inbox path when `source` is `mesh` or `merged`.

Paged mesh inbox read example (tool calls):

```json
{
  "tool": "vox_a2a_inbox",
  "arguments": {
    "agent_id": 12,
    "source": "mesh",
    "max_messages": 2
  }
}
```

```json
{
  "success": true,
  "data": {
    "agent_id": 12,
    "source": "mesh",
    "messages": [
      { "id": 105, "payload": "...", "acknowledged": false },
      { "id": 104, "payload": "...", "acknowledged": false }
    ]
  }
}
```

Use the smallest returned `id` as the cursor for the next page:

```json
{
  "tool": "vox_a2a_inbox",
  "arguments": {
    "agent_id": 12,
    "source": "mesh",
    "max_messages": 2,
    "before_message_id": 104
  }
}
```

### `fn a2a_ack`

Acknowledge a message in an agent's inbox.

Current contract details:

- `agent_session_id` is required when this agent has a mapped orchestrator session.

### `fn a2a_broadcast`

Broadcast an A2A message to all agents except sender.

Current contract details:

- Same **sender binding** rules as `vox_a2a_send`: required matching `sender_session_id` when the sender has a mapped session; otherwise success with `sender_identity_binding` / `binding_advisory` in the payload.

### `fn a2a_history`

Query the A2A audit trail.

### `fn set_context`

Set a key-value pair in the shared orchestrator context.

### `fn get_context`

Retrieve a value from the shared context.

### `fn list_context`

List available context keys by prefix.

### `fn context_budget`

Get the token budget status for an agent.

### `fn handoff_context`

Handoff summarized context from one agent to another.

### `struct ToolResult`

A standard envelope for all tool responses.

## Module: `vox-mcp\src\main.rs`

## vox-mcp binary

MCP server entry point. Runs on stdio for Vox Agent integration.

Startup flow:

1. Initialize logging → stderr (stdout reserved for MCP protocol)
2. Load orchestrator config from `Vox.toml` (or defaults)
3. Create shared `ServerState` with the orchestrator
4. Optionally spawn HTTP/WebSocket gateway when `VOX_MCP_HTTP_ENABLED=1`
5. Start MCP server on stdio via `rmcp::transport::stdio()`

## Module: `vox-mcp\src\memory.rs`

MCP tools for the persistent memory system.

### `fn memory_store`

Persist a key-value fact to long-term memory (MEMORY.md + VoxDb).

### `fn memory_recall`

Retrieve a fact from long-term memory by key.

### `fn memory_search`

Search memory (daily logs + MEMORY.md) by keyword.

### `fn memory_daily_log`

Append an entry to today's daily memory log.

### `fn memory_list_keys`

List all memory keys from MEMORY.md.

### `fn knowledge_query`

Query the knowledge graph by keyword.

### `fn compaction_status`

Get current context window usage and compaction recommendation.

### `struct SessionInfo`

Response type for session info.

### `fn session_create`

Create a new session for an agent.

### `fn session_list`

List all sessions.

### `fn session_reset`

Reset a session (clear history, keep metadata).

### `fn session_compact`

Compact a session with a summary.

### `fn session_info`

Get info about a specific session.

### `fn session_cleanup`

Cleanup archived sessions.

### `fn preference_get`

Get a user preference from VoxDb.

### `fn preference_set`

Set a user preference in VoxDb.

### `fn preference_list`

List user preferences from VoxDb, optionally filtered by key prefix.

### `fn learn_pattern`

Store a learned behavior pattern in VoxDb.

### `fn behavior_record`

Record a user behavior event and get triggered suggestions.

### `fn behavior_summary`

Analyze all behavior events for a user and return learned patterns summary.

### `fn memory_save_db`

Persist a fact directly into VoxDb agent_memory table.

### `fn memory_recall_db`

Recall facts from VoxDb agent_memory table.

### `fn vcs_status`

Unified VCS status: snapshots, oplog, conflicts, workspaces, and changes.

## Module: `vox-mcp\src\skills.rs`

MCP tools for the vox-skills marketplace.

### `struct SkillInfo`

Response shape for skill info.

## Module: `vox-mcp\src\tools.rs`

Tool handler implementations for the Vox MCP server.

Each public function corresponds to an MCP tool that AI agents can invoke.

### `fn submit_task`

Submit a new task to the orchestrator.

Routes the task to the best agent based on file affinity, acquires locks,
and enqueues it for processing.

### `fn task_status`

Get the current status of a specific task.

### `fn orchestrator_status`

Get a full snapshot of the orchestrator's state.

### `fn complete_task`

Mark a task as completed, releasing its file locks.

### `fn fail_task`

Mark a task as failed with a reason.

### `fn check_file_owner`

Check which agent owns a given file path.

### `fn validate_file`

Validate a .vox file using the full compiler pipeline (lexer → parser → typeck → HIR).

### `fn run_tests`

Run `cargo test` for a specific crate.

### `fn check_workspace`

Run `cargo check` for the entire workspace.

### `fn test_all`

Run `cargo test` for the entire workspace.

### `fn build_crate`

Run `cargo build` for a crate or the whole workspace.

### `fn lint_crate`

Run `cargo clippy` and TOESTUB for a crate or the whole workspace.

### `fn coverage_report`

Run `cargo llvm-cov` for a text coverage summary (install with `cargo install cargo-llvm-cov`; toolchain needs `llvm-tools-preview`).

### `fn git_log`

Run `git log` to show recent commits.

### `fn git_diff`

Run `git diff` for a file or the whole working tree.

### `fn git_status`

Run `git status` to see working tree status.

### `fn git_blame`

Run `git blame` for a specific file.

### `fn snapshot_list`

List recent snapshots for an agent.

### `fn snapshot_diff`

Show diff between two snapshots.

### `fn oplog_list`

List recent operations from the operation log.

### `fn oplog_undo`

Undo an operation.

### `fn oplog_redo`

Redo an operation.

### `fn conflicts_list`

List active conflicts.

### `fn resolve_conflict`

Resolve a conflict.

### `fn workspace_create`

Create a workspace for an agent.

### `fn workspace_status`

Show workspace status.

### `fn workspace_merge`

Merge workspace back to main.

### `fn change_create`

Create a new logical change.

### `fn change_log`

Show history of a change.

### `fn publish_message`

Publish a message to the bulletin board for all agents to receive.

### `fn generate_vox_code`

Generate validated Vox code using the QWEN inference server.

Calls the inference server at localhost:7863 to generate code from a prompt,
with automatic syntax validation and self-correction.

### `fn vox_db_schema`

Return the complete schema digest for a .vox file as JSON.

This is the primary LLM context tool — it tells AI models exactly
what tables, fields, indexes, and relationships exist in the database.

### `fn vox_db_relationships`

Return the entity-relationship graph: auto-detected `Id<X>` references between tables.

### `fn vox_db_data_flow`

Return the data flow map: which queries read which tables, which mutations write.

### `fn tool_registry`

Return full list of capabilities to the Vox agent client.

Mens **chat** intersects this surface with in-process execution via `vox-tools` (`DirectToolExecutor` + `mens_chat::chat_tool_definitions` / `execute_tool_calls`) and `vox-capability-registry` (Oratio: `vox_oratio_transcribe`, `vox_oratio_status` — same names as here).

### `fn handle_tool_call`

Routes from string name to underlying function
