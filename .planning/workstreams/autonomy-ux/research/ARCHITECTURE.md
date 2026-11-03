# Architecture: v1.2 Unified Autonomy Control

**Workstream:** autonomy-ux (phases start at 12)
**Researched:** 2026-10-08 (code inspection of the live tree; no external sources needed)
**Overall confidence:** HIGH for current-state facts (all cited file:line), MEDIUM for the proposed shape (design judgment).

## TL;DR

- **One resolver, daemon-side.** A new `vox-orchestrator::oversight` module turns `(OversightLevel, context)` into an `EffectivePolicy`. Both the sync chat path and the background-task path call it. The GUI never computes policy; it renders what the daemon returns.
- **The current level lives in the daemon, per session, persisted.** The GUI sets it through an authenticated daemon RPC (`orch.oversight_set`). It is deliberately **not** an MCP tool and **not** a tool-call param, so no agent can lower it. Any value an agent or a wire hint supplies can only *raise* oversight.
- **One admission point for interruptions.** Every "ask the human" emitter (dangerous-tool park, Review-tier task block, clarifications, doubts, A2A escalations) registers through one inbox in `vox-orchestrator`, built by generalizing the existing `FeedbackStore`. Hard-floor categories (external, deploy, irreversible-destructive, allowlist or policy mutation) always surface. They bypass budget, quota, and trust auto-approve at every level.
- **No new crate edges.** Everything fits on existing edges: `vox-orchestrator-mcp → vox-orchestrator`, `vox-gui → {vox-orchestrator, vox-orchestrator-mcp}`, `* → vox-foundation`, `vox-orchestrator → vox-db`.

---

## 1. Current state: three disconnected oversight systems

| # | System | Where it acts | Who sets it | Defects |
|---|---|---|---|---|
| A | `RiskPosture` (High/Moderate/Low) | Background `AgentTask` only: approval escalation `attention_fields.rs:32-40,104`; grounding/Socrates `socrates.rs:16-29,73-78`. Sync chat uses it **only for model resolution** (`message.rs:274-275`). | GUI DriveConsole (ephemeral), task hints `tasks.rs:991-995`, `vox_submit_task` params `submission.rs:181` | Inverted scale. `ModelLean::Intelligence` silently overrides Mode axes (`mode.rs:305-311`). The safety multiplier is defined (`mode.rs:262`) but never used (`orchestrator_policy.rs:273-283` TODO). |
| B | `PermissionMode` (ask/accept_edits/accept_all/plan) | Dangerous-tool HITL gate `dispatch.rs:235-250`, `permission_modes.rs:130-197` | `ApprovalsView.tsx:47-107` (localStorage), module-level TS state `transport.ts:308-325`, top-level `DispatchRequest.permission_mode` `protocol.rs:198-208` | **Sync chat drops it.** `try_run_agent_turn` passes `permission_mode: None` (`message.rs:351-353`), so every chat-turn tool call is fail-safe `Ask` whatever the GUI selected. The GUI chat client never sets it either (`chat.rs:569-570`). |
| C | "Check replies" (`grounding_check_enabled`) | **Runs in the GUI process (L5)**, post-reply, non-blocking: `vox-gui/src/commands/chat.rs:595-599` | localStorage `useGroundingCheck.ts` | Policy logic lives in the shell, not the daemon. Never flips enforcement. |

The attention and interruption machinery exists but is fragmented:

