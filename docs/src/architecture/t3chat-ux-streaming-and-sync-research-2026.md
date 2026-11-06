---
title: "T3 Chat UX, Resumable Streaming & Local-First Sync: thom-chat & helioschat (2026)"
description: "Source-anchored specification of TGlide/thom-chat (SvelteKit 5 + Convex + OpenRouter) and helioschat/app (SvelteKit 5 + IndexedDB v3 + 4 Browser SDKs): server-detached streaming, 2-tier cost reconciliation, fuzzy search, multiplexed reasoning streams, client tool loop, and temporary mode."
category: "Architecture SSOTs"
status: "research"
topic: "agent-orchestrator"
domain: "orchestration"
authorship: "llm-generated"
authorship_model: "claude-opus-4-7"
---

# T3 Chat UX, Resumable Streaming & Local-First Sync: `thom-chat` & `helioschat` (2026)

<!-- kb:summary -->
This document provides the line-verified specification of `TGlide/thom-chat` (`~/dev/thom-chat`, commit `062193d792303007b500e35c0c50ea4ceefeff2d`) and `helioschat/app` (`~/dev/helioschat`, commit `08e2c3fc784c4ed5bf58b38516182387f218bf26`). It covers `thom-chat`'s server-detached OpenRouter stream continuation into Convex (`api.messages.updateContent`), 2-tier cost provenance (`estimateTokenCount` vs `GET /api/v1/generation`), `/api/enhance-prompt`, `@` rule mentions, 3-mode fuzzy search (`fuzzy-search.ts`), and keyboard shortcuts, alongside `helioschat`'s IndexedDB v3 (`llmchat`) schema, chunked lazy loading (`CHUNK_SIZE = 16`), `[REASONING]` + JSON stream multiplexing, 10-round client tool loop (`manage_memories`, `exa_web_search`, `math_evaluate`), and 5-point zero-retention Temporary Chat mode.
<!-- /kb:summary -->

## 1. `TGlide/thom-chat` (`~/dev/thom-chat` — SvelteKit 5 + Convex + OpenRouter)

### 1.1 Convex Database Schema (`src/lib/backend/convex/schema.ts`)
1. **`conversations`:** `title`, `user_id`, `pinned?: boolean`, `generating?: boolean`, `cost?: number`, `branched_from?: Id<'conversations'>` (`by_user`).
2. **`messages`:** `conversation_id`, `role`, `content`, `reasoning?: string`, `model_id?`, `provider?`, `token_count?`, `cost_usd?`, `generation_id?`, `web_search_enabled?`, `reasoning_effort?`, `annotations?: any[]`, `images?` (`by_conversation`).
3. **`user_keys` (`user_keys.ts` & `chat.ts`):** `user_id`, `service` (`'openrouter' | 'huggingface'`), `key`. Saving an OpenRouter BYOK key in `user_keys.set` (`user_keys.ts:93-123`) automatically enables and pins 4 models (`google/gemini-2.5-flash`, `anthropic/claude-sonnet-4`, `openai/o3-mini`, `deepseek/deepseek-chat-v3-0324:free`), while `user_enabled_models.enable_initial` (`user_enabled_models.ts:132-177`) seeds 3 defaults (`google/gemini-2.5-flash`, `deepseek/deepseek-chat-v3-0324:free`, `microsoft/phi-4:free`).
4. **`user_settings`:** `user_id`, `privacy_mode`, `free_messages_used`, `Name`, `uid_for_ai`, `occupation`, `traits`.
5. **`user_enabled_models`:** `user_id`, `provider`, `model_id`, `pinned?: boolean` (`by_user`, `by_user_provider`, `by_model_id`).
6. **`user_rules`:** `user_id`, `name`, `attach` (`'always' | 'manual'`), `rule` (`by_user`, `by_user_attach`, `by_user_name`).
7. **Storage & Auth (`storage.ts`, `betterAuth.ts`):** `storage.ts` provides `generateUploadUrl`, `getUrl`, and `deleteFile`; `betterAuth.ts` wires `@better-auth-kit/convex/handler` (`ConvexHandler` + `publicGetSession`).

