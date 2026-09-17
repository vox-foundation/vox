---
title: "Chat UI & model-knob audit (2026-09-10)"
description: "Inventory of the chat composer, model-priority knobs, budget display, and mesh/online status in vox-gui, with condensation and wiring recommendations."
category: "Architecture SSOTs"
status: "current"
training_eligible: false
---

# Chat UI & Model-Knob Audit (2026-09-10)

> **Corrections (2026-09-10, second pass).** The first pass of this audit was
> written from a single survey and got five things wrong. An eight-track
> verification pass against the code corrected them. Where a section below is
> wrong, it now carries an inline **CORRECTION** block. Summary of what changed:
>
> 1. **§3 "90% already wired end-to-end" — FALSE.** The selection axes are
>    *discarded* on the primary selection path; `best_for_internal` never calls
>    the axis-aware scorer.
> 2. **§3 "Settings is a separate scope (global default)" — FALSE.** The
>    composer always sends a clutch, whose thread-local axes override beats the
>    Settings-derived base, so Settings → Routing never affects an interactive turn.
> 3. **§4 "new-model admission already covers OpenRouter dynamically" — FALSE.**
>    Cold start is a closed circuit: discovered models are `Shadowed`, routing is
>    `Confirmed`-only, and the eligibility check is called with no scoreboard.
> 4. **§5 "drop the two chat-menu copies" — mostly right, detail wrong.** The
>    burn rate and budget `source` are already dead code, not information at risk.
> 5. **§6 "a config gate, not a bug" — FALSE.** Mesh is *compiled out* of the
>    shipped daemon and GUI (`populi-transport` is not enabled on either).

Scope: a user request to (1) fix the run/enter button in the chat composer,
(2) make efficiency/intelligence/responsiveness "model knobs" fully wired and
less confusing, with a draggable priority-ranking control tied to live
benchmarks including OpenRouter-discovered models, (3) move budget display out
of the chat menu into the status bar, (4) find out why mesh/online doesn't
auto-connect, and (5) audit remaining chat-menu surfaces for consolidation or
removal. This doc is the researched map; it does not implement anything.

## 1. Run button / composer

Component: `Loquela` (`crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx`).

- Textarea starts at `rows={1}`, grows via manual JS up to 200px (360px in
  "expanded" mode) — `Loquela.tsx:222-226, 552-563`.
- Run button is a fixed inline button, `aria-label="Run (Enter)"`
  (`Loquela.tsx:645-655`) — the label is hardcoded text, not the live keybind,
  and it isn't shaped or positioned as a distinct "enter key" affordance.
- Submit binds to plain `Enter` (`Loquela.tsx:534-537`) and `Cmd/Ctrl+Enter`
  (`Loquela.tsx:529-533`); `Shift+Enter` falls through to newline by omission
  rather than explicit handling.

**Verdict:** bounded UI task — the flow exists, nothing new to wire. Fix:
flex-center the button independent of textarea height, keep it pinned
far-right, replace the static "Run (Enter)" label with the actual bound key
(read from wherever the keybind is defined — confirm single source of truth),
and restyle it as a compact enter-glyph chip. No backend involved.

## 2. Chat-menu widget inventory (the "confusing mess")

Everything below lives in the Loquela toolbar (`Loquela.tsx:663-795`) plus one
component passed in via `trailingSlot` from `App.tsx:1639-1646`:

| Widget | What it actually controls | Overlaps with |
|---|---|---|
| DriveConsole **clutch** (Free/Effic./Bal./Genius) | A 4-position preset — cheapest guess is it maps onto some subset of `AutoRoutingPriority`, but it is a **separate, parallel enum** (`ClutchId`, `driveConsole.ts:5-10`) from the routing-priority axes in Settings | Settings → Routing tab (item 3) |
| DriveConsole **risk** posture (High/Moderate/Low) | Separate `RiskId` enum, unrelated to model choice | — |
| **"Run on" tier picker** (local/mesh/cloud/auto) | Coarse execution-location hint (`LQ_TIERS`) | Model selector (item 6), mesh status (item 5) |
| **ChatModelPicker** | Pins a *concrete* model id or "auto-route" for this chat, via `model_override` | Tier picker, routing priority — three different "which model" controls |
| Skill picker | Unrelated (skill routing, not model routing) | — |
| Send-mode picker (Quick chat / Background task) | Unrelated (`executionMode`) | — |
| Cost estimate + session budget | Duplicates status-bar `budget_burn` tile | Status bar (item 4) |
| Structured-intent toggle, GroundingCheckToggle | Unrelated feature toggles | — |
| `MiniSlider` (`Loquela.tsx:84-97`) | **Dead code** — defined, never rendered | — |

