---
title: "T3 Code Harness Architecture: Orchestration v2, Event-Sourced Core & Wire Protocol (2026)"
description: "Source-anchored specification of pingdotgg/t3code Orchestration v2 (ORCHESTRATION_PROTOCOL_VERSION = 2, 55 public commands, 8 internal commands, 45 domain events, 24 TurnItem variants, 18 WS RPCs, 5 HTTP snapshot endpoints, IdAllocatorV2, Live Guest Share, and wire deduplication)."
category: "Architecture SSOTs"
status: "research"
topic: "agent-orchestrator"
domain: "orchestration"
authorship: "llm-generated"
authorship_model: "claude-opus-4-7"
---

# T3 Code Harness Architecture: Orchestration v2, Event-Sourced Core & Wire Protocol (2026)

<!-- kb:summary -->
This document provides the line-verified specification of `pingdotgg/t3code` (`~/dev/t3code`, commit `80455376f01f19c10a06dc08b0629486c017bf79`) Orchestration v2 (`ORCHESTRATION_PROTOCOL_VERSION = 2` in `packages/contracts/src/orchestrationV2.ts`). It details the separation between the durable execution graph and the 24 projected `OrchestrationV2TurnItem` variants wrapped in `ForwardCompatibleUnion`, `IdAllocatorV2`'s `{ allocate, derive }` colon-separated URI-encoded deterministic ID derivation, all 55 public commands and 8 internal commands, all 45 domain events, all 18 WebSocket RPC methods and 5 HTTP snapshot endpoints, wire ID-table interning (`orchestrationV2IdTable.ts`), body deduplication (`orchestrationV2DedupedBodies.ts`), Live Guest Share (`guestShare.ts`), and the 35-tool Orchestrator/Thread MCP surface.
<!-- /kb:summary -->

## 1. Architectural Thesis: Execution Graph vs. Projected Turn Items

`t3code` Orchestration v2 (`ORCHESTRATION_PROTOCOL_VERSION = 2` in `packages/contracts/src/orchestrationV2.ts`) enforces five core invariants:
1. **Execution Graph vs. Transcript Projection Separation:** Durable control state (`OrchestrationV2AppThread`, `OrchestrationV2Run`, `OrchestrationV2RunAttempt`, `OrchestrationV2ExecutionNode`, `OrchestrationV2Subagent`, `OrchestrationV2RuntimeRequest`, `OrchestrationV2Checkpoint`, `OrchestrationV2ContextHandoff`) is persisted separately from projected UI transcript items (`OrchestrationV2TurnItem` — 24 discriminated variants).
2. **Forward-Compatible Schema Evolution:** Discriminated unions across wire boundaries are wrapped in `ForwardCompatibleUnion` (`packages/shared/src/ForwardCompatibleUnion.ts`), preserving unknown future server tags as `{ tag: "unknown", rawTag, raw }` so older Desktop/Mobile clients never fail schema decoding when connected to a newer server.
3. **Deterministic Derived Entity IDs (`IdAllocatorV2`):** `packages/provider-core/src/server/IdAllocator.ts:183-433` exposes `{ allocate, derive }`. Fresh app-minted entities use `allocate.*` (`randomUuidV4` formatted as `${prefix}:${uuid}`), while provider-originated entities use `derive.*` (`joinId(prefix, ...parts)` joining `encodeURIComponent`-encoded segments with `:`) so replaying provider event streams produces identical entity IDs without a lookup table.
4. **Bounded Snapshots + Wire Interning & Body Deduplication:** Cold thread opens fetch `/api/orchestration/threads/:threadId/bounded-snapshot?compactIds=1&dedupeBodies=1` (`orchestrationV2IdTable.ts` + `orchestrationV2DedupedBodies.ts`), interning repeated UUIDs into a single `ids: Array<string | number>` table and omitting duplicate message/checkpoint/subagent bodies on `TurnItem`s (`textOmitted`, `filesOmitted`, `promptOmitted`, `resultOmitted`).
5. **Cross-Provider Context Handoff:** Switching providers mid-thread (`provider.switch`) or forking (`thread.fork`) synthesizes a bounded XML-tagged handoff prompt (`DEFAULT_HANDOFF_TOKEN_CAP = 16_000`, `HANDOFF_BYTE_CAP = 64_000` in `packages/provider-core/src/server/handoffBudget.ts`).

---

## 2. Core Execution Graph Entities (`packages/contracts/src/orchestrationV2.ts`)

