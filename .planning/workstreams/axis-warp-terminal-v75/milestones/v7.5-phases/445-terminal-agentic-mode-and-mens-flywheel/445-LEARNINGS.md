---
phase: 445
phase_name: "terminal-agentic-mode-and-mens-flywheel"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 25
  lessons: 3
  patterns: 9
  surprises: 3
missing_artifacts:
  - "UAT.md"
---

# Phase 445 Learnings: terminal-agentic-mode-and-mens-flywheel

## Decisions

### Task 1 (TERM-AGENT-01, TERM-AGENT-04)
**Task 1 (`TERM-AGENT-01`, `TERM-AGENT-04`):** Added `session_agent.rs` on `Session` (`begin_custom_block`, `append_block_chunk`, `finish_custom_block`, `cancel_custom_block`, `clear_unpinned_blocks`, `compact_blocks`, `control_epoch`, `bump_control_epoch`, `set_max_blocks`) with UTF-8 boundary-safe 256 KiB custom-block byte capping, unpinned-only ring eviction (`DEFAULT_MAX_BLOCKS = 2_000`), `SnapshotResync { resync_required: true, .. }` emission on `/clear` and `/compact`, and `clear` + `compact` entries in `CommandRegistry`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 445-01-SUMMARY.md

---

### Task 2 (TERM-AGENT-03)
**Task 2 (`TERM-AGENT-03`):** Extended `TranscriptSink` with `Send + Sync + 'static` and `flush()`, added 3-stage `scrub_ansi_and_redact_with` / `redact_block_with` in `corpus::redact`, hardened `CorpusWriter::record_with` to scrub both `Accepted { block }` and `Rejected { block }` (`input`, `output`, `cwd`, `git_branch`) and unconditionally drop `credential_prompt == true` blocks before disk writes, added `RedactingTranscriptSink` fan-out (`JournalSink` + `CorpusWriter`), and updated `DefaultPolicy::decide` to retain non-empty non-credential `Rejected` blocks and non-identical `Corrected` events.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 445-01-SUMMARY.md

---

### Task 3 (TERM-AGENT-01, TERM-AGENT-03, TERM-AGENT-04)
**Task 3 (`TERM-AGENT-01`, `TERM-AGENT-03`, `TERM-AGENT-04`):** Extended `SessionHost` (`LocalHost` + `DaemonHost`), fixed the missing `Some("snapshot")` arm in `DaemonHost::route`, added `Autopilot -> Manual` `bump_control_epoch()` + agent PTY `TermSignal::Interrupt` preemption, implemented the 8 `TermDispatch` RPCs in `term_session_ops.rs` enforcing `refuse_agent_origin` on `term.clear`, `term.compact`, and `term.feedback`, wired `LocalHost::open_default_journal_sink` under `.vox/terminal-journal` with `vox_secrets::scrub_resolved_secrets` in `vox_orchestrator_d`, and synced crate API docs via `./target/debug/vox ci crate-api-sync --write`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 445-01-SUMMARY.md

---

### Checked both raw lexed segments (tokenize_and_split_segments, rejecting ...
Checked both raw lexed segments (tokenize_and_split_segments, rejecting redirects, command/process substitutions, and leading ENV=val overrides) and unwrapped effective leaf commands (extract_effective_commands) in can_auto_approve_in_session so wrappers like sudo or inline LD_PRELOAD= assignments cannot smuggle commands past an allowed leaf prefix.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 445-02-SUMMARY.md

---

### Blocked floor escalation inside park_terminal_hitl when ApprovalOutcome:...
Blocked floor escalation inside park_terminal_hitl when ApprovalOutcome::Modified introduces a FloorCategory not present on the original parked approval, closing unauthenticated floor bypass via Modified.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 445-02-SUMMARY.md

---

### Extracted terminal_tools.rs unit tests into terminal_tools_tests.rs via ...
Extracted terminal_tools.rs unit tests into terminal_tools_tests.rs via #[path = "terminal_tools_tests.rs"] so both terminal_tools.rs (317 non-blank LoC) and terminal_approvals.rs (392 non-blank LoC) remain strictly below 420 non-blank LoC.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 445-02-SUMMARY.md

---

### Extended PendingApprovals (pending_approvals.rs) with PendingApprovalMet...
Extended `PendingApprovals` (`pending_approvals.rs`) with `PendingApprovalMeta`, optional terminal metadata fields on `PendingApprovalInfo` (`floor_category`, `session_id`, `command`, `control_mode`, `credential_prompt`), `PendingResolution` (`outcome`, `modified_command`, `steering_note`, `allow_pattern`), sync-safe `PendingWaiter`, and token-boundary session `Ask`-tier pattern storage.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 445-02-SUMMARY.md

