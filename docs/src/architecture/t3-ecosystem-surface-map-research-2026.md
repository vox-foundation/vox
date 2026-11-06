---
title: "T3 Ecosystem Surface Map & Repository Inventory (2026)"
description: "Complete source-anchored topology across 9 cloned T3 Chat and T3 Code repositories in ~/dev: pingdotgg/t3code (25 packages, 5 apps), vibheksoni/t3router, vibheksoni/pi-t3chat, TGlide/thom-chat, helioschat/app, lulkebit/t3-cloneathon, guitaripod/omnichat, shaltielshmid/NotT3Chat, and TomDoesTech/t3-chat."
category: "Architecture SSOTs"
status: "research"
topic: "agent-orchestrator"
domain: "orchestration"
authorship: "llm-generated"
authorship_model: "claude-opus-4-7"
---

# T3 Ecosystem Surface Map & Repository Inventory (2026)

<!-- kb:summary -->
This document serves as the root structural index for Vox's exhaustive source-code audit of the T3 Chat and T3 Code ecosystem. It maps all 9 cloned repositories in `~/dev` (`pingdotgg/t3code`, `vibheksoni/t3router`, `vibheksoni/pi-t3chat`, `TGlide/thom-chat`, `helioschat/app`, `lulkebit/t3-cloneathon`, `guitaripod/omnichat`, `shaltielshmid/NotT3Chat`, and the `create-t3-app` taxonomic control case `TomDoesTech/t3-chat`), cataloging exact git commit SHAs, all 25 `t3code` workspace packages and 5 applications, the 11 Rust modules of `t3router`, the 13 TypeScript modules of `pi-t3chat`, the SvelteKit/Convex/IndexedDB/Next.js/Cloudflare D1/SignalR architectures of the web clones, and the 9-volume architecture documentation series in `docs/src/architecture/`.
<!-- /kb:summary -->

## 1. Cloned Repository Matrix (`~/dev`)

| Repository | Local Clone Path | Pinned Commit SHA | Primary Language & Stack | Architectural Role in Audit |
|---|---|---|---|---|
| `pingdotgg/t3code` | `~/dev/t3code` | `80455376f01f19c10a06dc08b0629486c017bf79` | TypeScript, Effect-TS, Bun, SQLite (`statev2.sqlite`), React 19, Electron, Expo | Official T3 Code multi-provider coding agent harness (`Orchestration v2`, 9 provider drivers, 6 SCM forges, 14 MCP toolkits, Live Guest Share) |
| `vibheksoni/t3router` | `~/dev/t3router` | `5314bbbf0af66e724e3e07ffd458ef5919d1eed1` | Rust 2021, `rquest 5.1` (`Chrome136`), `tokio`, `regex`, `serde_json` | Production `t3.chat` wire-protocol client, `/assets/*.js` bundle model/pricing scraper, WorkOS session rotator, and tRPC client |
| `vibheksoni/pi-t3chat` | `~/dev/pi-t3chat` | `b39873245362c33f68a9a3e044423f69769069f9` | TypeScript, Node.js `http`, Playwright, Pi Coding Agent API | Coding-agent bridge & `127.0.0.1:42101` OpenAI-compatible proxy with prompt-engineered tool calling, refusal auto-correction, and lazy MCP wrapper |
| `TGlide/thom-chat` | `~/dev/thom-chat` | `062193d792303007b500e35c0c50ea4ceefeff2d` | SvelteKit 5, Convex, Better-Auth, OpenRouter API | Server-detached resumable OpenRouter streaming into Convex, 2-tier cost provenance, prompt enhancement, 3-mode fuzzy search, and `@rule` mentions |
| `helioschat/app` | `~/dev/helioschat` | `08e2c3fc784c4ed5bf58b38516182387f218bf26` | SvelteKit 5, IndexedDB v3 (`llmchat`), 4 Browser Provider SDKs | Local-first browser-direct multi-LLM chat with multiplexed `[REASONING]`/JSON streams, 10-round client tool loop, and 5-point zero-retention temporary mode |
| `lulkebit/t3-cloneathon` | `~/dev/t3-cloneathon` | `99eb9ab16c9349e9973138b5b71b0af5e9d70b8b` | Next.js 15.3.8, Supabase Auth/PostgreSQL/Storage, OpenRouter | T3 Cloneathon entry ("Convex Chat") with parallel Multi-Model Consensus SSE streaming (`POST /api/chat/consensus`) and Flesch-Kincaid `ResponseQualityAnalyzer` |
| `guitaripod/omnichat` | `~/dev/omnichat` | `b864415e55ba3e4bb3489b06919e405031b8bb8a` | Next.js 15.3.3 Edge, Cloudflare Workers/D1/R2, Drizzle ORM, Clerk + Apple JWT, Stripe, Go CLI | 3rd Place T3 Cloneathon Champion with Battery Unit (`1 BU = $0.001`) D1 token billing ledger, tree branching with `divergencePoint` comparison, dual-mode Ollama, and 43-endpoint Go CLI validator |
| `shaltielshmid/NotT3Chat` | `~/dev/NotT3Chat` | `913afaaea495a63076bf085d66597ca0f98a7b04` | C# 12 / .NET 10 ASP.NET Core, SignalR (`/chat`), EF Core SQLite, `LlmTornado` (10 providers), React 19 + `streamdown` | Single-file 1,147-LoC C# backend with `SemaphoreSlim(1)` atomic mid-stream multi-tab join and 1024-slot power-of-2 ring-buffer `RepetitionGuard` |
| `TomDoesTech/t3-chat` | `~/dev/t3-chat-tomdoestech` | `4cc8fac3ec947e7b9889588ff292d66862b0c5b0` | Next.js 12.2.1, tRPC v9 (`httpBatchLink` + `wsLink`), `ws.Server` (`:3001`), Prisma 4 SQLite | Taxonomic control case: 2022 `create-t3-app` ("T3 Stack") human-to-human WebSocket chat room tutorial (non-AI) |