| Entity Schema | Primary ID | Key Fields & State Machine (`orchestrationV2.ts`) |
|---|---|---|
| `OrchestrationV2AppThread` (`L386–439`) | `ThreadId` | `projectId`, `title`, `defaultProvider`, `providerInstanceId`, `modelSelection`, `runtimeMode` (`"read-only" \| "default" \| "full-access"`), `interactionMode` (`"default" \| "plan"`), `worktreePath`, `branch`, `pullRequests` (with `unlinkedAt`), `pullRequestWatch`, `lineage` (`rootThreadId`, `parentThreadId`, `relation: "root" \| "subagent" \| "fork"`), `queuedRuns`, `settledAt`, `snoozedUntil`, `autoSettle`, `limitRecovery` (`{ requestId?, runId, resetAt, autoResume, snooze? }`), `pendingBackgroundTasks` (with `awaited?: true`), `guestShare?: ThreadGuestShare \| null` |
| `OrchestrationV2Run` (`L520–569`) | `RunId` | `threadId`, `ordinal`, `kind` (`"user_turn" \| "steered_turn" \| "delegated_task" \| "scheduled_task" \| "system_follow_up" \| "rollback_recovery"`), `status` (`"queued" \| "starting" \| "running" \| "waiting_on_approval" \| "waiting_on_user_input" \| "waiting_on_subagent" \| "settling_background_work" \| "completed" \| "interrupted" \| "failed" \| "cancelled"`), `provider`, `driver`, `modelSelection`, `guestMessageId?` |
| `OrchestrationV2RunAttempt` (`L576–614`) | `RunAttemptId` | `runId`, `threadId`, `attemptOrdinal`, `status` (`"starting" \| "running" \| "completed" \| "interrupted" \| "failed" \| "abandoned"`), `providerSessionId`, `providerThreadId`, `providerTurnId`, `usage` |
| `OrchestrationV2ExecutionNode` (`L678–718`) | `ExecutionNodeId` | `threadId`, `runId`, `runAttemptId`, `parentNodeId`, `kind` (`"turn" \| "step" \| "tool_call" \| "subagent" \| "approval_gate" \| "user_input_gate" \| "checkpoint" \| "compaction" \| "handoff"`), `status` (`"idle" \| "pending" \| "running" \| "waiting" \| "completed" \| "interrupted" \| "failed" \| "cancelled" \| "rolled_back"`) |
| `OrchestrationV2Subagent` (`L825–866`) | `SubagentId` | `threadId`, `runId`, `parentNodeId`, `provider`, `driver`, `childThreadId`, `origin` (`"provider_native" \| "app_delegated"`), `agentLabel`, `prompt`, `promptOmitted?`, `result`, `resultOmitted?`, `status` (`"spawning" \| "running" \| "waiting" \| "completed" \| "interrupted" \| "failed" \| "cancelled"`) |
| `OrchestrationV2RuntimeRequest` (`L1072–1097`) | `RuntimeRequestId` | `threadId`, `runId`, `nodeId`, `kind` (`"command" \| "file-read" \| "file-change" \| "mcp-elicitation" \| "permission" \| "dynamic_tool_call" \| "user_input" \| "auth_refresh"`), `status` (`"pending" \| "resolved" \| "expired" \| "cancelled"`), `payload`, `resolution` |
| `OrchestrationV2Checkpoint` (`L1247–1261`) | `CheckpointId` | `threadId`, `scopeId` (`CheckpointScopeId`), `ordinalWithinScope`, `appRunOrdinal`, `runId?`, `nodeId?`, `ref: CheckpointRef`, `status` (`"ready" \| "missing" \| "error" \| "stale"`), `files` (`Array<OrchestrationV2CheckpointFile>`) |
| `OrchestrationV2ContextHandoff` (`L991–1033`) | `ContextHandoffId` | `threadId`, `sourceProviderInstanceId`, `targetProviderInstanceId`, `strategy` (`"delta_since_target_last_seen" \| "fork_delta_summary" \| "full_thread_summary" \| "checkpoint_summary" \| "manual_context"`), `status` (`"pending" \| "ready" \| "failed" \| "superseded"`), `coveredRunOrdinals: { from, to }`, `summaryText`, `history`, `delivery` |

