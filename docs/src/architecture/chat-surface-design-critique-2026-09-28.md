---
title: "Chat Surface Design Critique and Information Matrix (2026-09-28)"
description: "Audit of the vox-gui chat surface, execution rail, composer and status bar: duplicated metrics, missing engine trace information, progressive disclosure, vocabulary and visual language, with a one-home-per-fact design matrix."
category: "Architecture SSOTs"
status: "roadmap"
training_eligible: false
authored: "2026-09-28"
---

# Chat Surface Design Critique and Information Matrix

Evidence: review-bundle captures (`chat--{default,empty,error,composer-filled}--wide`, `chat-trust-receipts.png`, `chat-trust-claims.png`, `research-debugger-stepper.png`), axe results from `e2e/review/capture.spec.ts`, and a code inventory of `crates/vox-gui/ui/src` (file:line citations below are relative to that directory). Builds on [GUI Intuitiveness Implementation Plan](ai-first-plan-3-gui-intuitiveness-2026-07-02.md) (the attention inbox and intent panel it shipped are kept). The implementation plan, written to be driven task-by-task by Claude Code through the `agy` CLI, is [`docs/superpowers/plans/2026-09-28-chat-surface-trace-and-latest-models.md`](../../superpowers/plans/2026-09-28-chat-surface-trace-and-latest-models.md).

## Overall impression

The chat surface looks calm and branded, but it answers "what is the whole system doing?" in four places and "what did *this* turn do?" in none. The same numbers (agents, queue, spend, model, mesh) appear in the status bar, the Execution rail, the composer, and the Dashboard. Meanwhile the per-turn facts a user needs to trust or debug a reply are either missing or scattered as chips: which tools ran, whether they were receipted, why a model was chosen, what grounding concluded, and what was delegated. The biggest opportunity is a **collapsed per-turn trace** under each assistant message, with the status bar and rail each owning one scope.

## The one-home-per-fact rule

| Scope | Home | What lives there | What leaves |
|---|---|---|---|
| **Global engine** (whole orchestrator, all sessions) | Status bar cards, each with a popover for detail | Engine (agents · queue), Spend (budget burn, OpenRouter), Mesh, Routing (auto + resolved model family), Needs you (approvals + questions), Freshness (LIVE/POLL/OFFLINE) | The rail's "Resources" block, the composer's global "session $x/$y" text |
| **This session** | Execution rail | Tasks (with lock and phase chips), Routing decision for the last turn, Context window, Session spend | The Agents roster (moves to the Agents surface; the status-bar card links there), global Resources |
| **This turn** | Transcript: the message plus **one collapsed trace row** | Tools called and their receipt state, model resolution and reason, grounding verdict, delegations, research milestones, locks waited on, errors and timeouts | Free-floating chips that are not actionable |
| **Needs a human now** | Inline interrupt chip in the transcript, plus the status-bar Needs-you count | Approval required, fabricated/unverified claims, injection detected, budget exceeded, scope violation | — |
| **Deep inspection** | Inspector drawer that reuses the research debugger's stage stepper and payload panes | Full per-step input/output payloads, timings, raw event JSON | — |

## Design matrix: every chat-adjacent element

Verdicts: **Keep**, **Move** (to another home), **Merge**, **Rename**, **Collapse** (behind disclosure), **Add**, **Delete** (orphaned code).

