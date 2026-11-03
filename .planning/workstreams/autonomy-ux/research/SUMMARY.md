# Research Summary — v1.2 Unified Autonomy Control

**Workstream:** autonomy-ux (phases start at 12)
**Domain:** human-oversight / interruption / verification control for a multi-agent orchestrator (Tauri 2 GUI + Rust daemon)
**Researched:** 2026-10-08
**Inputs:** AUDIT.md (baseline), STACK.md, FEATURES.md, ARCHITECTURE.md, PITFALLS.md
**Confidence:** HIGH on current-state wiring (file:line cited by ≥2 researchers); MEDIUM on level count/naming and inbox scope (design judgment).

## Executive Summary

Vox already has every primitive a unified oversight control needs. They are driven by four unrelated UI controls, and only some of them are honoured on each execution path. `RiskPosture` governs background tasks only. `PermissionMode` is the real tool-approval gate, but sync chat passes `None` to it, so chat tools always park. "Check replies" is a GUI-process heuristic. The attention budget is live but only readable in the GUI. On top of that, the user-facing scale is inverted ("Risk: High" = less oversight). The field has converged on 3–4 named stops, each documented as "what runs without asking". Every peer product has a hard floor that no mode can override, and cost/model choice is a separate axis. No peer meters interruptions across agents, and that is Vox's open differentiator.

**Recommended approach:** fix safety first, then build the control. (1) Before any level can relax anything, land a typed hard floor and close every path by which an agent can lower oversight. (2) Define oversight as orchestration policy: a contract plus a pure daemon-side resolver that returns both the effective policy and its human-readable effect rows. (3) Keep the active level daemon-held, so sync chat, background tasks and the GUI all see one value. Only an authenticated, non-MCP RPC can set it, and agent hints can only raise it. (4) Wire both execution paths. (5) Route every "ask the human" through one admission point that explains itself. (6) Replace the four GUI controls with one Oversight segmented control. The GUI renders what the daemon returns.

**Key risks:**
- Today's floor is a description-substring heuristic, and `vox_deploy` is auto-approved under `accept_all`. Wiring a real "less oversight" end on top of that turns a no-op into an actual hole.
- Parity gates that stay green after an id is reused with inverted meaning.
- Tests that prove helpers but not call sites (this has happened before: the safety multiplier and delegate-free).
- Alarm fatigue at the strict end.
- Collision with uncommitted WIP in the Loquela/Chat/Popover/Tooltip files.

## Corrected Current State

Correcting AUDIT: attention is surfaced read-only (not invisible); sync chat does have a tool-approval gate (it is fed `None`); the safety-token TODO multiplies an input that is always `0.0`.

| Capability | State | Evidence |
|---|---|---|
| `RiskPosture` → approval escalation (escalate-only) for background tasks | **Wired** | `attention_fields.rs:32-41`, `budget.rs:126-132` |
| Grounding/Socrates enforce on background completion | **Wired** | `socrates.rs:16-29,73-90,188-215` |
| Trust auto-approve graduation; external ⇒ Blocked | **Wired** (floor is substring-based) | `routing.rs:290-311`; `attention_fields.rs:11-23` |
| Risk ⇒ model lean (Low forces axes 15/15/70) | **Wired, unwanted** (hidden Mode coupling) | `mode.rs:305-311`, `runtime.rs:734-744`, `message.rs:274-275` |
| `PermissionMode` gate for sync-chat tool calls | **Partially wired**: the gate exists, but chat passes `None` ⇒ always `Ask` | `dispatch.rs:235-260`, `message.rs:351-353` |
| ApprovalsView PermissionMode selector | **Partially wired**: direct MCP calls only, not chat | `ApprovalsView.tsx:47-59`, `mcp.rs:23-44` |
| "Check replies" (`assess_reply_confidence`) | **Partially wired**: sync only, advisory, runs in the GUI process | `vox-gui chat.rs:591-599` |
| `evaluate_interruption` | **Partially wired**: MCP emitters only. Background Review blocks and tool parks bypass it; Defer/Batch verdicts are dropped | `submission.rs:459-470`; `feedback_tools.rs:252` (no-op promote) |
| Background Review-tier HITL blocks | **Invisible**: written to `hitl_approvals`, never reach `vox_pending_approvals`/InlineApprovals | `success/mod.rs:280-292` |
| AttentionBudget meter | **Read-only surfaced**; settable only via MCP prefs / CLI | `ChatSurface.tsx:671-690`; `handlers_preferences.rs:60-90` |
| `safety_token_multiplier` / `safetyTokenBudget` | **Dead** (input always 0.0; FE never sends) | `orchestrator_policy.rs:51-52,273-283`; `driveConsole.ts:22-26` |
| `always_delegate_free` / `BudgetAggressiveness` | **Dead** (Mode claims, out of scope) | `mode.rs:101-102,140-168` |
| `vox ci attention-config-parity` | **Dead gate** (reads non-existent paths ⇒ silently green) | `vox-cli-ci/src/attention_parity.rs:5-12` |
| TS `driveConsole.ts` "kept in sync by BE gate" | **False claim** (no test reads YAML from TS) | `driveConsole.ts:1` |

