# Phase 442: AST Exec-Policy Gate, Daemon `term.*` Control Plane & Bidirectional MCP/Chat Terminal Harness - Context

**Gathered:** 2026-11-14
**Status:** Ready for planning

<domain>
## Phase Boundary

Expose shared `SessionHost` terminal execution, streaming inspection, interactive stall detection, AST-backed `L1–L5` command policy gating, and backend-redacted `@term` citations to Chat and MCP agents (`vox_terminal_exec`, `vox_terminal_read`, `vox_terminal_send_input`, `vox_terminal_sessions`) over `vox-orchestrator::terminal_bridge` with zero new workspace Cargo edges (`TERM-HARNESS-01`, `TERM-HARNESS-02`, `TERM-HARNESS-03`, `TERM-HARNESS-04`).

</domain>

<decisions>
## Implementation Decisions

### TerminalBridge, Catalog Sync & Foreground/Background Execution (`TERM-HARNESS-01`)
- Define the pure-Rust async `TerminalBridge` trait and wire DTOs in `vox_orchestrator::terminal_bridge` (`L3`) so `vox-orchestrator-mcp` (`L3`), `vox-terminal-core` (`L3`), `vox-orchestrator-d` (`L5`), and `vox-gui` (`L5`) connect with **zero new workspace Cargo edges** in `contracts/ci/crate-edges.allow.v1.json`.
- `vox-orchestrator-d` injects an in-process `LocalHostTerminalBridge` wrapping its singleton `Arc<LocalHost>` into MCP `ServerState`, while standalone stdio `vox mcp` uses an `OrchDaemonClient` `term.*` JSON-RPC adapter (falling back to an ephemeral in-proc host only when the daemon is offline).
- `vox_terminal_exec` defaults to `wait_ms = 10_000` (configurable `0..=120_000`); when `background: true` is passed, when `Ctrl+B` promotes an active foreground job, or when `wait_ms` expires while the command is still running, return `status: "running"` (`session_id`, `block_id`, `seq`, redacted output prefix) **without** killing the PTY job so `vox_terminal_read` continues streaming.
- When `session_id` is omitted, bind `chat_thread_id → session_id` inside `ServerState` (reusing the bound or latest live session, or opening a `120×32` default session on first exec) so multi-turn Chat and the Console Dock share stateful `cwd` and environment variables.
- Register `vox_terminal_exec`, `vox_terminal_read`, `vox_terminal_send_input`, and `vox_terminal_sessions` at their sorted positions in `contracts/operations/catalog.v1.yaml`, run `operations-sync --write` to update `contracts/mcp/tool-registry.canonical.yaml` / `TOOL_REGISTRY`, and expose them in `llm_bridge::tool_selection` without mutating `FROZEN_PREFIX_CORE_TOOLS`.

### Pager Neutralization, Stall Detection & AST `ExecPolicyV1` + `term.control` (`TERM-HARNESS-02`, `TERM-HARNESS-03`)
- Scope non-interactive pager environment defaults (`PAGER=cat`, `GIT_PAGER=cat`, `SYSTEMD_PAGER=cat`, `BAT_PAGER=cat`, `MANPAGER=cat`, `GH_PAGER=cat`; PowerShell/Nushell equivalents on `pwsh`/`nu`) to `BlockOrigin::Agent` command executions so `git log`, `git diff`, `gh`, and `systemctl` never hang on `less`, while leaving human-typed interactive commands untouched.
- Transition running blocks to `BlockStatus::AwaitingInput` immediately when `ALT_SCREEN` activates (`?1049h` / `?47h` / `?1047h`) or after `1_500 ms` of PTY output quiescence when the unterminated trailing line matches interactive prompt signatures (`[y/N]`, `(y/n)`, `Password:`, `passphrase`, `[sudo]`, `Continue?`), returning `status: "awaiting_input"` and prompt context instead of hanging the agent loop.
- Host the shared pure-Rust POSIX + PowerShell lexical/pipeline `ExecPolicyV1` (`contracts/terminal/exec-policy.v1.yaml`) + `classify_floor` evaluator in `vox_orchestrator::oversight::exec_policy` (consumed by `vox-cli-research::check_terminal`, `vox-terminal-core`, and `vox-orchestrator-mcp`), blocking `$()`, backticks, process substitution, `L1` redirections, and wrapper-binary (`pwsh -EncodedCommand`, `bash -c`, `sh -c`, `find -exec`/`-delete`, `env`, `xargs`) bypasses across `L1`–`L5`.
- Persist and synchronize `TermSecurityLevel` (`L1`–`L5`) and `ControlMode` (`Manual`, `CoPilot`, `Autopilot`) via `term.security` and `term.control` daemon RPCs (`CoPilot` routes mutating/above-floor commands through `PendingApprovals` HITL; `Manual` blocks unapproved agent execution).
- Fail-closed on `vox_terminal_send_input` from `BlockOrigin::Agent` whenever `AwaitingInput` classified a credential/secret prompt (`Password:`, `passphrase`, `[sudo]`), requiring the human operator to enter secrets directly in the attached terminal pane.