- **One global `AttentionBudget`** per `BudgetManager` (`budget/mod.rs:247`), not per session or per agent. There is no per-agent interrupt quota.
- `evaluate_interruption` (`interruption_policy.rs:132-229`) is called only from MCP-side emitters, via `attention_policy.rs:221-233`: chat Socrates (`chat_socrates_meta.rs:408`), plan review (`plan.rs:151`), task submit (`submission.rs:444`), A2A (`a2a_tools.rs:357,816`), and feedback (`feedback_tools.rs:40`). **Background gate blocks (`gates.rs:92-97` Review → `BlockedOnApproval`) and the dangerous-tool park (`dispatch.rs:235+`) never pass through admission.**
- `Defer`/`Batch` verdicts are logged as `PolicyDeferred` events and then effectively dropped (`submission.rs:459-470`, `chat_socrates_meta.rs:411-420`).
- `FeedbackStore` already has `Surface::{NeedsYou, Withheld}` and `promote_withheld` (`feedback/store.rs:118-165`, `surface_policy.rs:4-10`). But the only promotion call is an identity no-op: `promote_withheld(|item| item.surface)` (`feedback_tools.rs:252`). Withheld items are never surfaced.
- `InlineApprovals.tsx:28` and `useAgentApprovals.ts:20` poll only `vox_pending_approvals`, which lists only dangerous-tool parks (`dispatch.rs:731-734`, in-memory `pending_approvals.rs:52-150`). **Background Review-tier blocks are written only to the `hitl_approvals` DB table (`success/mod.rs:280-292,393`) and never reach that queue.**

### Demotion holes (must close before the control means anything)

1. **A hint can disable global grounding.** `resolve_grounding_socrates_enforce` uses `hint.unwrap_or(global)` (`socrates.rs:22-28`). A task submitted with `risk:"high"` therefore *turns off* `completion_grounding_enforce` even when the config sets it to `true`. `vox_submit_task` accepts an agent-composed `risk` (`submission.rs:181`), so a sub-agent can self-relax. Approval is safe only because it uses `max_strictness` (`attention_fields.rs:38`, `budget.rs:126-132`).
2. **An ungated preference tool flips safety config.** `vox_preference_set` (`dispatch.rs:1520`; catalog `tier: core`, not in `RISK_CLASSES` `permission-modes.v1.yaml:34-92`) writes `socrates_gate_enforce`, `attention_enabled`, `attention_budget_ms`, and `attention_alert_threshold` straight into live config (`handlers_preferences.rs:45-96`). Whether the chat agent loop's tool selection exposes it still needs to be checked. It is reachable through `orch.tool_call` either way.
3. **Trust relaxation is unconditional.** `trust_relax_gates` (`success/mod.rs:426-432`) bypasses Socrates enforcement (`socrates.rs:210-211`) whatever the user's posture.
4. **Split-brain config read.** The GUI resolves task policy from its *own process's* `OrchestratorConfig::snapshot()` (`vox-gui/src/commands/orchestrator.rs:615-656`), not from the daemon. Any oversight readout built that way can disagree with what the daemon enforces.

---

## 2. Recommended architecture

```
GUI (L5, TS)                     vox-gui (L5, Rust)           daemon: vox-orchestrator-mcp (L3)        vox-orchestrator (L3)
─────────────                    ──────────────────           ──────────────────────────────────       ─────────────────────
OversightControl ──set/get──▶ commands/oversight.rs ──RPC──▶ daemon_extra / orch_daemon ───────────▶ oversight::Registry ──persist──▶ vox-db user_preferences
   ▲   readout (effects[])          (forward only)              orch.oversight_{get,set,explain}        oversight::resolve() → EffectivePolicy
   │                                                                                                           │
InboxPanel ◀──orch.inbox_list── commands/oversight.rs ◀──── chat_message (sync)  ──resolve()──────────────────┤
 (replaces InlineApprovals poll)                             dispatch tool gate  ──mode from policy───────────┤
                                                             emitters → admit() ─────────────────────────────▶ attention::inbox (FeedbackStore v2)
                                                                                                               submit/attention_fields  ◀─ policy
                                                                                                               complete/gates, socrates ◀─ policy
```

### 2.1 Source of truth: three layers, no duplicates