## Reconciliation of Researcher Conflicts

**(a) Level count: 3 (STACK: light/standard/close) vs 4 (FEATURES: Relaxed/Standard/Careful/Strict; ARCHITECTURE: Light/Standard/Close/Strict).**
Recommendation: **ship 3 stops — Relaxed · Standard · Strict.** FEATURES' own rule is "do not ship a stop whose consequences duplicate another". On the sync path, the third and fourth stops would differ only by trust auto-approve being off and the attention gate being disabled (`InterruptNow`). That is exactly the "flood the user" end that PITFALLS C4 warns against. Fold the 4th stop's knobs (trust auto-approve off, budget size) into an advanced setting next to the attention panel.

Use **new ids that cannot collide with legacy ones** (PITFALLS C5): `relaxed|standard|strict`. Legacy `high|moderate|low` are accepted as inbound aliases only, mapped through an explicit inversion table (`high→relaxed`, `moderate→standard`, `low→strict`) and marked `vox-deprecated-since`. Internally each level still maps to a `RiskPosture` for background compatibility. The resolved policy row also carries the tool mode, reply check and trust gating, so the mapping is not 1:1 in effect. A 1:1 alias for *labels* is fine; reusing *ids* is not. Revisit a 4th stop only if a later phase shows it is behaviourally distinct.

**(b) Contract location: `contracts/gui/oversight.v2.yaml` (STACK) vs `contracts/orchestration/oversight-levels.v1.yaml` + `drive-console.v2.yaml` (ARCHITECTURE).**
Recommendation: **ARCHITECTURE's split.** Oversight is orchestration policy and belongs beside `permission-modes.v1.yaml`. `drive-console.v2.yaml` keeps only `clutch` (Mode), which makes the two axes separate at the contract level. Adopt STACK's and PITFALLS' content inside the new file:
- per-level `background`/`chat` blocks
- `claims:` ids, each backed by a named call-site test
- `floors:` that every level must resolve ≥ to
- `invariants.never_auto_approve`
- monotonicity (strictness non-decreasing along the declared order)
- "mode axes unchanged for every (mode, oversight) pair"

`axis-drive.v1.yaml` gets an `oversight` set-key, with `risk` kept as a one-release alias.

**(c) Safety tokens: drop (STACK) vs wire as optional phase 18 (ARCHITECTURE).**
Recommendation: **drop from oversight.** The multiplier scales `budget_token_fraction`, which no caller ever sets. Wiring it first requires inventing a spend input, which makes it a Mode/cost feature, not oversight. Remove `safetyTokenBudget`, the contract row and the popover copy. Log "verification spend budget" in the Mode backlog. This also removes a cost coupling (PITFALLS C7).

