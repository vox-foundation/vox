# Roadmap: Vox

## Milestones

- ✅ **v1.0 Architectural Housekeeping** — Phases 1–6 (shipped 2026-10-01)
- 🚧 **v1.1 Research Trial Flywheel** — Phases 7–11 (parallel workstream `research-trial-flywheel`)
- 🚧 **v1.2 Unified Autonomy Control** — Phases 12–19 (this workstream, `autonomy-ux`)

## Overview

v1.2 replaces four unrelated controls (inverted Risk popover, Check replies, Approvals PermissionMode selector, read-only attention meter) with one Oversight control — Relaxed · Standard · Strict, more = safer — whose every claimed effect is enforced in the daemon on both sync chat and background tasks. It is built safety-first: a typed hard floor and closed lowering paths land before any level can loosen anything (Phase 12). Then come a contract-driven resolver (13), daemon-held session state behind an authenticated RPC (14), the two execution-path consumers in parallel (15 ∥ 16), and one attention admission point with inbox, quotas and digest (17). The GUI is unified on top, coordinating with another agent's uncommitted WIP per D7 (18). The milestone closes with a claims ledger that proves every shown consequence is backed by a test (19). Mode (cost↔intelligence) stays a separate axis throughout.

## Phases

**Phase Numbering:**

- Integer phases (12, 13, …): Planned milestone work
- Decimal phases (12.1, 12.2): Urgent insertions (marked with INSERTED)

### 🚧 v1.2 Unified Autonomy Control

- [ ] **Phase 12: Safety Floor & Lowering Paths** - Typed level-independent hard floor (D3) and every agent path that can lower oversight closed, each guard mutation-proven
- [ ] **Phase 13: Oversight Contract & Resolver** - `oversight-levels.v1.yaml`, Mode-only `drive-console.v2`, one `oversight::resolve()` with effect rows, id-keyed monotonic parity gates, legacy `risk` alias
- [ ] **Phase 14: Daemon Session State & RPC** - Per-session level seeded from a saved per-user default, settable only via authenticated `orch.oversight_{get,set}`
- [ ] **Phase 15: Background Task Consumers** - Background approval/grounding/Socrates resolved from effective policy; Review blocks reach the inbox; tighten-now/loosen-later semantics
- [ ] **Phase 16: Sync Chat Consumers** - Effective permission threaded into chat tool dispatch; daemon-side reply check derived from level; chat/background parity
- [ ] **Phase 17: Attention Admission & Inbox** - One `admit()` for every human prompt with "why this reached you", per-agent fair-share quotas, digest, rubber-stamp-proof trust
- [ ] **Phase 18: GUI Unification** - One Oversight radiogroup with daemon-provided consequences, inbox/digest/budget panels, legacy migration, retired controls, Playwright review bundle
- [ ] **Phase 19: Claims Ledger & Close-out Verification** - Every GUI consequence string backed by a behavioural test; removed claims absent everywhere

## Phase Details

### Phase 12: Safety Floor & Lowering Paths

**Goal**: Before any level can relax anything, the D3 floor actions always require a human and no agent-supplied input can lower effective oversight.
**Depends on**: Nothing (first v1.2 phase; independent of v1.1)
**Requirements**: FLOOR-01, FLOOR-02, FLOOR-03, FLOOR-05
**Research flag**: Needs phase research — structured floor classification sources (tool name + args, file manifest, caps) and whether `vox_preference_set` is offered to chat agents (`llm_bridge/tool_selection.rs`).
**Success Criteria** (what must be TRUE):

  1. A force-push / history rewrite, an allowlist/policy grant, a `vox_deploy`/external publish, and a secrets write each park for human approval under `accept_all` and under the most permissive trust score, classified from the action (tool + args), not task-description wording; the floor function's signature takes no oversight level.
  2. `vox_deploy` has `always_requires_approval: true` in both `permission-modes.v1.yaml` and `permission_modes.rs`, and the YAML↔Rust parity test fails if either side drifts.
  3. An agent submitting a task with `risk:"high"`, calling `vox_preference_set` on `socrates_gate_enforce` / `attention_*` / `oversight.*`, or holding high trust cannot reduce configured grounding/Socrates enforcement — hints combine by `max`, the forbidden keys are refused with an error, and `trust_relax_gates` no longer bypasses Socrates unconditionally.
  4. Mutation proof recorded for every guard above: removing the guard call (verified absent by `grep -c`) makes its call-site test fail, and restoring it makes the test pass.

