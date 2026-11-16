# Phase 46: Terminal Session Host CLI Attach, Agent `term_run` Tool & Chat Context (`Tasks H7–H8`) - Context

**Gathered:** 2026-10-14
**Status:** Ready for execution
**Mode:** Autonomous (`/gsd-autonomous`)

<domain>
## Phase Boundary

Complete the backend and CLI halves of `docs/src/architecture/terminal-session-host-design-2026.md`:
- `46-01` (`TERM-01`): Add `BlockOrigin` (`User` | `Agent`) to `vox_terminal_core::block::Block`, `Session::open_block_with_origin`, and `LocalHost::submit_with_origin`.
- `46-02` (`TERM-02`): Wire `vox-term` (`vox term`) to auto-detect `vox-orchestrator-d` (`DaemonHost` default with `LocalHost` fallback), support `--attach <session_id>` and `--local`, and populate the session palette (`sessions_hint`).
- `46-03` (`TERM-03`): Add `TermDispatch::run_agent_command` (`term.agent_run`) gated by `vox_exec_grammar::evaluate_shell` (`Forbidden` rejected, `Risky` requires `approved: true`) and executing with `BlockOrigin::Agent`.
- `46-04` (`TERM-04`): Add `LocalHost::redacted_context_bundle(id, max_blocks)` scrubbed via `vox_redact::scrub` and expose `term.context` in `TermDispatch`.

</domain>

<decisions>
## Implementation Decisions

- **No AGPL-3.0 Warp code (`ADR-026`)**: All terminal session hosting, OSC-633 block tracking, and agent bridges live in our Apache-2.0 crates (`vox-terminal-core`, `vox-term`, `vox-orchestrator-d`).
- **Wire-level `origin: "agent"` refusal preserved**: External JSON-RPC callers to `term.submit` cannot spoof `origin: "agent"`; only `TermDispatch::run_agent_command` (`term.agent_run`) after passing `vox-exec-grammar` policy evaluation may open an `Agent`-origin block.
- **Secret scrubbing before LLM context**: `LocalHost::redacted_context_bundle` passes every block's `input` and `plain_output()` through `vox_redact::scrub` before returning.

</decisions>
