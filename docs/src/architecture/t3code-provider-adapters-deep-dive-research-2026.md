---
title: "T3 Code 9-Provider Adapter Deep-Dive: JSON-RPC, ACP, Connect-RPC, SSE & SQLite (2026)"
description: "File-and-symbol-anchored wire protocol reference across all 9 pingdotgg/t3code provider drivers: Codex, Claude (including ENABLE_TOOL_SEARCH=auto cache-write fix), ACP Registry (13 profiles), Antigravity, Grok, Cursor, OpenCode 1.x/2.x, Muse, and Pi."
category: "Architecture SSOTs"
status: "research"
topic: "agent-orchestrator"
domain: "orchestration"
authorship: "llm-generated"
authorship_model: "claude-opus-4-7"
---

# T3 Code 9-Provider Adapter Deep-Dive: JSON-RPC, ACP, Connect-RPC, SSE & SQLite (2026)

<!-- kb:summary -->
This document provides the wire-level engineering reference for all 9 provider drivers in `pingdotgg/t3code` (`~/dev/t3code`, commit `80455376f01f19c10a06dc08b0629486c017bf79`). It documents the transport protocols, Orchestration v2 adapters (`ClaudeAdapterV2.ts`, `CodexAdapterV2.ts`, `AntigravityAdapterV2.ts`), session engines, permission mappers, Claude's `ENABLE_TOOL_SEARCH=auto` 10x prompt-cache write mitigation, Antigravity's Connect-RPC CSRF harvesting and IPv6 reachability diagnostics, Cursor's SQLite hex-blob transcript DAG reader, OpenCode's dual 1.x/2.x runtime support, and Muse's parallel `git worktree` fan-out.
<!-- /kb:summary -->

## 1. Provider Driver Matrix (`packages/provider-*` & `apps/server/src/orchestration-v2/Adapters/`)

| Driver ID | Primary Packages & V2 Adapter Paths | Wire Transport & Framing | Native Rollback / Checkpoint |
|---|---|---|---|
| `codex` | `packages/effect-codex-app-server`, `apps/server/src/provider/Drivers/CodexDriver.ts`, `apps/server/src/orchestration-v2/Adapters/CodexAdapterV2.ts` | JSON-RPC 2.0 bidirectional over stdio (`codex app-server`) | Yes (`thread/rollback` RPC with `numTurns`) |
| `claude` | `apps/server/src/provider/Drivers/ClaudeDriver.ts`, `apps/server/src/orchestration-v2/Adapters/ClaudeAdapterV2.ts` | `@anthropic-ai/claude-agent-sdk` `query()` async generator + control stream | Yes (`Query.rewindFiles(userMessageId)`) |
| `acp-registry` | `packages/effect-acp`, `packages/provider-acp`, `packages/provider-acp-registry` | `@agentclientprotocol/sdk` (`ndJsonStream` over stdio) | Via T3 Git checkpoints (`refs/t3/checkpoints/`) |
| `antigravity` | `apps/server/src/provider/Drivers/AntigravityDriver.ts`, `apps/server/src/orchestration-v2/Adapters/AntigravityAdapterV2.ts`, `AntigravityLanguageServer.ts`, `AntigravityAcpSupport.ts`, `AntigravityAuth.ts` | Connect-RPC over local HTTP/2 TLS (`127.0.0.1:<httpsPort>`) + CSRF token | Trajectory step truncation |
| `cursor` | `packages/provider-cursor` (`CursorAcpExtension.ts`, `CursorTranscriptReader.ts`) | `cursor-agent acp` stdio + direct SQLite read (`~/.cursor/chats/<hash>/<session>/store.db`) | Via T3 Git checkpoints |
| `grok` | `packages/provider-grok` (`GrokCliClient.ts`, `GrokXaiApiDriver.ts`) | Dual engine: `grok` CLI subprocess OR direct `https://api.x.ai/v1/responses` SSE + 9 local tools | Via T3 Git checkpoints |
| `opencode` | `packages/provider-opencode` (`OpenCodeClient.ts`, `OpenCode2Client.ts`) | Gen 1 (`opencode serve` HTTP + SSE `/event`) OR Gen 2 (`opencode app-server` JSON-RPC) | Gen 1 `/session/:id/revert` / Gen 2 |
| `muse` | `packages/provider-muse` (`MuseDriver.ts`, `MuseWorktreePool.ts`) | Multi-variant fan-out (`1..4` parallel `git worktree` instances) wrapping `claude`/`codex`/`opencode` | Per-variant worktree branch reset |
| `pi` | `packages/provider-pi` (`PiRpcClient.ts`) | `@mariozechner/pi-coding-agent` (`pi --mode rpc --no-session` NDJSON stdio) | Via T3 Git checkpoints |

