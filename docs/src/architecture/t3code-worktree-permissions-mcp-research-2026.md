---
title: "T3 Code Server Runtime: statev2.sqlite, Worktree Safety, MCP OAuth/DPoP & Subsystems (2026)"
description: "Source-anchored architecture of pingdotgg/t3code apps/server: statev2.sqlite schema (migrations 055-069), Windows NTFS junction-safe worktree management, Git plumbing checkpoints, RFC 9449 DPoP & stateless MCP OAuth, 14-toolkit McpToolAccess, Scheduled Tasks, FFI Workspace Search, Preview/Device hosts, and transactional serviceLauncher."
category: "Architecture SSOTs"
status: "research"
topic: "agent-orchestrator"
domain: "orchestration"
authorship: "llm-generated"
authorship_model: "claude-opus-4-7"
---

# T3 Code Server Runtime: `statev2.sqlite`, Worktree Safety, MCP OAuth/DPoP & Subsystems (2026)

<!-- kb:summary -->
This document details the server-side architecture of `pingdotgg/t3code` (`~/dev/t3code`, commit `80455376f01f19c10a06dc08b0629486c017bf79`) across `apps/server/src/`. It covers the complete `statev2.sqlite` schema (`055_OrchestrationV2.ts` through `069_ThreadMessageSearch.ts`), `CommandOutputHub.ts`, Windows NTFS junction-safe worktree management (`worktreeRemoval.ts`, `storageCleanup.ts`, `workspaceLease.ts`), Git plumbing checkpoints (`refs/t3/checkpoints/`), RFC 9449 DPoP ES256 authentication (`EnvironmentAuth.ts`), stateless HMAC-SHA256 MCP OAuth 2.1 (`McpOAuth.ts`), capability-gated `McpToolAccess.ts`, Scheduled Tasks & Ed25519 Relay Webhooks, `@ff-labs/fff-node` FFI Workspace Search + FTS5 trigram thread search, Integrated Browser/Device hosts, and the zero-Effect transactional self-updater (`serviceLauncher.ts`).
<!-- /kb:summary -->

## 1. `statev2.sqlite` Schema & Persistence (`apps/server/src/persistence/Migrations/055`–`069`)

`apps/server/src/orchestration-v2/EventStore.ts` and `ProjectionStore.ts` persist both the append-only event log and 24 normalized SQLite projection/subsystem tables:

