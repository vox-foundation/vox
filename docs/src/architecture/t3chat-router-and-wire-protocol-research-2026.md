---
title: "T3 Chat Router & Wire Protocol Research: t3router & pi-t3chat (2026)"
description: "Source-anchored wire protocol, JS bundle regex scraper, tRPC procedures, WorkOS session rotation, credit-delta polling, tool-call translation, refusal auto-correction, and lazy MCP wrapper across vibheksoni/t3router and vibheksoni/pi-t3chat."
category: "Architecture SSOTs"
status: "research"
topic: "agent-orchestrator"
domain: "orchestration"
authorship: "llm-generated"
authorship_model: "claude-opus-4-7"
---

# T3 Chat Router & Wire Protocol Research: `t3router` & `pi-t3chat` (2026)

<!-- kb:summary -->
This document provides the line-verified reverse-engineering specification of `vibheksoni/t3router` (`~/dev/t3router`, commit `5314bbbf0af66e724e3e07ffd458ef5919d1eed1`) and `vibheksoni/pi-t3chat` (`~/dev/pi-t3chat`, commit `b39873245362c33f68a9a3e044423f69769069f9`). It documents the `POST https://t3.chat/api/chat` wire contract, Chrome 136 TLS impersonation (`rquest::Impersonate::Chrome136`), WorkOS `x-workos-session` automatic rotation, the `/assets/*.js` regex bundle scraper (`eval_js_number` vs `parseFloat`), all 6 tRPC endpoints, `poll_credit_delta`, `ToolCallTranslator`, `ToolRefusalDetector`, and the 3-tool lazy MCP wrapper (`list_mcps`, `list_mcp_tools`, `call_mcp`).
<!-- /kb:summary -->

## 1. Repository & Module Inventory

### 1.1 `vibheksoni/t3router` (`~/dev/t3router` — Rust 2021)
- **`src/lib.rs` & `src/t3/mod.rs:1-10`:** Declares `pub mod client; pub mod config; pub mod history; pub mod message; pub mod models; pub mod repl; pub mod session; pub mod usage;` (with zero `pub use` re-exports in `mod.rs`).
- **`src/t3/config.rs:1-58`:** Defines `ReasoningEffort` (`Low`, `Medium`, `High`, `Default` = `Medium`) and `Config` (`include_search: bool`, `reasoning_effort: ReasoningEffort`).
- **`src/t3/client.rs:174-792`:** Defines `ChatResponse` (`message: Message`, `thread_id: String`, `model: String`, `credits_before: Option<f64>`, `credits_after: Option<f64>`, `credits_deducted: Option<f64>`, `finish_reason: Option<String>`), `SseAccumulator`, `poll_credit_delta`, and `Client` (`new`, `warmup`, `init`, `refresh_session`, `send`, `send_stream`, `send_with_credits`, `send_with_credits_stream`, `send_with_image_download`, `finalize_credits`, `parse_response`, `new_conversation`, `resume_conversation`, `append_message`, `get_messages`, `clear_messages`, `download_image`, `get_thread_id`).
- **`src/t3/models.rs:1-665`:** `ModelsClient`, `ModelInfo`, `ModelCost`, `AttachmentSupport`, `ModelStatus`, `ModelBenchmarkEntry`, `BenchmarkMetric`, `parse_models_from_bundle`, `eval_js_number`.
- **`src/t3/usage.rs:1-415`:** `UsageClient`, `CustomerData`, `SubscriptionData`, `PricingData`, `CreditPack`, `TopUpInfo`, `SessionInfo` (`session_id`, `created_at`, `expires_at`, `ip_address`, `user_agent`), `UsageSummary`, `extract_trpc_result`.
- **`src/t3/session.rs:1-114`:** `SavedSession`, `SavedMessage`, `default_session_path()` (reads `T3CHAT_SESSION_PATH` before falling back to `~/.t3chat_session.json`), `save_session(&SavedSession)`, `load_session()`.
- **`src/t3/repl.rs:36-45, 176-254`:** Interactive REPL supporting `/help`, `/new`, `/resume`, `/save`, `/model [name]`, `/credits`, and `/quit` (alias `/exit`). *(Implementation note: `repl.rs:250` calls `poll_credit_delta` a second time after `client.send_with_credits_stream` already invoked it at `client.rs:744`.)*
- **`src/bin/t3chat.rs`:** Full CLI binary supporting `--list-models`, `--status`, `--benchmarks`, `--usage`, `--credits`, `--effort`, `--search`, `--save`, `--resume`, and interactive REPL mode.

