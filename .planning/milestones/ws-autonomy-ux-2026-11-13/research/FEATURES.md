# Feature Landscape — v1.2 Unified Autonomy Control

**Domain:** Oversight / interruption / verification control for a multi-agent desktop orchestrator (Tauri 2 + React GUI)
**Researched:** 2026-10-08
**Overall confidence:** HIGH for agent-tool patterns (official docs fetched), MEDIUM for human-factors literature (classic, well-cited; some recalled rather than re-fetched — marked).
**Input:** `.planning/PROJECT.md`, `research/AUDIT.md`, current composer screenshot (`review-bundle/latest/chat--composer-filled--wide--chromium.png`).

---

## 1. How the field expresses autonomy (evidence)

| Product / framework | Shape of control | Stops / names | Notable mechanics | Source | Conf. |
|---|---|---|---|---|---|
| **Claude Code** | Named modes, cycled with **Shift+Tab**; mode indicator in IDE/desktop | Manual (`default`) · acceptEdits · plan · auto · dontAsk · bypassPermissions | Docs table is literally "**What runs without asking**" + "Best for". Allow/ask/deny **rules layer on top of modes; deny wins in every mode**. Explicit list of "**Actions no mode auto-approves**" (critical-path `rm`, ask-rules, user-interaction tools). `auto` uses a classifier as reviewer. | https://code.claude.com/docs/en/permission-modes | HIGH |
| **Cursor** | Named Run Modes (settings) + allowlist + plain-English policy | Auto-review · Allowlist · Run Everything | Table "What runs without asking / Sandbox / Classifier / Use it when". `permissions.json` with `allow_instructions` / `block_instructions` in **plain English**, at user (`~/.cursor`) and project scope, team config overrides. Classifier explicitly "not a security boundary". | https://cursor.com/docs/agent/run-modes, https://cursor.com/docs/agent/security | HIGH |
| **OpenAI Codex** | Two orthogonal layers: **sandbox** (what it *can* do) × **approval policy** (when it *must ask*); `/permissions` to switch | Presets: Read-only · Auto (`workspace-write` + `on-request`) · Full access. (Original CLI named them Suggest · Auto Edit · Full Auto.) | Destructive connector/MCP calls **always require approval** when the tool advertises a destructive annotation, regardless of preset. Network off by default; domain allowlist, deny beats allow. | https://developers.openai.com/codex/agent-approvals-security ; Suggest/Auto Edit/Full Auto naming recalled from 2025 CLI README | HIGH / MEDIUM (old names) |
| **Windsurf Cascade / Devin Desktop** | 4 named auto-execution levels in a settings panel | Disabled · Allowlist Only · Auto · Turbo | Approve-time affordance on the command card: **Allow exact / Allow prefix / Deny** → teaches the list in context. **Admin max level** caps user choice. Deny list beats allow list. | https://docs.devin.ai/desktop/terminal | HIGH |
| **GitHub Copilot (VS Code agent)** | Per-tool confirmation with scoped persistence | Allow (once) · Allow ▾ for **session / workspace / always** · Skip | Persistence scope chosen at the moment of approval, not in a settings page. | https://code.visualstudio.com/docs/copilot/chat/chat-tools | HIGH |
| **Replit Agent** | Named autonomy level in chat input, **separate from cost Mode and Effort** | Autonomy: Low (task list only) · Medium (+code review) · High (full app review) · Max (autonomous planning, 200+ min). Modes: Free · Power · Max; Effort Low…Max | Higher autonomy is coupled to *more self-verification* (code review, app testing) — verification and autonomy move together. Cost axis kept separate — same split Vox wants. | https://docs.repl.it/updates/2025/09/19/changelog, https://replit.com/blog/introducing-agent-3-our-most-autonomous-agent-yet, https://docs.replit.com/features/agent/agent-modes | HIGH |
| **Feng, McDonald, Zhang — Levels of Autonomy for AI Agents (2025)** | 5 levels named by **user role** | Operator · Collaborator · Consultant · Approver · Observer | Argues autonomy is a design choice **independent of capability** (≙ Oversight independent of Mode). L4 open problem: "**prevent user disengagement and meaningless approvals**". L5 must-have: emergency off switch. | https://arxiv.org/abs/2506.12469 | HIGH |
| **SAE J3016 (driving L0–L5)** | Numbered levels defined by **who monitors / who is fallback** | L0 no automation … L5 full | Lesson: numbers are opaque to users; marketing names ("Autopilot") cause over-trust. Define levels by the human's duty, not the machine's skill. | https://www.sae.org/standards/content/j3016_202104/ | MEDIUM (recalled) |
| **Sheridan & Verplank (1978); Parasuraman, Sheridan & Wickens (2000)** | 10-point scale from "human does all" to "computer acts and ignores human"; applied **per stage** (acquire, analyse, decide, act) | e.g. L4 "suggests one alternative", L5 "executes if human approves", L6 "allows veto before auto-exec", L7 "executes then informs" | Key: different automation levels per function — i.e. *ask-before-act* and *verify-after-reply* are separate dials collapsed into one only if presets are coherent. | https://doi.org/10.1109/3468.844354 | MEDIUM (recalled) |
| **Bainbridge, "Ironies of Automation" (1983)** | — | — | More automation leaves humans with the rare, hardest interventions and less skill to do them → approvals must carry context. | https://doi.org/10.1016/0005-1098(83)90046-8 | MEDIUM (recalled) |
| **Fitz, Kushlev et al. (2019)** RCT n=237 | Notification batching | As-usual · batched 3×/day · hourly · never | **3×/day batching** improved attention, productivity, mood, stress; **hourly did not**; **never** increased anxiety/FoMO. User control over the setting matters (paternalism backfires). | https://doi.org/10.1016/j.chb.2019.07.016 | HIGH |
| **Mark, Gudith, Klocke (2008)** | Interruption cost | — | Interrupted people finish faster but with more stress, frustration, time pressure. | https://dl.acm.org/doi/10.1145/1357054.1357072 | HIGH |
| **Adamczyk & Bailey (2004); Horvitz (1999) mixed-initiative** | Timing of interruptions | — | Interrupting **at task breakpoints** avoids most cost; interrupt only when expected value > cost of attention. | https://erichorvitz.com/chi99horvitz.pdf ; Adamczyk cited in Fitz 2019 | MEDIUM |

