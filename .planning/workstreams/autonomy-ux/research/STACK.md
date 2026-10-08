# Technology Stack / Control Mechanics — v1.2 Unified Autonomy Control

**Workstream:** autonomy-ux (phases from 12)
**Researched:** 2026-10-08 (codebase-only; no external sources needed — every primitive already exists in-tree)
**Overall confidence:** HIGH for backend wiring claims (each cited at file:line, read directly); MEDIUM for the recommended level names/copy (product judgment).

> Builds on `AUDIT.md`. **Three audit claims are corrected below** (items marked ⚠): attention is already *surfaced* read-only; sync chat already has a working tool-approval gate (`PermissionMode`) that the risk control never touches; and the "safety token" budget TODO is wired to an input that is always `0.0`.

---

## 0. Bottom line

The oversight control needs **no new dependency and no new policy engine**. It is a *re-labelling and fan-out* of four existing primitives that today are driven by four different UI controls:

| Existing primitive | Today's UI driver | Scope that actually honours it |
|---|---|---|
| `RiskPosture` → `ResolvedRisk` (`mode.rs:220-267`) | DriveConsole "Risk ▾" (inverted) | Background `AgentTask` only |
| `PermissionMode` (`vox-orchestrator-mcp/src/permission_modes.rs:130-178`) | Approvals view selector (`ApprovalsView.tsx:47-56`, localStorage) | Direct MCP tool calls from the GUI; **not** chat turns |
| `grounding_check_enabled` → `assess_reply_confidence` (`vox-gui/src/commands/chat.rs:595-597`) | "Check replies" toggle (`useGroundingCheck.ts`) | Sync chat only, non-blocking |
| `AttentionBudget` + `evaluate_interruption` (`attention/budget.rs:235`, `interruption_policy.rs:132`) | Read-only meter; settable only via MCP prefs / `vox attention` CLI | Live for task submit, A2A, feedback, chat clarification |

**Recommendation:** one 3-level `oversight` control (`light` < `standard` < `close`) whose levels map 1:1 onto the existing `RiskPosture` variants (so Vox.toml `task_policy` labels, the precedence resolver and every escalation site keep working), and whose resolved row *additionally* carries the sync-chat `permission_mode` and `check_replies` values. Express it as a new `contracts/gui/oversight.v2.yaml` (successor of `drive-console.v1.yaml`), with the parity gates extended to cover every field the UI claims.

---

## 1. Backend primitives — wired vs dead

### 1.1 Effective-policy resolver (WIRED, reuse as-is)

- `RiskPosture::resolve()` → `ResolvedRisk { approval, grounding_enforce, socrates_enforce, safety_token_multiplier, model_lean }` — `crates/vox-orchestrator/src/mode.rs:207-267`.
- Precedence explicit hint > category > source > global: `resolve_task_policy` `mode.rs:521-538`; `effective_category_policy`/`effective_source_policy` `mode.rs:687-716`; `effective_risk_for_task` (returns `None` when nothing configured, so global toggles survive) `mode.rs:729-737`.
- Compiled default tables are **empty** (`DEFAULT_CATEGORY_POLICY`/`DEFAULT_SOURCE_POLICY`, `mode.rs:506,510`); live overrides come from `[orchestrator.task_policy.{category,source}]` (`config/orchestrator_fields.rs:18-40`), written by the GUI via `set_task_policy_override` → `mutate_task_policy_scope` (`vox-gui/src/commands/orchestrator.rs:688-740`, label-validated at `:688-698`), which hot-reloads the daemon.
- Composer default is fetched from `resolve_default_task_policy('Chat','interactive')` on mount (`Loquela.tsx:216-233`).

Consumers (all background `AgentTask`):