---

### Wired vox_resolve_approval (input_schemas/core_and_tasks.rs + dispatch_d...
Wired `vox_resolve_approval` (`input_schemas/core_and_tasks.rs` + `dispatch_domains/tasks_core.rs`) and `orch_daemon_method::RESOLVE_APPROVAL` (`daemon_extra.rs`) through `parse_resolution_from_params` + `resolve_with_detail`, while hardening `floor_gate::refuse_agent_floor_resolution` to inspect `info.floor_category` and `info.credential_prompt == Some(true)` alongside `[floor:` summary prefixes.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 445-02-SUMMARY.md

---

### Implemented terminal_approvals.rs (classify_terminal_floor, session_mode...
Implemented `terminal_approvals.rs` (`classify_terminal_floor`, `session_mode_and_tier`, `can_auto_approve_in_session`, `validate_edited_terminal_command`, `check_autopilot_preempted`, `park_terminal_hitl`, `gate_terminal_exec`) and slimmed `terminal_tools.rs` to delegate terminal HITL gating cleanly.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 445-02-SUMMARY.md

---

### Verified compound smuggling protection across &&, |, ;, >/< redirects, s...
Verified compound smuggling protection across `&&`, `|`, `;`, `>`/`<` redirects, `sudo` wrappers, and `LD_PRELOAD=...` env overrides by checking both `tokenize_and_split_segments` and `extract_effective_commands` alongside `evaluate_command_ast`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 445-02-SUMMARY.md

---

