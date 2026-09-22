---
title: "Chat Research & Deep Research — Real End-to-End Design"
description: "Make quick and deep research work inside Axis chat against a real model (google/gemini-3.8-flash via OpenRouter) and real search, with every stage surfaced in the transcript and verified live."
category: "Architecture SSOTs"
status: "proposed"
---

# Chat Research & Deep Research — Real End-to-End Design

**Date:** 2026-09-21 · **Branch:** `claude/research-detection-chat-gui-05d826`

## 1. Goal and definition of done

A user typing in Axis chat gets research **when it is warranted and only then**, sees
**every stage** of what happened, and receives an answer **synthesized by
`google/gemini-3.8-flash` over OpenRouter from real retrieved sources** — never a
template, never a silently substituted model.

Done means all of §8 passes against the **real** stack (real Axis app, real daemon, real
OpenRouter, real SearXNG). A green mocked suite is not done. Prior research work in this
area (the 2026-09-17 HITL walkthrough, `deep-research-honesty.spec.ts`) was validated
exclusively against `tauriMock.ts` fixtures; this design exists because that never
exercised a model.

Out of scope: MENS / VoxLocal / Ollama models (explicitly excluded by the user), the
standalone Research surface's UI (it keeps working but is not redesigned), live
token-level streaming of research (phase 2, §7).

## 2. Current state (verified against code, 2026-09-21)

| # | Defect | Evidence |
|---|---|---|
| D1 | Research trigger is inverted. `distinct_domain_count` is hardcoded to `1`, so the gate's maximum is `0.35+0.15+0.05+0.15 = 0.70` vs a `0.65` threshold. Zero local hits → score 0 → **"hi" fires web research**; well-covered repo questions don't. Intent is never considered. | `vox-orchestrator-mcp/src/memory_tools/retrieval.rs:408-453`, `vox-research-shim/src/research/gate.rs:101-121` |
| D2 | Chat research does no synthesis by default (`research_model_enabled = false`). Raw snippet lines are injected under the header "SYNTHESIS SUMMARY". | `vox-orchestrator/src/config/impl_default.rs:108`, `chat_tools/chat/message.rs:818-822` |
| D3 | Forced research only runs inside the `Ok(bundle)` arm of local retrieval; a retrieval error skips it. `/deepresearch` is not stripped and reaches search as literal text. | `message.rs:683-853`, `:766` |
| D4 | GUI `/research` and `/deepresearch` are sent down the **background** path, whose `SubmitTaskInput` has no research fields — `force_research`, `research_scope`, `domain_mode`, `site_scope` are dropped. `research_scope='deep'` is outside the schema enum. | `vox-gui/ui/src/lib/buildChatTurn.ts:91-112`, `vox-gui/src/commands/chat_turn.rs:234-266`, `input_schemas.rs:644` |
| D5 | Deep pipeline lane is hardcoded to `ResearchLane::default()` (Fast), so the planner never runs from the daemon. | `memory_tools/handlers_memory.rs:300,398` |
| D6 | Any synthesis failure returns `synthesize_answer_template` — fixed headings ("Key architectural considerations based on gathered evidence.") — with the session marked `completed` and no degraded flag. | `research/orchestrator/stages.rs:170-179,330-415` |
| D7 | `judge_max_tokens: 16` while the judge schema asks for reasoning strings; the judge's JSON is expected to truncate and fall back to a fixed score of 80. *(Truncation to be confirmed live in §8; the constant is verified.)* | `research/orchestrator/config.rs:142`, `stages.rs:154` |
| D8 | Cache short-circuit returns before any stage update; pre-created session stays `running` forever. | `pipeline.rs:65-69` |
| D9 | Model pin leaks: research stage dispatch calls `decide()`, which admits only `Confirmed` models and drops OpenRouter (`Shadowed`); free-floor models are appended after the configured model. | `research/orchestrator/model_dispatch.rs:31-37`, `vox-actor-runtime/src/llm/cascade.rs:185-237` |
| D10 | No general-web source: `DuckDuckGoClient::search` is a stub returning `Ok(vec![])`; SearXNG is unconfigured. Only Wikipedia / OpenAlex / arXiv run. | `vox-search/src/duckduckgo.rs`, `web_dispatcher.rs:86-330` |
| D11 | Nothing is visible. `ChatTurnEventRow` renders only `skill_activated`; `ResearchSummaryCard` is mounted nowhere; `ResearchExecuted` carries no session id and is dropped by `resolveSessionForEvent`. | `ui/src/components/surfaces/Chat/ChatTurnEventRow.tsx:22-44`, `ui/src/lib/sessionChatStore.ts:74-121` |
| D12 | `run_multi_hop_web_research` flattens hits to strings and discards per-provider outcomes (timeouts/errors only reach `warn!`). | `vox-search/src/research.rs:37-121` |

