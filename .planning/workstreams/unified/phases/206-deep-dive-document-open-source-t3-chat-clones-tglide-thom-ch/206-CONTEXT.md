# Phase 206: Deep-Dive & Document Open-Source T3 Chat Clones (TGlide/thom-chat & helioschat/app) - Context

**Gathered:** 2026-10-24
**Status:** Ready for planning
**Mode:** Autonomous (`/gsd-autonomous`)

<domain>
## Phase Boundary

Analyze and document `TGlide/thom-chat` (`~/dev/thom-chat` at `062193d792303007b500e35c0c50ea4ceefeff2d`) and `helioschat/app` (`~/dev/helioschat` at `08e2c3fc784c4ed5bf58b38516182387f218bf26`) in `docs/src/architecture/t3chat-ux-streaming-and-sync-research-2026.md`.

</domain>

<decisions>
## Implementation Decisions

### Source Coverage
- `thom-chat`: Server-resumable background generation (`generate-message/+server.ts`, `cache.ts`, `cancel-generation/+server.ts`), `enhance-prompt/+server.ts`, Cursor-style `@<rule_name>` rules (`user_rules`, `parseMessageForRules`), `conversations.createBranched`, OpenRouter `:free` vs. BYOK tier & `:online` search routing, and `/api/v1/generation` post-stream cost reconciliation.
- `helioschat`: Local-first IndexedDB v3 schema (`connection.ts`, `threads.ts`, `storage.ts`), chunked 16-thread hydration and `chat.temporary` isolation (`stores/chat.ts`), `branchOffChat`, `StreamingController`, and proactive client-side memory tools (`manage_memories`, `search_memories`).

</decisions>