**Cross-cutting observations**
1. **Every leading tool uses named detents (3–6), none uses a continuous slider.** Each detent is documented as "what runs without asking". (HIGH)
2. **A hard floor that no mode overrides is universal** (Claude "no mode auto-approves", Codex destructive annotation, Devin/Windsurf deny-wins, Cursor config-file approval). Vox already has it in the backend (external/deploy ⇒ Blocked, `routing.rs:291-292`) but does not show it. (HIGH)
3. **Autonomy and cost are separate axes** in the tools that have both (Replit Autonomy vs Mode/Effort; Codex sandbox vs model). Confirms keeping Vox Mode separate. (HIGH)
4. **Scoped persistence is chosen at approval time** (VS Code once/session/workspace/always; Windsurf exact/prefix/deny). (HIGH)
5. **Nobody in the agent-tool set ships a cross-agent interruption budget, batching, or "why this reached you".** That is open territory and Vox already has the backend primitives (`AttentionBudget`, `evaluate_interruption`, trust auto-approve). (MEDIUM — absence-of-evidence across docs read)
6. Names are mostly **"how much it does alone"** (Turbo, Full Auto, Run Everything, bypass) — the *inverse* of what Vox wants (more = safer). The only human-role framing is Feng et al. (HIGH)

---

## 2. Table Stakes

Missing any of these makes the control feel incomplete or untrustworthy relative to Claude Code / Cursor / Windsurf.