## 3. Architecture overview

One dispatch path: **GUI composer → `chat_turn` (Sync) → daemon `vox_chat_message`**.
Research becomes a structured, typed step inside `chat_message`:

```text
prompt ─► classify_research_intent ─► None ───────────────────────► normal chat turn
                     │                                                  (+ detection stage event)
                     ├─► Quick ─► retrieval (report) ─► numbered sources in context
                     │                                   ─► Gemini answers with [n] ─► citation check
                     └─► Deep  ─► Scientia pipeline (lane=Deep, pinned model)
                                   planner → retrieval → claims → synthesis → judge → audit
                                   ─► pipeline answer is the reply
        every branch ─► ResearchTrace ─► `research_stage` events in ChatTurnDto.events
                                        ─► GUI ResearchTrace panel under the bubble
```

The trace travels in the **existing** `events` array (`message.rs:1591` →
`vox-gui/src/commands/chat.rs:447` → `ChatTurnDto.events` → `App.tsx`), so no new IPC
command or channel is needed for phase 1.

## 4. Components

### 4.1 Intent detection — `classify_research_intent`

New pure module `vox-orchestrator-mcp/src/chat_tools/chat/research_intent.rs`.

```rust
pub enum ResearchMode { None, Quick, Deep }
pub struct ResearchIntent {
    pub mode: ResearchMode,
    pub explicit: bool,          // slash command or force_research
    pub reasons: Vec<String>,    // human-readable, shown in the UI
    pub query: String,           // prompt with slash prefix stripped
}
pub fn classify_research_intent(prompt: &str, force: Option<bool>, scope: Option<&str>) -> ResearchIntent;
```

Rules, in order (first match wins):

1. **Explicit.** `/deepresearch …` → Deep; `/research …` → Quick (but `/research search …` stays the
   existing local-KB search path); `force_research = Some(true)` → Quick, or Deep when
   `research_scope == "deep"`; `force_research = Some(false)` → None.
2. **Skip.** Greetings / thanks / small talk; under 3 words; coding or edit imperatives
   ("fix", "refactor", "implement", "write a function/test") or prompts containing code
   fences, file paths, or `@file` mentions. Reason recorded, e.g. `skip: greeting`.
3. **Deep.** Comparative / survey intent: `compare`, ` vs `, `versus`, `trade-offs`,
   `pros and cons`, `state of the art`, `literature review`, `deep dive`,
   `comprehensive overview`.
4. **Quick.** Question about the outside world with a time-sensitive cue (`latest`,
   `current`, `newest`, `recent`, `today`, a 4-digit year ≥ 2020, `version`, `release`,
   `price`, `news`) **or** an explicit evidence ask (`look up`, `search for`, `sources`,
   `cite`, `according to`).
5. Otherwise **None** (`no research cue`).

Deterministic, no LLM call. Replaces `should_trigger_autonomous_research` at the chat
call site (the function is kept for its other caller only if one exists; otherwise
deleted). Local retrieval (memory / KB / repo) still runs for every turn as today — it is
not "research" and is not gated.

### 4.2 Retrieval report — `vox-search`

Add a reporting variant alongside the existing dispatcher (existing signatures unchanged):

```rust
pub struct ProviderOutcome { pub provider: &'static str, pub status: ProviderStatus, pub elapsed_ms: u64 }
pub enum ProviderStatus { Ok { hits: usize }, Timeout, Error(String), NotConfigured, CircuitOpen }
pub struct SearchReport { pub hits: Vec<HybridSearchHit>, pub providers: Vec<ProviderOutcome> }
WebSearchDispatcher::search_with_report(query, lane, policy) -> SearchReport
```

`search_with_lane_and_registry` becomes a thin wrapper returning `report.hits`, so every
existing caller keeps identical behavior. DuckDuckGo stays out of the dispatcher (it is a
stub); the GUI prober's DDG row must report `not implemented`, not "200 OK".

**General web:** SearXNG via the repo's existing `docker/searxng/compose.yml` (colima),
`VOX_SEARCH_SEARXNG_URL=http://localhost:<port>`. JSON output must be enabled in
`docker/searxng/settings.yml` (verify; the dispatcher requests `format=json`). When unset,
the provider table shows `searxng: not configured` — honest, not hidden.

### 4.3 Quick research

