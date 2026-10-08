# Requirements — v1.2 Unified Autonomy Control (workstream `autonomy-ux`)

**Goal:** One human-legible Oversight control (Relaxed · Standard · Strict; more = safer) that replaces the inverted Risk popover and the Check replies toggle, whose every claimed effect is wired in the daemon for both sync chat and background tasks, plus a visible, controllable cross-agent attention system. Mode (cost↔intelligence) stays a separate axis.

Research: `research/SUMMARY.md` (+ AUDIT, STACK, FEATURES, ARCHITECTURE, PITFALLS).

## Decisions (user, 2026-10-08)

- D1: Three stops — Relaxed · Standard · Strict; new ids, legacy `high/moderate/low` mapped by explicit inversion table.
- D2: Relaxed auto-approves chat file edits; Standard and Strict ask.
- D3: Hard floor at every level = force-push / history rewrite, allowlist / policy grants, deploy / external publish, and secrets writes. Shell and delete are level-governed (not floored).
- D4: Level is per session, seeded from a saved per-user default.
- D5: Oversight is the only control — the Approvals view permission-mode selector is retired.
- D6: Full attention scope — inbox, "why this reached you", level-aware admission, per-agent fair-share quotas, batched digest.
- D7: Uncommitted Loquela/Chat/Popover/Tooltip edits belong to another agent — GUI phase must coordinate (land or rebase onto them), never overwrite.
- D8: Legacy drive-API `risk` key kept one release as a deprecated alias with a `vox-deprecated-since`/`retire-by` marker.
- Safety tokens and the unused delegate-to-free / budget-aggressiveness helpers are removed from oversight copy and moved to the Mode backlog.

## v1.2 Requirements

### Safety floor & lowering paths (FLOOR)

- [ ] **FLOOR-01**: Force-push / history-rewrite and allowlist/policy-grant actions require human approval at every oversight level, enforced by a typed floor that does not take the oversight level as input (effective tier = max(floor, level, trust)).
- [ ] **FLOOR-02**: Deploy/external-publish (including `vox_deploy`, auto-approved under accept-all today at `permission_modes.rs:87-92`) and secrets writes require human approval at every level, classified by action type rather than task-description keywords.
- [ ] **FLOOR-03**: No agent-supplied input can lower the effective oversight: task hints, `risk` on submitted tasks (`socrates.rs` `hint.unwrap_or`), `vox_preference_set`, and trust-based gate relaxation can only raise strictness.
- [ ] **FLOOR-04**: Only an authenticated daemon RPC (not an MCP tool) can lower a session's oversight level.
- [ ] **FLOOR-05**: Every floor and lowering-path guard has a call-site test proven by mutation (guard removed ⇒ test fails).

### Contract & resolver (OVR)

- [ ] **OVR-01**: `contracts/orchestration/oversight-levels.v1.yaml` defines the three levels, their background and chat effects, floors, and invariants (strictness monotonic in level order; oversight never changes Mode axes).
- [ ] **OVR-02**: `drive-console` contract moves to v2 containing Mode only; parity gates match rows by id, fail on extra/missing rows, and assert monotonic strictness so an inverted id reuse cannot pass.
- [ ] **OVR-03**: One `oversight::resolve()` produces an `EffectivePolicy` consumed by both sync chat and background gates, including human-readable "what this level does" rows generated from the same data.
- [ ] **OVR-04**: The Low-risk axes override (`effective_axes`) is removed — oversight level never changes model selection.
- [ ] **OVR-05**: User can keep using the legacy `risk` key for one release; it maps through the inversion table and logs a deprecation warning.
- [ ] **OVR-06**: A TS test reads the contract so the GUI's level labels/order cannot drift from it.

### Session state (STATE)

- [ ] **STATE-01**: User's chosen level persists per session and new sessions start from the user's saved default (stored in existing `user_preferences`; no schema change).
- [ ] **STATE-02**: Tightening the level mid-turn applies to running work immediately; loosening applies only to new work.
- [ ] **STATE-03**: Existing users migrate deterministically: old risk → new level via table; "Check replies on" lifts the migrated level to at least Standard.