### 1.2 Detached Server Stream & 2-Tier Cost Reconciliation (`src/routes/api/generate-message/+server.ts`)
- **Free-Tier Gate (`+server.ts:333-339`):** Users without an OpenRouter BYOK key are restricted to `< 10` non-`:free` messages; exceeding returns HTTP `403` with exact message `'Free message limit reached (10/10). Please add your own OpenRouter API key to continue chatting, or use a free model ending in ":free".'`.
- **Rule Injection Placement (`+server.ts:396-451`):** Collects all `attach: 'always'` rules plus any `@<rule_name>` `manual` rules matched via `parseRulesFromText` across all conversation messages, deduplicates by `r._id`, and pushes `{ role: 'system', content: ... }` **at the end** of `messagesToSend`.
- **Detached Background Continuation:** Returns `{ ok: true, conversation_id }` immediately while the async OpenRouter SSE reader runs on the server, writing live deltas via `client.mutation(api.messages.updateContent, { message_id, content, reasoning })` and registering an `AbortController` in `activeGenerations` (`POST /api/cancel-generation`).
- **Title Generation Quirk (`+server.ts:74-115, 392`):** When `messages.length === 1`, `generateConversationTitle` calls `google/gemini-2.0-flash-lite-001` and unconditionally overwrites `conversation.title` (replacing the initial first-sentence title set by `conversations.createAndAddMessage` in `conversations.ts:122-125`).
- **2-Tier Cost Provenance:**
  - **Tier 1:** Estimates tokens via `estimateTokenCount` (`tokenx`) and multiplies by `model.pricing.prompt` / `completion`.
  - **Tier 2:** Queries `GET https://openrouter.ai/api/v1/generation?id=${generationId}` to reconcile authoritative `usage` USD cost and `tokens_completion` into `messages` and `conversations.cost`.
- **Branch-and-Regenerate Order (`src/routes/chat/[id]/message.svelte:80-95`):** Calls `conversations.createBranched`, awaits `callGenerateMessage(...)` first (`L80–89`, omitting `reasoning_effort`), and then navigates via `await goto(\`/chat/${cid}\`)` (`L91`).

### 1.3 Client Cache, 3-Mode Fuzzy Search & Keyboard Shortcuts
- **Two-Layer Cache (`src/lib/cache/`):** `LRUCache<K, V>` (`lru-cache.ts`, `maxSize = 50`) + `SessionStorageCache<T>` (`session-cache.ts:16, 94`, default `storageKey = 'query-cache'`, `ttlMs = undefined`). `useCachedQuery` (`cached-query.svelte.ts:20, 29`) configures `storageKey = 'convex-query-cache'` and `ttl = 7 * 24 * 60 * 60 * 1000` (7 days).
- **3-Mode Fuzzy Search (`src/lib/utils/fuzzy-search.ts:66-214` & `conversations.ts:174-239`):** Supports `searchMode: 'exact' | 'words' | 'fuzzy'` (`minScore = 0.3`). `calculateMatchScore` weights exact (`1.0`), substring (`0.8 + (searchLen / targetLen) * 0.2`), all-words (`0.9`), partial-words (`(exactWords / total) * 0.7 + (matchedWords / total) * 0.3`), prefix (`0.6`), and character-order fuzzy (`max(0.1, (searchLen / (targetLen + totalDistance)) * 0.5)`).
- **Keyboard Shortcuts:** `Ctrl/Cmd+Shift+O` (New Chat), `Ctrl/Cmd+B` (Toggle Sidebar), `Ctrl/Cmd+K` (Search Modal), `Ctrl/Cmd+D` (Scroll to bottom), `Ctrl/Cmd+Shift+M` (Model Picker; inside picker: `Ctrl/Cmd+ArrowRight` 4-col grid `w-[620px]`, `Ctrl/Cmd+ArrowLeft` favorites `w-[320px]`, `Ctrl/Cmd+U` pin/unpin).

---

## 2. `helioschat/app` (`~/dev/helioschat` — SvelteKit 5 + IndexedDB v3 + 4 Browser SDKs)