New `perform_chat_research_quick(query) -> ResearchTrace` in
`vox-orchestrator/src/orchestrator/task_dispatch/research_dispatch.rs`:

- One Fast-lane wave of `search_with_report` on the stripped query (lane timeout raised
  for chat to a value measured in §8 — 1.5 s is expected to be too tight for SearXNG).
- Dedupe by URL, keep top N (default 8), number them `[1]..[N]`.
- **No second LLM call.** The numbered sources are injected into the main chat prompt as a
  `[WEB RESEARCH — N SOURCES]` block with the instruction: answer from these sources,
  cite as `[n]`, say explicitly when the sources don't answer the question. Gemini (the
  chat model) is the synthesizer, so there is no template branch to fall into.
- Zero sources → the block says `web research returned 0 sources` and the trace stage is
  `empty`; the model is told not to invent citations.

**Citation check (post-answer):** parse `[n]` markers from the reply; record
`cited: [..]`, `invalid: [..]` (n outside 1..N), `uncited_answer: bool`. Surfaced as the
final stage. This is the in-harness signal that the answer actually used the research.

### 4.4 Deep research

`chat_message` calls `run_research_with_context_and_session` **inline** (awaited) with
`lane = Deep`, `verify_claims = true`, and a `progress_callback` that appends
`(stage message, pct, elapsed_ms)` to the trace. The pipeline's `answer` + `sources` become
the reply (the chat model does not re-synthesize). Pipeline fixes:

| Fix | Change |
|---|---|
| D5 | Add `lane` to `ResearchStartParams` / pass through; chat path sets Deep explicitly. |
| D6 | Synthesis failure returns `Err(SynthesisFailed(msg))`; `synthesize_answer_template` is deleted. The session is marked `failed`. Test: mutation — reinstate the template call and the test must fail. |
| D7 | Judge `max_tokens` set to a value that fits its schema (≥ 400, the stage default); judge parse failure is recorded as `judge: failed(<reason>)`, never a synthetic 80. |
| D8 | Cache hit sets the session `completed` and the trace stage reads `served from cache (age Xs)`. |
| — | `ResearchMetadata` gains `synthesis_model: String` so the UI shows the model that actually answered. |

### 4.5 Strict model pinning

- `model_dispatch::primary_candidate_for_intent`: when `VOX_MODEL_FORCE` is set, return
  `LlmConfig::openrouter(force)` directly, bypassing `decide()`.
- `cascade.rs`: when a force is set, append **no** free-floor and **no** local candidates.
- Chat: `VOX_ROUTING_HARD_PIN_MODEL=google/gemini-3.8-flash` (already honored first in
  `resolve.rs:320`); GUI model picker set to the same id.
- Runtime check: every research trace records the model actually used per LLM stage; §8
  asserts it equals the pin. If OpenRouter errors, the turn fails with the real error.
- Where these env values live for the GUI-spawned daemon is decided in the plan (the
  daemon inherits the GUI's environment; a persisted user-config key is preferred if one
  already exists).

### 4.6 Trace → events

```rust
pub struct ResearchTrace { pub mode, pub intent: ResearchIntent, pub stages: Vec<ResearchStageEvent>, pub total_ms: u64 }
pub struct ResearchStageEvent {
    pub stage: String,            // detection|queries|retrieval|sources|planning|claims|synthesis|judge|citation_audit|citation_check|cache
    pub status: String,           // ok|empty|skipped|failed
    pub elapsed_ms: Option<u64>,
    pub summary: String,          // one line for the collapsed row
    pub detail: serde_json::Value // structured payload (providers table, source list, claims…)
}
```

Serialized into `events` as `{ "kind": "research_stage", ... }`, one per stage, in order,
plus one `{ "kind": "research_summary", mode, source_count, model, total_ms, status }`.
A detection event with `mode: none` is emitted on **every** turn (collapsed by default)
so "why didn't it research?" is always answerable.

### 4.7 GUI

- `buildChatTurn.ts`: research slash commands stay on **Sync** execution and send the raw
  text (`/deepresearch …`) so the daemon classifier sees the prefix; stop sending
  `research_scope: 'deep'`.
- `ChatTurnEventRow.tsx`: render `research_summary` as a header chip and `research_stage`
  rows inside a new `ResearchTracePanel` (collapsible). Per stage: status icon, name,
  elapsed, summary; expand → detail (provider table; sources as links with engine badge;
  subqueries; claim verdicts; judge score + rationale; citation check with invalid
  markers highlighted). `failed` is red with the literal error.