---

## 2. `pingdotgg/t3code` Monorepo Topology (`25` Workspace Packages + `5` Apps)

### 2.1 Applications (`apps/`)
1. **`apps/server`:** Effect-TS orchestration server (`Orchestrator.ts`, `apps/server/src/orchestration-v2/`), `statev2.sqlite` event store & read-model projections (`055_OrchestrationV2.ts` through `069_ThreadMessageSearch.ts`), HTTP/WebSocket RPC server, 14-toolkit MCP server (`McpHttpServer.ts` at `/mcp`), DPoP/PKCE auth (`EnvironmentAuth.ts`, `McpOAuth.ts`), Live Guest Share (`guest/GuestShareService.ts`), Git VCS/worktree manager (`GitVcsDriverCore.ts`, `worktreeRemoval.ts`, `storageCleanup.ts`), PR reactors (`PullRequestSyncReactor.ts`, `PullRequestWatchReactor.ts`), Scheduled Tasks & Webhooks (`scheduledTasks/ScheduledTaskService.ts`), Native Rust/C FFI Workspace Search (`workspace/WorkspaceSearchIndex.ts`), Integrated Browser & Device Hosts (`preview/`, `device/`), and `CloudLink.ts` relay client.
2. **`apps/web`:** React 19 + Vite + TanStack Router web UI, Tiptap rich-text composer (`composer-editor-mentions.ts`, `composerCodeBlockHighlight.ts`), `@pierre/highlights` WASM syntax highlighting (`lib/syntaxHighlighting.ts`), diff viewer, and Live Guest view (`components/guest/GuestPage.tsx`).
3. **`apps/desktop`:** Electron shell (`DesktopBackendManager.ts`), multi-browser cookie decryption (`BrowserImport/`), native screenshot/region picker (`DesktopSnapShot/` across macOS, Windows, and 6 Linux Wayland/X11 backends), and protocol/deep-link handler.
4. **`apps/mobile`:** Expo / React Native iOS & Android client connecting via T3 Connect relay, Tailscale, or LAN with live PR check/label/reaction management and guest share bars.
5. **`apps/marketing`:** Public documentation and landing surface.

### 2.2 Workspace Packages (`packages/` — 25 Packages)