### 2.1 IndexedDB v3 (`llmchat`) & Chunked Lazy Loading
- **5 Object Stores (`src/lib/database/connection.ts`):** `threads`, `messages`, `attachments`, `folders`, and `memories`.
- **Chunked Startup & Streaming Write Suppression (`src/lib/stores/chat.ts:50-157`):**
  - `initializeStore` loads the first `CHUNK_SIZE = 16` threads immediately and loads remaining threads in chunks of 16 with a `50ms` delay (`L51–91`), deduplicating concurrent `loadChat(id)` calls via `loadMutex` (`L161`).
  - During token streaming, the `chats.subscribe` persistence listener (`L135–157`) skips IndexedDB writes (`!activeChat.temporary && !isStreaming`) to prevent write amplification; `StreamingController` (`controller.ts:227, 297`) executes a single `saveChatAsThreadAndMessages` in `finally`.

### 2.2 Multiplexed Stream Protocol, `StreamMetrics` & Provider Registry
- **Known Providers (`src/lib/providers/known.ts:11-800`):** Defines `KNOWN_PROVIDERS` for `openai` (`gpt-5-mini`), `openrouter` (`openai/gpt-5-mini`), `featherless` (`Qwen/Qwen3-32B`), `groq` (`moonshotai/kimi-k2-instruct-0905`), and `cerebras` (`gpt-oss-120b`), with `detectKnownProvider`, `defaultDisabledPatterns`, and `modelOverrides`.
- **Dual OpenAI Wire Paths (`src/lib/providers/openai-compatible.ts:304-680`):**
  - `streamChatCompletions` (`/v1/chat/completions`) passes `web_search_options: { search_context_size }`, parses **four** reasoning tag pairs (`<think>`, `<thinking>`, `<reason>`, `<reasoning>`) plus `delta.reasoning_content` / `delta.reasoning`, and runs up to `MAX_TOOL_ROUNDS = 10` (`L367`).
  - `streamResponsesEndpoint` (`/v1/responses`, `L679`) passes `tools: [{ type: 'web_search_preview' }]` (without `search_context_size`).
- **`StreamMetrics` (`src/lib/streaming/types.ts:3-12` & `messageProcessor.ts:31-40`):**
  - Fields: `{ startTime: number, endTime?: number, totalTime?: number, promptTokens?: number, completionTokens?: number, totalTokens?: number, tokensPerSecond?: number, thinkingTime?: number }` (first-content latency is stored in `thinkingTime` when no `[REASONING]` chunk occurred, `messageProcessor.ts:64-66`).

### 2.3 Dynamic System Variables, Client Tool Suite, Regenerate/Edit & 5-Point Temporary Mode
1. **Dynamic System Prompt Variables (`src/lib/utils/systemPromptVariables.ts:26-88`):** Resolves `$current_datetime`, `$user_timezone`, `$user_locale`, `$user_language`, and `$user_platform`.
2. **Client-Side Tools (`src/lib/tools/`):** `ExaSearchTool` (`exa_web_search`, `numResults: 1..10`, `highlights.maxCharacters: 4000`), `MathEvaluatorTool` (`math_evaluate` via `mathjs`, `precision: 14`), and `ManageMemoriesTool` (`manage_memories` `mode: 'search' | 'save' | 'delete'` with keyword extraction `>= 3` chars).
3. **Regenerate vs Edit-Message (`src/routes/[chatId]/+page.svelte:132-297`):** `handleRegenerate` reuses the assistant message's stored `providerInstanceId`, `model`, and toggles; editing an assistant message (`allowAssistantMessageEditing`) mutates it in-place without truncation.
4. **Prompt History (`promptHistory.ts`), Global Shortcuts & JSON Export (`dataExport.ts`):** Stores 100 prompts in `sessionStorage` (`ArrowUp`/`ArrowDown` navigation), registers global shortcuts (`Shift+Esc`, `Ctrl/Cmd+Shift+O`, `Ctrl/Cmd+U`, `Ctrl/Cmd+K`, `Ctrl/Cmd+B`, `Ctrl/Cmd+Shift+Backspace`, `Ctrl/Cmd+ArrowUp/Down`, `Ctrl/Cmd+Shift+;`), and exports all IndexedDB stores + `localStorage` to `llmchat-export-<timestamp>.json`.
5. **5-Point Temporary Mode:** In-memory only (`!chat.temporary`), omits `manage_memories` tool, suppresses memory prompt injection, skips LLM title generation, and auto-purges on route leave.