- Delete dead `components/chat/ChatMessage.tsx` + `ResearchSummaryCard.tsx` if nothing
  else imports them (verified by grep in the plan), rather than leaving two renderers.
- `data-testid`s on panel, each stage row, and each source link for §8.

## 5. Error handling (no-template rule)

| Failure | User sees |
|---|---|
| OpenRouter key missing / 401 / 402 / 429 | Turn fails with the classified `ChatTurnError`; research stages up to that point still render. |
| All providers fail or time out | Retrieval stage `failed` with per-provider reasons; Quick: model told 0 sources; Deep: pipeline returns the zero-hit error (already exists) surfaced in the bubble. |
| Deep synthesis fails | Bubble shows `Deep research failed at synthesis: <error>` plus the retrieved sources list. No generated prose. |
| Judge fails | Stage `judge: failed(<reason>)`; answer still shown. |
| Answer cites nothing / invalid `[n]` | Citation-check stage `failed` (amber), answer still shown. |

Invariant enforced by tests: **no code path produces answer prose that did not come from
an LLM response.**

## 6. Testing strategy

1. **Unit (test-first):**
   - Classifier table test (~30 prompts incl. "hi", "thanks!", "fix the bug in foo.rs",
     "what's the latest Gemini Flash on OpenRouter?", "compare SearXNG vs Tavily",
     `/research x`, `/deepresearch x`, `/research search x`).
   - `search_with_report` with wiremock providers: ok / timeout / 500 / not configured.
   - Citation parser.
   - Trace → events serialization.
   - Deep synthesis failure → `Err` (mutation-verified: reinstating the template makes it fail).
2. **GUI unit (vitest):** `ResearchTracePanel` renders each status; `buildChatTurn` keeps
   research slash on Sync with raw text.
3. **Live E2E** — see §8.

## 7. Phasing

- **Phase 1 (this spec):** §4.1–4.7 + §8.
- **Phase 2 (only after phase 1 is green live):** live in-flight stage progress — emit
  research-stage frames on the agent-events bus with the chat `session_id` so
  `resolveSessionForEvent` routes them to the pending bubble.

## 8. Live verification (the definition of done)

Setup: colima + SearXNG container running; `OPENROUTER_API_KEY` resolvable by the
daemon; pins from §4.5; Axis built from this worktree and launched with `--drive`.

Driven through the existing Drive harness (`vox gui drive send / wait --until reply_ok /
state`), which pushes text through the **real composer** and records the reply plus agent
frames. Captured `state` JSON is saved per prompt under
`crates/vox-gui/ui/e2e/fixtures/live-research/` with a header recording date, model,
and commit.

| Prompt | Must hold |
|---|---|
| `hi` | detection `mode: none` (`skip: greeting`); no retrieval stage; no web request made. |
| `What is the latest Gemini Flash model on OpenRouter and when was it released?` | detection Quick with a time-sensitive reason; provider table includes `searxng: ok(n>0)`; ≥ 3 sources with http(s) URLs; answer mentions `3.8`; citation check: ≥ 1 cited, 0 invalid; `model_id == google/gemini-3.8-flash`. |
| `/deepresearch compare SearXNG and Tavily for agent web search` | planning stage with ≥ 2 subqueries; ≥ 1 claim verdict; judge stage `ok` with a score that came from the model (rationale present); synthesis model `google/gemini-3.8-flash`; answer names both systems. |

Anti-template assertions on every reply: none of the strings from the deleted template
appear; the two research answers share no paragraph; no `[autonomous_research:` raw lines
in the answer.

**Screenshots.**
- `screencapture` of the live Axis window for each prompt, with the trace expanded, to
  prove the real app rendered it.
- A Playwright spec (`e2e/chat-research-trace.spec.ts`) loads the **recorded live
  payloads** (never hand-written) into the mock transport, then screenshots the collapsed
  bubble and each expanded stage into `review-bundle/latest/`.
- Limitation, stated plainly: Playwright cannot attach to the macOS Tauri WKWebView
  (`tauri-driver` is Linux/Windows only), so the Playwright shots are replays of real
  data, while the `screencapture` shots are the live proof.

Results, including failures, are reported with the raw captured output.

## 9. Open items resolved during planning

- Exact chat lane timeout (measure SearXNG p95 in §8 setup).
- Whether `should_trigger_autonomous_research` has other callers (keep or delete).
- Persisting the model pin for the GUI-spawned daemon (env vs existing user-config key).
- Diagnosis of `vox secrets set` hanging is tracked separately (spawned task) and blocks
  §8 until a key resolves.
