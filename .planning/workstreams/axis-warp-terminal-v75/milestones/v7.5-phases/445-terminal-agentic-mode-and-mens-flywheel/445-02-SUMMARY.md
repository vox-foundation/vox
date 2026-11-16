---
phase: 445-terminal-agentic-mode-and-mens-flywheel
plan: 02
subsystem: mcp-hitl-approvals
tags: [terminal, hitl, pending-approvals, exec-policy, safety-floor, co-pilot, mens]

# Dependency graph
requires:
  - phase: 443-shell-command-safety-and-exec-policy
    provides: "ExecPolicyV1 AST/segment evaluator, SecurityTier L1-L5, and FloorCategory safety floor classifier"
provides:
  - "PendingApprovalMeta + enriched PendingApprovalInfo (floor_category, session_id, command, control_mode, credential_prompt) with sync-safe backward-compatible PendingWaiter"
  - "PendingResolution (outcome, modified_command, steering_note, allow_pattern) wired identically across MCP vox_resolve_approval (tasks_core.rs) and Daemon RPC RESOLVE_APPROVAL (daemon_extra.rs)"
  - "Compound-safe session Ask-tier pattern allowlist (can_auto_approve_in_session) blocking pipes, redirects, subshells, wrappers, env overrides, L1ReadOnly mutations, floor_category, and credential_prompt"
  - "Edit & Run AST + SecurityTier + floor-escalation re-validation, Reject + Steer 3-stage secret scrubbing, and Autopilot -> Manual preemption"
affects:
  - "445-03 (vox-gui terminal agent / approval commands)"
  - "445-04 (TerminalApprovalCard & ChatTerminalBlockCard UI)"

actuals:
  tokens: 14500
  tasks: 2
  commits: 0

tech-stack:
  added: []
  patterns:
    - "Dual-path PendingResolution parsing via shared parse_resolution_from_params across MCP dispatch and daemon RPC"
    - "Dual-layer AST + raw lexical segment + unwrapped leaf verification for session Ask-tier command prefix allowlists"
    - "Pre-truncation 3-stage ECMA-48 + Clavis + vox_redact scrubbing on operator steering notes"

key-files:
  created:
    - crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_approvals.rs
    - crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools_tests.rs
  modified:
    - crates/vox-orchestrator-mcp/src/pending_approvals.rs
    - crates/vox-orchestrator-mcp/src/floor_gate.rs
    - crates/vox-orchestrator-mcp/src/input_schemas/core_and_tasks.rs
    - crates/vox-orchestrator-mcp/src/dispatch_domains/tasks_core.rs
    - crates/vox-orchestrator-mcp/src/daemon_extra.rs
    - crates/vox-orchestrator-mcp/src/dispatch_domains/mod.rs
    - crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs

key-decisions:
  - "Checked both raw lexed segments (tokenize_and_split_segments, rejecting redirects, command/process substitutions, and leading ENV=val overrides) and unwrapped effective leaf commands (extract_effective_commands) in can_auto_approve_in_session so wrappers like sudo or inline LD_PRELOAD= assignments cannot smuggle commands past an allowed leaf prefix."
  - "Blocked floor escalation inside park_terminal_hitl when ApprovalOutcome::Modified introduces a FloorCategory not present on the original parked approval, closing unauthenticated floor bypass via Modified."
  - "Extracted terminal_tools.rs unit tests into terminal_tools_tests.rs via #[path = \"terminal_tools_tests.rs\"] so both terminal_tools.rs (317 non-blank LoC) and terminal_approvals.rs (392 non-blank LoC) remain strictly below 420 non-blank LoC."

patterns-established:
  - "Terminal HITL approvals populate PendingApprovalMeta and prefix floored summaries with [floor:<cat>] so floor_gate::refuse_agent_floor_resolution catches floored parks via both summary prefix and structured metadata."

requirements-completed:
  - TERM-AGENT-03