### Checked both tokenize_and_split_segments (rejecting redirections, substi...
Checked both `tokenize_and_split_segments` (rejecting redirections, substitutions, and leading `ENV=val` overrides) and `extract_effective_commands` in `can_auto_approve_in_session` because `extract_effective_commands` intentionally unwraps wrappers (`sudo`) and leading env assignments (`LD_PRELOAD=...`) and ignores redirection flags (`>`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 445-02-SUMMARY.md

---

### Exposed pub(crate) fn running_port(&self) -> Option<u16> on MensServeSta...
Exposed `pub(crate) fn running_port(&self) -> Option<u16>` on `MensServeState` (`mens_serve.rs`) and mapped local MENS HTTP servers to `LlmConfig::direct("ollama", format!("vox-mens:{port}"))` with `.with_base_url(format!("http://127.0.0.1:{port}"))` instead of inventing a non-existent `SecretId::VoxLocalEndpoint`.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 445-03-SUMMARY.md

---

### Mapped SecretId::GeminiApiKey strictly to vox_config::resolve_egress's c...
Mapped `SecretId::GeminiApiKey` strictly to `vox_config::resolve_egress`'s canonical `"gemini"` provider slug (never `"google"`), with zero hardcoded vendor URLs anywhere in `vox-gui`.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 445-03-SUMMARY.md

---

### Preserved 100% of Phase 444 (pub(crate) mod warp;, PtyWarpState, term_hi...
Preserved 100% of Phase 444 (`pub(crate) mod warp;`, `PtyWarpState`, `term_history_*`, `term_complete`, `term_runbooks_list`, `term_session_voxify`, and `pty/warp::tests`) while splitting new async agent/slash/feedback tests into `pty/agent_tests.rs` so every `pty` `.rs` file stays `< 420` non-blank LoC.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 445-03-SUMMARY.md

---

### Verified State<'_, Arc<MensServeState>>, State<'_, Arc<BackendState>>, S...
Verified `State<'_, Arc<MensServeState>>`, `State<'_, Arc<BackendState>>`, `State<'_, Arc<PersistentDaemon>>`, and `State<'_, PtyManager>` against `crates/vox-gui/src/main.rs` `app.manage(...)` registrations so Tauri command extraction never panics at runtime.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 445-03-SUMMARY.md

---

### Added MensServeState::running_port(&self) -> Option<u16> in crates/vox-g...
Added `MensServeState::running_port(&self) -> Option<u16>` in `crates/vox-gui/src/commands/mens_serve.rs` by probing the live child process state (`try_wait().ok().flatten().is_none()`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 445-03-SUMMARY.md

---

### Implemented resolve_terminal_llm_config / resolve_terminal_llm_config_wi...
Implemented `resolve_terminal_llm_config` / `resolve_terminal_llm_config_with` in `crates/vox-gui/src/commands/pty/agent.rs` honoring operator `model_get_active` preferences first, then active local MENS (`LlmConfig::direct("ollama", ...)` with loopback `.with_base_url("http://127.0.0.1:<port>")`), then Clavis `resolve_secret` (`AnthropicApiKey` -> `"anthropic"`, `OpenaiApiKey` -> `"openai"`, `GeminiApiKey` -> `"gemini"`, `OpenRouterApiKey` -> `"openrouter"`), and emitting `CLAVIS_UNCONFIGURED_DIAG` inside a closed `BlockKind::AgentTurn` block (`exit_code = 1`) when unconfigured.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 445-03-SUMMARY.md

---

### Routed 100% of terminal LLM streaming and summarization through vox_acto...
Routed 100% of terminal LLM streaming and summarization through `vox_actor_runtime::llm::{llm_stream_activity, llm_chat}` + `futures_util::StreamExt::next` with dual cancellation (`CancellationToken` + `SessionHost::control_epoch` polling) and 3-stage `scrub_for_agent_inbox` + 8 KiB / 80-line tail capping + `CREDENTIAL_PROMPT_EXPLAIN_DENIED` enforcement on `term_agent_explain_fix`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 445-03-SUMMARY.md

---

### Implemented build_local_host_with_journal / build_local_host_with_journa...
Implemented `build_local_host_with_journal` / `build_local_host_with_journal_at` and `term_block_feedback_impl` in `crates/vox-gui/src/commands/pty/approvals.rs` wiring `.vox/terminal-journal` (`transcript.journal` + `corpus.jsonl` with `vox_secrets::scrub_resolved_secrets`) on `PtyManager::local_host`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 445-03-SUMMARY.md

---

### Implemented term_slash_exec_impl in crates/vox-gui/src/commands/pty/slas...
Implemented `term_slash_exec_impl` in `crates/vox-gui/src/commands/pty/slash.rs` handling `/ai`, `/sh`, `/nu` (`voxify_subshell(Some("nu"))`), `/clear` (`clear_unpinned_blocks`), `/compact` (digest synthesis via `summarize_blocks_with_llm` with deterministic scrubbed Markdown fallback + `compact_blocks(..., 4)`), `/help`, `/model`, `/budget`, `/skills`, `/memory`, `/context`, and unknown `/foo` (`exit_code = 1`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 445-03-SUMMARY.md

---

### Shared Pending Approvals & Agentic Hook (crates/vox-gui/ui/src/hooks/use...
**Shared Pending Approvals & Agentic Hook (`crates/vox-gui/ui/src/hooks/useTerminalPendingApprovals.ts`)**

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 445-04-SUMMARY.md

---

### Added deriveSessionAllowPattern (token-boundary safe prefix matching Pen...
Added `deriveSessionAllowPattern` (token-boundary safe prefix matching `PendingApprovals::matches_leaf_command` semantics without trailing `*`) and `matchesSessionPattern` (rejecting compound shell operators).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 445-04-SUMMARY.md

---

### Implemented useTerminalPendingApprovals
Implemented `useTerminalPendingApprovals`: polls `listTerminalPendingApprovals`, supports local Co-Pilot command gating (`triggerPreemptionNotice`, `resumeCoPilot`, `dismissPreemptedBanner`), handles dual local/daemon `resolveApproval` with inline `Edit & Run` validation errors and `'corrected'` MENS trajectory feedback, dispatches `?` / `/ai` / `/ask` / `agent` prompts (`termAgentPrompt`), `/slash` commands (`termSlashExec`), and shell/`!nu`/`.vox` inputs (`termSessionSubmit`), plus `runExplainFix`, `submitBlockFeedback`, `cancelAgentTurn`, and `try/catch` partial-mock-safe `getTransportFn` helpers.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 445-04-SUMMARY.md

---

### Normalized backend error toasts through sanitizeErrorForToast(err).
Normalized backend error toasts through `sanitizeErrorForToast(err)`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 445-04-SUMMARY.md

---

### Terminal Approval Card & Preemption Banner (TerminalApprovalCard.tsx, Te...
**Terminal Approval Card & Preemption Banner (`TerminalApprovalCard.tsx`, `TerminalApprovalCard.test.tsx`)**

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 445-04-SUMMARY.md

## Lessons

### Initial compile caught ApprovalOutcome::AutoApproved non-exhaustive matc...
Initial compile caught `ApprovalOutcome::AutoApproved` non-exhaustive match arm in `park_terminal_hitl`; handled alongside `ApprovalOutcome::Approved => Ok(None)`. Zero compiler warnings in any touched module.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 445-02-SUMMARY.md

---

### None required to gate Phase 445 completion
None required to gate Phase 445 completion. Live multi-theme Playwright + computer-vision screenshot convergence across all 4 Vox Roma themes (`arcane`, `slate`, `travertine`, `high-contrast`) and live visual SSE token streaming cadence / Carrara & burnished-brass wax-seal polish are tracked in **Phase 446 (`TERM-ROMA-CV-01..04`)** and noted in `445-VALIDATION.md` Manual-Only Verifications.

**Context:** Verified in phase acceptance checks.
**Source:** 445-VERIFICATION.md

---

### No gaps found
No gaps found. All 4 Phase 445 requirements (`TERM-AGENT-01..04`), all 4 ROADMAP Success Criteria, all 10 Cross-Plan Must-Have Truths, all 8 Plan Prohibitions, all 22 Code & UI Review remediations (`445-REVIEW-FIX.md`), all 14 STRIDE threat mitigations (`445-SECURITY.md`), and all 11 Nyquist task gates (`445-VALIDATION.md`) are verified and passing.

**Context:** Verified in phase acceptance checks.
**Source:** 445-VERIFICATION.md

## Patterns

### Terminal HITL approvals populate PendingApprovalMeta and prefix floored ...
Terminal HITL approvals populate PendingApprovalMeta and prefix floored summaries with [floor:<cat>] so floor_gate::refuse_agent_floor_resolution catches floored parks via both summary prefix and structured metadata.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 445-02-SUMMARY.md

---

### Dual-path PendingResolution parsing via shared parse_resolution_from_par...
Dual-path PendingResolution parsing via shared parse_resolution_from_params across MCP dispatch and daemon RPC

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 445-02-SUMMARY.md

---

### Dual-layer AST + raw lexical segment + unwrapped leaf verification for s...
Dual-layer AST + raw lexical segment + unwrapped leaf verification for session Ask-tier command prefix allowlists

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 445-02-SUMMARY.md

---

### Pre-truncation 3-stage ECMA-48 + Clavis + vox_redact scrubbing on operat...
Pre-truncation 3-stage ECMA-48 + Clavis + vox_redact scrubbing on operator steering notes

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 445-02-SUMMARY.md

---

### Explain & Fix 3-stage context scrubbing
Explain & Fix 3-stage context scrubbing: `scrub_for_agent_inbox` + 8 KiB / 80-line tail cap + unconditional `CREDENTIAL_PROMPT_EXPLAIN_DENIED` refusal on `credential_prompt == true` blocks before any LLM call.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 445-03-SUMMARY.md

---

### Pure ASCII - separators across all pty/agent.rs and pty/slash.rs user-fa...
Pure ASCII `-` separators across all `pty/agent.rs` and `pty/slash.rs` user-facing strings (zero middle dots `·` and zero emojis/dingbats).

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 445-03-SUMMARY.md

---

### 100% model-agnostic terminal LLM routing via vox_actor_runtime::llm::{Ll...
100% model-agnostic terminal LLM routing via `vox_actor_runtime::llm::{LlmConfig::direct, llm_stream_activity, llm_chat}` + `futures_util::StreamExt::next` with Clavis `SecretId` resolution (`"gemini"` for `GeminiApiKey`) and local MENS discovery via `MensServeState::running_port` (`"ollama"` loopback `base_url`)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 445-03-SUMMARY.md

---

### Dual cancellation (CancellationToken + SessionHost::control_epoch pollin...
Dual cancellation (`CancellationToken` + `SessionHost::control_epoch` polling) guaranteeing cancelled/preempted `AgentTurn` blocks close as `BlockStatus::Cancelled`

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 445-03-SUMMARY.md

---

### Submodule decomposition (pty/agent.rs, pty/slash.rs, pty/approvals.rs, p...
Submodule decomposition (`pty/agent.rs`, `pty/slash.rs`, `pty/approvals.rs`, `pty/tests.rs`, `pty/agent_tests.rs`) keeping every `pty` Rust file strictly `< 420` non-blank LoC

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 445-03-SUMMARY.md

## Surprises

### Estimate variance on 445-01
Estimate variance on 445-01: planned 58000 tokens vs 13500 actual (0.23x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 445-01-PLAN.md, 445-01-SUMMARY.md

---

### Estimate variance on 445-02
Estimate variance on 445-02: planned 48000 tokens vs 14500 actual (0.30x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 445-02-PLAN.md, 445-02-SUMMARY.md

---

### Estimate variance on 445-03
Estimate variance on 445-03: planned 58000 tokens vs 28000 actual (0.48x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 445-03-PLAN.md, 445-03-SUMMARY.md