| Effect | Site | Status |
|---|---|---|
| Approval tier escalation (never demotes) | `attention_fields.rs:32-41` → `ApprovalTier::max_strictness` `attention/budget.rs:126-132` | WIRED |
| Confirm ⇒ requeue with attestation requirement; Review ⇒ `needs_review_approval` (HITL) | `complete/success/gates.rs:76-110` | WIRED |
| Grounding / Socrates enforce on completion | `complete/success/socrates.rs:16-29,73-90,119,188-215` | WIRED |
| Plan loop Socrates | `planning/phase_loop.rs:61` (`risk.socrates_enforce || Genius`) | WIRED (note: Mode leaks into oversight here) |
| Model lean (Low ⇒ intelligence axes) | `effective_axes` `mode.rs:305-311` → `model_route_policy/resolve.rs:45`; also `runtime.rs:734-744` | WIRED — **this is the hidden Risk→Mode coupling** |
| `safety_token_multiplier` | `budget_gate.rs:187-195`, TODO `orchestrator_policy.rs:279-281` | DEAD (see 1.3) |

Sync chat: `risk` is parsed into `McpChatModelResolution.risk` (`vox-orchestrator-mcp/src/chat_tools/chat/message.rs:274-275`) and used **only** for model resolution. No approval, grounding or Socrates effect.

### 1.2 ⚠ Sync-chat tool approval already exists — and is hard-wired to `Ask`

- `PermissionMode { Ask, AcceptEdits, AcceptAll, Plan }`, fail-safe default `Ask` — `permission_modes.rs:130-166`; auto-approve matrix `:171-192`; contract `contracts/orchestration/permission-modes.v1.yaml` with YAML↔Rust parity test (`tests/pending_approvals_tests.rs::risk_classes_yaml_matches_rust_table`, per module doc `permission_modes.rs:4-10`).
- Enforced in `dispatch.rs:235-260`: gated tools (`vox_write_file`, `vox_multi_replace*`, `vox_delete_file`, `vox_run_shell`, `vox_deploy`, `vox_add_approval_allowlist_entry`) park as HITL approvals unless mode or per-repo allowlist approves.
- The chat agent loop forwards `permission_mode` into every tool dispatch (`agent_loop.rs:775,804,974-979`) — **but the chat entry passes `None`** (`message.rs:351-353`, "ChatMessageParams carries no transport-authenticated permission mode"). So in sync chat every gated tool always parks, regardless of the Risk control *and* regardless of the Approvals-view selector.
- The GUI selector exists (`ApprovalsView.tsx:47-59`, key `vox.approvals.permission_mode`) and is threaded as a transport-authenticated value for direct MCP calls (`transport.ts:298-306,506,549`; `vox-gui/src/commands/mcp.rs:23-44` → `OrchDaemonClient::with_permission_mode`). `ChatSurface.tsx:644` displays it.
- These parked approvals are what `InlineApprovals.tsx` / `useAttentionInbox` (`vox_pending_approvals`, `useAttentionInbox.ts:32`) show — the link between "risk" and approvals the audit says is missing is actually *this* gate.

**⚠ Invariant hole:** `vox_deploy` is `destructive, always_requires_approval: false` (`permission_modes.rs:87-92`; YAML `permission-modes.v1.yaml:81-87`), so `AcceptAll` auto-approves deploy. That violates the milestone constraint "External/deploy must stay Blocked regardless of oversight". Fix before any oversight level maps to an auto-approving mode: set `always_requires_approval: true` for `vox_deploy` (and consider `vox_run_shell`) in both YAML and Rust table; the parity test enforces lockstep. This is a *stricter* change, so a v1 in-place edit is acceptable under the file's own lifecycle note.

### 1.3 ⚠ `budget_gate` safety multiplier — dead twice over