| Table Name (`055_OrchestrationV2.ts` – `069_ThreadMessageSearch.ts`) | Primary Key & Constraints | Purpose |
|---|---|---|
| `orchestration_v2_events` | `sequence INTEGER PRIMARY KEY AUTOINCREMENT`, `UNIQUE(event_id)` | Append-only event journal (`stream_id`, `event_type`, `command_id`, `causation_event_id`, `correlation_id`, `payload_json`) |
| `orchestration_v2_command_receipts` | `command_id TEXT PRIMARY KEY` | Idempotency ledger (`command_type`, `aggregate_id`, `status`, `first_sequence`, `last_sequence`, `result_json`) |
| `orchestration_v2_projection_threads` | `thread_id TEXT PRIMARY KEY` | Thread read model (`project_id`, `title`, `default_provider`, `provider_instance_id`, `runtime_mode`, `interaction_mode`, `active_provider_thread_id`, `created_at`, `updated_at`, `archived_at`, `deleted_at`, `payload_json`) |
| `orchestration_v2_projection_runs` | `run_id TEXT PRIMARY KEY`, `UNIQUE(thread_id, ordinal)` | Run state machine (`kind`, `status`, `provider`, `driver`, `payload_json`) |
| `orchestration_v2_projection_run_attempts` | `run_attempt_id TEXT PRIMARY KEY`, `UNIQUE(run_id, attempt_ordinal)` | Attempt execution records (`provider_session_id`, `provider_thread_id`, `provider_turn_id`) |
| `orchestration_v2_projection_nodes` | `node_id TEXT PRIMARY KEY` | Execution graph nodes (`thread_id`, `run_id`, `run_attempt_id`, `parent_node_id`, `kind`, `status`) |
| `orchestration_v2_projection_subagents` | `subagent_id TEXT PRIMARY KEY` | Subagent lineage (`thread_id`, `run_id`, `parent_node_id`, `child_thread_id`, `origin`, `status`, `payload_json`) |
| `orchestration_v2_projection_provider_sessions` | `provider_session_id TEXT PRIMARY KEY` | Active/detached provider session records |
| `orchestration_v2_projection_provider_threads` | `provider_thread_id TEXT PRIMARY KEY` | Native provider thread correlation |
| `orchestration_v2_projection_provider_turns` | `provider_turn_id TEXT PRIMARY KEY` | Native provider turn correlation |
| `orchestration_v2_projection_runtime_requests` | `request_id TEXT PRIMARY KEY` | Approval, user-input, dynamic-tool, and MCP elicitation requests |
| `orchestration_v2_projection_messages` | `message_id TEXT PRIMARY KEY` | Full user/assistant message records |
| `orchestration_v2_projection_plans` | `plan_id TEXT PRIMARY KEY` | Structured agent plan records |
| `orchestration_v2_projection_turn_items` | `turn_item_id TEXT PRIMARY KEY` | Projected UI turn items (`payload_json`) |
| `orchestration_v2_turn_item_positions` | `PRIMARY KEY (thread_id, turn_item_id)`, `UNIQUE(thread_id, ordinal)` | Ordered transcript position index (note: no `projection_` infix) |
| `orchestration_v2_projection_checkpoint_scopes` | `scope_id TEXT PRIMARY KEY` | Per-worktree/repo checkpoint scope metadata (`ProjectionStore.ts`) |
| `orchestration_v2_projection_checkpoints` | `checkpoint_id TEXT PRIMARY KEY` | Checkpoint records referencing Git plumbing refs |
| `orchestration_v2_projection_context_handoffs` | `context_handoff_id TEXT PRIMARY KEY` | Cross-provider context handoff summaries (`ContextHandoffService.ts`) |
| `orchestration_v2_projection_context_transfers` | `context_transfer_id TEXT PRIMARY KEY` | Fork/subagent context transfer records |
| `orchestration_v2_projection_metadata` | `key TEXT PRIMARY KEY` | Projection watermark and schema metadata |
| `scheduled_tasks` & `scheduled_task_runs` | `id TEXT PRIMARY KEY` | Cron/interval/one-shot and webhook-triggered task definitions & run history (`055`, `056`, `064`–`068`) |
| `scheduled_task_webhooks`, `scheduled_task_webhook_deliveries`, `webhook_relay_deliveries`, `webhook_dedupe` | `id TEXT PRIMARY KEY` / composite | Signed webhook endpoints, delivery logs, cloud relay queue, and replay deduplication (`057`, `058`, `060`, `062`, `067`) |
| `mcp_app_model_context` | `PRIMARY KEY (thread_id, item_id)` | Per-turn-item MCP Apps UI model context (`059_McpAppModelContext.ts`, capped at 16 KiB) |
| `pull_request_review_guides` | `PRIMARY KEY (host, repository, pull_request_number)` | AI-generated multi-section PR review guides (`061_PullRequestReviewGuides.ts`) |
| `thread_message_search` | `FTS5(..., tokenize = 'trigram')` | Trigram full-text search index over thread messages (`069_ThreadMessageSearch.ts`) |

### 1.1 Live Command Output Hub (`apps/server/src/orchestration-v2/CommandOutputHub.ts`)
- Buffers live tool/command stdout/stderr per `outputStreamId` (`MAX_CHUNKS_PER_STREAM = 64`, `MAX_BYTES_PER_STREAM = 262_144`, `ENDED_STREAM_RETENTION_MS = 120_000`).
- Calls `normalizeTerminalOutput` (`packages/shared/src/terminalOutput.ts:206`, `TERMINAL_OUTPUT_TAIL_CHARS = 64 * 1024`, `LIVE_LINES = 64`) to fold carriage-return (`\r`) progress-bar overwrites before persisting transcript previews.

---

