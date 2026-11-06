---
title: "T3-to-Vox Domain 5 Implementation Spec: @rule Mentions, Dynamic Variables, Weighted Fuzzy Search, Signed Webhooks, CLI Session Importer & Route Racing (2026)"
description: "Detailed Rust and TypeScript implementation specification for Phases 224-225: @rule manual/always user rules, $variable system prompt interpolation, regenerate-with-posture vs in-place assistant edit, 3-mode weighted fuzzy & link-stripped trigram search, signed/filtered webhook ingress, external CLI agent session importer, and 6-tier staggered route racing."
category: "Architecture SSOTs"
status: "current"
topic: "agent-orchestrator"
domain: "orchestration"
authorship: "llm-generated"
authorship_model: "claude-opus-4-7"
---

# T3-to-Vox Domain 5 Implementation Spec: `@rule` Mentions, Dynamic Variables, Weighted Fuzzy Search, Signed Webhooks, CLI Session Importer & Route Racing (2026)

<!-- kb:summary -->
This document specifies the exact Rust and TypeScript implementation for `Phases 224–225` (`T3-RULES-SEARCH-01..04` and `T3-WEBHOOK-IMPORT-01..04`) of Milestone `v4.1`. It details how to extend `mentions.rs` with `@rule:<name>` and `attach: Always` user rules (`thom-chat`), add `$variable` system prompt interpolation (`helioschat`), wire regenerate-with-stored-posture vs. in-place assistant message editing and 100-entry `promptHistory`, combine 3-mode weighted fuzzy search with markdown-link-stripped trigram search (`t3code` `ThreadSearch.ts`), upgrade `webhook_intake.rs` with signed/filtered/deduped webhook & cron triggers, import external `~/.claude`, `~/.codex`, and `~/.cursor` sessions, and add 6-tier staggered route racing (`150ms` head start).
<!-- /kb:summary -->

## 1. Phase 224: `@rule` Mentions, Dynamic `$variable` Prompts, Regenerate-with-Posture & 3-Mode Fuzzy/Trigram Search (`T3-RULES-SEARCH-01..04`)

### 1.1 `@rule:<name>` Manual & `Always` User Rules (`crates/vox-orchestrator-mcp/src/chat_tools/chat/mentions.rs`)
- **Source Inspiration:** `TGlide/thom-chat` `convex/schema.ts` (`user_rules`) & `src/routes/_chat.tsx:690-740` (`parseRulesFromText`).
- **Legacy Code Replaced:** File-path-only `MENTION_RE` in `crates/vox-orchestrator-mcp/src/chat_tools/chat/mentions.rs:14-68` that treats `@rule:foo` as a missing workspace file path (`[Mentioned file not found: rule:foo]`).
- **Implementation Design:**
  - Introduce `UserRuleEntry { id: String, name: String, attach: RuleAttachMode, rule_body: String }` where `RuleAttachMode` is `Always | Manual` and `name` is validated against `^[a-zA-Z0-9_-]+$` (excluding reserved `@file`, `@workspace`, `@web`).
  - Before resolving file `@mentions` in `resolve_mentions`, extract all `@<rule_name>` tokens matching configured `Manual` rules, strip the raw `@<rule_name>` tokens from the user prompt text, deduplicate the matched `Manual` rules with all enabled `Always` rules, and append a consolidated `<user_rules>` block at the end of the turn payload.

### 1.2 Dynamic System Prompt `$variable` Interpolation (`crates/vox-orchestrator-mcp/src/chat_tools/mod.rs` & `preamble_and_retrieval.rs`)
- **Source Inspiration:** `helioschat/app` `src/lib/utils/systemPromptVariables.ts`.
- **Legacy Code Replaced:** Static string assembly in `build_system_prompt_with_skill` (`crates/vox-orchestrator-mcp/src/chat_tools/mod.rs:88-90`).
- **Mechanics:**
  - Single-pass `$variable` resolver (`interpolate_system_prompt_variables`) supporting both `helioschat`'s 5 standard variables and 3 Vox-native workspace variables:
    1. `$current_datetime` — ISO-8601 timestamp with local offset
    2. `$user_timezone` — IANA timezone identifier
    3. `$user_locale` — BCP-47 locale tag
    4. `$user_language` — primary language code
    5. `$user_platform` — OS/arch (`std::env::consts::OS` / `ARCH`)
    6. `$workspace_root` — canonical repository root name
    7. `$git_branch` — active Git/VCS branch
    8. `$active_spoke` — active Vox routing spoke (`coding`, `research`, `planning`, `review`, `general`)