**(d) Persistence: daemon per-session `user_preferences` (ARCHITECTURE) vs `set_task_policy_override('Chat','interactive')` in Vox.toml + per-session localStorage (STACK).**
Recommendation: **daemon-held.** Use an `oversight::Registry` with a global default plus per-session overrides, written through to the existing `user_preferences` table (keys `oversight.default`, `oversight.session.<id>`; no schema change). It is set only via authenticated `orch.oversight_set`. Reasons:
- background tasks outlive requests;
- localStorage is invisible to the daemon and to the drive API;
- per-request values are agent-forgeable;
- the GUI's in-process `OrchestratorConfig::snapshot()` reads are split-brain (`vox-gui orchestrator.rs:615-656`).

Vox.toml `task_policy` category/source overrides remain, but only as **floors** combined by `max`. On rehydrate, missing or corrupt state ⇒ `max(default, standard)`.

**(e) Sync-chat approvals: "no approval path, new enforcement surface" (PITFALLS C2) vs "gate exists, fed `None`" (STACK/ARCHITECTURE).**
STACK/ARCHITECTURE are right. The approval effect is threading an authenticated `permission_mode` into `run_agent_turn` as dispatch context, never via `ChatMessageParams` args. PITFALLS' substance still holds: no requeue semantics in sync turns, and a visible bounded "waiting for you" state.