coverage:
  - id: D1
    description: "PendingApprovals supports sync-safe register/resolve alongside register_with_meta, PendingResolution, and dual-path MCP (vox_resolve_approval) + Daemon RPC (RESOLVE_APPROVAL) resolution while preserving floor_gate self-approval refusal"
    requirement: TERM-AGENT-03
    verification:
      - kind: unit
        ref: "crates/vox-orchestrator-mcp/src/pending_approvals.rs#sync_register_resolve_backward_compat_and_register_with_meta"
        status: pass
      - kind: unit
        ref: "crates/vox-orchestrator-mcp/src/daemon_extra.rs#dual_path_resolve_approval_forwards_detail_and_validates_modified"
        status: pass
      - kind: unit
        ref: "crates/vox-orchestrator-mcp/src/floor_gate.rs#refuse_agent_floor_resolution_matrix"
        status: pass
    human_judgment: false
  - id: D2
    description: "Compound-safe session Ask-tier allowlist (can_auto_approve_in_session) auto-approves matching leaf prefixes while rejecting compound chains, pipes, redirects, sudo wrappers, inline env overrides, L1ReadOnly mutations, floor_category, and credential prompts"
    requirement: TERM-AGENT-03
    verification:
      - kind: unit
        ref: "crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_approvals.rs#session_allowlist_compound_tier_floor_and_credential_guards"
        status: pass
      - kind: integration
        ref: "crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools_tests.rs#edit_and_run_allow_pattern_reject_steer_and_autopilot_preemption_e2e"
        status: pass
    human_judgment: false
  - id: D3
    description: "Edit & Run re-validates modified_command against ExecPolicyV1 AST, L1ReadOnly SecurityTier, and safety floor escalation before executing modified_command; Reject + Steer scrubs secrets via scrub_terminal_mcp_str; Autopilot -> Manual preemption clears session patterns and aborts execution"
    requirement: TERM-AGENT-03
    verification:
      - kind: unit
        ref: "crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_approvals.rs#validate_edited_terminal_command_enforces_ast_and_l1_tier"
        status: pass
      - kind: unit
        ref: "crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_approvals.rs#park_hitl_reject_steer_scrubs_secrets_and_autopilot_preempts"
        status: pass
      - kind: integration
        ref: "crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools_tests.rs#edit_and_run_allow_pattern_reject_steer_and_autopilot_preemption_e2e"
        status: pass
    human_judgment: false

duration: 32min
completed: 2026-11-15
status: complete
---

# Phase 445 Plan 02: Terminal HITL PendingApprovals Contract & Gate Upgrade Summary

**Rich terminal HITL metadata and `PendingResolution` (`Edit & Run`, compound-safe `Allow Pattern in Session`, scrubbed `Reject + Steer`, and `Autopilot -> Manual` preemption) wired across MCP and Daemon RPC**

## Performance

- **Duration:** 32 min
- **Started:** 2026-11-15T21:06:22Z
- **Completed:** 2026-11-15T21:38:45Z
- **Tasks:** 2
- **Files modified:** 9 (2 created, 7 modified; all inside `crates/vox-orchestrator-mcp/src/`)

## Accomplishments

- Extended `PendingApprovals` (`pending_approvals.rs`) with `PendingApprovalMeta`, optional terminal metadata fields on `PendingApprovalInfo` (`floor_category`, `session_id`, `command`, `control_mode`, `credential_prompt`), `PendingResolution` (`outcome`, `modified_command`, `steering_note`, `allow_pattern`), sync-safe `PendingWaiter`, and token-boundary session `Ask`-tier pattern storage.
- Wired `vox_resolve_approval` (`input_schemas/core_and_tasks.rs` + `dispatch_domains/tasks_core.rs`) and `orch_daemon_method::RESOLVE_APPROVAL` (`daemon_extra.rs`) through `parse_resolution_from_params` + `resolve_with_detail`, while hardening `floor_gate::refuse_agent_floor_resolution` to inspect `info.floor_category` and `info.credential_prompt == Some(true)` alongside `[floor:` summary prefixes.
- Implemented `terminal_approvals.rs` (`classify_terminal_floor`, `session_mode_and_tier`, `can_auto_approve_in_session`, `validate_edited_terminal_command`, `check_autopilot_preempted`, `park_terminal_hitl`, `gate_terminal_exec`) and slimmed `terminal_tools.rs` to delegate terminal HITL gating cleanly.
- Verified compound smuggling protection across `&&`, `|`, `;`, `>`/`<` redirects, `sudo` wrappers, and `LD_PRELOAD=...` env overrides by checking both `tokenize_and_split_segments` and `extract_effective_commands` alongside `evaluate_command_ast`.

## Non-Blank LoC Verification (`< 420` Budget)

| File | Non-Blank LoC (`rg -c '\S'`) |
|---|---|
| `crates/vox-orchestrator-mcp/src/dispatch_domains/mod.rs` | 9 |
| `crates/vox-orchestrator-mcp/src/input_schemas/core_and_tasks.rs` | 195 |
| `crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs` | 317 |
| `crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools_tests.rs` | 334 |
| `crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_approvals.rs` | 392 |
| `crates/vox-orchestrator-mcp/src/pending_approvals.rs` | 396 |
| `crates/vox-orchestrator-mcp/src/dispatch_domains/tasks_core.rs` | 404 |
| `crates/vox-orchestrator-mcp/src/daemon_extra.rs` | 408 |
| `crates/vox-orchestrator-mcp/src/floor_gate.rs` | 409 |

## Task Commits

Shared-worktree parallel execution rule enforced: no git index/commit operations were run by the executor; changes remain ready on disk for coordinator verification and atomic commit.