---

## 2. Codex Driver (`packages/effect-codex-app-server` & `CodexAdapterV2.ts`)

- **Process Lifecycle & Protocol:** Spawns `codex app-server` over stdio using JSON-RPC 2.0 (`CodexAppServerClient.ts`).
- **Client-to-Server Requests:** `initialize`, `thread/start`, `thread/resume`, `thread/fork`, `thread/rollback` (`{ threadId, numTurns }`), `thread/compact/start`, `turn/start`, `turn/steer`, `turn/interrupt`, `review/start`, `command/exec`, `account/rateLimits/read`.
- **Server-to-Client Requests (Interactive Gates):**
  - `item/commandExecution/requestApproval` → `OrchestrationV2RuntimeRequest` (`kind: "command"`).
  - `item/fileChange/requestApproval` → `OrchestrationV2RuntimeRequest` (`kind: "file-change"`).
  - `item/tool/requestUserInput` → `OrchestrationV2RuntimeRequest` (`kind: "user_input"`).
  - `item/tool/call` → `OrchestrationV2RuntimeRequest` (`kind: "dynamic_tool_call"`).
  - `mcpServer/elicitation/request` → `OrchestrationV2RuntimeRequest` (`kind: "mcp-elicitation"`).

---

## 3. Claude, ACP Registry & Antigravity Drivers

### 3.1 Claude Agent SDK (`ClaudeAdapterV2.ts` & `ClaudeDriver.ts`)
- **SDK Execution:** Invokes `query({ prompt: promptInputQueue, options })` from `@anthropic-ai/claude-agent-sdk`.
- **Critical `ENABLE_TOOL_SEARCH=auto` 10x Prompt-Cache Write Fix (`ClaudeAdapterV2.ts:66-105, 1295-1356`, commit `24c80f34`):**
  - Claude Code's default `ENABLE_TOOL_SEARCH=auto` (or `auto:N` where `0 < N < 100`) sends all MCP tool schemas inline whenever they fit under `N%` of a 1M context window, causing ~10x prompt-cache write billing per session.
  - `claudeToolSearchOverride` inspects `.claude/settings.local.json` (at both the main git checkout root and worktree `cwd`), `.claude/settings.json`, `<claudeHome>/settings.json`, and `environment.ENABLE_TOOL_SEARCH`. Whenever the effective setting is unset or `auto` / `auto:N` (`0 < N < 100`), T3 overrides `effectiveQuerySettings.env.ENABLE_TOOL_SEARCH = "true"` (preserving explicit `false`/`0` or `auto:0`/`auto:100`).
- **Background Task Settlement (`t3_background_wait`, commit `91ea0e18`):** Injects `CLAUDE_BACKGROUND_WAIT_INSTRUCTIONS` and marks `OrchestrationV2PendingBackgroundTask.awaited = true` so background tasks only hold turn settlement when explicitly awaited by the agent.
- **Effort & Thinking Resolution (`resolveEffort`):** Maps `low` / `medium` / `high` / `xhigh` / `max` / `"ultrathink"` (`maxThinkingTokens`) onto `SDKOptions.effort` and `thinking`.

### 3.2 ACP Registry (`packages/provider-acp` & `packages/provider-acp-registry`)
- **Protocol Engine (`AcpSessionEngine.ts`):** Uses `@agentclientprotocol/sdk` (`ClientSideConnection`, `ndJsonStream`, `PROTOCOL_VERSION`).
- **13 Curated Profiles (`packages/provider-acp-registry/src/server/adapter.ts`):** `claude-acp`, `github-copilot-cli` (or local `copilot` binary), `cursor`, `devin`, `factory-droid`, `gemini`, `goose`, `junie`, `kimi`, `kiro`, `mistral-vibe`, `opencode`, `qwen-code`.
- **Mistral Vibe Extension (`adapter.ts:86-129`):** `registerMistralVibeAcpExtensions` handles `_session/retrying` notifications and rate-limit error code `MISTRAL_VIBE_RATE_LIMITED = -31001`.

