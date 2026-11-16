---
phase: 206-deep-dive-document-open-source-t3-chat-clones-tglide-thom-ch
plan: 01
subsystem: architecture-docs
tags: [thom-chat, helioschat, resumable-streaming, prompt-enhancement, branching, indexeddb, privacy]
requires: [T3-CLONE-GRAPH-02]
provides:
  - docs/src/architecture/t3chat-ux-streaming-and-sync-research-2026.md
affects: [vox-orchestrator, vox-gui, vox-db]
tech-stack:
  added: []
  patterns: [server-resumable-streaming, prompt-enhancer, temporary-chat-isolation]
key-files:
  created:
    - docs/src/architecture/t3chat-ux-streaming-and-sync-research-2026.md
  modified: []
key-decisions:
  - "Mapped thom-chat background generation + generationAbortControllers to Phase 209 ResumableTurnStreamBuffer in vox-orchestrator"
  - "Mapped thom-chat enhance-prompt and helioschat temporary chat isolation to Phase 209 VOX-HARNESS-T3-03"
patterns-established:
  - "Fire-and-forget generation dispatch with server-buffered chunk replay on client reconnect"
requirements-completed:
  - T3-CHAT-UX-01
  - T3-CHAT-UX-02
  - T3-CHAT-UX-03
  - T3-CHAT-UX-04
duration: 5min
completed: 2026-10-24
---

# Phase 206 Plan 01: T3 Chat UX, Resumable Streaming, and Privacy Sync Research Summary

**Completed deep-dive research and architecture mapping for `TGlide/thom-chat` and `helioschat/app` in `docs/src/architecture/t3chat-ux-streaming-and-sync-research-2026.md`.**

## Accomplishments
- Documented server-resumable background generation (`generate-message/+server.ts`, `generationAbortControllers`, `cancel-generation/+server.ts`, `getGenerationStats`) (`T3-CHAT-UX-01`).
- Documented `POST /api/enhance-prompt` and Cursor-style `@<rule_name>` attachment (`user_rules`, `parseMessageForRules`) (`T3-CHAT-UX-02`).
- Documented conversation branching (`createBranched`, `branchOffChat`), thread pinning, and OpenRouter `:free` vs. BYOK tier + `:online` web-search routing (`T3-CHAT-UX-03`).
- Documented IndexedDB v3 local-first storage, 16-thread chunked hydration, `chat.temporary` incognito isolation, and `manage_memories` / `search_memories` client-side tools (`T3-CHAT-UX-04`).
- Verified with `cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/t3chat-ux-streaming-and-sync-research-2026.md` (`no hard errors`).