| # | Feature | Why expected | Complexity | Notes for this GUI |
|---|---|---|---|---|
| T1 | **One "Oversight" control with 4 named detents, more = safer** | Every peer uses named modes; Vox's inverted "Risk: High" is the core defect (AUDIT) | Low (UI) / Med (backend mapping) | Segmented control inline in the composer strip, replacing `Risk ▾` + "Check replies" toggle. Same footprint as Mode radio. Not a slider. |
| T2 | **Per-detent consequence list ("What happens at this level")** | Claude/Cursor docs are literally tables of "what runs without asking" | Low | Popover/tooltip on hover/focus of each detent: 3 bullets — *Asks you before…*, *Checks replies by…*, *Interrupts you…*. Every bullet must be backed by wired behaviour (AUDIT shows "safety tokens" and "delegates free" are currently fiction — remove or wire). |
| T3 | **Visible hard floor ("Always asks, at every level")** | Universal across peers | Low | Locked row at bottom of the popover: *Deploys & external actions · Spending over budget · Destructive file ops*. Lock icon, not editable from the composer. Mirrors `routing.rs:291-292`. |
| T4 | **Same level governs sync chat AND background tasks** | Users assume one control = one policy; today sync chat ignores risk except for model choice | Med–High (backend) | AUDIT: `message.rs` uses risk only for model resolution; grounding/Socrates only on background completion. Requirement must say "every consequence listed in T2 applies to both paths, verified by test". |
| T5 | **Keyboard operable, WAI-ARIA radio group** | Claude's Shift+Tab precedent; accessibility baseline | Low | `role="radiogroup"`, arrow keys move between detents, Home/End jump, visible focus ring, `aria-describedby` → consequence text. Optional global shortcut to cycle (avoid Shift+Tab conflict with focus traversal in a GUI; use e.g. ⌘⇧O). Pattern: https://www.w3.org/WAI/ARIA/apg/patterns/radio/ |
| T6 | **Mode stays a separate, visually distinct axis** | Replit/Codex split cost from autonomy | Low | Mode = cost↔intelligence (Free/Efficiency/Balanced/Genius). Oversight must not reuse "Balanced". Remove the hidden coupling where Low risk overrides Mode axes to (15,15,70) (`mode.rs` `effective_axes`) — or show it explicitly. |
| T7 | **Clear persistence semantics** | VS Code / Cursor make scope explicit | Low–Med | Global default (Settings) + per-session override shown as a dot/“session” badge on the control. Today risk/clutch are ephemeral but Check-replies is per-session localStorage — inconsistent (AUDIT). Background tasks **snapshot the level at enqueue** and show it on the task card. |
| T8 | **Monotonic, non-alarming colour** | Rose on "High risk" signals danger on a legitimate choice (AUDIT) | Low | Single-hue intensity ramp (e.g. neutral → deeper accent/shield fill), no red/green traffic light. Reserve rose for the hard-floor lock and real blocked approvals. |
| T9 | **Approvals link back to the level** | Inline approvals exist (`InlineApprovals.tsx`) but are unrelated to risk | Low | Each approval card states the level that produced it. |
| T10 | **Spend shown once** | Three spend readouts waste the whitespace the milestone wants back | Low | Keep status-bar BUDGET (or composer spend), drop the other two. Not an oversight feature, but frees the space T1 needs. |

## 3. Differentiators

Not expected from peers; high value for a *multi-agent* orchestrator.

| # | Feature | Value proposition | Complexity | Notes |
|---|---|---|---|---|
| D1 | **Cross-agent interruption budget, visible** | Nine agents (screenshot) can each ping you; nobody else meters this | Med | Surface existing `AttentionBudget` as a small meter ("3 of 6 interrupts used this hour") inside the Oversight popover. Each detent sets a default budget. |
| D2 | **Batched delivery at breakpoints** | Fitz 2019: predictable batching beats live stream; *never* causes FoMO; Adamczyk/Horvitz: breakpoints minimise cost | Med–High | "Deliver: Now · At breakpoints · Batched (every N min)". Batches flush when you send a message, finish reading a reply, or the timer fires. Blocking items (hard floor) bypass the batch. Do **not** offer "never". |
| D3 | **"Why this reached you" on every interruption** | Combats approval fatigue / rubber-stamping (Feng L4 open question; Bainbridge) | Med | One line per card: *rule fired* (e.g. "external action — always asks"; "trust 0.62 < 0.85"; "Careful: asks before file writes") + *what happens if ignored*. Data exists in `classify_tier` / `effective_approval_tier`. |
| D4 | **Counterfactual preview from recent history** | Makes consequences concrete instead of adjectives | Med | Hover a detent: "At *Careful* you would have been asked 7× in the last hour (now 2×)." Computed by replaying recent decisions through `effective_approval_tier` for each level. |
| D5 | **Earned autonomy made visible** | Trust auto-approve (≥10 repeats, trust ≥ 0.85) silently demotes asks today | Low–Med | "Auto-approved 4 actions this session because you approved them before — review / revoke". Prevents the control appearing to lie. |
| D6 | **Teach-in-place exceptions** | Windsurf exact/prefix/deny and VS Code session/workspace/always on the approval card | Med | Approval card actions: *Allow once · Allow for this session · Always ask for this*. Writes to a per-project rule list; never can relax the hard floor. |
| D7 | **Per-agent override (optional, advanced)** | Lets a risky agent (e.g. deploy/infra) run stricter than the session | Med | Shown on agent rows in the Agents panel as a small badge only when it differs from session level. Defer to post-MVP. |
| D8 | **Plain-English policy lines** | Cursor `block_instructions` precedent | High (needs classifier wiring) | Defer; note as future. |

## 4. Anti-Features