### 3.3 Antigravity Language Server (`AntigravityAdapterV2.ts`, `AntigravityLanguageServer.ts`, `AntigravityAcpSupport.ts`, `AntigravityAuth.ts`)
- **Connect-RPC over Local HTTP/2 TLS (`AntigravityLanguageServer.ts`):** Spawns `language_server_<platform>` with `--csrf_token <uuid> --random_port`, discovers listening HTTPS/HTTP/extension ports via `lsof` / `ss` / `netstat`, and calls `/exa.language_server_pb.LanguageServerService/{GetUserStatus, StartCascade, SendUserCascadeMessage, StreamCascadeReactiveUpdates, HandleCascadeUserInteraction, RevertToCascadeStep, CancelCascadeInvocation}`.
- **CSRF Header Harvesting:** Intercepts the language server's callback to the local `--extension_server_port` HTTP listener to capture any rotated `x-codeium-csrf-token`.
- **RuntimeMode Mapping (`AntigravityAcpSupport.ts:83-93`):** Maps `"full-access"` → `"yolo"`, `"auto-accept-edits"` → `"auto_edit"`, and both `"auto"` and `"approval-required"` → `"default"`.
- **IPv6 Google OAuth Reachability Diagnostic (`AntigravityAuth.ts:36-41`, commit `28878555`):** Emits `NO_RESPONSE_HINT_MS = 30_000` (`"Antigravity has not reached Google after 30 seconds. Check this machine's network connection. Broken IPv6 is a common cause."`) when bundled Python lacks Happy Eyeballs IPv4 fallback.

---

## 4. Cursor, Grok, OpenCode, Muse & Pi Drivers

### 4.1 Cursor Driver (`packages/provider-cursor`)
- **ACP Extension (`CursorAcpExtension.ts`):** Handles `cursor/ask_question`, `cursor/create_plan`, `cursor/update_todos`, and `cursor/task`.
- **SQLite Hex-Blob Transcript Reader (`CursorTranscriptReader.ts`):** Opens `~/.cursor/chats/md5(workspacePath)/<sessionId>/store.db` (`meta` table hex-encoded JSON + `blobs` table protobuf/JSON DAG nodes) to recover full message and tool history when resuming native Cursor sessions.

### 4.2 Grok Driver (`packages/provider-grok`)
- **Dual Engine:** `GrokCliClient.ts` wraps the `@vibe-kit/grok-cli` subprocess; `GrokXaiApiDriver.ts` speaks directly to `https://api.x.ai/v1/responses` (SSE) and executes 9 local workspace tools (`bash`, `read_file`, `write_file`, `edit_file`, `list_directory`, `glob`, `grep`, `web_search`, `x_search`).

### 4.3 OpenCode 1.x & 2.x (`packages/provider-opencode`)
- **OpenCode 1.x (`OpenCodeClient.ts`):** Spawns `opencode serve --hostname 127.0.0.1 --port <port>`, consuming `GET /event` SSE (`message.part.updated`, `permission.asked`, `question.asked`) and dispatching `POST /session/:id/prompt_async`.
- **OpenCode 2.x (`OpenCode2Client.ts`):** Connects to `opencode app-server` over JSON-RPC (`session.create`, `turn.start`, `turn.steer`, `request.respond`).

### 4.4 Muse Multi-Variant Worktree Orchestrator (`packages/provider-muse`) & Pi Driver (`packages/provider-pi`)
- **Muse (`MuseDriver.ts`, `MuseWorktreePool.ts`):** Provisions `variantCount` (`1..4`) isolated git worktrees under `<worktreesDir>/muse/<threadId>/variant-<i+1>`, dispatches the prompt in parallel across inner providers (`claude`, `codex`, `opencode`), and exposes an `adopt_variant` tool to cherry-pick or reset the parent workspace to the winning variant's branch.
- **Pi (`PiRpcClient.ts`):** Spawns `pi --mode rpc --no-session` (`@mariozechner/pi-coding-agent`) over NDJSON stdio (`prompt`, `steer`, `abort`, `set_model`, `set_thinking_level`, `extension_ui_response`).
