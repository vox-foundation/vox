# v1.2 Baseline Audit — Drive Console / Risk / Attention (2026-10-08)

Input for the v1.2 researchers and roadmapper. Paths are relative to the repo root.

## Surfaces today (composer + chrome)

- `crates/vox-gui/ui/src/components/surfaces/Loquela/DriveConsole.tsx` — strip: Mode radio (Free/Effic./Bal./Genius) | Spend + bar + burn | "Risk: X ▾" trigger.
- `.../Loquela/RiskPopover.tsx` — "Acceptable risk" High (rose) / Moderate (amber) / Low (emerald); copy promises "auto-approve more, gates shadow-only, fewer safety tokens" (High) and "spend safety tokens, lean model up" (Low). Hosts "After each reply" slot (Check replies).
- `.../Chat/GroundingCheckToggle.tsx` — "Check replies: on/off" (rendered in `App.tsx:1795`), plus a "grounding: off" footer chip.
- `.../Chat/ResearchModeDropdown.tsx` — Auto (Socrates) / Fast / Deep / Off (`App.tsx:1801`); research also shown in status bar.
- Spend is shown three times: DriveConsole, status bar BUDGET, composer footer "session $x / $y".
- `InlineApprovals.tsx` (`App.tsx:1872`) — polls `vox_pending_approvals`; not linked to the risk control.
- Contract: `contracts/gui/drive-console.v1.yaml`; FE mirror `crates/vox-gui/ui/src/lib/driveConsole.ts`.
- Screenshot: `crates/vox-gui/ui/review-bundle/latest/chat--composer-filled--wide--chromium.png`.

## Core UX defect

"Acceptable risk: High" = *less* oversight. The scale is inverted relative to how people read safety (more = safer), and colour (rose=high) signals danger on the option users may legitimately want.

## Backend reality (wired vs. claimed)

| Claim | Reality | Evidence |
|---|---|---|
| Risk changes approvals | Background `AgentTask` only. `effective_approval_tier` = trust `classify_tier` then `max_strictness(risk)` — escalates, never demotes. Confirm ⇒ requeue to agent with gate text; only Review ⇒ HITL `BlockedOnApproval`. | `attention_fields.rs:32-40,100-105`; `gates.rs:76-97`; `mode.rs:187-196` |
| Risk enforces grounding/Socrates | Background task completion only (requeue on violation). Sync chat ignores. | `socrates.rs:16-29,73-78,119-139,186-215` |
| Risk sent on chat | Sent (`chat_turn.rs:201-202,249-250`) but sync path uses it for model resolution only (`message.rs:274-275,1185-1186`). | |
| Low risk = safety | Also overrides clutch axes to intelligence-weighted (15,15,70) via `effective_axes` — hidden coupling. | `mode.rs` |
| "Check replies" | Separate `grounding_check_enabled` → non-blocking post-reply `assess_reply_confidence`; does not flip `grounding_enforce`. Persisted per session (localStorage); risk/clutch are ephemeral. | `chat.rs:595-597`; `useGroundingCheck.ts:4-15` |
| Safety tokens | `safetyTokenBudget` never set by UI, never sent. `safety_token_multiplier` (1.0/1.0/1.5) + `evaluate_with_safety_multiplier` unit-tested only; TODO in live path. | `driveConsole.ts:25`; `mode.rs:248-262`; `orchestrator_policy.rs:273-283` |
| "Delegates to free agents on simple tasks" | `always_delegate_free` / `delegate_free_when_simple` defined, never consumed. Budget-gate aggressiveness helpers unused. | `mode.rs`; `budget_gate.rs:165+` |
| Mode axes | Real: `(cost, responsiveness, intelligence)` → `SelectionAxes`; Free ⇒ free-tier only. | `model_route_policy/resolve.rs:39-53,316-319`; `runtime.rs:66-94` |

## Attention / interruption (exists, invisible)

- Session `AttentionBudget` (default 1h) and `evaluate_interruption` can defer interrupts when exhausted (`budget.rs:91-92,235+`; `interruption_policy.rs:128-172`).
- Trust auto-approve: entropy < 0.15, ≥10 repeated approves, trust ≥ 0.85 (`routing.rs:300-306`; `budget.rs:50-52`). External/deploy ⇒ Blocked (`routing.rs:291-292`).
- Nothing is surfaced or controllable in the GUI; no per-agent interrupt quota; no batching across agents; no "why this reached you".

## Constraints

- Playwright visual stepper + review-bundle screenshots required for GUI changes (AGENTS.md GUI Visual Verification).
- Contract parity gates: `vox-orchestrator control/mod.rs:23` (`code_matches_contract`), `vox-gui drive/protocol.rs:337`.
- External/deploy actions must stay Blocked regardless of the oversight setting.
- Uncommitted WIP exists in Loquela/Chat/Popover/Tooltip files — coordinate before editing.