### Background tasks (BG)

- [ ] **BG-01**: Background tasks resolve approval tier, grounding, and Socrates enforcement from the session's effective policy.
- [ ] **BG-02**: Review-tier blocked background tasks reach the user's approval inbox (today only dangerous-tool parks surface).

### Sync chat (CHAT)

- [ ] **CHAT-01**: Sync chat passes the effective permission setting to tool dispatch (`message.rs:351` no longer `None`), so Relaxed auto-approves edits and Standard/Strict ask.
- [ ] **CHAT-02**: Reply verification ("Check replies") is derived from the level — on at Standard and Strict — and runs in the daemon, replacing the separate toggle and the GUI-process check.
- [ ] **CHAT-03**: User sees the same oversight behavior in chat and background work for equivalent actions.

### Attention & interruptions (ATTN)

- [ ] **ATTN-01**: Every human prompt (tool approvals, review blocks, clarifications) flows through one inbox with a single admission function: floors always surface, blocking approvals are never deferred, the rest are scored with a per-level threshold.
- [ ] **ATTN-02**: Each surfaced item shows "why this reached you" (level, action class, trust score, floor/tier, originating agent).
- [ ] **ATTN-03**: Each agent gets a fair share of the interruption budget with aging, so one chatty agent cannot starve others; blocked tasks have a bounded lock-hold time.
- [ ] **ATTN-04**: Deferred, non-blocking items are batched into a digest delivered at natural breaks, and remain reviewable with their reasons (including silent trust auto-approvals).
- [ ] **ATTN-05**: User can adjust the attention budget from the GUI (not only via MCP/CLI); the dead `attention-config-parity` gate is repointed at real files.
- [ ] **ATTN-06**: Rubber-stamp approvals (very fast, repeated) do not raise agent trust.

### GUI (GUI)

- [ ] **GUI-01**: Composer shows one compact Oversight segmented radiogroup (Relaxed · Standard · Strict; keyboard arrow navigation; single hue deepening with oversight, no red/green) next to Mode; the Risk popover and Check replies toggle are removed.
- [ ] **GUI-02**: Each stop shows its daemon-provided consequences and a locked "always asks" row for floor actions.
- [ ] **GUI-03**: Spend is shown once in the composer area (duplicate composer-footer readout removed; status bar remains the breakdown).
- [ ] **GUI-04**: The Approvals view permission-mode selector is retired; Approvals reflects the session's Oversight level.
- [ ] **GUI-05**: Inbox/digest UI renders ATTN-01..04 items with "why this reached you".
- [ ] **GUI-06**: Playwright visual stepper covers every level, the floor row, inbox, and digest with review-bundle screenshots; `tauriMock` extended for the new RPCs.

### Verification (VERIF)

- [ ] **VERIF-01**: A claims ledger test fails if any consequence string shown in the GUI lacks a backing behavioral test.
- [ ] **VERIF-02**: Removed/unwired claims (safety tokens, delegate-to-free) are absent from all GUI copy and contracts.

## Future Requirements

- Per-agent oversight overrides.
- "You'd have been asked N times in the last hour" preview per level.
- Org-level minimum oversight (OrchestratorConfig field).
- Safety-token budgeting and delegate-to-free behavior (Mode backlog).

## Out of Scope

- A "never interrupt" stop — research shows zero-notification raises anxiety and complacency; floors make it impossible anyway.
- Merging Mode into Oversight (2D pad) — user chose separate axes.
- New crates or new crate edges — logic lives in `vox-orchestrator`, consumers in `vox-orchestrator-mcp`, RPC constants in `vox-foundation`, `vox-gui` forwards only.
- Continuous slider — named stops are the cross-industry norm and map to discrete policy.

## Traceability

| Requirement | Phase | Status |
|---|---|---|
| (filled by roadmapper) | | |