**Plans**: TBD

### Phase 13: Oversight Contract & Resolver

**Goal**: One contract and one pure resolver define what each oversight level does, with drift-proof parity gates and no coupling to Mode.
**Depends on**: Phase 12 (contract encodes the floor)
**Requirements**: OVR-01, OVR-02, OVR-03, OVR-04, OVR-05, OVR-06
**Success Criteria** (what must be TRUE):

  1. `contracts/orchestration/oversight-levels.v1.yaml` lists `relaxed|standard|strict` with background effects, chat effects, floors, and invariants; `drive-console.v2.yaml` contains Mode (`clutch`) only.
  2. `oversight::resolve()` returns an `EffectivePolicy` plus human-readable effect rows from the same data; Relaxed maps chat to AcceptEdits, Standard/Strict to Ask, and no level maps to AcceptAll.
  3. Parity gates match rows by id, fail on extra or missing rows, and fail when a level's strictness is reordered (an inverted-id reuse is rejected); a TS vitest fails if GUI level labels or order drift from the contract.
  4. Choosing any oversight level leaves model selection unchanged (the `effective_axes` Low-risk override is deleted).
  5. A drive-API call with legacy `risk=high` resolves to Relaxed (the least oversight) via the inversion table and logs a deprecation warning; the alias carries `vox-deprecated-since`/`retire-by` markers.

**Plans**: TBD

### Phase 14: Daemon Session State & RPC

**Goal**: The active level lives in the daemon, per session, and only an authenticated human-facing RPC can lower it.
**Depends on**: Phase 13
**Requirements**: STATE-01, FLOOR-04
**Success Criteria** (what must be TRUE):

  1. Setting a level for a session survives daemon restart; a new session starts from the user's saved default; missing or corrupt state rehydrates to at least Standard — all stored in existing `user_preferences` with no schema change.
  2. `orch.oversight_set` lowers a level only over the authenticated transport; it is absent from the MCP tool catalog, and an MCP/agent attempt to lower is refused while raising is accepted.
  3. The GUI reads effective policy via `orch.oversight_get` from the daemon, not from the GUI-process `snapshot()`.
  4. Mutation proof: removing the authentication check or the escalate-only guard on the set path makes its call-site test fail.

**Plans**: TBD

### Phase 15: Background Task Consumers

**Goal**: The session's oversight level governs background tasks, and blocked reviews become visible to the user.
**Depends on**: Phase 14 (can run in parallel with Phase 16)
**Requirements**: BG-01, BG-02, STATE-02
**Success Criteria** (what must be TRUE):

  1. A background task submitted at Relaxed vs Strict gets the approval tier, grounding, and Socrates enforcement listed for that level by the resolver, with `effective = max(floor, level, trust)`.
  2. A Review-tier blocked background task appears in `vox_pending_approvals` / the approval inbox instead of only in `hitl_approvals`.
  3. Raising the level while a task is queued or running applies at its next gate (`max(stamped, current)`); lowering it leaves already-enqueued tasks at their stamped level and affects only new tasks.
  4. Chat-spawned tasks inherit the turn's level as an escalate-only hint.

**Plans**: TBD

### Phase 16: Sync Chat Consumers

**Goal**: The same level governs sync chat tool calls and reply checks, so chat and background behave the same.
**Depends on**: Phase 14 (can run in parallel with Phase 15)
**Requirements**: CHAT-01, CHAT-02, CHAT-03
**Success Criteria** (what must be TRUE):

  1. At Relaxed a chat file edit runs without a prompt; at Standard and Strict it shows an inline bounded "waiting for you" approval; the permission mode reaches dispatch from authenticated context, never from `ChatMessageParams` args (`message.rs:351` no longer passes `None`).
  2. Reply confidence checking (`reply_confidence_check`) runs in the daemon at Standard and Strict as a non-blocking badge, is off at Relaxed, and the GUI-process heuristic and separate toggle path are gone.
  3. Each turn emits an `oversight_applied` event naming the level and effects applied.
  4. A parity test runs equivalent actions (file edit, floor action) via chat and background at each level and gets the same approve/ask/floor outcome.

**Plans**: TBD

### Phase 17: Attention Admission & Inbox