**Root cause of "confusing mess":** there are **three independent
model-related controls** stacked in the same toolbar (tier picker, model
picker, DriveConsole clutch) plus a **fourth** living in Settings (routing
priority sliders) that none of the other three visibly reflect or feed into.
A user has no way to tell, at a glance, which of these four things is
authoritative for "what model runs my next message."

## 3. Model knobs — what's already wired vs. what you're asking for

The efficiency/intelligence/responsiveness sliders you described **already
exist**, but only in Settings, not the chat menu:
`SettingsView.tsx:1467-1529` (section `routing`) — preset buttons
(Balanced/Intelligence/Efficiency/Responsiveness mapping to
`precision`/`efficiency`/`latency`) plus an "Advanced (all 6 axes)" expansion
for the full `AutoRoutingPriority` struct (`efficiency, precision, latency,
availability, balance, mobile` — `vox-config/src/routing_policy.rs:44-64`).
Persisted via `voxTransport.setRoutingPriority` → `VOX_AUTO_ROUTING_PRIORITY`,
and this **is** the real input to the scorer
(`vox-orchestrator/src/models/scoring.rs`, `install_base_routing_priority`).

> **CORRECTION.** The claim below that this is "90% already wired" is wrong,
> and it was the most consequential error in this audit. Verified against the
> code: `select_via_scorer` (`models/select.rs:801`) installs the axes as a
> thread-local (`AxesOverrideGuard`, `select.rs:812`) and then calls
> `registry.best_for_with_filter` → `best_for_internal` (`registry.rs:813`),
> **which never calls `auto_score_model`** — it ranks by
> `cost_per_success_usd × (2.0 - quality_score)` with success-rate/latency
> tiebreaks (`registry.rs:879-941`). The axis-aware scorer runs only in
> `explain_selection` (`registry.rs:1007`, i.e. `vox model explain`) and as an
> `.or_else` fallback. So the axes survive only as (a) `to_cost_preference()`
> gating free models (`select.rs:390`, `registry.rs:830-836`) and (b) an
> `intelligence >= 50` premium-alias branch (`select.rs:699`). **The existing
> clutch control is therefore already largely theatre**, and any finer-grained
> replacement inherits that unless the shared ranking path is made axis-aware
> first. Both the sync and background lanes converge on `best_for_internal`, so
> that is one fix serving all lanes, not N.

So "fully wire the model knobs" is **90% already true end-to-end** — the gap
is exposure and interaction model, not plumbing:

- No drag-and-drop or click-to-reorder UI exists anywhere; Settings uses flat
  numeric/preset controls, not a ranked list.
- Nothing in the chat menu reflects or edits routing priority — a user
  changing the DriveConsole clutch has no visible relationship to the
  Settings sliders, and vice versa.
- The tier picker and model picker are two more channels that can silently
  override what routing priority would have chosen.

> **CORRECTION.** This section treats the Settings routing sliders as a
> legitimately separate scope ("global default for non-interactive paths").
> That is wrong. `setRoutingPriority` (`SettingsView.tsx:1277-1292` →
> `models.rs:278-300`) sets `VOX_AUTO_ROUTING_PRIORITY` in the GUI process and
> persists a DB pref the daemon reads **only at startup**
> (`vox_orchestrator_d.rs:168-185`). Per-turn precedence is thread-local axes >
> `BASE_AXES` > env (`scoring.rs:307-317`), and the composer *always* sends a
> clutch (default `efficiency`, `driveConsole.ts:25`), which always installs the
> thread-local. **Settings → Routing therefore never affects an interactive chat
> turn at all**, and needs a daemon restart to affect anything else. One knob,
> silent winner — not two scopes.

**Recommendation:** don't invent a fifth control. Replace the DriveConsole
clutch (a parallel, disconnected enum) with the drag/click priority-ranking
control you described, reading and writing the *existing*
`AutoRoutingPriority` axes (start with the 3 the user named — efficiency,
intelligence→`precision`, responsiveness→`latency` — plus `availability` as a
4th "mesh/reachability" rung if mesh auto-connect ships, see §5). Keep the
Settings page as the detail view for the same underlying value (advanced 6-axis
view), not a second source of truth.

## 4. Benchmark dynamics ("measured as new models are added, especially via OpenRouter")

Two different things are both called "benchmarking" here, and only one is real:

- **`quality_score`** (`scoring.rs:149-162`) is a **heuristic**, not a
  benchmark — `log10(max_context)` plus a free/paid constant. It does not
  reflect actual model capability.