| Category | Package Path | Key Modules & Responsibility |
|---|---|---|
| **Core Contracts & Shared** | `packages/contracts` | `orchestrationV2.ts` (`ORCHESTRATION_PROTOCOL_VERSION = 2`, 55 public commands, 8 internal commands, 45 domain events, 24 `TurnItem` variants, 18 WS RPC methods), `orchestrationV2IdTable.ts`, `guestShare.ts`, `guestStream.ts`, `pullRequest.ts`, `model.ts`, `editor.ts` |
| | `packages/shared` | `ForwardCompatibleUnion.ts`, `orchestrationV2DedupedBodies.ts`, `terminalOutput.ts` (`normalizeTerminalOutput`), `model.ts`, `git.ts` |
| | `packages/client-runtime` | `connection/supervisor.ts` (6-tier staggered route racing), `state/threadFamilies.ts`, `pullRequestEditing.ts`, `pullRequestReactions.ts`, `pullRequestChecks.ts`, `pullRequestLinkInput.ts` |
| **Provider Framework & 9 Drivers** | `packages/provider-core` | `IdAllocator.ts` (`IdAllocatorV2` `{ allocate, derive }`), `handoffBudget.ts` (`DEFAULT_HANDOFF_TOKEN_CAP = 16_000`, `HANDOFF_BYTE_CAP = 64_000`), `RuntimeEvent.ts` |
| | `packages/effect-codex-app-server` | Typed JSON-RPC 2.0 stdio client/server protocol for `codex app-server` |
| | `packages/effect-acp` | Effect-TS bindings for `@agentclientprotocol/sdk` (ACP stdio NDJSON) |
| | `packages/provider-acp` | `AcpSessionEngine.ts`, `McpConverter.ts`, `PlanConverter.ts`, `PermissionMapper.ts` |
| | `packages/provider-acp-registry` | Dynamic registry + 13 curated ACP agent profiles (`claude-acp`, `github-copilot-cli`, `cursor`, `devin`, `factory-droid`, `gemini`, `goose`, `junie`, `kimi`, `kiro`, `mistral-vibe`, `opencode`, `qwen-code`) |
| | `packages/provider-cursor` | `CursorAcpExtension.ts` (`cursor/*` methods) + `CursorTranscriptReader.ts` (`~/.cursor/chats/<hash>/<session>/store.db` SQLite hex-blob DAG) |
| | `packages/provider-grok` | Dual-engine driver: `GrokCliClient.ts` (`grok` CLI subprocess) + `GrokXaiApiDriver.ts` (`https://api.x.ai/v1/responses` SSE + 9 native tools) |
| | `packages/provider-opencode` | Dual-generation driver: OpenCode 1.x (`opencode serve` HTTP + SSE) and OpenCode 2.x (`opencode app-server` JSON-RPC) |
| | `packages/provider-muse` | Multi-variant parallel `git worktree` fan-out orchestrator (`variantCount: 1..4`) wrapping inner providers |
| | `packages/provider-pi` | Subprocess RPC driver for `@mariozechner/pi-coding-agent` (`pi --mode rpc`) |
| | `packages/provider-testing` | Shared provider conformance test harness and mock fixtures |
| **Source Control (6 Forges)** | `packages/source-control-core` | `definition.ts` (`SourceControlDefinition`), `PullRequestProvider.ts` (35-method `PullRequestProviderApi`), `SourceControlProvider.ts`, `discovery.ts` (`DEFAULT_PROBE_TIMEOUT_MS = 5_000`), `diffFileRevisions.ts` |
| | `packages/source-control-definitions` | `registry.ts` (`BUILT_IN_SOURCE_CONTROL_DRIVER_LIST`: `github`, `gitlab`, `gitcafe`, `forgejo`, `bitbucket`, `azure-devops`) |
| | `packages/source-control-github` | `GitHubSourceControlProvider.ts` (GraphQL batching), `GitHubPullRequestApi.ts`, `GitHubCredentials.ts` (`TOKEN_TTL = 5m`), `gitHubGraphiteStack.ts` |
| | `packages/source-control-gitlab` | `GitLabPullRequestProvider.ts`, `GitLabCli.ts` (`glab`), `gitlabDiffPosition.ts` |
| | `packages/source-control-bitbucket` | `BitbucketPullRequestProvider.ts`, `BitbucketPullRequestApi.ts` (`parseDiffFileRevisions`), `BitbucketCredentials.ts` |
| | `packages/source-control-azure-devops` | `AzureDevOpsPullRequestProvider.ts` (`DIFF_FILE_CONCURRENCY = 4`), `azureDevOpsDiff.ts`, `AzureDevOpsIdentity.ts` |
| | `packages/source-control-forgejo` | `ForgejoPullRequestProvider.ts`, `ForgejoCli.ts` (`fj` / `forgejo-cli` `keys.json` reader) |
| | `packages/source-control-gitcafe` | `GitCafePullRequestProvider.ts` (`git.cafe` REST API + `canonicalizeGitCafePullRequestUrl`) |
| | `packages/source-control-testing` | Shared SCM provider test harness |
| **Remote & Networking** | `packages/ssh` | Zero-Node remote archive bootstrap (`RemoteBootstrap.ts`), `ControlMaster` multiplexing, port forwarding |
| | `packages/tailscale` | Local Tailscale CLI status/IP discovery (`tailscale status --json`) for direct P2P routing |