## 2. Worktree Lifecycle, Windows NTFS Junction Safety & Git Plumbing Checkpoints

### 2.1 Managed Worktree Validation & Safe Removal (`worktreesDirectory.ts`, `worktreeRemoval.ts`, `storageCleanup.ts`)
- **Path Validation (`apps/server/src/worktreesDirectory.ts:12-97`):** Rejects filesystem roots (`isFilesystemRoot`) and verifies managed worktrees by resolving symlinks (`realLocation`) and enforcing a strict 2-segment relative path (`<managed-root>/<repo>/<branch>`) across `worktreesDirectory` and `previousWorktreesDirectories`.
- **Windows NTFS Junction Safety (`apps/server/src/vcs/worktreeRemoval.ts:17-157`):** Because Git for Windows `< 2.54` traverses into NTFS directory junctions during `git worktree remove` (and `>= 2.54` orphans them), `removeLinkedWorktree` verifies `.git` admin linkage (`rev-parse --absolute-git-dir` / `--git-common-dir`), checks the `locked` file, checks `status --porcelain` (`-c status.showUntrackedFiles=normal`), unlinks the worktree via Node's `fs.rm` (which unlinks junctions without traversing targets), runs `git worktree prune`, and prunes orphaned pnpm virtual-store entries (`storageCleanup.ts:61-67`).
- **Workspace Lease (`apps/server/src/workspace/workspaceLease.ts:1-23`):** Keyed 1-permit semaphore (`withWorkspaceLease`) acquired by `GitWorkflowService.ts`, `storageCleanup.ts`, `ProviderTurnStartService.ts`, `CheckpointRollbackService.ts`, and `terminal/Manager.ts`.
- **Staged Worktree Setup & Automated Cleanup (`WorktreeSetupTracker.ts`, `storageCleanup.ts`):** `WorktreeSetupTracker` streams setup stages and a 4-line output tail (`TAIL_LINE_LIMIT = 4`, `FINISHED_RETENTION = "30 seconds"`). `StorageCleanupService` enforces `WorktreeCleanupRules` (`SETTLE_SCRIPT_GRACE_MS = 60_000`, `ORPHAN_WORKTREE_GRACE_MS = 3_600_000`).

### 2.2 Git Plumbing Checkpoints (`apps/server/src/checkpointing/CheckpointStore.ts`)
- Delegates to `VcsDriver.checkpoints` (`VcsCheckpointOps` in `GitVcsDriverCore.ts`) to capture and restore workspace trees as hidden Git refs under `refs/t3/checkpoints/` (`captureCheckpoint`, `hasCheckpointRef`, `restoreCheckpoint`, `diffCheckpoints`, `listAuthoredPaths`, `deleteCheckpointRefs`) without polluting branch commit history.

---

## 3. Security, DPoP Proof Binding, Stateless MCP OAuth 2.1 & `McpToolAccess`

### 3.1 RFC 9449 DPoP ES256 Session Binding (`apps/server/src/auth/EnvironmentAuth.ts`)
- Exchanges bootstrap tokens for ES256 (P-256) DPoP-bound session tokens (`SESSION_TTL_MS = 30 days`).
- Verifies `dpop+jwt` proofs on HTTP/WS requests (`htm`, `htu`, `iat` within `DPOP_PROOF_MAX_AGE_SECONDS = 300`, `jti` replay cache `DPOP_JTI_CACHE_LIMIT = 10_000`, and JWK SHA-256 thumbprint `jkt`).

### 3.2 Stateless HMAC-SHA256 MCP OAuth 2.1 (`apps/server/src/auth/McpOAuth.ts`)
- Encodes dynamic client registrations (`POST /oauth/register`) directly into `client_id = t3_<base64url(payload)>.<base64url(hmac_sha256)>` (`MAX_REDIRECT_URIS = 10`, `MAX_CLIENT_NAME_LENGTH = 120`) so zero unauthenticated SQLite writes occur during registration. Enforces RFC 7636 S256 PKCE (`AUTH_CODE_TTL_MS = 120_000`, `ACCESS_TOKEN_TTL_MS = 3_600_000`, `REFRESH_TOKEN_TTL_MS = 30 days`).