### 2.1 The 24 `OrchestrationV2TurnItem` Variants (`orchestrationV2.ts:1388-1616`)
1. `notification` (`L1404–1410`)
2. `user_message` (`L1411–1425`, supports `textOmitted?: true`)
3. `assistant_message` (`L1426–1434`, supports `textOmitted?: true`)
4. `reasoning` (`L1435–1440`: `summary`, `text`, `encryptedContentPresent`)
5. `proposed_plan` (`L1441–1447`: `planId`, `title`, `markdown`, `steps`)
6. `todo_list` (`L1448–1454`: `items: [{ id, content, status }]`)
7. `user_input_request` (`L1455–1462`: `requestId`, `questions`, `status`, `answers`)
8. `file_change` (`L1463–1473`: `changes: [{ path, kind, diff, additions, deletions }]`, `status`)
9. `command_execution` (`L1474–1485`: `command`, `cwd`, `status`, `exitCode`, `durationMs`, `outputPreview`, `outputTruncated`, `outputStreamId`)
10. `file_search` (`L1486–1491`: `query`, `matches`, `truncated`)
11. `web_search` (`L1492–1497`: `query`, `action`, `results`)
12. `approval_request` (`L1498–1508`: `requestId`, `requestKind`, `title`, `detail`, `status`, `decision`)
13. `checkpoint` (`L1509–1517`: `checkpointId`, `scopeId`, `appRunOrdinal`, `status`, `files`, `filesOmitted?: true`)
14. `run_interrupt_request` (`L1518–1522`)
15. `run_interrupt_result` (`L1523–1527`)
16. `system_notice` (`L1528–1532`)
17. `error` (`L1533–1538`)
18. `compaction` (`L1539–1546`: `trigger`, `direction`, `tokensBefore`, `tokensAfter`, `summary`)
19. `handoff` (`L1547–1565`: `handoffId`, `fromProvider`, `toProvider`, `strategy`, `status`)
20. `fork` (`L1566–1580`: `sourceThreadId`, `targetThreadId`, `sourceRunOrdinal`)
21. `thread_created` (`L1581–1588`: `childThreadId`, `relation`, `title`)
22. `secret_request` (`L1393–1400, 1589`: `secretName`, `envVarName`, `description`, `status`)
23. `subagent` (`L1590–1607`: `subagentId`, `childThreadId`, `agentLabel`, `prompt`, `promptOmitted?`, `result`, `resultOmitted?`, `status`)
24. `dynamic_tool` (`L1608–1615`: `toolName`, `arguments`, `result`, `status`)

### 2.2 `IdAllocatorV2` (`packages/provider-core/src/server/IdAllocator.ts:183-433`)
- **`allocate` (`randomUuidV4`, `${prefix}:${uuid}`):** Mints random UUIDv4 IDs for 17 entity types: `thread`, `run`, `runAttempt`, `node`, `subagent`, `providerSession`, `providerThread`, `providerTurn`, `turnItem`, `message`, `plan`, `runtimeRequest`, `checkpointScope`, `checkpoint`, `contextHandoff`, `contextTransfer`, `event`.
- **`derive` (`joinId(prefix, ...parts)`, `encodeURIComponent` segments joined by `:`):**
  - `derive.threadFromProviderThread` → `thread:provider:<driver>[:provider-instance:<instanceId>]:native-thread:<nativeThreadId>`
  - `derive.runFromThreadOrdinal` → `run:thread:<threadId>:ordinal:<ordinal>`
  - `derive.runAttemptFromOrdinal` → `run-attempt:run:<runId>:attempt:<attemptOrdinal>`
  - `derive.providerSessionShared` → `provider-session:provider-instance:<instanceId>:shared`
  - `derive.providerSessionForThread` → `provider-session:provider-instance:<instanceId>:thread:<threadId>[:native-session:<id>|:native-thread:<id>]`
  - `derive.providerThread` → `provider-thread:provider:<driver>[:provider-instance:<id>][:thread:<id>]:native-thread:<id>`
  - `derive.providerTurn` → `provider-turn:provider-thread:<providerThreadId>:native-turn:<nativeTurnId>`
  - `derive.nodeFromProviderItem` → `node:provider:<driver>[:provider-turn:<id>]:native-item:<id>`
  - `derive.subagentFromProviderTask` → `subagent:provider:<driver>[:thread:<id>]:native-task:<id>`
  - `derive.turnItemFromProviderItem` → `turn-item:provider:<driver>[:provider-turn:<id>]:native-item:<id>[:segment:<segment>]`
  - `derive.messageFromProviderItem` → `message:provider:<driver>[:provider-turn:<id>]:native-item:<id>[:segment:<segment>]`

---

## 3. Commands, Domain Events, Wire Protocol & MCP Surface