| Anti-feature | Why avoid | Do instead |
|---|---|---|
| Continuous slider for oversight | Intermediate positions have no describable meaning; no peer does it | 4 named detents with listed consequences |
| The word **"Risk"** / "Acceptable risk" | Inverted mental model (High = less safe) — the AUDIT's core defect | "Oversight" (more = safer) |
| Hype names: Turbo, YOLO, Full Auto, Autopilot | Signal *capability*, invite over-trust (SAE/Autopilot lesson) | Human-duty names (§6) |
| Traffic-light colour on the oversight stops | Makes a legitimate choice look dangerous | Single-hue intensity; rose reserved for the hard floor |
| A "never interrupt me" stop | Fitz 2019: no-notification group got anxiety/FoMO, no benefit; also violates hard floor | Lowest stop still interrupts for floor items and blockers; batching handles volume |
| Listing consequences that aren't wired | "Safety tokens", "delegates to free agents" are currently fiction (AUDIT) | Requirement: each consequence string has a backend test referencing it |
| Separate toggles for "Check replies", grounding chip, risk | Three controls for one concept | Fold reply-checking into the detent definition; keep an advanced override in Settings only |
| Silent coupling of oversight to model choice | Low risk currently forces intelligence-weighted axes | Keep Mode independent, or state "Strict uses a stronger checker model" explicitly |
| Hourly batching as default | Fitz: hourly didn't help | Breakpoint-based delivery by default; timer batching opt-in with ≥ 15–30 min |

## 5. Feature Dependencies

```
Wire consequences in backend (T4) ─┬→ Consequence list (T2) → Counterfactual preview (D4)
                                   └→ "Why this reached you" (D3)
Oversight control (T1) → Persistence/session badge (T7) → Task-card level snapshot
Hard floor surfaced (T3) → Teach-in-place exceptions (D6)   [exceptions can never touch floor]
AttentionBudget surfaced (D1) → Batching/breakpoints (D2)
Trust auto-approve surfaced (D5) depends on D3 reason codes
Spend dedupe (T10) frees space for T1
```

## 6. Naming recommendation

**Control label:** `Oversight` (short, monotonic, pairs naturally with `Mode`). Composer strip reads: `Mode: Effic. | Oversight: Standard ▾`.

**Recommended 4 stops (low → high oversight):**

| Stop | One-line promise (shown under the name) | Maps to today's backend (to be wired) |
|---|---|---|
| **Relaxed** | Works on its own; asks only for blockers and always-ask actions. | risk=High; trust auto-approve on; grounding check off; batched delivery |
| **Standard** *(default)* | Asks before risky steps; spot-checks important replies. | risk=Moderate; check replies on for sync chat (non-blocking); breakpoint delivery |
| **Careful** | Asks before any change to your files or tools; checks every reply. | risk=Low; Confirm tier for writes; grounding enforce on background + sync check |
| **Strict** | Asks at every step; replies must cite sources before you see them. | Review tier (HITL `BlockedOnApproval`) for all mutating actions; grounding enforce blocking on both paths; trust auto-approve off |

Why this scheme:
- Plain adjectives read monotonically without a legend ("Relaxed < Standard < Careful < Strict"); none collides with Mode names (Free/Efficiency/Balanced/Genius) — note "Balanced" is deliberately avoided.
- Describes **your stance**, not the agent's power, avoiding the Turbo/Autopilot over-trust trap.
- 4 stops match the industry centre (Windsurf 4, Replit 4, Codex 3, Cursor 3) and give one more than the current 3 so "Check replies" folds in without a separate toggle.

**Alternatives considered**
- *Role-based (Feng et al.)*: Observe · Approve · Consult · Co-pilot — academically grounded but makes users parse roles; reverse direction is confusing. Keep as tooltip flavour only.
- *Frequency phrasing*: Rarely · When it matters · Often · Every step — very legible as the **subtitle/aria-valuetext** for "how often agents interrupt you"; too long for the segmented button. Recommend using it as the second line in the popover.
- *3 stops* (Relaxed · Standard · Strict): acceptable fallback if "Careful" can't be made behaviourally distinct from both neighbours in the backend — do not ship a stop whose consequences duplicate another.

## 7. MVP recommendation for requirements

Prioritise:
1. T1 + T2 + T3 + T8 (the visible control, its consequences, the locked floor, colour fix).
2. T4 (backend parity sync + background) — gate: no consequence string ships unwired.
3. T5, T6, T7, T10 (a11y, Mode separation, persistence, spend dedupe).
4. One differentiator: **D3 "why this reached you"** (cheap, reason data exists, directly attacks approval fatigue), then **D1 budget meter**.

Defer: D2 batching (needs delivery queue + breakpoint detection), D4 counterfactual preview, D6 teach-in-place, D7 per-agent, D8 plain-English policies.

## 8. Gaps / open questions

- Whether `Careful` vs `Strict` can be made behaviourally distinct on the **sync chat** path without adding latency the user won't accept (needs backend phase research).
- Breakpoint detection signal in the GUI (send, reply-read, idle) is undefined.
- Admin/team max level (Windsurf precedent) — irrelevant for single-user FOSS today; note for later.
- Playwright review-bundle screenshots required for each stop + popover + floor row (AGENTS.md GUI Visual Verification).