- **`model_scoreboard`** (`vox-db-types/src/store_types/rows_core.rs:286-308`)
  is real, telemetry-derived, per-`(model_id, task_category, strength_tag)`
  data — success rate, p50/p99 latency, cost-per-success, TTFT/TPOT, goodput —
  built from observed `llm_attempts`, i.e. actual production/shadow traffic.
- **New-model admission already exists and is dynamic**:
  `discovery_pipeline.rs` fetches the OpenRouter catalog, classifies, and
  admits new models as `Provisional`/`Shadowed`
  (`autonomic.rs::ModelConfidence`), auto-promoting to `Confirmed` once
  scoreboard evidence clears a threshold (`autonomic::should_promote`). This
  already covers "as new models are added, especially via OpenRouter."
> **CORRECTION.** The bullet above claiming discovery/promotion "already covers
> 'as new models are added, especially via OpenRouter'" is wrong — it is a
> closed circuit. `decide()` gates on `confidence_state_for_model`
> (`select.rs:131-137, 265-271`), which calls `resolve_eligibility(m, None, 0.0)`
> — the scoreboard argument is **never supplied in production** (only in tests).
> A discovered model maps to `Shadowed` (`discovery_pipeline.rs:27-33`) and
> `eligible_for_routing()` is `Confirmed`-only (`autonomic.rs:66-68`), so it is
> excluded from selection and can never earn the evidence that would promote it,
> unless `VOX_ROUTING_ENABLE_EXPLORATION=1` (`select.rs:266-270`).
> Two further defects: the telemetry rollup's `quality_score` is
> `AVG(llm_feedback.rating)/5 COALESCE 1.0` over an empty table with callers
> hardcoding `Some(1.0)` (`ops_scientia.rs:162-167`, `chat.rs:334`,
> `infer.rs:1079`) — i.e. **flat 1.0 for every model**; and
> `refresh_model_scoreboard` (`telemetry.rs:107-120`) keys its cache by
> `model_id` alone while the table PK is
> `(model_id, task_category, strength_tag, window_days)`, so rows clobber each
> other last-write-wins. Real measured quality *does* exist, written by
> `vox model eval` / `vox model eval-corpus` (`eval.rs:196`,
> `eval_corpus.rs:304`, `quality_score = pass@1`) — it is simply not read at
> selection time.

- **Gap:** there is no external benchmark-dataset ingestion (e.g. MMLU/LMSYS
  Arena numbers) — confidence is earned purely by in-fleet trial, so a brand
  new model has *no* quality signal until it accumulates scoreboard rows. If
  the intent behind "dynamic benchmarks" was "seed new models with a known
  external eval score before they've proven themselves in production," that's
  a real gap, not a wiring problem — worth a explicit decision on whether it's
  wanted.

## 5. Budget display

Redundant, not missing. It's already live in the status bar today:
`BottomStatusBar.tsx:129-134,158-167` (`budget_burn` tile), on by default
(`useHudTiles.ts:6-14,45-54`), independently toggleable via "Configure ▾"
(`BottomStatusBar.tsx:245-279`). It's *also* duplicated in the chat menu
(Loquela composer row `Loquela.tsx:779-794` + DriveConsole
`DriveConsole.tsx:69-85`). Recommendation: keep the status-bar tile, drop the
two chat-menu copies. No new plumbing needed — this is a deletion.

> **CORRECTION (detail).** Less is at risk than implied: `burnPerMin` is never
> passed by `Loquela.tsx:692-700`, so DriveConsole's `↑$N/m` has **never
> rendered**, and `formatSessionBudget` (`slashRouter.ts:45-47`) ignores the
> `source` App bothers to pass. The only real loss is a cosmetic progress bar.
> Conversely `kpis.budgetBurn` already carries `delta` and `spark`
> (`App.tsx:677`) that nothing displays — the status-bar tile is the place to
> surface them. Note also the per-draft cost estimate this section preserves is
> usually absent in practice: `tierObj.cost` is null unless the tier picker's
> model backfill has run.

## 6. Mesh / online auto-connect

