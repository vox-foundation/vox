---
title: "T3 Cloneathon & Community Clones Research: t3-cloneathon, omnichat, NotT3Chat & TomDoesTech/t3-chat (2026)"
description: "Source-anchored architecture of 4 additional T3 repositories in ~/dev: lulkebit/t3-cloneathon (Multi-Model Consensus SSE & ResponseQualityAnalyzer), guitaripod/omnichat (3rd Place Cloneathon Champion Battery Unit D1 ledger & divergence-point branching), shaltielshmid/NotT3Chat (C# .NET 10 SignalR SemaphoreSlim mid-stream join & RepetitionGuard), and TomDoesTech/t3-chat (create-t3-app taxonomic control case)."
category: "Architecture SSOTs"
status: "research"
topic: "agent-orchestrator"
domain: "orchestration"
authorship: "llm-generated"
authorship_model: "claude-opus-4-7"
---

# T3 Cloneathon & Community Clones Research: `t3-cloneathon`, `omnichat`, `NotT3Chat` & `TomDoesTech/t3-chat` (2026)

<!-- kb:summary -->
This document provides the file-and-symbol-anchored audit of 4 additional T3 repositories cloned in `~/dev`: `lulkebit/t3-cloneathon` (`99eb9ab1`), `guitaripod/omnichat` (`b864415e`, 3rd Place T3 Cloneathon Champion), `shaltielshmid/NotT3Chat` (`913afaae`), and `TomDoesTech/t3-chat` (`4cc8fac3`). It details `t3-cloneathon`'s parallel Multi-Model Consensus SSE fan-out and Flesch-Kincaid `ResponseQualityAnalyzer`, `omnichat`'s Cloudflare D1 Battery Unit (`1 BU = $0.001`) billing ledger and tree-branch `divergencePoint` comparison, `NotT3Chat`'s single-file C# .NET 10 SignalR `SemaphoreSlim(1)` zero-drop mid-stream multi-tab join and 1024-slot n-gram `RepetitionGuard`, and `TomDoesTech/t3-chat` as a taxonomic control case (`create-t3-app` human WebSocket chat).
<!-- /kb:summary -->

## 1. `lulkebit/t3-cloneathon` (`~/dev/t3-cloneathon` — Multi-Model Consensus & Quality Analytics)

- **Commit SHA:** `99eb9ab16c9349e9973138b5b71b0af5e9d70b8b`
- **Stack Clarification:** Although branded `"convex-chat"` in `package.json` and `README.md`, its backend is **Next.js 15.3.8 App Router + Supabase Auth/PostgreSQL/Storage (`@supabase/supabase-js` `^2.50.0`) + OpenRouter** (zero Convex dependencies).
- **PostgreSQL Schema (`LOCAL_TESTING_GUIDE.md:97-262` & `src/types/chat.ts`):** `profiles` (`openrouter_api_key`), `conversations` (`model` stores `"consensus:modelA,modelB,..."`), `messages` (`content` stores JSON-serialized `ConsensusResponse[]` in consensus mode), `attachments`, and `message_quality_metrics`.
- **Multi-Model Consensus SSE Fan-Out (`src/app/api/chat/consensus/route.ts`, `src/components/ConsensusMessage.tsx`):**
  - Dispatches `models.map(...)` in parallel via `Promise.all(modelPromises)` over a single multiplexed `ReadableStream` SSE response emitting `consensus_start`, per-model `consensus_update` (`{ modelIndex, model, content, isStreaming }`), `consensus_complete` (with `responseTime` and `qualityMetrics`), `consensus_error`, `title_update`, and `consensus_final`.
  - `ConsensusMessage.tsx` toggles between **Stacked View** (`List`) and **Side-by-Side Grid View** (`Grid3X3`, 1–4 columns) with average response latency metrics.