### 3.3 14-Toolkit MCP Server & `McpToolAccess.ts` (`apps/server/src/mcp/`)
- **`McpToolAccess.ts:31-188`:** Guards both `Declaration` and `HandlersLayer` (`McpToolAccess.toLayer`) with an unexported `const builtHere: unique symbol = Symbol("t3/mcp/McpToolAccess/builtHere")`:
  - `reads(handle)`: Permitted for all authenticated MCP callers.
  - `writes(handle)`: Calls `refuseReadOnlyClient` (`L68–80`), failing `"read-only"` clients with `new OrchestratorMcpFailure({ code: "capability_denied", message: "This tool changes the environment, and this MCP client was approved for read-only access." })`.
  - `writesWithCapability(handle)`: Runs `writingCaller.pipe(Effect.flatMap(() => handle(params)))`.
  - `startsThreads(modes, handle)`: Calls `resolveRuntimeMode` and `resolveInteractionMode` (`OrchestratorMcpService.ts:679,714`), enforcing `runtimeModeRank` (`"read-only": 0, "default": 1, "full-access": 2`) and `interactionModeRank` (`"plan": 0, "default": 1`) ceilings with `OrchestratorMcpFailure({ code: "runtime_mode_escalation_denied" | "interaction_mode_escalation_denied" })`.
- **DOM Snapshot Shedding (`PreviewToolkit` in `McpHttpServer.ts`):** `boundSnapshotMetadata` strips `snapshot` strings when `Buffer.byteLength(snapshot) > 20 * 1024` (20 KB) and instructs the agent to use `preview_find` or `preview_Snapshot` with `selector`.

---

## 4. Scheduled Tasks, Native FFI Search, Preview/Device Hosts & Service Launcher

1. **Scheduled Tasks & Signed Webhooks (`apps/server/src/scheduledTasks/ScheduledTaskService.ts`):** Supports cron/interval/one-shot tasks and webhook ingress at `/api/hooks` or via T3 Connect relay (`RelayDeliveryProof.ts` Ed25519 JWT proofs), with HMAC verification (`webhookVerification.ts`), JSONPath/header filtering (`webhookFilter.ts`), deduplication (`webhookDedupe.ts`), and rate limits (`WEBHOOK_RATE_LIMIT_PER_MINUTE = 60`, `WEBHOOK_MAX_QUEUED_PER_TASK = 20`).
2. **Native Rust/C FFI Workspace Search & FTS5 Trigram Thread Search (`WorkspaceSearchIndex.ts`, `ThreadSearch.ts`):** Wraps `@ff-labs/fff-node` (`FileFinder`) with `WORKSPACE_INDEX_MAX_ENTRIES = 25_000` and `CONTENT_SEARCH_TIME_BUDGET_MS = 250`; `ThreadSearch.ts` queries the SQLite FTS5 trigram table `thread_message_search` (`MESSAGE_HIT_LIMIT = 2000`).
3. **Integrated Browser, Device Hosts & MCP Apps (`preview/`, `device/`, `mcpApps/`):** `preview/Manager.ts` + `PreviewAutomationBroker.ts` broker Playwright browser inspection; `device/DeviceService.ts` manages iOS Simulator and Android Emulator sessions across `LocalDeviceHost`, `SshDeviceHost`, and `EasDeviceHost` (Expo cloud devices); `mcpApps/McpAppModelContext.ts` persists 16 KiB per-item MCP App UI contexts across turns and forks.
4. **Observability, Resource Telemetry, Session Importer & Transactional Updater:** `Observability.ts` + `ResourceTelemetry.ts` fuse JSONL/OTLP traces with a native process-monitoring sidecar; `AgentSessionScanner.ts` imports `~/.claude` and `~/.codex` transcripts (`MAX_IMPORTED_TRANSCRIPT_BYTES = 4 GiB`); `UsageService.ts` prices transcripts against LiteLLM rates; `serviceLauncher.ts` supervises `"active"` and `"trial"` server binaries with automatic `statev2.sqlite` backup and rollback on failed upgrades.