**Goal**: Every human prompt passes one explainable admission point that protects the user's attention fairly across agents without ever hiding a floor or deferring a blocker.
**Depends on**: Phase 15 and Phase 16 (full emitter set)
**Requirements**: ATTN-01, ATTN-02, ATTN-03, ATTN-04, ATTN-06
**Success Criteria** (what must be TRUE):

  1. Tool approvals, review blocks, and clarifications all enter one inbox through `admit()`: floor items always surface, blocking approvals are never deferred, and other items are scored against a per-level threshold.
  2. `orch.inbox_explain` returns for each item the level, action class, trust score, floor/tier, and originating agent.
  3. With one agent flooding prompts, another agent's legitimate prompt still surfaces (per-agent fair share with aging), and a blocked task's lock-hold time is bounded.
  4. Deferred non-blocking items, including silent trust auto-approvals, are delivered as a digest at natural breaks and remain listable with their reasons (`promote_withheld` no longer a no-op).
  5. Rapid repeated approvals do not raise agent trust, and mutation proof shows removing the floor-first or never-defer-blocking check fails its test.

**Plans**: TBD

### Phase 18: GUI Unification

**Goal**: The user sees and controls oversight and attention through one coherent composer control and inbox, rendering only what the daemon returns.
**Depends on**: Phase 14 (control can start), Phase 17 (inbox, digest, budget)
**Requirements**: GUI-01, GUI-02, GUI-03, GUI-04, GUI-05, GUI-06, STATE-03, ATTN-05
**Gate (D7)**: First task resolves ownership of the uncommitted Loquela/Chat/Popover/Tooltip edits (land them or rebase onto them); nothing in those files is overwritten.
**Success Criteria** (what must be TRUE):

  1. The composer shows one Oversight radiogroup (Relaxed · Standard · Strict) next to Mode, operable by arrow keys/Home/End with visible focus, in a single hue that deepens with oversight; Risk popover, Check replies toggle, and Approvals PermissionMode selector are gone, and Approvals reflects the session level.
  2. Each stop's popover lists the daemon-provided consequences plus a locked "always asks" floor row; spend appears once in the composer area, with the status bar keeping the breakdown.
  3. An existing user with risk `high` / Check replies on opens the app and lands deterministically on the mapped level (Check replies on ⇒ at least Standard).
  4. The inbox/digest panel shows "why this reached you" per item, and the user can change the attention budget from the GUI; the `attention-config-parity` gate points at real files and fails on missing ones before the budget writer lands.
  5. `pnpm --dir crates/vox-gui/ui typecheck` and `test:e2e` pass; the Playwright stepper (with `tauriMock` extended for the new RPCs) covers every level, the floor row, inbox, and digest, writing screenshots to `review-bundle/latest/`.

**Plans**: TBD
**UI hint**: yes

### Phase 19: Claims Ledger & Close-out Verification

**Goal**: Nothing the GUI or contracts claim about oversight is unbacked, and removed claims are gone everywhere.
**Depends on**: Phases 15–18
**Requirements**: VERIF-01, VERIF-02
**Success Criteria** (what must be TRUE):

  1. A claims-ledger test maps every consequence string shown in the GUI and every contract claim id to an existing behavioural call-site test, and fails when a string is added without one.
  2. Safety-token and delegate-to-free claims are absent from all GUI copy and contracts (a test fails if reintroduced).
  3. `cargo mutants` scoped to the floor/resolver/admission gate files finds no surviving mutants in the guards, and a latency/cost check per (Mode × Oversight) shows no unexpected regression.

**Plans**: TBD

## Parallelism

- 12 → 13 → 14 is the critical path.
- 15 and 16 run in parallel after 14.
- 18's Oversight control can start once 14 lands; its inbox/digest/budget work waits on 17.
- 19 closes after 15–18.

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 12. Safety Floor & Lowering Paths | 0/TBD | Not started | - |
| 13. Oversight Contract & Resolver | 0/TBD | Not started | - |
| 14. Daemon Session State & RPC | 0/TBD | Not started | - |
| 15. Background Task Consumers | 0/TBD | Not started | - |
| 16. Sync Chat Consumers | 0/TBD | Not started | - |
| 17. Attention Admission & Inbox | 0/TBD | Not started | - |
| 18. GUI Unification | 0/TBD | Not started | - |
| 19. Claims Ledger & Close-out Verification | 0/TBD | Not started | - |