---

## 3. T3 Chat Routers, Bridges & Web Clones (`~/dev`)

- **`t3router` (`~/dev/t3router/src/`):** `lib.rs`, `bin/t3chat.rs`, `t3/{mod.rs, config.rs, models.rs, client.rs, usage.rs, session.rs, history.rs, message.rs, repl.rs}` + 11 runnable `examples/*.rs`.
- **`pi-t3chat` (`~/dev/pi-t3chat/`):** `index.ts`, `auth.ts`, `catalog.ts`, `models.ts`, `config.ts`, `chat.ts`, `sse.ts`, `tools.ts`, `mcp.ts`, `proxy.ts`, `tokens.ts`, `usage.ts`, `log.ts`.
- **`thom-chat` (`~/dev/thom-chat/src/`):** SvelteKit 5 + Convex (`schema.ts`, `conversations.ts`, `messages.ts`, `storage.ts`, `betterAuth.ts`, `user_enabled_models.ts`, `user_keys.ts`, `user_rules.ts`, `user_settings.ts`, `chat.ts`) + `routes/api/{generate-message,cancel-generation,enhance-prompt}/+server.ts` + `lib/cache/{lru-cache.ts,session-cache.ts,cached-query.svelte.ts}` + `lib/utils/{fuzzy-search.ts,rules.ts,model-capabilities.ts}`.
- **`helioschat` (`~/dev/helioschat/src/lib/`):** SvelteKit 5 + IndexedDB v3 (`llmchat`: `threads`, `messages`, `attachments`, `folders`, `memories`) + `streaming/{state.ts,types.ts,controller.ts,streamProcessor.ts,messageProcessor.ts}` + `providers/{base.ts,known.ts,openai-compatible.ts,registry.ts}` + `tools/{index.ts,manageMemories.ts,memorySearch.ts,exaSearch.ts,mathEvaluator.ts}`.
- **`t3-cloneathon` (`~/dev/t3-cloneathon/src/`):** Next.js 15.3.8 + Supabase PostgreSQL + OpenRouter with `app/api/chat/consensus/route.ts` (`ConsensusResponse[]` parallel SSE fan-out) and `lib/response-quality-analyzer.ts` (`ResponseQualityAnalyzer`).
- **`omnichat` (`~/dev/omnichat/`):** Next.js 15.3.3 Edge + Cloudflare Workers/D1/R2 (`src/lib/db/schema.ts` — 13 D1 tables) + `src/lib/{battery-pricing.ts,usage-tracking.ts}` (`1 BU = $0.001`) + `src/services/branching/branch-manager.ts` (`divergencePoint`) + `go-cli/cmd/omnichat-validator/main.go`.
- **`NotT3Chat` (`~/dev/NotT3Chat/`):** Single-file C# 12 / .NET 10 `backend/Program.cs` (1,147 LoC) + SignalR `/chat` Hub + `SemaphoreSlim(1)` mid-stream multi-tab join + `RepetitionGuard` (1024-slot ring buffer) + React 19 `streamdown` frontend.
- **`t3-chat-tomdoestech` (`~/dev/t3-chat-tomdoestech/`):** 2022 `create-t3-app` Next.js 12 + tRPC v9 WebSocket (`wsLink` on `:3001`) multi-user human chat room tutorial (taxonomic control case).

---

## 4. Architecture & Implementation Documentation Series Cross-Index (15 Volumes)