### 3.1 The 55 Public Commands & 8 Internal Commands (`orchestrationV2.ts:2677-3236`)
- **Thread Lifecycle & Organization (29):** `thread.stop`, `thread.create`, `thread.archive`, `thread.guest-share.close`, `thread.guest-message.dismiss`, `thread.unarchive`, `thread.delete`, `thread.settle`, `thread.auto-settle`, `thread.unsettle`, `thread.snooze`, `thread.unsnooze`, `thread.auto-settle.set`, `thread.pin`, `thread.unpin`, `thread.pin.reorder`, `thread.active.reorder`, `thread.visit`, `thread.mark-unread`, `thread.metadata.update`, `thread.pull-request.link`, `thread.pull-request.unlink`, `thread.pull-request-link.sync`, `thread.pull-request.watch`, `thread.pull-request.sync`, `thread.title.regeneration.complete`, `thread.runtime-mode.set`, `thread.interaction-mode.set`, `thread.model-selection.set`.
- **Session, Message Dispatch, Prepared Runs & Queue (15):** `provider-session.detach`, `message.dispatch` (supports `guestMessageId?: GuestMessageId`), `prepared-run.release`, `notification.delivery.accept`, `prepared-run.progress`, `prepared-run.fail`, `prepared-run.retry`, `subagent.stop`, `run.interrupt`, `queued-message.promote-to-steer`, `queue.resume`, `queued-run.start`, `queued-run.reorder`, `queued-run.cancel`, `queued-run.edit`.
- **Runtime Requests & Checkpoints (3):** `runtime-request.respond`, `thread.user-input.dismiss`, `checkpoint.rollback`.
- **Lineage, Delegation & Provider Switching (8):** `thread.fork`, `thread.merge_back`, `delegated_task.request`, `delegated_task.wake-policy`, `delegated_task.completion-delivery.acknowledge`, `delegated_task.completion-delivery.dispose`, `thread.created.record`, `provider.switch`.
- **Internal Commands (`OrchestrationV2InternalCommand`, 8):** `thread.guest-share.open`, `thread.guest-share.redeem`, `thread.guest-message.submit`, `thread.pull-request-watch.sync`, `checkpoint.rollback.fail`, `runtime-request.delivery.fail`, `thread.background-work.settle`, `secret_request.record`.

### 3.2 The 45 Domain Events (`OrchestrationV2DomainEvent`, `orchestrationV2.ts:1677-1821`)
- **Thread State Events (23):** `thread.created`, `thread.archived`, `thread.unarchived`, `thread.deleted`, `thread.settled`, `thread.unsettled`, `thread.snoozed`, `thread.unsnoozed`, `thread.pinned`, `thread.auto-settle-set`, `thread.unpinned`, `thread.pin-reordered`, `thread.active-reordered`, `thread.visited`, `thread.marked-unread`, `thread.metadata-updated`, `thread.pull-request-synced`, `thread.pull-request-link-updated`, `thread.runtime-mode-updated`, `thread.interaction-mode-updated`, `thread.model-selection-updated`, `thread.provider-switched`, `thread.guest-share-updated`.
- **Run, Attempt, Node & Subagent Events (7):** `run.created`, `run.updated`, `run.background-work-cancelled`, `run-attempt.created`, `run-attempt.updated`, `node.updated`, `subagent.updated`.
- **Provider & Transcript Events (9):** `provider-session.attached`, `provider-session.updated`, `provider-session.detached`, `provider-thread.updated`, `provider-turn.updated`, `runtime-request.updated`, `message.updated`, `turn-item.updated`, `plan.updated`.
- **Checkpoint & Context Transfer Events (6):** `checkpoint-scope.created`, `checkpoint.captured`, `checkpoint.rollback-requested`, `context-handoff.updated`, `context-transfer.created`, `context-transfer.updated`.