| # | Element (where) | Shows today | Duplicates | Verdict | Target |
|---|---|---|---|---|---|
| 1 | Status bar AGENTS (`BottomStatusBar.tsx:141`) | active agents | rail Resources, rail roster, Dashboard | Merge | "Engine" card: `9 agents · 44 queued`; popover lists busy agents and links to Agents |
| 2 | Status bar QUEUE | queue depth | rail Resources, composer "N queued", Dashboard | Merge | into Engine card; the composer pill stays only when this session has queued work |
| 3 | Status bar BUDGET | global `budgetBurn` | DriveConsole meter, composer "session $x/$y", Dashboard | Merge | "Spend" card `$12.34 / $50`; popover splits OpenRouter vs local and by session |
| 4 | Status bar OR SPEND | `useLlmSpend` | rail OpenRouter | Merge | into Spend card popover |
| 5 | Status bar MESH | `X/Y online` via `useMeshNodes` | rail "N peers" (different source) | Keep, single source | one hook, one phrasing: `Mesh 2/2 online` |
| 6 | Status bar MODEL | `activeModel ?? 'auto-route'` | rail Model, rail "Intents", composer Run on, badge | Rename + Merge | "Routing" card: `Auto → claude-opus (latest)`, showing a version **only when read from the OpenRouter catalog** |
| 7 | Status bar APPROVALS | approvals count | Approvals dock, Needs You, Review | Rename | "Needs you" card (approvals + questions), matching the attention inbox |
| 8 | Status bar RESEARCH | lane + Tavily quota; popover mostly hardcoded (`StatusBarCluster.tsx:80-139`) | — | Keep, make honest | popover from real engine status only; no hardcoded "Online" or "✓ Wikipedia (Live)" |
| 9 | Status bar LIVE / POLL / OFFLINE | freshness | — | Keep | rename POLL to "Polling" in the tooltip |
| 10 | Status bar Configure ▾ | tile toggles, "Budget burn" label | — | Keep | labels match the card names |
| 11 | Rail "Execution" task list | session tasks + lifecycle | — | Keep + Add | lock chips (Phase 5), phase chip (revive `PhaseChip`) |
| 12 | Rail "Intents" | routing selection · bandit state, "Alt:" rows | status bar MODEL | Rename + Collapse | "Routing": `Routed to claude-opus (latest: claude-opus-5-5) — exploit`; alternatives behind a disclosure; the word "exploit" gets a tooltip ("using the best-known model; explore = trying an alternative") |
| 13 | Rail "Agents" roster | all agents + tasks | status bar, Agents surface | Move | out of the chat rail; the Engine card links to Agents |
| 14 | Rail "Resources" | agents, queue, mesh, model, OpenRouter, session | status bar ×5 | Delete block | keep only "Session spend" in the rail (the one session-scoped row) |
| 15 | Rail ContextWindowMeter | context budget, fetched once | — | Keep + Fix | poll on each turn completion (it goes stale today) |
| 16 | Transcript message bubble | text, streaming state | — | Keep | — |
| 17 | ModelBadge | model id, guessed provider/cost (`ChatTranscript.tsx:69-73`), selection line never rendered | status bar MODEL | Merge | folds into the trace summary row: resolved model, selection reason (your pick / auto / fell back), real cost from the turn |
| 18 | `tool_receipt` chips | one chip per tool call | — | Collapse | into the trace: summary shows `3 tools · 3 receipts ✓`; per-tool rows appear on expand |
| 19 | `receipt_claims` chip | claims verdict counts | — | Keep as interrupt | clean verdicts render inside the trace; any fabricated/unverified claim stays an inline amber interrupt |
| 20 | `skill_activated` chip | skill id + "not this one" | — | Keep | inline, because it is actionable |
| 21 | `delegation_spawned` event | emitted, **never rendered** | — | Add | trace row `Delegated to agent 3 · task 812` linking to the task |
| 22 | `research_milestone` event | emitted, **never rendered** | — | Add | trace row `Research · 3 waves · 12 claims verified · 1 contradiction` |
| 23 | ~75 `AgentEventKind`s never reaching chat | — | — | Add, filtered | only turn-correlated kinds, into the trace: `tool_timed_out`, `compaction_triggered`, `context_truncated`, `injection_detected`, `budget_alert`, `scope_violation`, `replan_triggered`, `semantic_drift_detected`, `doubt_reported`, `pav_phase_changed`, `llm_call_completed`; hopper/mesh/workflow families stay on their surfaces |
| 24 | StatusLine "{phase} · Ns" | in-flight task phase | — | Keep | becomes the live header of the trace row while streaming |
| 25 | "Done · $x" row | turn cost | — | Merge | into the trace summary |
| 26 | Harness issue line (8 s poll) | harness issues | Harness surface | Keep as interrupt | interrupt chip only when it affects this session |
| 27 | Composer DriveConsole clutch | Free / Effic. / Bal. / Genius; meaning only in `title` | — | Rename | full words, `Free · Efficient · Balanced · Genius`, with a one-line visible hint under the group on hover/focus; add Responsive if routing defines it |
| 28 | DriveConsole cost meter | global spend | status bar | Keep, relabel | `Spend $12.34 / $50` (global), or scope it to the session and say so |
| 29 | Composer "session $x / $y" (`slashRouter.ts:48`) | global cost labelled "session" | DriveConsole meter | Delete | wrong label and a duplicate |
| 30 | Risk button "Moderate" | risk level | grounding toggle overlaps | Rename | `Risk: Moderate` |
| 31 | "grounding: off" toggle | post-reply confidence check | Risk popover text | Rename + Move | `Check replies` inside the Risk popover, so the two controls stop contradicting each other |
| 32 | "Run on Auto" tier picker | tiers + keyed models | status bar MODEL | Rename | `Model: Auto` with tiers Auto / Local / Mesh / Cloud; version strings only from the catalog (see the model track) |
| 33 | "Intent" toggle | Goal / Constraints / Acceptance / Effort | rail "Intents" (a different meaning) | Keep | the rail stops using the word (row 12) |
| 34 | Send mode "Quick chat / Background task" | execution mode; `plan` mode has no menu entry | — | Keep + Fix | add "Plan", or delete the unreachable state |
| 35 | Run button | aria says "(Enter)", hint shows ⌘↵ | — | Fix | aria and hint agree |
| 36 | AttentionBudgetMeter | attention minutes, auto-collapse | Needs You | Keep | rename its "budget" to "attention" |
| 37 | SecretaryToast | suggested task | — | Rename | "Suggested task" |
| 38 | Panels ▾ menu | dock panel toggles, "Mercatus" | sidebar says "Market" | Rename | one label source (`LEXICON`), drop `NAV_LABELS` drift |
| 39 | Orphans: `ChatModelPicker`, `ChatMessage`, `ResearchSummaryCard`, `AttentionStrip`, `buildTranscriptTimeline`, dead `chatDock` block (`App.tsx:1865`) | — | — | Delete or revive | revive `ChatAgentEventRow` + `PhaseChip` inside the trace; delete the rest |
| 40 | `useChatVerbosity` (no UI; `verbose` == `normal`) | — | — | Add | a Quiet / Normal / Verbose control that sets the trace's default expansion |