- `evaluate_with_safety_multiplier` / `evaluate_with_aggressiveness` exist and are unit-tested only (`budget_gate.rs:165-195,298-335`).
- The TODO site (`orchestrator_policy.rs:273-283`) calls plain `evaluate(ctx.budget_token_fraction, ctx.budget_cost_fraction)`. **No caller anywhere sets those fields** — they are only defaulted to `0.0` (`orchestrator_policy.rs:51-52,86-87`); the sole live constructor is `AgentosPolicyLedger::evaluate_for_agent` with `..PolicyContext::default()` (`agentos/policy_runtime.rs:63-66`). Wiring the multiplier here multiplies zero.
- The *live* spend guard is MCP-side `enforce_budget_guard` → `budget_guard::check(spend, daily, per_session, warn_pct)` (`vox-orchestrator-mcp/src/chat_model_resolve.rs:58-99`), called before every sync chat turn (`message.rs:256-258`). It has no clutch or risk input.
- `safetyTokenBudget` in FE `ControlState` is never set or sent (`lib/driveConsole.ts:22-26`).

**Recommendation:** drop "safety tokens" from the oversight claims entirely (remove `safetyTokenBudget`, remove the multiplier from the contract row, leave the helper or delete it). If the roadmap insists on a spend effect, the only meaningful place is `budget_guard::check`'s warn threshold — but that's a Mode (cost) concern, not oversight. Same verdict for `always_delegate_free` / `delegate_free_when_simple` (`mode.rs:101-102,140-168`, consumed nowhere) and `BudgetAggressiveness` (consumed nowhere outside tests): these are **Mode** claims (`CLUTCH_DETENTS` hint "delegates to free agents", `driveConsole.ts:11`) — either wire or delete the copy; out of oversight scope.

### 1.4 Trust auto-approve (WIRED, keep orthogonal)

- `classify_tier`: external ⇒ `Blocked` (`attention/routing.rs:290-292`); untrusted wide writes ⇒ `Blocked` (`:293-297`); entropy graduation ⇒ `AutoApprove` when entropy < 0.15, ≥10 observations, trust ≥ 0.85, Trusted/System (`:300-306`, defaults `attention/budget.rs:47-56`); read-only ⇒ `AutoApprove` (`:309-311`).
- Oversight composes with this only via `max_strictness` (escalate-only). **Keep that property**: oversight `light` must never demote a trust-classified Review/Blocked; `Blocked` is unreachable from any oversight level (`ApprovalLean` has no Blocked, `mode.rs:180-196`).
- `TierGateConfig` is config (`orchestrator_fields.rs:501`), not a user control. Do not expose thresholds in the oversight UI; at most show "auto-approved because trusted (n repeats)" as explanation.

### 1.5 ⚠ Attention / interruption budget — live and already surfaced (read-only)

- `AttentionBudget` (`attention/budget.rs:235-330`; default 1h, `max_attention_ms`, `spent_ms`, `auto_approved`, `interrupt_freq_per_hour`, `focus_depth()`, `inbox_suppressed_count`). Owned by `BudgetManager` (`budget/mod.rs:247`), snapshotted into orchestrator status (`orchestrator/accessors.rs:16,114`) and passed verbatim to the GUI (`vox-gui/src/commands/orchestrator.rs:244-246,360`; TS `types/tauri.ts:181-185`).
- `evaluate_interruption` is **live** in MCP: task submit (`task_tools/submission.rs:430-449`), A2A (`a2a_tools.rs:11`), feedback/clarification (`feedback_tools.rs:20-79`, produces `needs_you` vs `withheld` surface and `policy_reason`), chat Socrates meta (`chat_socrates_meta.rs:16,636,951`), plan review (`chat_tools/plan.rs:11`); orchestrator-side `Gate::can_interrupt` (`gate.rs:256-267`). Decisions include `DeferUntilCheckpoint` / `BatchWithExistingPrompt` (`feedback_tools.rs:56-58`) — batching already exists as a decision, just not across agents.
- Sync chat debits the budget (`message.rs` test `chat_message_debits_attention_budget_on_success`, :2268).
- **Surfaced**: `AttentionBudgetMeter` in the chat composer dock (`ChatSurface.tsx:671-690`) and Dashboard (`Dashboard.tsx:401`); `NeedsYouSurface` + `useAttentionInbox` (approvals + feedback, polled + event-driven, `useAttentionInbox.ts:22-54`).
- **Controllable only out-of-band**: `attention_enabled` / `attention_budget_ms` / `attention_alert_threshold` via MCP preference handler (`memory_tools/handlers_preferences.rs:60-90`, live-applies `init_attention`) and `vox attention` CLI (`vox-cli/src/commands/attention.rs:131-221`). No Tauri command, no GUI control.
- **Missing for the milestone**: per-agent attribution (`FeedbackRow` has no agent id or `policy_reason`, `transport.ts:927-947`, though `AttentionEvent` records both, `feedback_tools.rs:65-78`); per-agent quota; cross-agent batching policy; "why this reached you".
- **⚠ Dead parity gate**: `vox ci attention-config-parity` (`vox-cli-ci/src/attention_parity.rs:5-12`) reads `crates/vox-orchestrator/src/config.rs` and `crates/vox-mcp/src/memory/handlers_preferences.rs`, neither of which exists → early `return Ok(())`, silently green. Repoint at `config/orchestrator_fields.rs` + `vox-orchestrator-mcp/src/memory_tools/handlers_preferences.rs` (and fail, not skip, on missing files) before adding a GUI writer for these keys.

