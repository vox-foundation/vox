---
title: "GUI observability SSOT: categories, surfaces and rules"
description: "How the Vox desktop GUI categorises information and errors, which surface each category belongs on, and the rules that keep the chat free of log spam while nothing is lost."
category: "Architecture SSOTs"
status: "roadmap"
sort_order: 40
---

# GUI observability SSOT

The chat is for the conversation. Everything else the engine knows — routing, spend, health, failures, locks,
receipts — has **one category, one home surface and one owner** so it is findable when needed and silent when not.
This page is the rule set; the plans listed under [Implementation](#implementation) make it true.

It extends the one-home-per-fact rule and anti-spam policy in
[`chat-surface-design-critique-2026-09-28.md`](chat-surface-design-critique-2026-09-28.md) from the chat surface to the
whole shell, using the inventory measured on 2026-09-29 (see [Current state](#current-state-2026-09-29)).

## Categories

Every message the GUI shows is a **notice** with four properties. The producer sets them; no surface re-derives them
from strings.

| Property | Values | Meaning |
|---|---|---|
| `severity` | `success` · `info` · `warning` · `error` (engine events also `debug`) | How much it matters. `debug` is never shown outside the Activity log and Verbose trace. |
| `scope` | `action` · `turn` · `session` · `engine` · `app` | What it is about: the user's own click, one chat turn, the open chat session, the engine as a whole, or the app shell itself. |
| `needs` | `none` · `review` · `decision` | Whether a human must look (`review`) or choose (`decision`). |
| `source` | producer id (`engine`, a toast's `cause` such as `backend-error`; later `routing`, `budget`, `harness`, …) | Who said it; used for grouping and filtering. |

Engine events get their severity from one exhaustive Rust function (`AgentEventKind::severity`), so a new event kind
cannot ship unclassified.

## Surfaces and routing

| Surface | Holds | Lifetime | Gets |
|---|---|---|---|
| **Chat transcript** | Conversation, one collapsed trace row per turn, and inline interrupt chips | Session | `turn` notices as trace steps; `needs: decision` for this session as an interrupt chip. Nothing else. |
| **Chat rail** | This session: its tasks, locks, next-turn routing, context meter | Session | `session` notices |
| **Status bar cards** | Global engine facts (Engine, Spend, Mesh, Routing, Needs you) and the notification bell | Live | A card shows a status dot when its fact is degraded or has a `warning`/`error`; the bell shows unread `warning`/`error` count |
| **Notification center** (bell → drawer) | Every notice except `debug`, newest first, coalesced (`×N`), filterable by severity (by source once there are per-fact sources), mark-as-read | App session (bounded) | All `action`, `engine` and `app` notices; a copy of every toast |
| **Toast** | Immediate feedback for the user's own action | 5 s | Only `scope: action`. Also recorded in the center, so nothing is lost when it expires. |
| **Banner** | App-level conditions that block work (backend unreachable, version mismatch) | Until resolved | `scope: app` with `severity ≥ warning` |
| **Needs-you inbox** | Everything awaiting a human | Until resolved | `needs: decision` / `review` |
| **Activity log** | The durable engine history, including `debug` | Durable | Loggable engine events (Rust `activity::is_loggable`) |
| **Models surface → Routing panel** | How task dispatch chooses a model, and routing health | Live | Routing explanations; routing-health violations also go to the center and the Routing card |
| **Diagnostics** (`vox doctor` checks) | Environment and routing health | On demand | Not yet in the GUI (see Deferred) |

Routing, in one line per scope:

- `action` → toast + center.
- `turn` → trace step (auto-expanded on failure) → interrupt chip only if `needs: decision`.
- `session` → rail.
- `engine` → center + the matching status card's dot; durable kinds also go to the Activity log. Never a toast,
  never the chat.
- `app` → banner (while blocking) + center.
- `needs: decision` (any scope) → Needs-you count, plus an inline chip where the user already is.

## Rules

1. **Chat text is conversation only.** Engine chatter never becomes chat text; it is a trace step, a chip, or lives on
   its own surface.
2. **Toasts are for the user's own actions**, and every toast is also recorded in the notification center.
3. **Never look healthy when data failed.** A source that failed shows a degraded state (a dash plus the reason) —
   never 0, an empty list, or "Online".
4. **One poll per fact.** A fact is fetched by one hook and shared; no surface polls the same data again.
5. **Coalesce repeats** (`×N`) on every surface that lists notices.
6. **Severity comes from the producer**, not from string matching in TypeScript.
7. **Status colour only through tokens** (`--color-status-pass|fail|warn|info`).
8. **Every surface change ships a Playwright spec with screenshots** (AGENTS.md, GUI visual verification).
9. **An explanation comes from the decision it explains**, never from a parallel re-implementation (routing plan,
   design rule 1).

## Current state (2026-09-29)

Measured by reading the code at `f50e9fa26`:

- Toasts have tones `ok | warn | info` only (`types/tauri.ts`); every error is `warn`. They expire after 5 s and are
  kept nowhere else.
- Engine events and activity rows carry **no severity**; about 70 of the 88 event kinds reach only the dashboard
  stream or nothing.
- About 49 `.catch(() => …)` sites swallow errors. Some make a surface look healthy: failed approvals zero the Review
  badge (`useAttentionInbox.ts`), and the research popover says "Online" on failure (`StatusBarCluster.tsx`).
- Duplicated polling: harness issues ×3, pending approvals ×3. Global KPIs are duplicated between the status bar and
  the rail.
- The GUI listens for `vox://activity-appended` but nothing emits it, so the Activity view re-queries on every engine
  event, including streamed tokens.
- The chat status line always reads "Working", because `mapAgentEvent` drops `phase` from the metadata the timeline
  reads.
- `pushToast` is untyped (`(t: any)`, once `(t: unknown)`) in 12 files, which bypasses the required `cause`.
- None of the trace, surfaces or visual-language plans' files exist yet.

## Implementation

| Part | Plan |
|---|---|
| Turn trace, interrupts, verbosity | [`2026-09-28-chat-turn-trace.md`](../../superpowers/plans/2026-09-28-chat-turn-trace.md) |
| Status bar cards, rail scope, composer | [`2026-09-28-chat-surfaces-consolidation.md`](../../superpowers/plans/2026-09-28-chat-surfaces-consolidation.md) |
| Vocabulary, status tokens, orphan removal | [`2026-09-28-chat-visual-language.md`](../../superpowers/plans/2026-09-28-chat-visual-language.md) |
| Routing explainer and routing health | [`2026-09-29-model-routing-self-maintaining.md`](../../superpowers/plans/2026-09-29-model-routing-self-maintaining.md) |
| Notices, severity, notification center, degraded states, one poll per fact | [`2026-09-29-gui-observability-notices.md`](../../superpowers/plans/2026-09-29-gui-observability-notices.md) |

## Deferred

- **Diagnostics in the GUI**: `vox doctor`'s checks (including routing health) as an Engine-card popover section.
- **OS notifications** for `needs: decision` while the window is unfocused (needs `tauri-plugin-notification`, a new
  dependency).
- **Severity on durable activity rows** (a schema change); rows are classified by kind at read time until then.
- **One poll per fact** (rule 4) as its own plan, after the trace and surfaces plans land.
- **Source filter and status-card dots**, once notices carry per-fact sources (`routing`, `budget`, `mesh`).
- **App banners recorded in the center**, and the research popover's hard-coded "Online" (rule 3).