> **CORRECTION — this section's conclusion is wrong.** Mesh is not merely
> unconfigured, it is **compiled out of the shipped binaries**. The
> `populi-transport` feature is not enabled on `vox-orchestrator-d`
> (`Cargo.toml:15,17`) or `vox-gui` (`Cargo.toml:22,24`); only `vox-cli`'s
> `mcp-server` turns it on. So `discover_populi_mesh_models`
> (`catalog.rs:515-576`) compiles to the `#[cfg(not(...))]` arm returning
> `Ok(vec![])`, and **no `ProviderType::PopuliMesh` model can ever enter the
> registry** — "Run on → Mesh" is empty by construction. Three further defects:
> `publish_local_registry_best_effort` no-ops unless `VOX_MESH_ENABLED=1`
> (`vox-populi/src/lib.rs:17-25, 327-334`) and its only MCP caller sits behind
> both that feature and `run_stdio_server_blocking`, which the daemon never
> takes — so the local node never self-registers and `node_count` is
> permanently 0; `ServerState::spawn_populi_federation_poller`
> (`server_state.rs:619-621`) is an **empty stub body**; and
> `populi_http_join_best_effort` (`http_lifecycle.rs:132-196`) spawns its
> heartbeat loop only inside the `Ok` arm, so a control plane that is down at
> startup is never retried. Also `VOX_MESH_CONTROL_ADDR` does **not** set
> `populi_control_url` (only `VOX_ORCHESTRATOR_MESH_CONTROL_URL` does,
> `impl_env.rs:240-248`), and `populi_inference_base_url` is read by nothing.
> Most of this is plain plumbing to fix; only defaulting `VOX_MESH_ENABLED=1`,
> auto-dialing with no configured URL, or enabling discovery-publish are
> genuine policy decisions.

**Root cause: a config gate, not a bug or a missing feature.**
`populi_control_url` defaults to `None`
(`vox-orchestrator/src/config/impl_default.rs:74`) and is only ever set via
explicit env/config
(`VOX_ORCHESTRATOR_MESH_CONTROL_URL`/`VOX_MESH_CONTROL_ADDR`). With nothing
configured, `vox_mesh_nodes` (`populi_tools.rs:113-140`) falls back to the
on-disk `LocalRegistry`, which is empty on a fresh install — so "mesh" looks
broken when really nothing ever told it where to dial. Separately,
`vox-populi`'s discovery-*publish* path is opt-in by explicit design
(`discovery_publish.rs:6-9`, feature-gated,
"never broadcasts without explicit operator action") — that's a deliberate
privacy default, not something to silently flip on.

There is genuinely **no code path today that attempts to reach a mesh/control
plane automatically on launch** — the GUI only polls whatever
`populi_control_url` already resolves to. Making "online" work "as it should
be automatically brought online whenever access is loaded" requires a
decision, not a bug fix:
1. Ship a default public/first-party control-plane URL to dial on launch
   (changes the privacy posture — needs explicit sign-off), or
2. Auto-retry/auto-reconnect *only* when a control-plane URL is already
   configured (fixes flakiness without changing the opt-in default), or
3. Leave mesh manual, but fix the status-bar `mesh_peers` tile / tier picker
   to clearly distinguish "not configured" from "configured but unreachable"
   (today both likely read as "0 online").

This needs your call before it becomes a spec — see open question below.

## 7. Model selector consolidation

`ChatModelPicker.tsx` pins a concrete model id or "auto-route" via
`model_override`, explicitly on a **separate channel** from both the tier
picker (item 2) and Settings routing priority (item 3) — three ways to
influence model choice with no visible precedence order between them. This is
the single biggest source of "which knob actually wins" confusion and should
be resolved by the same redesign that replaces the DriveConsole clutch: one
visible priority-ranking control (§3) as the default path, tier picker
collapsed into an "auto" state of it, and the concrete-model override kept as
an explicit escape hatch clearly labeled as overriding everything else.

## 8. Dead code

`MiniSlider` (`Loquela.tsx:84-97`) is defined and never rendered — safe to
delete regardless of what else ships.

## Proposed decomposition (for greenlighting, not yet spec'd)

1. **Bounded, ready now:** run-button redesign (§1) — no dependencies, small
   diff, can go straight to a short in-chat design + implementation.
2. **Bounded, ready now:** delete duplicate budget UI from the chat menu (§5)
   and `MiniSlider` dead code (§8) — pure deletion.
3. **Architectural, needs a spec:** replace DriveConsole clutch + tier picker
   with one drag/click priority-ranking control wired to `AutoRoutingPriority`,
   with ChatModelPicker demoted to an explicit override (§3, §7).
4. **Needs your decision before scoping:** mesh/online auto-connect posture
   (§6, three options above) — this has a privacy/default-behavior dimension
   only you can settle.
5. **Optional, separate from all of the above:** external benchmark-dataset
   seeding for brand-new models (§4 gap) — only worth scoping if in-fleet-only
   confidence scoring isn't sufficient for your purposes.