### 1.6 "Check replies" (WIRED, sync only)

- `grounding_check_enabled` → `assess_reply_confidence` — a deterministic hedge-ratio heuristic, explicitly *not* an LLM call (`vox-orchestrator/src/grounding.rs:146-157`), run in the Tauri layer after the reply (`vox-gui/src/commands/chat.rs:591-597`). Also a `DriveSet`/`TaskCapabilityHints` knob (`drive/protocol.rs:44,189,230`; `types/tasks.rs:997-999`; `control_plane.rs:45,102`).
- Because it's free and non-blocking, it can safely be **on** for `standard` and `close` without cost or latency claims.

---

## 2. Contract & parity-gate implications

Current gates:

| Gate | Checks | Gaps |
|---|---|---|
| `vox-orchestrator/src/control/mod.rs:22-92` `code_matches_contract` | per-index clutch quality/force_free_pool/axes; risk approval/grounding/socrates | Index-based — extra rows unchecked; ignores `id`, `safety_token_multiplier`, `model_lean`; message mislabels axes "(quality, cost, speed)" vs code `(cost, responsiveness, intelligence)` (`mode.rs:93-94`) |
| `vox-gui/src/drive/protocol.rs:336-369` | drive-console ids == `axis-drive.v1.yaml` `set_keys.clutch/risk.values` (`contracts/gui/axis-drive.v1.yaml:3,16-18`) | Values only |
| FE `lib/driveConsole.ts:1` "kept in sync by the BE parity gate" | **Nothing** — no test reads the YAML from TS | Claim is false; precedent for a vitest reading a contract exists (`lib/turnTrace.test.ts:10`) |
| `permission-modes.v1.yaml` ↔ `permission_modes.rs` | risk classes | Modes matrix itself not tied to any oversight level |
| `axisDrive.ts:13-32` `ALLOWED_SET_KEYS` ↔ `protocol.rs:187-189` | (manual) | No cross-language parity |

### Recommended contract: `contracts/gui/oversight.v2.yaml`

Successor to `drive-console.v1.yaml` (bump per data-storage policy: breaking rename ⇒ new file, `x-vox-version: 2`). Keep `clutch` block verbatim (Mode is a separate axis); replace `risk` with `oversight`. Every UI-claimed effect is a field so the gate can assert it.

