---
phase: 206
phase_name: "deep-dive-document-open-source-t3-chat-clones-tglide-thom-ch"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 7
  lessons: 0
  patterns: 4
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 206 Learnings: deep-dive-document-open-source-t3-chat-clones-tglide-thom-ch

## Decisions

### Mapped thom-chat background generation + generationAbortControllers to P...
Mapped thom-chat background generation + generationAbortControllers to Phase 209 ResumableTurnStreamBuffer in vox-orchestrator

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 206-01-SUMMARY.md

---

### Mapped thom-chat enhance-prompt and helioschat temporary chat isolation ...
Mapped thom-chat enhance-prompt and helioschat temporary chat isolation to Phase 209 VOX-HARNESS-T3-03

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 206-01-SUMMARY.md

---

### Documented server-resumable background generation (generate-message/+ser...
Documented server-resumable background generation (`generate-message/+server.ts`, `generationAbortControllers`, `cancel-generation/+server.ts`, `getGenerationStats`) (`T3-CHAT-UX-01`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 206-01-SUMMARY.md

---

### Documented POST /api/enhance-prompt and Cursor-style @<rule_name> attach...
Documented `POST /api/enhance-prompt` and Cursor-style `@<rule_name>` attachment (`user_rules`, `parseMessageForRules`) (`T3-CHAT-UX-02`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 206-01-SUMMARY.md

---

### Documented conversation branching (createBranched, branchOffChat), threa...
Documented conversation branching (`createBranched`, `branchOffChat`), thread pinning, and OpenRouter `:free` vs. BYOK tier + `:online` web-search routing (`T3-CHAT-UX-03`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 206-01-SUMMARY.md

---

### Documented IndexedDB v3 local-first storage, 16-thread chunked hydration...
Documented IndexedDB v3 local-first storage, 16-thread chunked hydration, `chat.temporary` incognito isolation, and `manage_memories` / `search_memories` client-side tools (`T3-CHAT-UX-04`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 206-01-SUMMARY.md

---

### Verified with cargo run -p vox-doc-pipeline -- --lint-only --paths archi...
Verified with `cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/t3chat-ux-streaming-and-sync-research-2026.md` (`no hard errors`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 206-01-SUMMARY.md

## Lessons

## Patterns

### Fire-and-forget generation dispatch with server-buffered chunk replay on...
Fire-and-forget generation dispatch with server-buffered chunk replay on client reconnect

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 206-01-SUMMARY.md

---

### server-resumable-streaming
server-resumable-streaming

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 206-01-SUMMARY.md

---

### prompt-enhancer
prompt-enhancer

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 206-01-SUMMARY.md

---

### temporary-chat-isolation
temporary-chat-isolation

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 206-01-SUMMARY.md

## Surprises