## Usability

| Finding | Severity | Recommendation |
|---|---|---|
| No per-turn account of what happened: tools, receipts, routing reason and grounding are scattered or missing | 🔴 Critical | Collapsed trace row per assistant turn (matrix rows 17–25), with an inspector for depth |
| Two emitted event kinds and ~75 engine events never reach the user; `delegation_spawned` vanishes silently | 🔴 Critical | Render them in the trace with a filter policy (row 23); a contract test keeps Rust and TS in sync |
| The same global numbers appear 3–4 times; "session" labels a global figure | 🔴 Critical | One home per fact (the table above); delete the composer duplicate |
| Model names are pinned, stale versions ("opus-4-8", "sonnet-4-6"), shown as if current | 🔴 Critical | Latest-model resolution from the OpenRouter catalog; display only resolved versions (plan track B) |
| Clutch abbreviations "Effic." / "Bal." with meaning only in tooltips | 🟡 Moderate | Full words and a visible hint |
| "Intents" in the rail means routing; "Intent" in the composer means a goal spec | 🟡 Moderate | Rename the rail section "Routing" |
| Risk "Moderate" and "grounding: off" read as contradictory controls | 🟡 Moderate | Merge grounding into the Risk popover |
| Context meter is fetched once per session and goes stale | 🟡 Moderate | Refresh on turn completion |
| Research popover shows hardcoded health | 🟡 Moderate | Real data or nothing |
| Run button aria text disagrees with its visible hint | 🟢 Minor | Align |
| `plan` send mode is unreachable | 🟢 Minor | Add it to the menu or delete it |

## Visual hierarchy

- **What draws the eye first:** the brass-outlined composer and the RUN/RESUME button. That is correct for an idle session; while a turn runs, the eye should go to the live trace header, which today is a small `StatusLine`.
- **Reading flow:** sidebar → transcript → composer, with a right rail that is taller than the transcript and full of global counters (`chat--default` capture: nine agent rows push Resources below the fold). The rail competes with the conversation instead of annotating it.
- **Emphasis:** amber (`text-amber-*`, ×39) marks prices, the active clutch, risk and warnings alike, so a warning does not stand out. Reserve the warn token for states needing attention.