```yaml
# contracts/gui/oversight.v2.yaml — SSOT for the composer Mode + Oversight controls.
version: 2
x-vox-version: 2
supersedes: contracts/gui/drive-console.v1.yaml
clutch: [ ...unchanged from v1... ]
oversight:            # ordered least -> most oversight; "more = safer"
  - id: light
    legacy_risk: high          # wire/Vox.toml compat: RiskPosture::from_label
    background:
      approval: auto_approve_more   # escalate-only floor; trust may still raise it
      grounding_enforce: false
      socrates_enforce: false
    chat:
      permission_mode: accept_edits # never accept_all
      check_replies: false
  - id: standard
    legacy_risk: moderate
    background: { approval: confirm, grounding_enforce: true, socrates_enforce: false }
    chat:       { permission_mode: ask, check_replies: true }
  - id: close
    legacy_risk: low
    background: { approval: review, grounding_enforce: true, socrates_enforce: true }
    chat:       { permission_mode: ask, check_replies: true }
invariants:
  never_auto_approve: [vox_deploy, vox_add_approval_allowlist_entry]  # asserted against permission-modes.v1.yaml
  model_lean: none     # oversight does not move Mode axes (removes effective_axes coupling)
```

Decisions baked in (opinionated):

1. **Three levels, not four.** 1:1 with `RiskPosture` means zero changes to `effective_risk_for_task`, `TaskPolicyEntry.risk` labels, `resolve_default_task_policy`, the `max_strictness` sites, or persisted Vox.toml. Add `OversightLevel` as a thin newtype/alias with `from_label` accepting both `light|standard|close` and legacy `high|moderate|low`; serialise the new ids.
2. **Remove `ModelLean::Intelligence` from `Low`** (`mode.rs:263`, `effective_axes` `:305-311`, `runtime.rs:734-744`). The milestone says Mode stays separate; this is the hidden coupling the audit flagged. Update `model_route_policy/resolve.rs:743` test and `tasks.rs`/routing tests accordingly. Also decide `phase_loop.rs:61`'s `|| Genius` (Mode turning on Socrates) — recommend keep but document as Mode behaviour, not oversight.
3. **No `close` → `accept_all` anywhere; `light` caps at `accept_edits`.** Even after the `vox_deploy` fix, `accept_all` auto-approves `vox_run_shell`/`vox_delete_file`; that belongs to the explicit Approvals view, not a composer quick-control.
4. **Drop `safety_token_multiplier` from the claim surface** (see 1.3).
5. Gate upgrades: make `code_matches_contract` key rows by `id` (not index) and assert row count; add a vitest that parses `oversight.v2.yaml` and asserts the TS level table; extend `protocol.rs` parity to the `oversight` set-key; assert `invariants.never_auto_approve` against `permission_modes::mode_auto_approves` for every mode used.

---

## 3. How sync chat should honour oversight (minimal wiring)

1. **Permission mode (the real "approvals" effect):** add `permission_mode: Option<String>` to the chat turn path, sourced from the *resolved oversight level in the Tauri layer* (`vox-gui/src/commands/chat_turn.rs` / `chat.rs`), sent via the existing transport-authenticated channel (`OrchDaemonClient::with_permission_mode`, `vox-gui/src/commands/mcp.rs:43-44`), and have `vox_chat_message`'s dispatch forward its own authenticated `permission_mode` into `run_agent_turn` instead of the hard `None` at `message.rs:351`. Must **not** become a field on `ChatMessageParams` (LLM-composable args — the T0.1 invariant documented at `dispatch.rs:48` and `agent_loop.rs:19-21`). Test-first: a turn under `light` auto-approves `vox_write_file`; under `standard`/`close` it parks; `vox_deploy` parks under every level.
2. **Check replies:** derive `grounding_check_enabled` from the level (`standard`/`close` ⇒ true) instead of `useGroundingCheck` localStorage; keep an advanced per-session override only if product wants it. Retire `GroundingCheckToggle` and the "grounding: off" chip.
3. **Delegated background work from chat** (`vox_spawn_agent` / `vox_submit_task`, pinned at `agent_loop.rs:840`, session-injected at `:956-972`): also inject the turn's `risk` (legacy label) so tasks spawned from a chat turn inherit the composer's oversight as an *explicit hint* (top precedence). Today they inherit nothing and fall to category/source/global.
4. **Do not claim** blocking grounding or Socrates for sync chat; there is no enforce path and adding one (re-generate on violation) is a latency/cost feature, not wiring. UI copy for sync chat: "asks before edits / flags shaky answers". For background: "requires attestation / human review; checks citations".
5. Precedence for sync chat stays explicit composer value > `resolve_default_task_policy('Chat','interactive')`.