- **Heuristic `ResponseQualityAnalyzer` (`src/lib/response-quality-analyzer.ts` & `/api/quality-analytics`):**
  - Computes a 0–100 `qualityScore = relevance * 0.3 + clarity * 0.25 + coherence * 0.2 + completeness * 0.2 + readability * 0.05` (minus latency/brevity penalties) using Flesch Reading Ease (`206.835 - 1.015*(words/sentences) - 84.6*(syllables/words)`), 19 logical connectors, structural markdown regexes, lexical repetition ratio, and prompt keyword overlap.
  - `GET /api/quality-analytics` (`src/app/api/quality-analytics/route.ts`) aggregates `message_quality_metrics` over `7d`/`30d`/`90d` windows into daily trends, per-model rankings, and 5-tier quality distributions (`excellent`, `good`, `fair`, `poor`, `veryPoor`).

---

## 2. `guitaripod/omnichat` (`~/dev/omnichat` — 3rd Place T3 Cloneathon Champion)

- **Commit SHA:** `b864415e55ba3e4bb3489b06919e405031b8bb8a`
- **Stack:** Next.js 15.3.3 Edge (`runtime = 'edge'`), Cloudflare Pages/Workers + D1 (SQLite via Drizzle ORM `0.44.2`, 13 tables in `src/lib/db/schema.ts`) + R2 (`R2_STORAGE`), Clerk (web `/api/*`) + native Apple Sign-In JWT (`/api/v1/*`), Stripe, and 6 direct AI providers (`openai`, `anthropic`, `google`, `xai`, `deepseek`, `ollama`).
- **"Battery System" Pay-Per-Use Token Billing Ledger (`src/lib/battery-pricing.ts`, `src/lib/usage-tracking.ts`, `src/services/ai/stream-wrapper.ts`):**
  - Defines **1 Battery Unit (BU) = \$0.001** (150% markup + 30% safety buffer over provider token rates: e.g., `gemini-1.5-flash` = 0.61 BU/1K, `gpt-4.1-mini` = 3.25 BU/1K, `claude-sonnet-4` = 29.25 BU/1K, `claude-opus-4` = 146.25 BU/1K, local Ollama = 0 BU).
  - Pre-flight check (`checkBatteryBalance` in `src/app/api/chat/route.ts:219`) returns HTTP `402 Payment Required` when `totalBalance < estimatedCost`.
  - `createTokenTrackingStream` (`stream-wrapper.ts`) counts streamed tokens via `StreamingTokenTracker` and on stream finish executes a 4-step D1 write (`trackApiUsage` in `usage-tracking.ts:130-198`): (1) insert `api_usage`, (2) decrement `user_battery.total_balance`, (3) append signed row to `battery_transactions` (`balance_after`), (4) upsert `daily_usage_summary`, then emits a final `data: {"type":"usage","usage":...,"battery":{"batteryUsed":...,"newBalance":...}}` SSE frame before `data: [DONE]`.
- **Tree Branching & `divergencePoint` Comparison (`src/services/branching/branch-manager.ts`, `src/components/chat/branch-comparison.tsx`):**
  - `BranchManager.buildMessageTree` constructs a `MessageNode` tree from `messages.parent_id` (`main`, `main-<msgId>-0`), and `BranchComparison` renders a multi-column side-by-side comparison modal across any N selected branches, automatically computing `divergencePoint` to visually separate shared prefix history from divergent branch responses.
- **Dual-Mode Ollama (`src/services/ai/providers/ollama.ts`, `ollama-client.ts`) & Offline Sync (`offline.ts`, `sync.ts`):**
  - `OllamaClientProvider` talks directly from the browser to `http://localhost:11434/api/chat`, bypassing the Cloudflare Worker edge route and battery ledger (`isOllamaModel`).
  - `SyncService` queues offline mutations in IndexedDB (`omnichat-offline` v1 `syncQueue`) and replays them on `window.addEventListener('online')` with 3 retries.