### 3.3 The 18 WebSocket RPCs, 5 HTTP Snapshot Endpoints & Wire Compression
- **`ORCHESTRATION_V2_WS_METHODS` (`orchestrationV2.ts:3240-3259`, 18 methods):** `orchestration.dispatchCommand`, `orchestration.getTurnDiff`, `orchestration.getFullThreadDiff`, `orchestration.searchThreads`, `orchestration.searchThread`, `orchestration.searchThreadStream`, `orchestration.getArchivedShellSnapshot`, `orchestration.getThreadFamily`, `orchestration.getWorktreeThreadIds`, `orchestration.getThreadProjection`, `orchestration.getWorkflowScript`, `orchestration.stopWorkflow`, `orchestration.getTurnItem`, `orchestration.launchThread`, `orchestration.subscribeArchivedShell`, `orchestration.subscribeShell`, `orchestration.subscribeThread`, `orchestration.subscribeCommandOutput` (plus `guestShare.create` in `rpc.ts:528`).
- **HTTP Snapshot & Transcript Endpoints (`apps/server/src/orchestration-v2/http.ts:159-273`, 5 GET routes):**
  1. `GET /api/orchestration/shell-snapshot` (supports `?omitIdleSubagents=1` via `threadFamilies.ts`, skipping ~1/3 of cold thread list payloads)
  2. `GET /api/orchestration/threads/:threadId/snapshot`
  3. `GET /api/orchestration/threads/:threadId/transcript`
  4. `GET /api/orchestration/threads/:threadId/bounded-snapshot` (`defaultTurnItemLimit = 100`, `maxTurnItemLimit = 500`, supports `?compactIds=1&dedupeBodies=1`)
  5. `GET /api/orchestration/threads/:threadId/history-page` (`defaultLimit = 50`, `maxLimit = 200`, supports `?compactIds=1`)
- **ID Table Interning (`packages/contracts/src/orchestrationV2IdTable.ts`) & Body Deduplication (`packages/shared/src/orchestrationV2DedupedBodies.ts`):**
  - `withRecordIdTable` (`internRecordIds` / `expandRecordIds`) replaces all `id`, `*Id`, and `*Ids` fields across snapshot/stream records with integer indices into a single `ids: Array<string | number>` array when `idTable: true`.
  - `dedupeSnapshotBodies` and `WireProjection.ts:makeLiveBodyDeduper()` (`LIVE_DEDUPE_MEMORY = 32`) strip duplicate message text, checkpoint file arrays, and subagent prompt/result strings on `TurnItem`s and stream updates (`textOmitted`, `filesOmitted`, `promptOmitted`, `resultOmitted`).

### 3.4 Live Guest Share (`packages/contracts/src/guestShare.ts`, `GuestShareDecider.ts`)
- Enables sharing a live thread via a T3 Connect link (`GUEST_SHARE_TTL_MS = 86_400_000` / 24h) signed with a 128-bit truncated HMAC (`<shareId>.<hmac>`).
- Single-browser redemption (`thread.guest-share.redeem` binds `guestName`; max `GUEST_NAME_MAX_LENGTH = 40`).
- Guests view a redacted live stream (`SNAPSHOT_ROW_LIMIT = 600`, `SNAPSHOT_USER_TURN_LIMIT = 20`) and can submit up to `GUEST_PENDING_MESSAGE_LIMIT = 3` pending messages (`GUEST_MESSAGE_MAX_LENGTH = 4_000`, up to 4 attachments / 10 MiB each).
- Owner approval gate: guest messages stay `pending` until the owner approves via `message.dispatch` with `guestMessageId` (which prefixes `"Message from guest ${name}, sent by the owner:\n\n${guestText}"` in `GuestShareDecider.ts`) or rejects via `thread.guest-message.dismiss`.

### 3.5 Orchestrator & Thread MCP Toolkits (`35` Tools Total)
- **`OrchestratorToolkit` (`apps/server/src/mcp/toolkits/orchestrator/tools.ts:296-315`, 18 tools):** `orchestrator_capabilities`, `delegate_task` (`relation: "subagent" | "fork"`, `mode: "async" | "wait"`), `task_status`, `task_cancel`, `t3_workflow_stop`, `t3_background_wait`, `schedule_task`, `list_scheduled_tasks`, `update_scheduled_task`, `delete_scheduled_task`, `request_secret`, `create_threads`, `t3_thread_list`, `t3_thread_read`, `t3_thread_update`, `t3_thread_send`, `t3_thread_wait`, `t3_thread_interrupt`.
- **`ThreadToolkit` (`apps/server/src/mcp/toolkits/thread/tools.ts:285-303`, 17 tools):** `run_scheduled_task_now`, `t3_thread_search`, `t3_thread_fork`, `t3_thread_merge_back`, `t3_thread_transfers`, `t3_thread_configuration`, `t3_thread_configure`, `t3_pending_request_list`, `t3_pending_request_read`, `t3_pending_request_respond`, `t3_thread_organize`, `t3_queue_list`, `t3_queue_read`, `t3_queue_edit`, `t3_queue_cancel`, `t3_queue_reorder`, `t3_queue_promote_to_steer`.