## Consistency

| Element | Issue | Recommendation |
|---|---|---|
| Status colour | tokens `--color-status-pass/fail/warn/info` exist but only 2 components use them; chat, composer and status bar hardcode amber/rose/emerald/cyan Tailwind classes and raw `#0b0b0e` | Route all status colour through the four tokens; add a lint rule |
| Type scale | chips and values use 8, 9, 10, 11 and 12 px; tracked uppercase uses ~7 letter-spacings (0.12–0.32em) | Two label styles (display caps at 0.13em per Limes `conventions.md`, micro caps at 0.08em) and a minimum of 11 px for data |
| Glow effects | Loquela glows (`Loquela.tsx:711, :819`) and `index.css:125` | Remove; Limes says "No glows, no neon" |
| Label sources | `NAV_LABELS` vs `LEXICON` drift ("Mercatus"/"Market", "Runs"/"Review", "Search Index"/"graphify"/"VoxGraph") | One label source |
| Latin mode | every label has an `la` variant; internal names (Loquela, Oratio, Secretary, Axis Inspector) leak into English mode | English mode never shows Latin or code names |

## Accessibility (axe, wide viewport, chromium)

- **Color contrast: fail.** Two serious violations per chat state, for example `text-zinc-500` mono at 11 px on `#11151a` (3.79:1) and `text-brass/70` at 10 px (4.12:1); both need 4.5:1. Approvals has 3, Activity 1.
- **Heading order: fail (moderate)** on chat empty/error; the Activity and Approvals surfaces lack an `h1`; Activity has 2 unlabeled selects (critical).
- **Text readability:** 9–10 px mono data is below comfortable reading size and makes the contrast problems worse.
- **Review coverage gap:** 3 chat review states (`model-picker-open`, `session-menu-open`, `rails-overlay-open`) target elements that no longer exist, so those views are not captured.

## What works well

- The Phase 5 receipt and claims chips carry only server-derived data, and model-supplied ids never reach trusted chrome. Keep that rule for every trace row.
- The research debugger's stage stepper, with its Payloads / Violations inspector, is the right pattern for deep inspection. Reuse it; don't build a second one.
- The dockview layout already supports opt-in panels and a condensed width mode, so moving blocks between homes does not need new layout machinery.
- The LIVE / POLL / OFFLINE freshness pill is honest and cheap.
- The attention-budget meter's auto-collapse is a good precedent for progressive disclosure.

## Anti-spam policy for the trace

1. Transcript level shows messages, one trace summary per turn, and **interrupt chips** only for things a human must act on (approval, fabricated claim, injection, budget exceeded, scope violation, harness issue affecting the session).
2. Everything else is inside the trace, grouped by step, with consecutive identical events coalesced (`3× tool_timed_out`).
3. Default expansion follows the verbosity control: Quiet = collapsed, Normal = collapsed with failures auto-expanded, Verbose = expanded.
4. Every trace row is server-derived (registry names, ledger ids, counts); raw model text is never rendered as chrome.
5. Engine families not correlated to a turn (hopper, mesh, workflow) stay on their own surfaces.

## Canonical vocabulary

| Concept | Use | Stop using |
|---|---|---|
| Model selection for a turn | **Routing** | Intents, auto-route, Auto · Router, Cascade |
| Money spent | **Spend** (with scope: global / session / turn) | Budget burn, OR Spend, "session" for global |
| Attention minutes | **Attention** | budget |
| Cost/quality preset | **Mode**: Free, Efficient, Balanced, Genius (and Responsive if defined) | Clutch, Effic., Bal. |
| Acceptable risk | **Risk** | bare "Moderate" |
| Post-reply confidence check | **Check replies** | grounding |
| Items waiting on a human | **Needs you** | Approvals (as the umbrella term) |
| Model version | the catalog-resolved id, e.g. `claude-opus-5-5`, or the family (`claude-opus (latest)`) when unresolved | any hardcoded version string |