### 1.3 Regenerate-with-Stored-Posture vs. In-Place Assistant Edit & 100-Entry `promptHistory` (`crates/vox-orchestrator-mcp/src/chat_tools/chat/history.rs` & `crates/vox-gui/ui`)
- **Source Inspiration:** `helioschat/app` `src/routes/+page.svelte:132-297`, `shaltielshmid/NotT3Chat` `ChatPage.razor`, and `t3code` composer history.
- **Mechanics:**
  1. **Per-Message Stored Generation Posture:** Every persisted assistant message stores `GenerationPostureSnapshot { model_id, provider, reasoning_effort, web_search_enabled, temperature, top_p }`.
  2. **Regenerate (`vox_chat_regenerate`):** Truncates from the target assistant message (or user message) onward and re-runs the turn reusing the message's stored `GenerationPostureSnapshot` by default, or substituting a newly chosen `model_override` from the regenerate dropdown.
  3. **In-Place Assistant Edit (`vox_chat_edit_message_in_place`):** Mutates an assistant message's `content` in-place in the session transcript **without** truncating subsequent turns, enabling manual clean-up of model outputs before continuing a long thread.
  4. **100-Entry `promptHistory` (`ArrowUp`/`ArrowDown`):** Persists the last 100 submitted prompts in local storage; pressing `ArrowUp` at line 0 stashes the current unsent text into `currentDraft` and walks backward through history, while `ArrowDown` walks forward and restores `currentDraft` at the bottom.

### 1.4 3-Mode Weighted Fuzzy Search, Link-Stripped Trigram Search & In-Thread Occurrence Jumper (`crates/vox-search` & `crates/vox-gui/ui`)
- **Source Inspiration:** `TGlide/thom-chat` `src/lib/fuzzy-search.ts`, `t3code` `apps/server/src/search/ThreadSearch.ts`, and `apps/web/src/components/ThreadFind.ts`.
- **Mechanics:**
  1. **Markdown-Link URL Stripper (`strip_markdown_link_urls`):** Before indexing or matching chat messages, replaces `[label](url)` with `label` so long URLs inside markdown links never pollute trigram or fuzzy search scores.
  2. **Weighted `calculate_match_score` (`min_score = 0.3`):**
     - Exact match: `1.0`
     - Substring match: `0.8 + 0.2 * (query.len() as f32 / target.len() as f32)`
     - All-words match: `0.9`
     - Partial-words match: `(matched_words / total_words) * 0.7`
     - Prefix match: `0.6`
     - Character-order fuzzy match: `(matched_chars / query.len()) * 0.5 + (max_consecutive / query.len()) * 0.2`
     - Recency tie-breaker (`+0.05` max boost for threads updated within 24h) and title boost (`1.4x` vs. `1.0x` content).
  3. **In-Thread Find (`ThreadFind`):** Searches across both message bodies and inline plan artifacts within the active thread, exposing `next_match` / `prev_match` occurrence jumpers.

---

## 2. Phase 225: Signed/Filtered Webhook Ingress, Cron Scheduled Tasks, CLI Session Importer & 6-Tier Route Racing (`T3-WEBHOOK-IMPORT-01..04`)

### 2.1 Replace Minimal `webhook_intake.rs` with Signed/Filtered Webhook Ingress & Scheduled Tasks (`crates/vox-orchestrator-mcp/src/server_state/webhook_intake.rs`)
- **Source Inspiration:** `t3code` `apps/server/src/scheduledTasks/ScheduledTasksService.ts` & `packages/contracts/src/scheduledTasks.ts`.
- **Legacy Code Replaced & Deleted:**
  - Replace the minimal 64-line `crates/vox-orchestrator-mcp/src/server_state/webhook_intake.rs` and **delete** the deprecated `VOX_WEBHOOK_GITLAB_TOKEN` warning stub at lines 27–32.