- **Go CLI API Validator (`go-cli/cmd/omnichat-validator/main.go`):** Validates all 43 OpenAPI endpoints (`openapi/openapi.json`) across 10 categories using `--clerk` or `--bearer` JWT auth.

---

## 3. `shaltielshmid/NotT3Chat` (`~/dev/NotT3Chat` — C# .NET 10 + SignalR + `RepetitionGuard`)

- **Commit SHA:** `913afaaea495a63076bf085d66597ca0f98a7b04`
- **Stack:** Single-file 1,147-LoC C# 12 / .NET 10 (`net10.0`) ASP.NET Core `backend/Program.cs` + SignalR (`ChatHub` at `/chat`, `MaximumReceiveMessageSize = 64 KB`) + EF Core SQLite + `IMemoryCache` + `LlmTornado 3.8.26` (10 providers) + `ExaApiService` + React 19 (`streamdown 1.4.0` with `parseIncompleteMarkdown={true}`).
- **Zero-Drop Mid-Stream Multi-Tab Join via SignalR Groups + `SemaphoreSlim(1)` (`backend/Program.cs:623-707, 750-795`):**
  - In-flight streams are cached in `_memoryCache` under `convoId` as `StreamingMessage(StringBuilder SbMessage, NotT3Message Message, SemaphoreSlim Semaphore, CancellationTokenSource Ct)`.
  - When a client refreshes or opens an active chat in a second tab (`ChooseChat(convoId)` while `conversation.IsStreaming == true`), the hub acquires `await currentMsg.Semaphore.WaitAsync()`, sends `BeginAssistantMessage` + the accumulated `currentMsg.SbMessage.ToString()` via `NewAssistantPart`, and adds the connection to the `convoId` SignalR group **before** releasing the semaphore—guaranteeing zero dropped or duplicated tokens at the join boundary.
- **1024-Slot Power-of-2 Ring-Buffer `RepetitionGuard` (`backend/Program.cs:1080-1145`):**
  - Uses a fixed ring buffer (`BufferSize = 1024`, bitmask `ModuloNumber = 1023`, `[MethodImpl(MethodImplOptions.AggressiveInlining)]`) of token hash codes (`token.GetHashCode()`) to detect both single-token loops (`MaxTokenRep = 32`) and repeating n-gram cycles of period `p ∈ [3, 12]` (`MinGramRep = 5`), automatically cancelling `streamingMessage.Ct` when a model enters a degeneration loop.
- **Inline `<think>` & `<WebSearch>` Tag Interleaving & Context Sanitization (`backend/Program.cs:554-586, 629-670`):**
  - Streams `<think>...</think>` and `<WebSearch>[...]</WebSearch>` inline (capping Exa calls at `totalSearches < 3` per turn). On subsequent turns, `CleanMessageFromSpecialEntities` strips `<think>[\S\s]*?</think>` and replaces `<WebSearch>.*?</WebSearch>` with `[web_search was called, results removed to save context.]` while escaping prompt-injection tokens (`<|im_start|>`, `<tool_call>`).

---

## 4. `TomDoesTech/t3-chat` (`~/dev/t3-chat-tomdoestech` — Taxonomic Control Case)

- **Commit SHA:** `4cc8fac3ec947e7b9889588ff292d66862b0c5b0`
- **Taxonomic Role:** Created in July 2022 (`prisma/migrations/20220717035410_init`) as a `create-t3-app` ("T3 Stack") tutorial project, this repository is a **human-to-human WebSocket chat room app**, not an AI assistant clone of `t3.chat`.
- **Architecture:** Next.js 12.2.1 Pages Router + tRPC v9 (`httpBatchLink` on `:3000` + `wsLink` on `:3001` via `src/server/wsServer.ts` `applyWSSHandler`) + Prisma 4 SQLite + NextAuth.js 4. Chat messages are ephemeral in-memory events bridged via a Node `EventEmitter` (`room.send-message` mutation → `ctx.ee.emit` → `room.onSendMessage` `trpc.Subscription`).