## Files Created/Modified

- `crates/vox-orchestrator-mcp/src/pending_approvals.rs` - `PendingApprovalMeta`, `PendingResolution`, `parse_resolution_from_params`, `PendingWaiter`, and session pattern allowlist helpers
- `crates/vox-orchestrator-mcp/src/floor_gate.rs` - Refuses agent-originated resolution when `floor_category.is_some()` or `credential_prompt == Some(true)`
- `crates/vox-orchestrator-mcp/src/input_schemas/core_and_tasks.rs` - Optional `modified_command`, `steering_note`, and `allow_pattern` properties on `vox_resolve_approval`
- `crates/vox-orchestrator-mcp/src/dispatch_domains/tasks_core.rs` - Forwards `PendingResolution` via `resolve_with_detail` after `refuse_agent_floor_resolution`
- `crates/vox-orchestrator-mcp/src/daemon_extra.rs` - Daemon `RESOLVE_APPROVAL` parity with `tasks_core.rs`
- `crates/vox-orchestrator-mcp/src/dispatch_domains/mod.rs` - Declares `pub(crate) mod terminal_approvals;`
- `crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_approvals.rs` - Compound-safe session allowlist gate, `Edit & Run` AST/tier/floor validator, `Reject + Steer` scrubber, and `Autopilot -> Manual` preemption check
- `crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs` - Wires `gate_terminal_exec` and `park_terminal_hitl` into `vox_terminal_exec` and `vox_terminal_send_input`
- `crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools_tests.rs` - Extracted + expanded E2E terminal MCP tests covering `Edit & Run`, `Allow Pattern in Session`, `Reject + Steer`, and `Autopilot -> Manual` preemption

## Verification Proof

All test suites executed natively against `/Users/brbrainerd/dev/vox/target/ws-axis-warp-terminal-v75`:

1. `cargo test -p vox-orchestrator-mcp pending_approvals`
   - `pending_approvals::tests` (`3 passed; 0 failed`) + `daemon_extra_tests::daemon_list_and_resolve_pending_approvals_via_extra_dispatch` (`1 passed; 0 failed`)
2. `cargo test -p vox-orchestrator-mcp terminal_approvals`
   - `dispatch_domains::terminal_approvals::tests` (`3 passed; 0 failed`)
3. `cargo test -p vox-orchestrator-mcp terminal`
   - `8 passed; 0 failed` (including `edit_and_run_allow_pattern_reject_steer_and_autopilot_preemption_e2e`, `terminal_mcp_schemas_scrubbing_sticky_hitl_and_credential_guard`, `scrub_utf8_ecma48_background_wait_and_pinned_filter_before_slice`)
4. `cargo test -p vox-orchestrator-mcp daemon_extra`
   - `3 passed; 0 failed` (including `dual_path_resolve_approval_forwards_detail_and_validates_modified`)
5. `cargo test -p vox-orchestrator-mcp floor_gate`
   - `9 passed; 0 failed` (including `refuse_agent_floor_resolution_matrix`)
6. `cargo test -p vox-orchestrator-mcp --test pending_approvals_tests` & `--test pending_approvals_floor_tests`
   - `15 passed; 0 failed` and `8 passed; 0 failed`

## Decisions Made

- Checked both `tokenize_and_split_segments` (rejecting redirections, substitutions, and leading `ENV=val` overrides) and `extract_effective_commands` in `can_auto_approve_in_session` because `extract_effective_commands` intentionally unwraps wrappers (`sudo`) and leading env assignments (`LD_PRELOAD=...`) and ignores redirection flags (`>`).
- Prevented floor-category escalation during `ApprovalOutcome::Modified` in `park_terminal_hitl` unless the original approval already carried that `floor_category`.
- Scrubbed `steering_note` via `scrub_terminal_mcp_str` *before* UTF-8 byte-cap truncation (`1024` bytes) so secrets straddling the truncation boundary cannot evade redaction.

## Deviations from Plan

None - plan executed as specified (with `terminal_tools.rs` unit tests extracted into `terminal_tools_tests.rs` as anticipated by Task 2 Step 3 to stay well under `420` non-blank LoC).

## Issues Encountered

- Initial compile caught `ApprovalOutcome::AutoApproved` non-exhaustive match arm in `park_terminal_hitl`; handled alongside `ApprovalOutcome::Approved => Ok(None)`. Zero compiler warnings in any touched module.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `vox-orchestrator-mcp` HITL approval pipeline is ready for Wave 2 (`445-03`) `vox-gui` command wrappers and Wave 3 (`445-04`) `<TerminalApprovalCard>` / `<ChatTerminalBlockCard>` UI integration.

---
*Phase: 445-terminal-agentic-mode-and-mens-flywheel*
*Completed: 2026-11-15*