---

## 4. Frontend control state

- State lives in `Loquela.tsx:206-253` (`useState<ControlState>(defaultControl)`; ephemeral; mount-time backend default; drive-API setters `setClutch`/`setRisk`) and is sent as `clutch`/`risk` on every turn (`Loquela.tsx:677-678` → `chat_turn.rs` `non_blank(&input.risk)`).
- `driveConsole.ts:3-30`: `RiskId` + `RISK_POSTURES` (rose/amber/emerald — inverted tones) + `safetyTokenBudget?`. FE cold default `clutch:'efficiency'` differs from BE `resolved_clutch` fallback `Balanced` (`types/tasks.rs:1010-1014`) — harmless because of the mount fetch, but note it.
- `axisDrive.ts:13-32,36-55` `DriveSet` keys include `risk`, `grounding_check_enabled`. Recommended: keep `risk` as the wire key for one release (accept `oversight` alias in `protocol.rs::apply_set` and `ALLOWED_SET_KEYS`), keep `grounding_check_enabled` accepted but derived. `axis-drive.v1.yaml:16-18` values must follow (`oversight: { type: enum, values: [light, standard, close] }`), gate in `protocol.rs:337`.
- Persistence: risk/clutch are ephemeral while Check replies is per-session localStorage (`useGroundingCheck.ts:4-15`, key `gui.chat.groundingCheck.v1.<session>`) — inconsistent. Recommend: oversight persists **per workspace** via the existing `set_task_policy_override('Chat','interactive')` path (Vox.toml, hot-reloaded) when the user picks "make default", otherwise per-session `useLocalStorage` (hook already exists, `hooks/useLocalStorage`). Drop the `groundingCheck.v1` key (one-time read for migration is optional).
- Spend chrome: three copies (DriveConsole, status bar BUDGET, composer footer). Keep the status-bar one (global), remove DriveConsole's spend + footer duplicate.

### UI primitives (no new deps)

Installed (`crates/vox-gui/ui/package.json`): `@radix-ui/react-dialog`, `@radix-ui/react-slot`, `@radix-ui/react-tooltip`, `lucide-react`, `clsx`, `tailwind-merge`, `zustand`. **No Radix slider, popover, radio-group or toggle-group.** In-repo: `components/ui/Segment.tsx` (segmented control, `aria-pressed`, `title` hints), `components/ui/Popover.tsx` (minimal absolute div, no focus management — WIP-modified), `components/ui/Tooltip.tsx` (new, untracked WIP).

- Use **`Segment`** for the 3-level oversight control (same widget family as the Mode radio, legible ordering left→right = more oversight). Improve it to `role="radiogroup"`/`aria-checked` + arrow-key nav (the Mode radio in `DriveConsole.tsx:62` already uses `aria-checked`) rather than adding `@radix-ui/react-toggle-group`.
- Use the WIP **`Tooltip`** (Radix-backed) for per-level "what this does" copy generated from the contract row, so copy cannot drift from wired effects.
- Attention controls (budget minutes, alert threshold, enable) belong in a small panel off `AttentionBudgetMeter` (expand state already exists) or the Policies view — `Dialog` is available if a modal is needed. Do **not** add a slider dependency; a `Segment` of presets (30m / 1h / 2h / off) is enough.

---

## 5. Minimal wiring checklist (for roadmapper)