- **Mechanics:**
  1. **Signed Webhook Verification (`vox_crypto::keyed_hash`):** Verifies `X-Vox-Signature-256` (or `X-Hub-Signature-256`) using `vox_crypto::keyed_hash` and Clavis secret `SecretId::VoxWebhookSigningSecret`.
  2. **Idempotency Deduplication (`webhook_dedupe`):** Tracks `(webhook_id, delivery_id, received_at_ms)` with a 24h TTL so retried deliveries never spawn duplicate agent turns.
  3. **Header & JSONPath Payload Filtering + Rate/Queue Caps:** Matches incoming headers and dot-path JSON fields before waking a task; enforces `WEBHOOK_RATE_LIMIT_PER_MINUTE = 60` and `MAX_QUEUED_SCHEDULED_RUNS = 20`.
  4. **Cron & Interval Scheduled Tasks:** Supports `ScheduleTrigger::Cron(String)`, `ScheduleTrigger::IntervalMs(u64)`, and `ScheduleTrigger::Webhook(WebhookFilterSpec)` with configurable thread target (`NewThread` vs. `ReuseLastThread`).

### 2.2 External CLI Agent Session Scanner & Importer (`crates/vox-orchestrator/src/session/external_importer.rs`)
- **Source Inspiration:** `t3code` `apps/server/src/sessionImport/AgentSessionScanner.ts` & `AgentSessionImporter.ts`, plus `helioschat` `src/lib/utils/dataExport.ts`.
- **Mechanics:**
  - Discovers and imports external coding agent sessions into Vox threads (`vox session import-external`):
    1. **Claude Code (`~/.claude/projects/**.jsonl`):** Parses user/assistant turns, tool uses, and token usage metadata.
    2. **Codex CLI (`~/.codex/sessions/**.jsonl`):** Parses rollout items, command executions, and patches.
    3. **Cursor (`~/.cursor/chats/<workspace-hash>/<session-id>/store.db`):** Reads SQLite `meta` (hex-encoded JSON root blob pointer) and recursively walks `blobs` DAG nodes to reconstruct the conversation history.
  - Also exposes `vox session export-bundle` and `vox session import-bundle` (`helioschat` `dataExport.ts` parity) for single-file portable JSON backup and restore of threads, branches, user rules, and pinned skills.

### 2.3 6-Tier Staggered Route Racing (`crates/vox-gui/ui/src/transport/routeRacer.ts`)
- **Source Inspiration:** `t3code` `packages/client-runtime/src/connection.ts` & `packages/shared/src/route-Order.ts`.
- **Mechanics:**
  - Ranks candidate daemon endpoints in priority order: `LoopbackWs (0) < LanDirect (1) < TailscaleDirect (2) < CloudflareTunnel (3) < RelayWebRtc (4) < RelayWs (5)`.
  - Races candidates with a **`150ms` staggered head start** (`STAGGER_DELAY_MS = 150`): Tier `i + 1` starts dialing `150ms` after Tier `i` if Tier `i` has not yet connected, aborting slower dials as soon as the highest-priority reachable transport completes handshake.
  - Runs a background better-route probe every `60_000ms` (`BETTER_ROUTE_PROBE_INTERVAL_MS = 60_000`) when connected on a lower-priority relay, with a `300_000ms` (`5m`) failure cooldown per failed candidate.

---

## 3. Verification Gates (`Phases 224–225`)
- `timeout 900s cargo test -p vox-orchestrator-mcp --lib chat_tools::chat::mentions::tests` (verifying `@rule:<name>` manual extraction, prompt stripping, and `Always` rule deduplication).
- `timeout 900s cargo test -p vox-orchestrator-mcp --lib chat_tools::tests::system_prompt_variable_interpolation` (verifying all 8 `$variable` substitutions).
- `timeout 900s cargo test -p vox-orchestrator-mcp --lib server_state::webhook_intake::tests` (verifying `vox_crypto::keyed_hash` signature verification, `webhook_dedupe` idempotency, and `60/min` rate limit).
- `timeout 900s cargo test -p vox-orchestrator --lib session::external_importer::tests` (verifying Claude JSONL, Codex JSONL, and Cursor SQLite hex-blob session import round-trips).
