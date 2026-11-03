# Domain Pitfalls — v1.2 Unified Autonomy Control

**Domain:** human-oversight / autonomy control for a multi-agent coding orchestrator (Tauri GUI + Rust orchestrator)
**Researched:** 2026-10-08
**Overall confidence:** HIGH for repo-specific findings (verified against source at the cited lines); MEDIUM for human-factors literature (well-established, cited from the standard references, not re-fetched this session).

Phase labels used below are suggestions for the roadmapper:
- **P0 Safety floor + contract** — define the oversight enum, invariant floors, contract v2, migration.
- **P1 Backend wiring** — make every claimed effect real for background tasks *and* sync chat.
- **P2 Attention budget** — cross-agent interruption budget, batching, "why this reached you".
- **P3 GUI control** — the single dial, copy, a11y, spend-chrome dedup.
- **P4 Verification** — mutation proofs, Playwright stepper, review bundle.

---

## Critical Pitfalls

### C1. "Hands-off" quietly lowers a floor that today is only a substring heuristic
**What goes wrong:** Today the only hard block is `action.external` (`crates/vox-orchestrator/src/attention/routing.rs:291-293`), and `external` is computed by keyword matching on the task description — `deploy`, `production`, `publish `, `http(s)://`, `curl `, `terraform`, `kubectl`, `api key`, `secret ` (`orchestrator/task_dispatch/submit/attention_fields.rs:11-23`) — plus a `deploy` substring in a file path (`:62-68`). Destructive local actions (`rm -rf`, `git push --force`, `git reset --hard`, `DROP TABLE`, deleting a worktree, writing `.env`/`~/.ssh`) are **not** classified at all; they fall through to trust-based tiers, and a Trusted/System agent with low entropy and ≥10 repeated approvals auto-approves (`routing.rs:300-308`, `:316`).
Right now that is masked: risk can only *escalate* (`attention/budget.rs:123-133` `max_strictness`; `attention_fields.rs:32-40`), so "High / auto-approve more" is effectively a no-op on approvals. The moment the new control actually *does* something at its low end (which is the milestone's goal — "wire every claimed effect"), the weak floor becomes the only thing standing between the user and a destructive action.
**Prevention:**
- Before wiring any demotion, introduce an explicit, typed `SafetyFloor` (external/network egress, deploy/publish, destructive VCS/fs ops, secret read/write, credential-bearing env) classified from structured signals (tool name + args, file manifest, capability caps) — not description substrings. Keep the substring check as a *supplementary* escalator only.
- Encode the invariant as `effective_tier = max(floor_tier(action), oversight_tier(setting), trust_tier)` — the setting may only move *between* the floor and the ceiling; it is never an input to the floor function. Make the floor function not take the oversight setting as a parameter at all, so it cannot be coupled.
- Contract it: add `floors:` to the drive-console contract (e.g. `external: blocked`, `destructive: review`, `secrets: blocked`) and assert in `code_matches_contract` that every oversight level resolves to ≥ the floor.
**Detection:** a test matrix `oversight_level × floor_category` where any cell resolves below the floor; grep for `external` being the only `return ApprovalTier::Blocked` precondition.
**Phase:** P0 (must land before P1 wires any demotion).

### C2. Sync chat has no approval path at all — "wired for sync chat" is a new enforcement surface, not a toggle flip
**What goes wrong:** The sync chat path receives `risk` but uses it only for model resolution (`crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs:274-275`, `:1185-1186`); `message.rs` contains no approval / `BlockedOnApproval` handling. Approvals, grounding requeue and Socrates enforcement exist only on `AgentTask` completion (`socrates.rs:119-139,186-215`). Teams typically "wire" sync chat by reusing the background enforcement, which requeues to an agent — there is no agent queue in a sync turn, so the result is either a hung turn, a silent drop, or the turn being promoted into a background task without telling the user.
**Prevention:** Define sync-chat semantics explicitly per oversight level in the contract: (a) tool calls with floor ≥ Review surface an inline approval in the same turn (reuse `InlineApprovals`, `App.tsx:1872`, rather than polling); (b) grounding enforcement on sync = block-and-annotate the reply (not requeue); (c) a bounded timeout with a visible "waiting for you" state. Do not silently convert sync turns to background tasks.
**Detection:** an integration test that a sync turn whose tool call hits a floor returns a pending-approval event, and a mutation test (delete the gate) that the test then fails.
**Phase:** P1.

### C3. Automation complacency / over-trust when interruptions are suppressed
**What goes wrong:** The user picks "less oversight" once, interruptions disappear, and they stop reading what the agents do (Parasuraman & Riley 1997, "misuse"; Bainbridge 1983, "Ironies of Automation" — the less the operator is involved, the worse they are at intervening when it matters). Repo-specific amplifiers: trust auto-approve graduates on *repetition* (≥10 repeated approves, entropy < 0.15, trust ≥ 0.85 — `routing.rs:300-306`), so rubber-stamping trains the system to ask less; `evaluate_interruption` will `ProceedAutonomously` when the attention budget is exhausted and information gain is marginal (`attention/interruption_policy.rs:166-174`) — again silently.
**Prevention:**
- Every suppressed/deferred interruption must be **recorded and reviewable** (a "handled without you" digest with reasons — the existing `reason` strings like `attention_budget_exhausted_marginal_gain_insufficient` are the raw material).
- Approvals given in under N ms or in long uninterrupted runs should not count toward trust graduation (anti-rubber-stamp), or should count at reduced weight.
- Periodic low-cost "check-ins" even at the lowest-oversight setting (Sheridan-style supervisory control keeps the human in the loop).
**Detection:** telemetry: median approval dwell time trending down; approve rate → 1.0; zero "reviewed digest" opens.
**Phase:** P2 (digest + reasons), P1 (trust weighting).

### C4. Alarm fatigue at the high end makes "more = safer" unsafe in practice
**What goes wrong:** Maximum oversight escalates everything to Review; with several agents this floods the user, who then either approves blindly (fatigue → same failure as C3) or flips the dial to minimum. Clinical-alarm literature (Joint Commission Sentinel Event Alert #50, 2013) documents exactly this: high false-alarm rates cause desensitisation and disabled alarms.
**Prevention:** the top setting should raise *scrutiny* (verification, grounding, review of irreversible actions) more than *interrupt count*. Batch low-severity confirms; reserve immediate interrupts for floor-category actions and `RequireHumanBeforeContinue` (`interruption_policy.rs:150-158`). Show the projected interrupt rate next to the dial ("≈ 6 asks/hour at this level").
**Detection:** interrupts/hour per level in telemetry; users oscillating between extremes.
**Phase:** P2 + P3 (copy).

### C5. Contract migration breaks parity gates — or "passes" them by being renamed in lockstep
**What goes wrong:** Two parity tests read `contracts/gui/drive-console.v1.yaml`: `vox-orchestrator/src/control/mod.rs:23` (`code_matches_contract`, asserts clutch quality/axes/force_free_pool and risk rows) and `vox-gui/src/drive/protocol.rs:337` (`clutch_risk_match_drive_console`, asserts `axis-drive.v1.yaml` `set_keys.risk.values` == drive-console risk ids). `axis-drive.v1.yaml:3,17` hard-codes `risk: [high, moderate, low]`. Changing the vocabulary (e.g. `oversight: [light, standard, strict]`) requires editing three contract files, two Rust enums/label parsers (`RiskPosture::from_label`), the FE mirror `ui/src/lib/driveConsole.ts`, and the `vox gui drive` `set` key list (`protocol.rs:188,227,314`). Typical failure: an agent edits contract and code in the same commit so the parity test stays green while the *semantics* inverted (high used to mean less oversight; a new `high` meaning more oversight would satisfy the test).
**Prevention:**
- Use **new identifiers** that cannot collide with the old ones (never reuse `high`/`low`), bump to `drive-console.v2.yaml` per data-storage policy ("bump `x-vox-version` and rename the file if breaking"), and keep `risk` as a deprecated alias in `apply_set` that maps old→new with an inversion table, marked with a `vox-deprecated-since` annotation.
- Add a semantic parity assertion: oversight levels are ordered so that `approval strictness` and `grounding/socrates enforce` are monotonic non-decreasing along the declared order.
- The drive protocol is used by agents/automation (`vox gui drive`); an unknown key is rejected (`protocol.rs:371-381`), so scripted callers sending `risk` will hard-fail after removal — keep the alias for at least one release.
**Detection:** grep for `"risk"` across `contracts/`, `crates/vox-gui`, `crates/vox-orchestrator*`, `e2e/lib/tauriMock.ts` after the change; a test that `risk=high` via drive maps to the *least* oversight level.
**Phase:** P0.

### C6. Migration of existing user state inverts silently
**What goes wrong:** Persistence is inconsistent: grounding check is per-session localStorage (`ui/src/hooks/useGroundingCheck.ts:3`), risk/clutch are ephemeral. A naive migration that maps the stored or last-sent string by position, or reads `risk` from session state and assigns it to the new control, turns "High risk" (least oversight) into "High oversight" or vice versa. Worse: a user who had "Check replies: on" (wants checking) gets mapped into a low-oversight default because risk was unset.
**Prevention:** explicit migration table `{risk: high→light, moderate→standard, low→strict}`; `grounding_check=on` raises the migrated level to at least the level that includes reply checking; default for unknown/absent state = the middle (standard), never the least. Persist the new setting in one place (decide: per-workspace backend config vs. localStorage — prefer backend so background tasks and the drive API see the same value) and delete the old localStorage key after migration.
**Detection:** unit tests for each legacy combination; a Playwright test seeding legacy localStorage and asserting the rendered level.
**Phase:** P0 (table) + P3 (FE migration).

### C7. Hidden coupling between oversight and cost/model selection survives the rename
**What goes wrong:** `effective_axes` forces `(15,15,70)` when risk's `model_lean` is Intelligence, regardless of the user's Mode (`crates/vox-orchestrator/src/mode.rs:305-311`), and the sync path feeds risk into model resolution (`message.rs:274-275`). PROJECT.md says Mode stays a separate axis, but if this coupling is kept, "strict oversight" silently disables Free mode's spending intent (Free ⇒ `force_free_pool`, but axes jump to intelligence-weighted) and surprises users with cost. `safety_token_multiplier` 1.5 (`mode.rs:262`) is a second cost coupling, currently unwired (`budget_gate.rs:181-193` additive helper, unit-tested only).
**Prevention:** delete `effective_axes`'s risk branch (or make oversight produce a *verification* budget that is shown separately). If strict oversight needs a stronger *verifier* model, select it as a separate verifier role with its own visible cost line, never by mutating the user's Mode axes. Add a contract assertion: for every (mode, oversight) pair, the generation axes equal the mode's axes.
**Detection:** `rg "effective_axes"` call sites; test matrix over (mode × oversight) comparing resolved axes to mode axes; Free mode + strict oversight must keep `force_free_pool`.
**Phase:** P1.

### C8. Tests that prove the function, not the wiring
**What goes wrong:** This exact pattern already exists: `safety_token_multiplier` and `evaluate_with_safety_multiplier` are unit-tested while the live path has a TODO (`orchestrator_policy.rs:273-283`); `always_delegate_free` / `delegate_free_when_simple` are defined and tested (`mode.rs:101-102,140-168,358`) but never consumed. AGENTS.md §PR & Review Discipline names the failure: "a validator unit-tested directly still passed with every call site deleted."
**Prevention:**
- For every claim in the UI copy, one **end-to-end-ish** test at the call site (`populate_task_attention_fields`, sync `message.rs` handler, budget gate in the live path) — not on the pure helper.
- Mutation proof per guard: delete/neutralise the call (e.g. make `effective_approval_tier` return `classified`), confirm the test fails, restore; record the before/after in the PR body. Guard with a presence check (`grep -c` of the call) because concurrent fmt/agents can revert edits.
- A "claims ledger" test: parse the copy strings for the control from the contract and assert each claim id maps to a test name that exists (cheap drift guard that forbids copy promising unwired effects again).
**Detection:** `cargo mutants` scoped to `mode.rs`, `attention_fields.rs`, `socrates.rs`, `message.rs` (repo already uses `cargo mutants` for compiler crates).
**Phase:** P4, but tests are written first in P1 (test-first policy).

---

## Moderate Pitfalls

### M1. Mid-turn / mid-task changes of the setting
**What goes wrong:** Tier is stamped at submit time (`task.approval_tier = Some(effective_tier)`, `attention_fields.rs:105-106`). Lowering oversight mid-run therefore doesn't affect queued tasks (users think it did); raising it mid-run doesn't protect in-flight work (dangerous). Sync turns read `risk` from params per turn, so a dial change during streaming applies to the *next* turn only.
**Prevention:** asymmetric rule — **tightening applies immediately** (re-evaluate queued and in-progress tasks at their next tool call/checkpoint; escalate only); **loosening applies to new work only**. Show a toast: "Stricter oversight applied to 3 running tasks" / "Looser oversight applies to new tasks". Snapshot the effective level into each turn/task record so audits know what was in force.
**Detection:** test: submit task at light, switch to strict, next destructive tool call requires Review.
**Phase:** P1 (semantics) + P3 (feedback).

### M2. Multi-agent starvation when batching interruptions
**What goes wrong:** A single session-wide `AttentionBudget` (default 1h, `attention/budget.rs`) plus batching means one chatty agent can exhaust the budget, after which *other* agents' marginal asks are auto-proceeded (`interruption_policy.rs:166-174`) — or, if batching holds asks until a batch window, a blocked agent idles (starvation) while others keep running. Agents blocked on approval also hold `ResourceLockManager` locks (ADR-025), so starvation cascades into lock contention.
**Prevention:** per-agent fair share (e.g. deficit round-robin over agents within the shared budget); max batch delay per ask (aging) with priority boost; floor-category asks bypass batching; blocked-on-human tasks release or time-bound their resource locks; surface "Agent X waiting 12 min" in the budget panel.
**Detection:** simulation test with one high-rate and one low-rate agent asserting the low-rate agent's p95 wait is bounded; telemetry of wait time per agent.
**Phase:** P2.

### M3. Latency and cost surprises from enforced verification on sync chat
**What goes wrong:** Turning on grounding/Socrates enforcement for sync chat adds a verification pass (and possibly a retrieval round) to every reply; with the Low-risk 1.5× safety multiplier and intelligence axes (C7) cost can more than double while the user's Mode says "Free/Efficiency". Today `assess_reply_confidence` is non-blocking post-reply (`chat.rs:595-597`) — making it blocking changes perceived latency abruptly.
**Prevention:** stream the reply, then show a verification badge that resolves (pending → verified/flagged), instead of blocking first token; bound verification by a per-turn budget shown in the *single* spend surface; make verification model selection respect Mode (Free ⇒ free-tier verifier or heuristic-only `grounding.rs:157` check). Record verification cost as its own line item.
**Detection:** p50/p95 time-to-first-token and $/turn before vs. after per (mode × oversight); fail CI tier budget if sync-chat latency in e2e mocks regresses beyond a threshold.
**Phase:** P1 (policy) + P3 (display).

### M4. Two "grounding" concepts merged incorrectly
**What goes wrong:** `grounding_check_enabled` (post-reply confidence heuristic, advisory) and `grounding_enforce` (contract-level, background requeue) are different things (`chat.rs:595-597` vs `drive-console.v1.yaml` risk rows). Folding "Check replies" into the dial by aliasing one to the other either makes strict mode only advisory (claims enforcement, delivers a chip) or makes light mode lose the advisory check users liked.
**Prevention:** rename in code to `reply_confidence_check` (advisory) vs `grounding_enforcement` (blocking); the oversight levels declare both explicitly in the contract; the advisory check can stay on at every level (it's cheap and informative).
**Phase:** P0 (naming/contract) + P1.

### M5. Spend shown three times → removing two breaks budget visibility for some flows
**What goes wrong:** Spend appears in DriveConsole, the status-bar BUDGET, and the composer footer (AUDIT). Removing duplicates usually deletes the one specific screens rely on (e.g. status bar visible when the composer is collapsed or on non-chat surfaces).
**Prevention:** pick one canonical component, render it in one place per layout, and verify via the review-bundle screenshots on every surface that previously showed spend. Verification cost (M3) should appear inside that one surface.
**Phase:** P3.

### M6. Copy that promises effects again
**What goes wrong:** The current popover promises "fewer safety tokens" and "delegates to free agents" that are unwired (AUDIT table). New copy written during P3 will drift ahead of P1 unless gated.
**Prevention:** copy lives in the contract next to each level, with a `claims:` list of ids; a test (C8) fails if a claim id lacks a backing wiring test. Write copy as observable behaviour ("asks before deleting files"), not mechanism ("spends safety tokens").
**Phase:** P0 contract shape, enforced in P4.

---

## Minor Pitfalls

### m1. Accessibility of a dial/slider
**What goes wrong:** Custom dials are commonly mouse-only, colour-coded only (the current rose/amber/emerald scheme already signals "danger" on a legitimate option — AUDIT §Core UX defect), and announce a raw number.
**Prevention:** use a discrete control with 3–4 stops implemented as `role="radiogroup"` (or `role="slider"` with `aria-valuetext` naming the level, e.g. "Strict — asks before any file change") per the WAI-ARIA APG slider/radio patterns; arrow keys/Home/End; no drag-only interaction (WCAG 2.2 SC 2.5.7 Dragging Movements); meaning not by colour alone (SC 1.4.1); visible label and focus ring; text contrast ≥ 4.5:1. Order must read left→right as less→more oversight, matching "more = safer".
**Detection:** axe check in Playwright; keyboard-only stepper test.
**Phase:** P3.

### m2. Uncommitted WIP in the same files
**What goes wrong:** `git diff --stat` shows uncommitted edits in `Loquela/Loquela.tsx` (+126/-?), `DriveConsole.tsx`, `RiskPopover.tsx`(+test), `Chat/ChatSurface.tsx`(+test), `GroundingCheckToggle.tsx`, `ResearchModeDropdown.tsx`, `ChatModelPicker.tsx`, `ui/Popover.tsx`, and new untracked `ui/Tooltip.tsx`(+test) — exactly the files this milestone replaces. AGENTS.md: "Never edit a tree another agent owns."
**Prevention:** first task of P3 (or before P0 if contract FE mirror changes) is to confirm with the operator who owns that WIP, get it committed or stashed on its own branch, and branch the milestone work from a clean base. Do not delete `RiskPopover.tsx` while it has pending edits; check `git show HEAD:<path>` vs working tree before assuming state.
**Phase:** P0 kickoff / P3.

### m3. Playwright visual-verification obligations missed
**What goes wrong:** AGENTS.md §GUI Visual Verification requires a deterministic stepper spec under `crates/vox-gui/ui/e2e/` with `installTauriMock`/`installTauriMockRich`, screenshots into `review-bundle/latest/`, and passing `pnpm --dir crates/vox-gui/ui test:e2e` + `typecheck`. Existing specs that will see layout changes: `chat-composer-dock.spec.ts`, `chat-interactions.spec.ts`, `chat-trust-chips.spec.ts`, `approvals-interactions.spec.ts`; baseline screenshot `review-bundle/latest/chat--composer-filled--wide--chromium.png`. The mock (`e2e/lib/tauriMock.ts`) does not currently model risk/oversight or attention budget state, so new states can't be screenshotted without extending it.
**Prevention:** extend the mock with oversight level, pending approvals, deferred-interruption digest, and per-agent wait; stepper covers: each level selected, migration from legacy state, mid-turn tighten toast, floor-blocked action in sync chat, budget panel with starvation warning, narrow + wide viewports, both themes.
**Phase:** P4 (but mock extension in P3).

### m4. `attention_enabled=false` path labels itself "shadow"
**What goes wrong:** When attention is disabled, every interruption is `InterruptNow` with reason `attention_policy_disabled_or_shadow` (`interruption_policy.rs:138-143`). If the new UI exposes an "off" for the budget, users may expect fewer interruptions but get all of them, and any UI reading the reason string will mislabel the state.
**Prevention:** don't expose "off"; expose budget size. Distinguish disabled vs shadow in the reason enum.
**Phase:** P2.

---

## Phase-Specific Warnings

| Phase | Likely pitfall | Mitigation |
|---|---|---|
| P0 Floor + contract | C1 weak substring floor; C5 id reuse passes parity with inverted semantics; C6 migration inversion | Typed `SafetyFloor` independent of setting; new ids + v2 contract + alias; explicit migration table, default = middle |
| P1 Wiring | C2 sync chat has no approval path; C7 Mode coupling; M1 mid-run semantics; M3 latency/cost | Inline sync approvals; delete risk→axes override; tighten-now/loosen-later; stream-then-verify |
| P2 Attention budget | C3 complacency; C4 fatigue; M2 starvation; m4 disabled≠shadow | Reviewable digest with reasons; batch low-severity; per-agent fair share + aging; no "off" |
| P3 GUI | m1 a11y; M5 spend dedup; M6 copy drift; m2 WIP collision | radiogroup/slider APG; one spend surface; copy as behaviour tied to claim ids; resolve WIP ownership first |
| P4 Verification | C8 green tests without wiring; m3 Playwright obligations | Call-site tests + mutation proofs recorded in PR; extended tauri mock + stepper screenshots |

## Sources

- Repo (HIGH, read this session): `attention/routing.rs:284-332`; `task_dispatch/submit/attention_fields.rs:11-23,32-40,62-106`; `attention/budget.rs:110-133`; `attention/interruption_policy.rs:132-180`; `mode.rs:101-168,213-262,305-311`; `budget_gate.rs:181-193`; `control/mod.rs:23`; `vox-gui/src/drive/protocol.rs:188,227,284,314,337-381`; `contracts/gui/drive-console.v1.yaml`; `contracts/gui/axis-drive.v1.yaml:3,17`; `vox-orchestrator-mcp/src/chat_tools/chat/message.rs:268-280,1180-1190`; `ui/src/hooks/useGroundingCheck.ts:3`; `git diff --stat` of `crates/vox-gui/ui/src`.
- `.planning/workstreams/autonomy-ux/research/AUDIT.md` (baseline claims table).
- AGENTS.md §GUI Visual Verification Invariant, §PR & Review Discipline (mutation verification, never edit another agent's tree).
- Parasuraman, R. & Riley, V. (1997). "Humans and Automation: Use, Misuse, Disuse, Abuse." *Human Factors* 39(2). (MEDIUM — standard reference, not re-fetched)
- Bainbridge, L. (1983). "Ironies of Automation." *Automatica* 19(6). (MEDIUM)
- The Joint Commission, Sentinel Event Alert #50 "Medical device alarm safety in hospitals" (2013). (MEDIUM)
- W3C WAI-ARIA Authoring Practices: Slider and Radio Group patterns; WCAG 2.2 SC 1.4.1, 2.5.7, 1.4.3. (MEDIUM)