### 1.2 `vibheksoni/pi-t3chat` (`~/dev/pi-t3chat` — TypeScript)
- **`index.ts:118-191`:** Pi coding-agent extension entrypoint registering flags `--t3chat-login` and `--t3chat-cookies` and slash commands `/t3chat-status`, `/t3chat-logout`, and `/t3chat-refresh`.
- **`auth.ts`:** Playwright browser login (`loginWithBrowser`), `~/.pi/agent/t3chat-auth.json` persistence (`chmod 0o600`), and `refreshSession` (`auth.getActiveSessions`).
- **`catalog.ts:147-276`:** Exports `getCachedCatalog` (`L262`) and `clearCachedCatalog` (`L276`), wrapping internal `fetchCatalog` (`L231`, scanning up to 12 JS chunks in parallel via `chunkPaths.slice(0, 12)`).
- **`config.ts:1-28`:** Exports `type ReasoningEffort`, `interface ChatConfig`, `defaultConfig()`, and `buildConfigObject()`.
- **`chat.ts:59-108` & `sse.ts:1-95`:** `chat.ts` defines `T3_BASE_URL = "https://t3.chat"`, `mapSSEEvent` (`L85–108`), and `streamT3Chat`; `sse.ts` defines `SSEEvent` (`text-delta`, `reasoning-delta`, `finish-step`, `finish`, `error`) and `parseSSEStream`.
- **`tools.ts:1-310`:** `ToolRegistry`, `ToolCallTranslator`, `ToolCallDeltaAccumulator`, and `ToolRefusalDetector`.
- **`mcp.ts:1-210`:** Lazy MCP wrapper (`list_mcps`, `list_mcp_tools`, `call_mcp`, `MAX_WRAPPER_ROUNDS = 6`).
- **`proxy.ts:40-48, 704-733`:** Localhost OpenAI-compatible HTTP proxy (`T3_PROXY_HOST = "127.0.0.1"`, `T3_PROXY_PORT = 42101`, fallback to `server.listen(0)` on `EADDRINUSE`) exporting `startProxy`, `stopProxy`, `PROXY_SECRET` (32-byte random hex verified via `crypto.timingSafeEqual`), `proxyCredentials`, and `setProxyCredentials`.
- **`tokens.ts`, `usage.ts`, `log.ts:1-32`:** `log.ts` exports `setNotifier`, `logInfo`, `logWarn`, `logError` with a pre-session `_buffer` queue.

---

## 2. `POST https://t3.chat/api/chat` Wire Contract & WorkOS Rotation

- **TLS Fingerprint (`t3router/src/t3/client.rs:205`):** Uses `rquest::Impersonate::Chrome136` to satisfy Cloudflare JA3/JA4/HTTP2 checks.
- **Automatic Session Rotation (`client.rs:235-252` & `pi-t3chat/auth.ts:112-160`):** Calls `GET https://t3.chat/api/trpc/auth.getActiveSessions?batch=1&input=%7B%220%22%3A%7B%22json%22%3Anull%2C%22meta%22%3A%7B%22values%22%3A%5B%22undefined%22%5D%7D%7D%7D`. If the response carries `x-workos-session` or `set-cookie: wos-session=...`, the client hot-swaps `wos-session` in memory and persists it.
- **Request Payload (`POST https://t3.chat/api/chat`):**
  ```json
  {
    "messages": [
      {
        "id": "<uuid-v4>",
        "role": "user",
        "parts": [{ "type": "text", "text": "..." }],
        "attachments": []
      }
    ],
    "threadMetadata": { "id": "<thread-uuid-v4>" },
    "responseMessageId": "<response-uuid-v4>",
    "model": "claude-sonnet-4-6",
    "convexSessionId": "<session-uuid-v4>",
    "modelParams": {
      "reasoningEffort": "medium",
      "includeSearch": false
    },
    "preferences": {
      "name": "",
      "occupation": "",
      "selectedTraits": [],
      "additionalInfo": ""
    },
    "userInfo": { "timezone": "America/Los_Angeles" },
    "isEphemeral": false
  }
  ```
- **SSE Event Stream (`client.rs:60-170`, `sse.ts:1-95`):** Emits `data: {"type":"reasoning-delta","delta":"..."}`, `data: {"type":"text-delta","delta":"..."}`, `data: {"type":"finish-step",...}` (mapped to `null` by `mapSSEEvent` in `chat.ts:104-105`), `data: {"type":"finish","finishReason":"stop"}`, and `data: [DONE]`.