**(f) Sync reply verification: bounded LLM self-check at Close/Strict (ARCHITECTURE) vs advisory only, claim no blocking (STACK) vs stream-then-verify (PITFALLS M3).**
Recommendation for v1.2: move the heuristic check daemon-side. It is on at Standard and Strict, shown as a non-blocking badge, and named `reply_confidence_check` (distinct from `grounding_enforcement`, PITFALLS M4). Defer any LLM self-check pass. Mapping is Relaxed → AcceptEdits, Standard and Strict → Ask (today's default, so not loosened); **never** AcceptAll from the composer.

## Security-First Items (must land before any level can loosen anything)

1. **Typed hard floor.** Add a `SafetyFloor` classification from structured signals (tool name + args, file manifest, caps), not description substrings. Categories: external/network egress, deploy/publish, destructive fs/VCS (`rm -rf`, `git push --force`, `reset --hard`, `DROP`), secret/credential read-write, allowlist/policy/oversight mutation. `effective = max(floor(action), level_tier, trust_tier)`. The floor function must not take the level as a parameter. Floor items bypass budget, quota, digest and trust at every level.
2. **`vox_deploy` auto-approved under `accept_all`** (`permission_modes.rs:87-92`, YAML `:81-87`). Set `always_requires_approval: true` in both places; the YAML↔Rust parity test enforces lockstep. Decide the same for `vox_run_shell`/`vox_delete_file` (irreversible) and `vox_add_approval_allowlist_entry`.
3. **Close agent lowering paths:**
   - `socrates.rs:22-28` `hint.unwrap_or(global)` lets an agent-supplied `risk:"high"` on `vox_submit_task` turn off configured grounding. Change it to `max(config, hint)`.
   - `vox_preference_set` (ungated, `tier: core`) writes `socrates_gate_enforce` and `attention_*` live. Refuse those keys and `oversight.*`.
   - `trust_relax_gates` (`success/mod.rs:426-432`) bypasses Socrates unconditionally. Gate it by level.
   - All wire `risk` hints become escalate-only.
4. **Parity-gate integrity:**
   - Key `code_matches_contract` rows by `id` and assert the row count; today it is index-based and ignores `id`/`model_lean`.
   - Add a semantic monotonicity assertion and a test that legacy `risk=high` via the drive API maps to the *least* oversight.
   - Add the missing TS↔YAML vitest.
   - Fix the axes mislabel message.
5. **Dead `attention-config-parity` gate.** Repoint it at `config/orchestrator_fields.rs` + `vox-orchestrator-mcp/src/memory_tools/handlers_preferences.rs`, and make it fail on missing files *before* any GUI writer for attention keys exists.
6. **Verify by mutation** (AGENTS.md): for each guard, delete the call, see the test fail, restore. Use a `grep -c` presence check and record the result in the PR.

## Feature Scope (for requirements)

**Table stakes**
- One Oversight segmented control, 3 stops, more = safer. Replaces Risk ▾, Check replies and the ApprovalsView PermissionMode selector. `plan` stays a separate mode.
- A per-stop "what happens at this level" list, generated from the daemon's effect rows (not hard-coded TSX).
- A locked "always asks, at every level" floor row.
- The same level governs sync chat and background tasks, verified per claim by a call-site test.
- WAI-ARIA radiogroup: arrow keys, Home/End, visible focus, meaning not conveyed by colour alone. Built on the in-repo `Segment` + WIP Radix `Tooltip`; no new dependencies.
- Mode separate, with the risk→axes coupling deleted (Free + Strict keeps `force_free_pool`).
- Clear persistence (global default + session badge). Background tasks snapshot the level at enqueue and show it.
- Single-hue intensity colour; rose reserved for the floor and blocked items.
- Approval cards state the level and rule that produced them.
- Spend shown once (keep status-bar BUDGET; verify every surface in screenshots).
- Mid-flight changes: tightening applies now, to queued and in-flight work at the next gate; loosening applies only to new work. A toast confirms.
- Legacy state migration (inversion table; `grounding_check=on` raises the migrated level to ≥ standard).

**Differentiators (v1.2 targets, in priority order)**
- D3 "Why this reached you" on every interrupt (reason codes already exist).
- One admission point: background Review blocks become visible in the inbox.
- D1 interruption budget meter + per-agent attribution (`agent_id`/`policy_reason` on `FeedbackRow`).
- D5 "auto-approved because trusted (n repeats)", with revoke.
- A reviewable "handled without you" digest that fixes the no-op `promote_withheld`.
- Per-agent quota (fair share + aging; blocking items never deferred).

**Deferred (v1.3+)**
- Timer/breakpoint batching UX (D2; breakpoint signal undefined).
- Counterfactual preview (D4).
- Teach-in-place exceptions (D6).
- Per-agent level override (D7).
- Plain-English policies (D8).
- A 4th stop.
- LLM self-check on sync replies.
- Safety-token / verification spend budget, and delegate-free (Mode backlog).
- Anti-rubber-stamp trust weighting.
- Admin/org max level.

**Anti-features:** continuous slider; the word "Risk"; hype names (Turbo/Autopilot); traffic-light colours; a "never interrupt me" stop or a budget "off" (it actually means *interrupt everything*, `interruption_policy.rs:138-143`); copy for unwired effects; hourly batching default.

## Suggested Phase Order

| Phase | Name | Delivers | Depends on |
|---|---|---|---|
| **12** | Safety floor & lowering-path closure | Security items 1–3, 5. No UI change. Mutation-verified. | — |
| **13** | Oversight contract & resolver | `contracts/orchestration/oversight-levels.v1.yaml`, `drive-console.v2.yaml`, axis-drive `oversight` key + `risk` alias. New `vox-orchestrator::oversight` module (`OversightLevel`, `EffectivePolicy`, `EffectRow`). Id-keyed/monotonic gates + TS vitest (item 4). Delete ModelLean coupling. Drop safety-token/delegate-free claims. | 12 |
| **14** | Daemon level state & RPC | `oversight::Registry` + `user_preferences` persistence; `orch.oversight_{get,set}` (transport-authenticated, not MCP); GUI reads policy via the daemon, not `snapshot()`. | 13 |
| **15** | Background-task consumers | Floor + level + trust composition in `attention_fields.rs`; `trust_relax_gates` gated; `oversight_level_at_enqueue` stamp; `max(stamped, current)` at gate time; chat-spawned tasks inherit the turn level as a hint. | 14 |
| **16** | Sync-chat consumers | Authenticated `permission_mode` threaded into `run_agent_turn`; reply check moved daemon-side + renamed; `oversight_applied` turn event; inline "waiting for you" with bounded timeout. | 14 (parallel with 15) |
| **17** | Attention inbox & admission | `FeedbackStore` v2 kinds (ToolApproval, TaskReview, Escalation); `admit()` pipeline (floor → blocking → `evaluate_interruption` → per-agent quota → digest); `orch.inbox_{list,explain}` with `AdmissionRecord`; attention-config GUI commands. | 15, 16 |
| **18** | GUI unification | **First task: resolve WIP ownership.** Then: Oversight control, consequence popover + floor row, inbox panel + "why", budget panel, legacy migration, spend dedupe, removal of RiskPopover/GroundingCheckToggle/PermissionMode toggle/InlineApprovals poll; extended `tauriMock` + Playwright stepper + review bundle. | 14 (control), 17 (inbox) |
| **19** | Verification & claims ledger | Claims-ledger test (every contract claim id ↔ an existing call-site test); `cargo mutants` scoped to the gate files; latency/cost check per (mode × oversight); close-out audit. | 15–18 |

**Ordering rationale:**
- Phase 12 comes first because wiring a real low end is what turns today's weak floor into an exploitable one (PITFALLS C1).
- The contract precedes the state store, and the state store precedes the consumers, so no consumer reads ad-hoc values.
- 15 and 16 run in parallel.
- The GUI control can start against the phase 14 RPCs; only the inbox panel waits on 17.
- Write tests first in every phase; phase 19 only adds the cross-cutting ledger and mutation sweep.
- Run `vox-arch-check` after 13 and 17 (`vox-orchestrator` max_loc budget). No new crate edges are needed.

**Research flags:**
- Phase 12 needs research: structured floor classification sources and a `vox_preference_set` exposure check (`llm_bridge/tool_selection.rs`).
- Phase 17 needs research: quota/aging design, digest checkpoints, lock release for blocked tasks (ADR-025).
- Phase 18 needs a UI-SPEC.
- Phases 13, 14 and 16 follow standard in-repo patterns (hand-mirrored contract + parity test, existing RPC/auth envelope).

## Confidence

| Area | Confidence | Notes |
|---|---|---|
| Stack / wiring facts | HIGH | Cross-confirmed file:line by STACK, ARCHITECTURE and PITFALLS |
| Features | HIGH (peer tools) / MEDIUM (human factors) | Official docs fetched; some literature recalled |
| Architecture | MEDIUM | The resolver/registry/inbox shape is design judgment; LoC estimates rough |
| Pitfalls | HIGH (repo) / MEDIUM (literature) | |

**Gaps:**
- Whether `vox_preference_set` is offered to chat agents (decides whether hole #2 is exploitable today).
- Whether a GUI owner exists for `hitl_approvals` rows (`ApprovalsWidget.tsx` uninspected).
- The breakpoint/digest UX.
- `org_floor` ownership (recommend an `OrchestratorConfig` field).
- Whether data-storage policy prefers `vox-schema` generation over a hand-mirrored contract.

## Open Questions (user decisions)

1. **Level count & names:** accept 3 stops Relaxed · Standard · Strict (recommended), or 4 with Careful/Close?
2. **Relaxed loosens sync chat:** OK for Relaxed to auto-approve file edits (`accept_edits`) in chat? Today every chat edit parks.
3. **Irreversible tools on the floor:** should `vox_run_shell` and `vox_delete_file` join `vox_deploy` as always-requires-approval, or stay mode-dependent?
4. **Persistence scope:** daemon per-session + global default (recommended), and is "make default" per-workspace (Vox.toml) or per-user?
5. **Retire the ApprovalsView PermissionMode selector:** keep `accept_all` reachable only from an advanced setting, or remove it?
6. **v1.2 inbox scope:** include per-agent quota + digest, or ship only admission + "why" + attribution and defer quota/digest?
7. **WIP ownership:** who owns the uncommitted Loquela/Chat/Popover/Tooltip edits, and should they land before phase 18?
8. **Drive API `risk` alias lifetime:** how long must scripted `vox gui drive set risk=…` keep working (one release recommended)?

## Sources

STACK.md, FEATURES.md, ARCHITECTURE.md, PITFALLS.md, AUDIT.md (this directory). Peer docs: Claude Code permission modes, Cursor run modes, OpenAI Codex approvals, Devin/Windsurf terminal, VS Code chat tools, Replit Agent; Feng et al. 2025 (arXiv 2506.12469); Fitz et al. 2019; Parasuraman & Riley 1997; Bainbridge 1983; WAI-ARIA APG radio/slider; WCAG 2.2.

---
*Research completed: 2026-10-08 · Ready for roadmap: yes (pending open questions 1–2, which change phase 13 contract content)*