| Concern | SSOT | Format | Rationale |
|---|---|---|---|
| **What each level means** (level → effect table + human copy) | **New** `contracts/orchestration/oversight-levels.v1.yaml`. Bump `drive-console.v1.yaml` → `v2` with the `risk:` block removed (`drive-console.v1.yaml:21-33`), keeping `clutch:` only. | Contract YAML + hand-written Rust table + parity test (the repo's existing pattern: `control/mod.rs:22-92`, `permission-modes.v1.yaml:6-11`). | Oversight is orchestration policy, not GUI chrome. Putting it under `contracts/orchestration/` next to `permission-modes.v1.yaml` and `risk-confidence-matrix.v1.yaml` keeps policy contracts together. Removing `risk` from drive-console makes Mode and Oversight separate axes at the contract level. |
| **Which level is active** | **Daemon-held** `oversight::Registry` inside `Orchestrator`: a global default plus per-session overrides. | In-memory map, write-through to Tier A. | The daemon is the only process that sees both the sync and background paths, and there is one shared orchestrator (`protocol.rs:85-89`). It cannot live per-request: background tasks outlive the request, and chat turns have no authenticated policy field today (`message.rs:351`). |
| **Persistence** | Existing `user_preferences` table (`vox-db/src/schema/domains/foundation.rs:30-36`). Keys: `oversight.default` and `oversight.session.<session_id>`. | Tier A, existing table. | No schema change and no new contract fragment. Already written through the `set_user_preference` facade (`ops_learning.rs:270`). Prune session keys on session delete. Fail-safe on rehydrate: missing or corrupt → `max(default, Standard)`, never lower. |

**Reject:** keeping risk in localStorage or ephemeral GUI state (today's state), sending risk per request as the SSOT (agent-forgeable), and a new crate (no justification; the logic is about 300 LoC of pure resolver plus about 300 LoC of inbox).

### 2.2 OversightLevel → EffectivePolicy

There are four ordinal levels; "more = safer" is the integer order. Four, not three, because "never trust-auto-approve, surface every interrupt" (Strict) is materially different from "require review" (Close).

| Field of `EffectivePolicy` | Light (0) | Standard (1, default) | Close (2) | Strict (3) | Consumer |
|---|---|---|---|---|---|
| `approval_floor: ApprovalTier` | AutoApprove | Confirm | Review | Review | `attention_fields.rs:104` (replaces `effective_approval_tier`) |
| `trust_auto_approve` (`classify_tier` graduation, `routing.rs:300-306`) | yes | yes | yes | **no** | `attention_fields.rs:99` |
| `trust_relax_gates` | yes | yes | no | no | `success/mod.rs:426-432` |
| `tool_auto_approve` → maps to `PermissionMode` | AcceptEdits | **Ask** (today's default; do not loosen silently) | Ask | Ask | `message.rs:351` and `dispatch.rs:242` |
| `grounding` (Off / Advisory / Enforce) | Advisory | Enforce | Enforce | Enforce | `socrates.rs:73-78` (bg); new sync post-reply check |
| `socrates` (Off / Shadow / Enforce) | Shadow | Shadow | Enforce | Enforce | `socrates.rs`, `chat_socrates_meta.rs` |
| `reply_verification` (sync chat) | advisory flag | advisory flag | flag + one bounded self-check | flag + self-check + reply marked unverified until checked | `message.rs` post-reply (moved from `vox-gui chat.rs:595`) |
| `admission_offset` (added to `min_utility_threshold`) | +0.10 | 0 | −0.05 | attention gating disabled → `InterruptNow` | `interruption_policy.rs:212-216` |
| `per_agent_quota` (interrupts / 15 min) | 2 | 4 | 8 | ∞ | new inbox |
| `safety_token_multiplier` | 1.0 | 1.0 | 1.25 | 1.5 | `orchestrator_policy.rs:273-283` (late phase) |
| `hard_floors` | **identical at every level** | | | | inbox admission |
| `model axes` | **none.** Delete risk's `ModelLean` coupling (`mode.rs:202-215,305-311`); Mode alone drives `SelectionAxes`. | | | | `message.rs:274-275` drops `risk` |

**Precedence (the core invariant):**
`effective = max(session_level ?? global_default, org_floor, hint_escalation, category/source policy)`.
Every non-human input goes through `max`. Only `orch.oversight_set` can lower a level. Apply the same rule *per field* inside `resolve()`: config globals such as `completion_grounding_enforce` become floors, never fallbacks. That closes demotion hole #1.

**Mid-flight changes:** a background task stamps `oversight_level_at_enqueue` in `apply_hints` (`tasks.rs:902`). At gate time (`gates.rs:76`, `socrates.rs:33`) the policy is `max(stamped, current_session)`. *Raising* the level therefore takes effect immediately on queued and in-flight work, and *lowering* never retroactively relaxes work queued under a stricter level.

**Explainability is a resolver output, not TS logic.** `resolve()` returns `EffectivePolicy { level, source: LevelSource, fields…, effects: Vec<EffectRow> }`. Each `EffectRow { id, label_key, value, consumer }` is generated from the same match arm that sets the field. A parity test asserts that every contract `effects:` row maps to a resolver field and that changing the level changes that field. The GUI can then never claim an effect the backend does not implement, which is the failure mode audited in `AUDIT.md` (for example "fewer safety tokens", "delegates to free agents").

### 2.3 Wire contract and authentication

| RPC (constants in `vox-foundation/src/protocol.rs` `orch_daemon_method`, near `ATTENTION_SNAPSHOT` `:133`) | Params → Result |
|---|---|
| `orch.oversight_get` | `{session_id?}` → `{level, source, effects[], hard_floors[], attention:{spent_ratio, per_agent:[…]}}` |
| `orch.oversight_set` | `{session_id?, level}` → same as `get`. **Daemon RPC only, never an MCP tool.** It is authenticated by the existing `auth_token` envelope (`protocol.rs:197`), mirroring how `permission_mode` is a top-level, transport-only field (`protocol.rs:198-208`). |
| `orch.inbox_list` | `{session_id?}` → `{needs_you[], digest[], withheld_count}`, each item carrying `why{}` |
| `orch.inbox_explain` | `{item_id}` → `AdmissionRecord` (rule, reason code, level, inputs, budget state) |

Deprecations:

- The `risk` field on `ChatMessageParams`, `SubmitTaskInput` (`chat_turn.rs:202,250`), and `vox_submit_task` (`submission.rs:181`) becomes escalate-only, then is removed.
- `grounding_check_enabled` (`tasks.rs:298-300,997`, `orch_daemon/mod.rs:580-585`) is folded into the level.
- `vox_preference_set` refuses the keys `socrates_gate_enforced`, `attention_*`, and `oversight.*` (`handlers_preferences.rs:45-96`). Those keys get settable only via `orch.oversight_set`.
- The ApprovalsView `PermissionMode` toggle (`ApprovalsView.tsx:47-107`) is replaced. Oversight derives the mode; `plan` stays a separate *mode*, not an oversight level.
- Each vestigial site carries a `vox-deprecated-since` marker per AGENTS.md.

### 2.4 Sync chat consumption (`vox_chat_message`)

1. `chat_message` (`message.rs:505`) resolves `EffectivePolicy` once per turn, from `session_id`, via `state.orchestrator.oversight().resolve(..)`.
2. **Thread the tool gate.** Replace `None` at `message.rs:351` with `stricter(policy.permission_mode, transport_mode)`. `transport_mode` is the request's top-level `permission_mode` (`daemon_extra.rs:185-191`), passed to `chat_message` as a dispatch-context argument, **never via `args`**. Strictness: Ask = Plan > AcceptEdits > AcceptAll.
3. **Remove `risk` from model resolution** (`message.rs:275,316`); Mode only.
4. **Move reply verification daemon-side.** Call `vox_orchestrator::grounding::assess_reply_confidence` in `chat_message` after the reply and delete the GUI-side call (`vox-gui chat.rs:591-599`). At Close and Strict, run one bounded self-check pass, capped by the safety multiplier. **Never hold the reply text hostage indefinitely.**
5. **Turn trace.** Emit a new `oversight_applied` event (level, source, gate outcomes) next to `routing_decision` (`turn_events.rs`; producer `message.rs:300-316`). This needs an additive kind in `contracts/gui/turn-event-kinds.v1.json`.

### 2.5 Background task consumption

1. `apply_hints` (`tasks.rs:902-1000`): parse `risk` as escalation only and stamp `oversight_level_at_enqueue`.
2. `populate_task_attention_fields` (`attention_fields.rs:44-114`): `tier = classify_tier(..)`. If `!policy.trust_auto_approve`, an AutoApprove that came from trust graduation (not from read-only) becomes Confirm. Then `tier.max_strictness(policy.approval_floor)`. `classify_tier`'s external → Blocked (`routing.rs:291-292`) stays first and absolute. Replace keyword sniffing (`attention_fields.rs:11-23`) with hard-floor classification in later hardening; for now keep it as a floor *input*.
3. `check_approval_gate` (`gates.rs:76-97`) is unchanged semantically, but Review → `BlockedOnApproval` must also `admit()` into the inbox (§2.6) so the GUI sees it.
4. `check_socrates_gate` (`socrates.rs:16-29,73-78,210-211`): enforce flags = `max(config, policy)`; `trust_relax_gates &= policy.trust_relax_gates`.
5. `orchestrator_policy.rs:273-283`: wire `safety_token_multiplier` and clutch `budget_gate` aggressiveness once `PolicyContext` carries the task policy. The code comment flags this as high blast radius; it goes in the last phase.

### 2.6 Interruption decision pipeline (cross-agent)

Generalize `FeedbackStore` (`feedback/store.rs`, `feedback/types.rs:9-61`) into the **Attention Inbox**, owned by `vox-orchestrator` so background gates (no `-mcp` dep) and MCP emitters (existing `-mcp → orchestrator` edge) share it.

Add `FeedbackKind::{ToolApproval, TaskReview, Escalation}` alongside the existing `Clarification`, `Doubt`, and `SkillProposal` (`types.rs:9-13`). `PendingApprovals` (`pending_approvals.rs`) keeps its `oneshot` parking but mirrors each park into the inbox. The inbox rehydrates via the existing `hitl_rehydrate.rs` path.

`Orchestrator::admit(candidate, &EffectivePolicy) -> AdmissionVerdict` runs a fixed, ordered pipeline. Every step writes a reason code into an `AdmissionRecord`:

1. **Hard floor.** Category ∈ {External, Deploy, IrreversibleDestructive, AllowlistMutation, OversightOrPolicyChange} → `NeedsYou` and `RequireHumanBeforeContinue`. This skips steps 2-5, ignores level and trust, is never digested, and is never auto-approved. Sources: `routing.rs:291`, `permission-modes.v1.yaml` `always_requires_approval` and `class: destructive, reversible: false` (`vox_run_shell`, `vox_delete_file`, `vox_deploy`), and `vox_add_approval_allowlist_entry`.
2. **Blocking work items** (ToolApproval, TaskReview): always `NeedsYou`. They are already blocking an agent, and deferring them only deadlocks. Order them by `priority × attention_weight` (`attention_fields.rs:113`). Debit attention.
3. **Non-blocking items** (Clarification, Doubt, Escalation, SkillProposal): call `evaluate_interruption` (`interruption_policy.rs:132`) with calibrated signals (`attention_policy.rs:221-233`) and `policy.admission_offset`. Strict forces `attention_enabled = false` semantics, which means `InterruptNow` (`interruption_policy.rs:138-143`).
4. **Per-agent quota.** Add a token bucket keyed by `AgentId`, next to the global budget in `BudgetManager` (`budget/mod.rs:247-249`). An item over quota is `Withheld`, even after `InterruptNow`.
5. **Digest and deferral.** `Defer`, `Batch`, and over-quota verdicts → `Withheld`. Flush at checkpoints (task completion, the session's next user turn, a 10-minute timer, or the budget dropping below the alert threshold) into **one** digest item per session. Fix `promote_withheld` by passing the real re-evaluation closure (`feedback_tools.rs:252`).
6. **Telemetry.** Record `AttentionEvent` with the verdict so `attention/calibrator.rs` keeps learning.

What reaches the user: `NeedsYou` items appear in the inbox, and toasts fire only for hard floors and blocking items. Withheld items show as a count plus the digest. Each item exposes `why = AdmissionRecord`, rendered as "Reached you because: hard floor (deploy)", "…because: blocking agent 3 (Review tier at Close)", or "Held for digest: agent quota 4/15 min".

### 2.7 GUI: render, don't compute

- Add new Tauri commands `crates/vox-gui/src/commands/oversight.rs` that **forward only** to the daemon RPCs. Remove the in-process `OrchestratorConfig::snapshot()` policy reads (`orchestrator.rs:615-656`) or route them through the daemon too.
- The TS mirror reads only static labels and order from `oversight-levels.v1.yaml`, the same pattern as `driveConsole.ts:1`. Effects, "why", and current state come from `orch.oversight_get` and `orch.inbox_*`. No effect strings are hard-coded in TSX (unlike today's `RiskPopover.tsx`).
- One control replaces the RiskPopover, GroundingCheckToggle, and the ApprovalsView PermissionMode toggle. InboxPanel replaces the `InlineApprovals` / `useAgentApprovals` poll of `vox_pending_approvals`.
- GUI parity gate: extend the `vox-gui drive/protocol.rs:336+` test to the new contract. Playwright stepper and review-bundle screenshots are required (AGENTS.md GUI Visual Verification).

---

## 3. Component boundaries and crate ownership

| Component | Crate (layer per `docs/src/architecture/layers.toml`) | New edge? |
|---|---|---|
| Contract `oversight-levels.v1.yaml`; `drive-console.v2.yaml` | `contracts/` (parity tests in consuming crates) | — |
| `oversight::{OversightLevel, EffectivePolicy, EffectRow, resolve}` (pure) | `vox-orchestrator` (L3, `:210`), **new module `src/oversight/`**, not `mode.rs` (already 920 lines) | no |
| `oversight::Registry` + persistence | `vox-orchestrator` → `vox-db` `user_preferences` | no (orchestrator already uses `vox_db`, `budget/mod.rs:244`) |
| Attention Inbox (`FeedbackStore` v2), `admit()`, per-agent quota | `vox-orchestrator` `feedback/` + `attention/` + `budget/` | no |
| RPC method constants | `vox-foundation` (L1, `:145`) `protocol.rs` | no |
| RPC handlers | `vox-orchestrator` `orch_daemon/mod.rs` (`get`/`set`) and `vox-orchestrator-mcp` `daemon_extra.rs` (inbox, needs `ServerState`) | no |
| Sync chat consumer, dispatch mode threading, `PermissionMode` mapping | `vox-orchestrator-mcp` (L3, `:234`): `message.rs`, `dispatch.rs`, `permission_modes.rs` | no |
| Tauri forwarders | `vox-gui` (L5, `:271`) `commands/oversight.rs` | no (edges to orchestrator and -mcp exist; `chat.rs:585` already uses `vox_orchestrator_mcp`) |
| UI | `crates/vox-gui/ui/src/components/surfaces/Loquela/*`, new `lib/oversight.ts` | — |

`PermissionMode` stays in `-mcp`. The resolver emits an orchestrator-local `ToolAutoApprove` enum, and `-mcp` maps it, so `vox-orchestrator` never needs an `-mcp` type and no upward edge appears. Do **not** move these types to `vox-orchestrator-types` (L0): nothing below L3 consumes them, and moving them adds churn without removing an edge.

Budget watch: `vox-orchestrator` `max_loc = 70_000` and `vox-gui` `max_loc = 25_000`. Run `cargo run -p vox-arch-check` after phase 12 and phase 16.

---

## 4. Build order (explicit)

| Phase | Lands | Depends on | Behavior change |
|---|---|---|---|
| **12 — Contract and resolver** | `oversight-levels.v1.yaml`; `oversight` module with `resolve()` and `EffectRow`; parity test; `drive-console.v2` (risk removed); delete `ModelLean` coupling (`mode.rs:305-311`, `message.rs:275`) | — | Only Low risk stops forcing intelligence axes. |
| **13 — Daemon state, auth, demotion holes** | `Registry` + `user_preferences` persistence; `orch.oversight_{get,set}`; escalate-only hint parsing (`tasks.rs:991-995`, `submission.rs:181`); `max` semantics in `socrates.rs:22-28`; `vox_preference_set` key refusal; GUI reads via daemon, not `snapshot()` | 12 | Closes self-relax paths. **Security-relevant: verify by mutation** (AGENTS.md PR discipline). |
| **14 — Background consumers** | `attention_fields.rs` floor + trust-graduation gate; `trust_relax_gates` gating; `max(stamped, current)` at gate time | 13 | Level now governs background tasks. |
| **15 — Sync chat consumers** | Thread mode into `run_agent_turn` (`message.rs:351`) and context-pass from `daemon_extra.rs:190`; move reply check daemon-side; `oversight_applied` turn event | 13 | Level now governs chat tool calls and reply checks. |
| **16 — Attention Inbox** | `FeedbackStore` v2 kinds; `admit()` pipeline with hard floors; per-agent quota; digest flush (fix `feedback_tools.rs:252`); `PendingApprovals` and `BlockedOnApproval` registration; `orch.inbox_{list,explain}` | 13 (14 and 15 for the full emitter set) | Every interrupt goes through one gate; background reviews become visible. |
| **17 — GUI unification** | One oversight control; inbox panel + "why"; effects readout from RPC; remove RiskPopover, Check replies, PermissionMode toggle, and duplicated spend chrome; Playwright stepper | 13 (control), 16 (inbox) | UX. Coordinate with the uncommitted Loquela/Chat/Popover/Tooltip WIP. |
| **18 — Spend-side wiring (optional)** | `PolicyContext` carries task policy → safety multiplier and clutch budget aggressiveness (`orchestrator_policy.rs:273-283`, `budget_gate.rs:165+`) | 14 | High blast radius; isolate it. |

Phases 14 and 15 can run in parallel after 13. The phase 17 control can start against the phase 13 RPCs while phase 16 lands. Phase 12 is the only true prerequisite for everything else.

---

## 5. Pitfalls (architecture-specific)

- **Agent-forgeable lowering.** Any oversight input reachable from tool-call `args` must be escalate-only. Keep `set` off the MCP catalog. Test this by mutation: delete the `max`, assert the test fails, restore.
- **The default must not loosen.** Today chat tools always park (`Ask`). Mapping Standard → AcceptEdits would be a silent loosening; only Light gets AcceptEdits.
- **Deferring blocking items deadlocks agents.** Never run ToolApproval or TaskReview through `Defer`.
- **Global attention budget versus many agents.** One chatty agent can exhaust the global budget (`budget/mod.rs:247`) and starve the others' legitimate asks. That is why the per-agent quota sits *before* digest, not instead of the global budget.
- **GUI-process policy reads** (`orchestrator.rs:615-656`) diverge from the daemon. Do not build the readout on them.
- **The contract parity pattern is hand-mirrored** (`control/mod.rs`, `permission_modes.rs:56`). The data-storage rule prefers `vox-schema` generation. Match the existing hand-mirror plus parity-test precedent unless the roadmapper opts into schema generation.

## 6. Open questions for phase research

- Is `vox_preference_set` offered inside chat agent turns (`llm_bridge/tool_selection.rs`)? This decides whether hole #2 is exploitable today or only reachable through `orch.tool_call`.
- Is there an existing GUI owner of `hitl_approvals` rows (a dashboard widget)? `ApprovalsWidget.tsx` was not inspected.
- What is the digest checkpoint UX: in-chat bubble or inbox only? This needs the phase 17 UI spec.
- Who owns `org_floor`: `OrchestratorConfig` or `/etc/vox` like the telemetry org policy? Recommendation: the `OrchestratorConfig` field for v1.2.