### `@term` Citations, Backend ANSI + `vox-redact` / Clavis Scrubbing & Context Budgets (`TERM-HARNESS-04`)
- Parse `@term` mentions via a dedicated `TERM_MENTION_RE` in `crates/vox-orchestrator-mcp/src/chat_tools/chat/mentions.rs` (running before file `MENTION_RE` so `#` and `:L` tokens survive intact), supporting `@term`, `@term:last`, `@term:failed`, `@term:pinned` (`Block.pinned: bool` + `term.pin` RPC), `@term:<sid>`, `@term:<sid>#<id>`, and 1-indexed inclusive `@term:<sid>#<id>:L<a-b>`.
- Enforce strict backend scrubbing order across `vox_terminal_*`, `@term`, `term.context`, and `SendToAgent`: (1) strip all ANSI CSI/OSC/DCS escape sequences and normalize `\r` overwrites first, (2) run upgraded `vox-redact` covering `/Users/<name>`, `/home/<name>`, `C:\Users\<name>`, PEM private keys, URL `user:pass@host` credentials, and `AIza`/`hf_`/`npm_`/`ghp_`/`sk-` tokens, and (3) scrub exact matches of resolved Clavis secret values (`vox-secrets`, length `>= 6` → `[REDACTED:CLAVIS]`).
- Enforce a `16 KiB` / `200`-line default per-block budget using **head `40` lines + tail `160` lines** (preserving exit diagnostics) with a pagination hint for `vox_terminal_read(session_id, block_id, line_start, line_end)`, and a `32 KiB` total `@term` prompt envelope per turn.
- Route `SendToAgent` via structured `{ session_id, block_id, line_range }` / `@term:<sid>#<id>[:L<a-b>]` references resolved on the Rust backend so raw unredacted frontend DOM text never enters an LLM prompt.

### the agent's Discretion
- Exact method signatures on `TerminalBridge` and exact wire field names on `term.control` / `term.pin` / `term.promote_bg` so long as `LocalHost`, `DaemonHost`, `TermDispatch`, and `vox-orchestrator-mcp` stay symmetric and stay under the `< 500` non-blank LoC `arch/god_object` budget per file.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/vox-terminal-core/src/{host.rs,session.rs,block.rs,daemon_host.rs,utf8_stream.rs,vt_shadow.rs}` (upgraded in Phase 441): Stateful PTY sessions, `TermSecurityLevel` (`L1`–`L5`), `ControlMode` (`Manual`, `CoPilot`, `Autopilot`), `VtShadow` `ALT_SCREEN` tracking, `normalize_cr_overwrites`, and `2 MiB` `after_seq` delta ring.
- `crates/vox-orchestrator-d/src/term_dispatch.rs`: Authenticated JSON-RPC dispatcher (`term.open`, `term.list`, `term.attach`, `term.input`, `term.submit`, `term.signal`, `term.resize`, `term.close`, `term.security`, `term.agent_command`, `term.context`).
- `crates/vox-cli-research/src/check_terminal.rs` & `contracts/terminal/exec-policy.v1.yaml`: Existing `ExecPolicyV1` schema and contract file.
- `crates/vox-redact/src/lib.rs` & `crates/vox-terminal-core/src/corpus/redact.rs`: Existing redaction primitives to unify/expand.
- `crates/vox-orchestrator-mcp/src/chat_tools/chat/mentions.rs`: Existing `RULE_MENTION_RE` pre-pass pattern before `MENTION_RE`.

### Established Patterns
- `contracts/operations/catalog.v1.yaml` is the SSOT for MCP/CLI operations: edit only at sorted row positions (never re-serialize the whole YAML file per `00-core.md` Rule 11) and run `operations-sync --write` to regenerate `contracts/mcp/tool-registry.canonical.yaml`.
- `vox-arch-check` enforces `layers.toml`, `crate-edges.allow.v1.json`, `< 500` non-blank LoC per new/modified module (`god_object`), and `[no-hardcoded-abs-path]` (`"/usr`, `"/bin`, `"/home`, `"/tmp` forbidden in non-`#[cfg(test)]` Rust literals — build path regexes dynamically or via character classes like `r"/(?:Users|home)/[^/\s]+"` without literal `"/home` or `"/usr` substrings!).

### Integration Points
- `crates/vox-orchestrator/src/{lib.rs,terminal_bridge.rs,oversight/floor.rs,oversight/exec_policy.rs}`
- `crates/vox-orchestrator-d/src/{term_dispatch.rs,bin/vox_orchestrator_d.rs}`
- `crates/vox-orchestrator-mcp/src/{server_state.rs,permission_modes.rs,floor_gate.rs,llm_bridge/tool_selection.rs,chat_tools/chat/mentions.rs}` + new `terminal_schemas.rs` / `terminal_tools.rs`
- `crates/vox-cli-research/src/check_terminal.rs`
- `crates/vox-redact/src/lib.rs`

</code_context>

<specifics>
## Specific Ideas

- Remember `vox-arch-check`'s `[no-hardcoded-abs-path]` rule when implementing `/Users/<name>` and `/home/<name>` path redaction in `vox-redact`: construct patterns without literal `"/usr`, `"/bin`, `"/etc`, `"/tmp`, or `"/home` substrings if flagged by `vox-arch-check`, and verify with `cargo run -p vox-arch-check`.
- Follow `00-core.md` Rule 11 strictly on `contracts/operations/catalog.v1.yaml`: insert the 4 `vox_terminal_*` rows at their sorted position with a targeted edit and generate downstream files via `operations-sync --write` (never re-serialize `catalog.v1.yaml`).

</specifics>

<deferred>
## Deferred Ideas

- Full Roman wax-seal UI rendering for Co-Pilot Approval Cards and `Console.tsx` / `ChatTranscript.tsx` React components (scheduled in Phases 443 and 445; Phase 442 wires the backend/MCP/RPC approval & session plumbing).

</deferred>