| # | Change | Crates/files | Gate impact |
|---|---|---|---|
| W1 | `vox_deploy` (+ consider `vox_run_shell`) `always_requires_approval: true` | `permission-modes.v1.yaml`, `permission_modes.rs:87-92` | YAML↔Rust parity test must stay green |
| W2 | `oversight.v2.yaml` + `OversightLevel` labels (legacy aliases) | `contracts/gui/`, `mode.rs` | Rewrite `control/mod.rs` gate by id; `protocol.rs:337`; new vitest |
| W3 | Remove `ModelLean::Intelligence` from Low | `mode.rs:258-264,305-311`, `runtime.rs:734-744`, `model_route_policy/resolve.rs` tests | Route-policy tests |
| W4 | Thread authenticated `permission_mode` into chat agent loop | `vox-gui chat_turn.rs/chat.rs`, `vox-orchestrator-mcp chat_tools/chat/message.rs:351`, dispatch of `vox_chat_message` | New tests (park vs auto) |
| W5 | Derive `check_replies` from level; retire toggle | `chat.rs:595`, `useGroundingCheck`, `GroundingCheckToggle.tsx`, `App.tsx:1794-1796` | Playwright stepper + review-bundle |
| W6 | Inherit oversight into chat-spawned tasks | `agent_loop.rs:956-972` | Unit test |
| W7 | Drop safety-token / delegate-free claims (or move to Mode backlog) | `driveConsole.ts:11,25`, `RiskPopover.tsx` copy, `mode.rs` fields | Contract no longer lists them |
| W8 | Attention: Tauri commands for `attention_budget_ms`/`attention_enabled`/`attention_alert_threshold` (reuse MCP preference handler); add `agent_id` + `policy_reason` to `FeedbackRow` | `vox-gui/src/commands/orchestrator.rs`, `handlers_preferences.rs:60-90`, `transport.ts:927-947` | Fix `attention_parity.rs` paths first |
| W9 | Per-agent interrupt quota / cross-agent batching | `attention/interruption_policy.rs` (new signal: per-agent recent interrupts), `BudgetManager` | Genuinely new logic — needs its own phase research |

W1–W7 are wiring of existing primitives; W8 is plumbing; only W9 is new behaviour.

## 6. Alternatives considered

| Category | Recommended | Alternative | Why not |
|---|---|---|---|
| Level model | 3 levels mapped to `RiskPosture` | 4–5 levels (add "ask everything"/"autopilot") | New tier semantics in every escalation site; `Blocked` must stay unreachable; YAGNI |
| Contract | New `oversight.v2.yaml` | Edit `drive-console.v1.yaml` in place | Breaking id rename; data-storage policy requires bump + rename |
| Sync-chat approvals | Reuse `PermissionMode` | Route chat through `ApprovalTier`/`AgentTask` gates | Sync chat has no task lifecycle; `PermissionMode` is already the dispatch gate there |
| Control widget | In-repo `Segment` + Radix Tooltip | `@radix-ui/react-slider` / `react-toggle-group` | New deps for a 3-option control; slider implies a continuum that doesn't exist |
| Safety budget | Drop claim | Wire `evaluate_with_safety_multiplier` | Input fractions are always 0.0; would be a fake wiring |

## Sources

All in-repo, read 2026-10-08 (HIGH): file:line citations inline above. Key files: `crates/vox-orchestrator/src/{mode.rs,budget_gate.rs,orchestrator_policy.rs,attention/*.rs,control/mod.rs,gate.rs,types/tasks.rs}`, `crates/vox-orchestrator/src/orchestrator/task_dispatch/{submit/attention_fields.rs,complete/success/{gates,socrates}.rs}`, `crates/vox-orchestrator-mcp/src/{permission_modes.rs,dispatch.rs,chat_model_resolve.rs,feedback_tools.rs,attention_policy.rs,chat_tools/chat/{message,agent_loop}.rs,memory_tools/handlers_preferences.rs}`, `crates/vox-gui/src/{commands/{chat,chat_turn,mcp,orchestrator}.rs,drive/protocol.rs}`, `crates/vox-gui/ui/src/{lib/{driveConsole,axisDrive}.ts,hooks/{useGroundingCheck,useAttentionInbox}.ts,components/surfaces/Loquela/Loquela.tsx,components/ui/*}`, `crates/vox-cli-ci/src/attention_parity.rs`, `contracts/{gui/drive-console.v1.yaml,gui/axis-drive.v1.yaml,orchestration/permission-modes.v1.yaml}`.