### Part A: Source-Verified Research & Ecosystem Audit (Volumes 1–9)
1. **Volume 1 — Ecosystem Surface Map (this document):** [t3-ecosystem-surface-map-research-2026.md](t3-ecosystem-surface-map-research-2026.md)
2. **Volume 2 — `t3code` Orchestration v2 & Event-Sourced Core:** [t3code-harness-architecture-research-2026.md](t3code-harness-architecture-research-2026.md)
3. **Volume 3 — `t3code` Server Runtime, `statev2.sqlite`, Worktree Safety, MCP OAuth/DPoP & Subsystems:** [t3code-worktree-permissions-mcp-research-2026.md](t3code-worktree-permissions-mcp-research-2026.md)
4. **Volume 4 — `t3code` 9-Provider Driver Deep-Dive (JSON-RPC, ACP, Connect-RPC, SSE, SQLite):** [t3code-provider-adapters-deep-dive-research-2026.md](t3code-provider-adapters-deep-dive-research-2026.md)
5. **Volume 5 — `t3code` 6-Forge SCM, SSH Bootstrap, CloudLink Relay & Client/Desktop Deep-Dive:** [t3code-scm-remote-and-client-deep-dive-research-2026.md](t3code-scm-remote-and-client-deep-dive-research-2026.md)
6. **Volume 6 — `t3router` & `pi-t3chat` Wire Protocol, Scraper & Coding-Agent Bridge:** [t3chat-router-and-wire-protocol-research-2026.md](t3chat-router-and-wire-protocol-research-2026.md)
7. **Volume 7 — `thom-chat` & `helioschat` Streaming, Convex/IndexedDB Sync & Agentic UX:** [t3chat-ux-streaming-and-sync-research-2026.md](t3chat-ux-streaming-and-sync-research-2026.md)
8. **Volume 8 — Best-of-All-Worlds Vox Blueprint & Extraction Ledger:** [t3-vs-vox-harness-router-blueprint-2026.md](t3-vs-vox-harness-router-blueprint-2026.md)
9. **Volume 9 — T3 Cloneathon & Community Clones (`t3-cloneathon`, `omnichat`, `NotT3Chat`, `TomDoesTech/t3-chat`):** [t3chat-cloneathon-and-community-clones-research-2026.md](t3chat-cloneathon-and-community-clones-research-2026.md)

### Part B: Milestone `v4.1` End-to-End Implementation, Legacy Code Retirement & Static Enforcement Specifications (Volumes 10–15)
10. **Volume 10 — Wave 2 Master Implementation & Legacy Retirement Plan (`Phases 216–226`):** [t3-vox-wave2-master-implementation-plan-2026.md](t3-vox-wave2-master-implementation-plan-2026.md)
11. **Volume 11 — Domain 1 Spec (`Phases 216–217` — `VoxIdAllocatorV2`, Wire Interning/Deduplication, Atomic Stream Resumption, Guest Share & `ThreadRunQueue`):** [t3-vox-orchestration-wire-and-id-migration-spec-2026.md](t3-vox-orchestration-wire-and-id-migration-spec-2026.md)
12. **Volume 12 — Domain 2 Spec (`Phases 218–220` — `RepetitionGuard`, `ToolRefusalDetector`, Battery Ledger, Multi-Benchmark Priors, `up_to` Compaction & Consensus):** [t3-vox-router-consensus-battery-and-stream-guards-spec-2026.md](t3-vox-router-consensus-battery-and-stream-guards-spec-2026.md)
13. **Volume 13 — Domain 3 & 4 Spec (`Phases 221–223` — Junction-Safe Worktrees, Git Plumbing Checkpoints, PR Reactors, Cross-Provider Handoff, `McpToolAccess`/DPoP & Branch Divergence UI):** [t3-vox-worktree-checkpoints-mcp-and-local-sync-spec-2026.md](t3-vox-worktree-checkpoints-mcp-and-local-sync-spec-2026.md)
14. **Volume 14 — Domain 5 Spec (`Phases 224–225` — `@rule` Mentions, Dynamic `$variable` Prompts, Weighted Fuzzy/Trigram Search, Signed Webhooks, CLI Session Importer & 6-Tier Route Racing):** [t3-vox-agentic-ux-rules-webhooks-and-session-interop-spec-2026.md](t3-vox-agentic-ux-rules-webhooks-and-session-interop-spec-2026.md)
15. **Volume 15 — Domain 6 Spec (`Phase 226` — TOESTUB Static Analysis Detectors, Wire/SQL Transfer Budget CI Harness, V8 Memory/CSS Rules & Critical T3 Evaluation):** [t3-vox-toestub-rules-and-enforcement-research-2026.md](t3-vox-toestub-rules-and-enforcement-research-2026.md)