---

## 3. Dynamic JS Bundle Scraper & tRPC Procedures

### 3.1 `/assets/*.js` Scraper Differences (`t3router/src/t3/models.rs` vs `pi-t3chat/catalog.ts`)
Both scrapers fetch `https://t3.chat/`, extract `/assets/[^"]+\.js[^"]*`, locate the chunk containing `usdCostPerToken` and `modelToInternalId`, and extract model entries:
- **`t3router/src/t3/models.rs:437-662`:**
  - `eval_js_number` (`L647–662`) explicitly evaluates `/1e6` and `/1e3` division expressions in `usdCostPerToken: { input: 3/1e6, output: 15/1e6 }`.
  - Handles both `features: new Set([...])` and `features: [...]` (`L437–449`).
- **`pi-t3chat/catalog.ts:147-253`:**
  - Scans up to 12 chunks in parallel (`chunkPaths.slice(0, 12)` via `Promise.all`), filters out image-only models (`features.every(f => f === "images")`) and retired models (`retiredOn`), and parses `usdCostPerToken` with `parseFloat(costMatch[1])` (which drops `/1e6` suffixes).

### 3.2 tRPC Endpoints & Header Differences (`t3router/src/t3/usage.rs` vs `pi-t3chat/usage.ts`)
1. `auth.getActiveSessions`: `Client::refresh_session` (`client.rs:237-246`) sends `trpc-accept: application/jsonl`, whereas `UsageClient::get_active_sessions` (`usage.rs:397-408`) sends `x-trpc-source: web-client` without `trpc-accept`.
2. `getCustomerData`: Returns `creditsRemaining`, `totalCredits`, `NextCreditRefill`, `TopUpInfo`.
3. `getSubscriptionData`: `t3router` (`usage.rs:367-368`) sends `x-trpc-batch: true` + `trpc-accept: application/jsonl`; `pi-t3chat` (`usage.ts:204`) calls `trpcCall(cookies, "getSubscriptionData", null, false)`.
4. `getPricingProducts`: `t3router` (`usage.rs:309`) sends `{"0":{"json":{"sessionId":null},"meta":{"values":{"sessionId":["undefined"]}}}}`; `pi-t3chat` (`usage.ts:214`) sends `{"0":{"json":null,"meta":{"values":["undefined"]}}}`.
5. `getModelStatuses` & `getAllModelBenchmarks` (`t3router/src/t3/models.rs:212-345`): Returns live `status` (`operational`, `degraded`, `outage`) and benchmark scores across Artificial Analysis, LiveBench, SweBench, SnitchBench, SkateBench, etc.

### 3.3 Post-Turn Credit Delta Polling (`t3router/src/t3/client.rs:770-792`)
- `poll_credit_delta` loops `for attempt in 0..8`, sleeping `200ms` only when `attempt + 1 < 8` (7 sleeps = `1.4s` max sleep). If none of the 8 loop checks observes `(before - after).abs() > 0.000_1`, line 786 performs a **9th** unconditional `usage_client.get_balance().await.ok()` call and returns `(final_after, final_after.map(|after| before - after))`.

---

## 4. Coding-Agent Tool Emulation, Refusal Auto-Correction & Lazy MCP (`pi-t3chat`)

- **`ToolCallTranslator` (`tools.ts:45-210`):** Serializes OpenAI tool definitions into system prompt instructions requiring ```` ```tool:<exact_tool_name>\n{...}\n``` ```` fenced blocks.
- **`ToolRefusalDetector` (`tools.ts:265-305`):** Detects false refusals in the first 350 characters when `tools.length > 0`, `toolCallsFound === 0`, and `text.length < 700` using two exact regexes (`tools.ts:270-271`):
  - `REFUSAL_RE = /\b(can't|cannot|can not|don't have|do not have|unable to|not able to|no access|without access|can't actually|cannot actually)\b/i`
  - `CAPABILITY_RE = /\b(tool|terminal|shell|command|run|execute|file|filesystem|browse|browser|web search|search|calendar|schedule|environment)\b/i`
  - When both match (`!retriedRefusal`), `proxy.ts` discards the refusal and retries once with a corrective system reminder listing the available tool names.
- **Lazy MCP Wrapper (`mcp.ts:1-210`):** Collapses all `mcp__<server>__<tool>` tools into 3 lightweight meta-tools (`list_mcps`, `list_mcp_tools`, `call_mcp`) resolved locally in `proxy.ts` up to `MAX_WRAPPER_ROUNDS = 6`.
