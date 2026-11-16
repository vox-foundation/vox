---
phase: 442-bidirectional-chat-harness-and-ast-security
verified: 2026-11-15T12:57:00Z
status: passed
score: 29/29 must-haves verified
covered_files:
  - .planning/workstreams/axis-warp-terminal-v75/REQUIREMENTS.md
  - .planning/workstreams/axis-warp-terminal-v75/ROADMAP.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/442-bidirectional-chat-harness-and-ast-security/442-01-PLAN.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/442-bidirectional-chat-harness-and-ast-security/442-01-SUMMARY.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/442-bidirectional-chat-harness-and-ast-security/442-02-PLAN.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/442-bidirectional-chat-harness-and-ast-security/442-02-SUMMARY.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/442-bidirectional-chat-harness-and-ast-security/442-03-PLAN.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/442-bidirectional-chat-harness-and-ast-security/442-03-SUMMARY.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/442-bidirectional-chat-harness-and-ast-security/442-04-PLAN.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/442-bidirectional-chat-harness-and-ast-security/442-04-SUMMARY.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/442-bidirectional-chat-harness-and-ast-security/442-05-PLAN.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/442-bidirectional-chat-harness-and-ast-security/442-05-SUMMARY.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/442-bidirectional-chat-harness-and-ast-security/442-06-PLAN.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/442-bidirectional-chat-harness-and-ast-security/442-06-SUMMARY.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/442-bidirectional-chat-harness-and-ast-security/442-REVIEW-FIX.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/442-bidirectional-chat-harness-and-ast-security/442-REVIEW.md
  - contracts/mcp/http-read-role-governance.yaml
  - contracts/mcp/tool-registry.canonical.yaml
  - contracts/operations/catalog.v1.yaml
  - contracts/orchestration/permission-modes.v2.yaml
  - contracts/terminal/exec-policy.v1.yaml
  - crates/vox-cli-research/src/check_terminal.rs
  - crates/vox-gui/src/commands/console_a2a.rs
  - crates/vox-gui/src/commands/pty.rs
  - crates/vox-gui/src/main.rs
  - crates/vox-gui/ui/src/transport.ts
  - crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs
  - crates/vox-orchestrator-d/src/lib.rs
  - crates/vox-orchestrator-d/src/term_bridge_impl.rs
  - crates/vox-orchestrator-d/src/term_dispatch.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/mentions.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/mod.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/term_mentions.rs
  - crates/vox-orchestrator-mcp/src/dispatch.rs
  - crates/vox-orchestrator-mcp/src/dispatch_domains/mod.rs
  - crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs
  - crates/vox-orchestrator-mcp/src/dispatch_timeout.rs
  - crates/vox-orchestrator-mcp/src/input_schemas.rs
  - crates/vox-orchestrator-mcp/src/input_schemas/terminal.rs
  - crates/vox-orchestrator-mcp/src/llm_bridge/tool_selection.rs
  - crates/vox-orchestrator-mcp/src/permission_modes.rs
  - crates/vox-orchestrator-mcp/src/server_state.rs
  - crates/vox-orchestrator-mcp/tests/fixtures/dev-advertised-tools.default-features.txt
  - crates/vox-orchestrator/src/lib.rs
  - crates/vox-orchestrator/src/oversight/exec_policy.rs
  - crates/vox-orchestrator/src/oversight/floor.rs
  - crates/vox-orchestrator/src/oversight/mod.rs
  - crates/vox-orchestrator/src/terminal_bridge.rs
  - crates/vox-orchestrator/src/terminal_bridge_ephemeral.rs
  - crates/vox-redact/src/lib.rs
  - crates/vox-secrets/src/lib.rs
  - crates/vox-secrets/src/redact.rs
  - crates/vox-term/src/app.rs
  - crates/vox-term/src/ui/blocks.rs
  - crates/vox-terminal-core/src/agent_exec.rs
  - crates/vox-terminal-core/src/block.rs
  - crates/vox-terminal-core/src/corpus/redact.rs
  - crates/vox-terminal-core/src/daemon_host.rs
  - crates/vox-terminal-core/src/host.rs
  - crates/vox-terminal-core/src/lib.rs
  - crates/vox-terminal-core/src/pty.rs
  - crates/vox-terminal-core/src/session.rs
  - crates/vox-terminal-core/src/shell_init.rs
  - crates/vox-terminal-core/src/stall_detector.rs
covered_digest: "v1:sha256:264877b12d04147a7b87644e34ee62aa93655775c346ddf7700043332b763fe3"
behavior_unverified: 0
overrides_applied: 0
---

# Phase 442: Bidirectional Chat Harness & AST Security Verification Report

**Phase Goal:** Agents and users collaborate safely over the same live PTY sessions through structured MCP tools, interactive hang protection, AST-aware command approval, and redacted `@term` block context.
**Verified:** 2026-11-15T12:57:00Z
**Status:** passed
**Re-verification:** No — initial verification (incorporating `442-REVIEW-FIX.md` remediation)

## Goal Achievement

### Observable Truths

#### ROADMAP Success Criteria (4/4)

| # | Success Criterion | Status | Evidence |
| --- | --- | --- | --- |
| SC-1 | Chat/MCP agents can list sessions, execute commands in the user's active or background PTY session (`cd`/`export` state preserved), read block outputs, and respond to non-credential interactive prompts via `vox_terminal_exec`, `vox_terminal_read`, `vox_terminal_send_input`, and `vox_terminal_sessions`. | ✓ VERIFIED | All 4 tools registered in `contracts/operations/catalog.v1.yaml:35682-35765`, `input_schemas/terminal.rs`, and `dispatch_domains/terminal_tools.rs:1-602`, wired through `ServerState::{resolve_terminal_bridge, bind_or_resolve_terminal_session}` (`server_state.rs:261-349`) into `LocalHostTerminalBridge` (`term_bridge_impl.rs`) / `LocalHost::{exec_agent_command, agent_send_input}` (`agent_exec.rs`) with `EphemeralTerminalBridge` fallback (`terminal_bridge_ephemeral.rs`). Verified by `vox-orchestrator-mcp --lib` (`1,078` passed) and `vox-orchestrator-d --lib` (`35/35` passed). |
| SC-2 | Agent-invoked commands never hang indefinitely on `git`/`less`/`man` pagers, stalled interactive prompts (`[y/N]`, password/passphrase, REPL) are surfaced within 2 seconds without auto-injecting credentials, and long-running dev servers can be promoted to background tasks (`Ctrl+B` or timeout handoff). | ✓ VERIFIED | `shell_init.rs` injects non-subshell `__vox_pager_enter` + `__vox_pager_restore` across `zsh`, `bash`, `fish`, `pwsh`, and `nu` setting `PAGER=cat`, `GIT_PAGER=cat`, `LESS=FRX`, `MANPAGER=cat`, `BAT_PAGER=cat`, `DELTA_PAGER=cat` while preserving `cd`/`export` and `$?`/`$status`/`$LASTEXITCODE`; `stall_detector.rs` (`DEFAULT_STALL_QUIESCENCE_MS = 1_500`) classifies `ALT_SCREEN` immediately and `[y/N]`/`Credential`/`PressEnter`/`Selection` prompts within 1.5s; `CREDENTIAL_PROMPT_USER_ONLY` blocks agent input at both `terminal_tools.rs` and `agent_exec.rs`/`session.rs`; `promote_bg` (`Ctrl+B` / `wait_ms == 0` / timeout handoff) detaches the agent wait loop while leaving the PTY job running. |
| SC-3 | Compound commands, pipelines, subshells, process substitutions, redirects, and shell wrappers (`sh -c`, `env`, `xargs`, `find -exec`, `pwsh -EncodedCommand`) are parsed and evaluated against `contracts/terminal/exec-policy.v1.yaml` and session control mode (`manual` / `co_pilot` / `autopilot`), parking out-of-policy commands in HITL approval before a byte reaches the PTY. | ✓ VERIFIED | Pure-Rust `oversight::exec_policy` (`exec_policy.rs:1-789`) tokenizes POSIX + PowerShell quotes/escapes/operators (`MAX_DEPTH = 6`), unwraps `sh`/`bash`/`zsh`/`dash`/`ksh`/`fish`/`nu`/`cmd`/`pwsh`/`powershell`, `env`, `sudo`, `nohup`, `timeout`, `nice`, `time`, `command`, `builtin`, `exec`, `stdbuf`, `setsid`, `xargs`, and `find -exec`/`-execdir`/`-ok`/`-okdir`/`-delete`, and gates `floor.rs` (`SHELL_COMMAND_TOOLS`), `terminal_tools.rs` (`PendingApprovals` with `outer_hitl_approved` single-park semantics), `agent_exec.rs`, and `term_bridge_impl.rs` (`validate_daemon_agent_exec`). User `submit`/`input` demotes `Autopilot -> Manual` (`host.rs`). |
| SC-4 | Referencing `@term`, `@term:last`, `@term:failed`, `@term:pinned`, or `"Send to Agent"` injects ANSI-stripped, Clavis-secret-redacted, head/tail-truncated block output with line-range citations into the chat turn without leaking secrets or home paths. | ✓ VERIFIED | `term_mentions.rs:1-737` resolves all 7 `@term` selector forms prior to `resolve_mentions_with_rules` (`message.rs:118-177`), scrubs via 3-stage ECMA-48/CR + Clavis `scrub_resolved_secrets` (`[REDACTED:CLAVIS]`) + `vox_redact` (`~[REDACTED_PATH]`, `[REDACTED_PEM]`, `[REDACTED_URL_CRED]`, `[REDACTED_KEY]`), escapes XML envelope tag breakouts (`WR-04`), caps `<terminal_context>` at `32 KiB`, and `console_a2a.rs:28-182` resolves `send_terminal_block_to_agent` backend-side via `vox_terminal_read` with `@term:{sid}#{bid}:L{s}-{e}` citations. |

#### Plan Must-Have Truths (25/25)

| # | Plan | Truth | Status | Evidence |
| --- | --- | --- | --- | --- |
| 1 | 442-01 | Compound commands (`&&`, `||`, `;`, `|`, `&\n`, `\r\n`) are split into distinct segments outside single/double quotes before allowlist evaluation — a second segment (`echo ok; rm -rf /`) can never ride on the first segment's allowlist hit. | ✓ VERIFIED | `tokenize_and_split_segments` (`exec_policy.rs:208-395`) tracks single/double/PowerShell backtick/backslash quoting, splits unquoted `;`, `&&`, `||`, `|`, `&`, `\n`, `\r\n`, flushes trailing `\` (`CR-02`), and evaluates every segment independently. |
| 2 | 442-01 | Subshells `(...)`, command substitutions `$(...)` and `` `...` `` (outside POSIX single quotes), process substitutions `<(...)` / `>(...)`, and PowerShell subexpressions `$(...)` / `@(...)` fail closed on `L1..L3` and in `ExecPolicyV1`. | ✓ VERIFIED | `exec_policy.rs:290-395,545-645` flags `has_subshell`, `has_command_subst`, and `has_process_subst` across unquoted and double-quoted tokens while exempting POSIX `'...'` literal `$()` / backticks, blocking on `ExecPolicyV1` and `SecurityTier::L1..L3`. |
| 3 | 442-01 | Output/heredoc redirections (`>`, `>>`, `2>`, `&>`, `>|`, `<<`, `<<<`) are blocked on `L1` (`ReadOnly`) and in read-only `ExecPolicyV1`. | ✓ VERIFIED | `exec_policy.rs:320-365,555-635` detects unquoted redirection operators (`has_output_redirect` / `has_heredoc`) and denies them in `evaluate_command_ast` and `SecurityTier::L1`. |
| 4 | 442-01 | Shell wrappers (`sh -c`, `bash -lc`, `zsh -c`, `pwsh -Command`, `pwsh -EncodedCommand`, `cmd /c`, `env VAR=1 <cmd>`, `sudo`, `nohup`, `timeout`, `xargs`, `find ... -exec` / `-execdir` / `-delete`) recursively unwrap inner commands (bounded depth) or fail closed on opaque encoded payloads (`-EncodedCommand` / `-enc`). | ✓ VERIFIED | `extract_effective_commands` (`exec_policy.rs:425-542`) enforces `MAX_DEPTH = 6`, recursively unwraps inner wrapper argv/script payloads, blocks `find -delete`, and blocks PowerShell `-e..` / `-c..` / `/c` / `/encodedcommand` prefix & slash flags (`CR-03`). |
| 5 | 442-01 | `oversight::floor::classify_floor` inspects every unwrapped command segment for `vox_run_shell`, `vox_skill_run`, and `vox_terminal_exec`, catching `echo ok && git push --force` as `Irreversible` even under `AcceptAll`. | ✓ VERIFIED | `SHELL_COMMAND_TOOLS` (`floor.rs:36-40`) lists all 3 tools; `classify_floor` (`floor.rs:375-426`) checks `"command"` and `"cmd"` through `extract_effective_commands` and flags history-rewriting git verbs in any segment (`CR-06`). |
| 6 | 442-01 | `vox-cli-research::check_terminal` delegates its pure-Rust command check to `vox_orchestrator::oversight::exec_policy` so `vox ci exec-policy-contract` and runtime terminal gating share one canonical implementation. | ✓ VERIFIED | `check_terminal.rs:14-18,175-210` re-exports `ExecPolicyV1` and delegates `run_check_rust_fallback` directly to `evaluate_command_ast`. Confirmed live via `./target/debug/vox ci exec-policy-contract` (`exit 0`). |
| 7 | 442-02 | `vox_redact::redact` scrubs home paths on macOS (`/Users/<name>`), Linux (`/home/<name>`), and Windows (`C:\Users\<name>`), PEM private keys, `scheme://user:pass@host` URL credentials, and modern API tokens (`AIza`, `hf_`, `npm_`, `ghp_`, `github_pat_`, `glpat-`, `xox[baprs]-`, `sk-ant-`, `sk-proj-`, `sk-`). | ✓ VERIFIED | `vox-redact/src/lib.rs:24-118` orders `RE_PEM` and `RE_URL_CRED` ahead of `RE_EMAIL`, expands `RE_TOKEN`, and uses non-capturing `(?:` home-directory matching on `/Users/`, `/home/`, and `[A-Za-z]:\Users\` → `~[REDACTED_PATH]`. |
| 8 | 442-02 | `vox_secrets::redact::scrub_resolved_secrets` replaces every resolved Clavis credential value (`len >= 6`, excluding non-secret config URLs/usernames) using `aho-corasick` `LeftmostLongest` matching with `[REDACTED:CLAVIS]` without exposing raw secrets outside `vox-secrets`. | ✓ VERIFIED | `vox-secrets/src/redact.rs:1-195` filters `SPECS` via `is_scrubbable_secret_spec`, resolves through `resolve_secret` (zero `std::env::var`), enforces `MIN_CLAVIS_SCRUB_LEN = 6`, and builds `AhoCorasickBuilder::new().match_kind(MatchKind::LeftmostLongest)` replacing matches with `"[REDACTED:CLAVIS]"`. |
| 9 | 442-02 | `Block::plain_output` and `vox_terminal_core::corpus::redact` strip full ECMA-48 CSI (including private `?`/`>`/`!`), OSC (`BEL` and `ESC \`), DCS/SOS/PM/APC strings, 2-byte ESC sequences, C0 controls, and `\r` progress-bar overwrites before running secret/path redaction. | ✓ VERIFIED | `strip_ansi_and_normalize` (`block.rs:110-198`) runs `normalize_cr_overwrites` first and strips full ECMA-48 CSI/OSC/DCS/SOS/PM/APC/2-byte ESC/C0 sequences; `scrub_ansi_and_redact` (`corpus/redact.rs:34-48`) applies `strip_ansi_and_normalize` before `vox_redact::redact_owned`. |
| 10 | 442-02 | Large block outputs truncate deterministically to head 40 lines + tail 160 lines (hard cap `16 KiB` per block) on UTF-8 char boundaries with an explicit omission marker citing `@term:<session_id>#<block_id>:L<omitted_start>-<omitted_end>`. | ✓ VERIFIED | `truncate_terminal_block` (`corpus/redact.rs:60-165`) enforces `DEFAULT_BLOCK_HEAD_LINES = 40`, `DEFAULT_BLOCK_TAIL_LINES = 160`, `DEFAULT_BLOCK_MAX_BYTES = 16 * 1024`, UTF-8 `floor_char_boundary`/`ceil_char_boundary`, and emits exact `@term:<sid>#<bid>:L<start>-<end>` omission markers. |
| 11 | 442-02 | `Block` carries `pinned: bool` (`#[serde(default)]`), `credential_prompt: bool` (`#[serde(default)]`), and `BlockStatus::AwaitingInput` without breaking Phase 441 wire compatibility. | ✓ VERIFIED | `block.rs:23,67,69` adds `BlockStatus::AwaitingInput` and `#[serde(default)]` fields `pinned` and `credential_prompt`; `vox-term` (`app.rs:244-259`, `ui/blocks.rs:23`) exhaustively renders `AwaitingInput` in `Color::Cyan`. |
| 12 | 442-03 | Agent-submitted commands execute inside the existing persistent shell PTY (not a subshell) with `PAGER=cat`, `GIT_PAGER=cat`, `LESS=FRX`, `MANPAGER=cat`, `BAT_PAGER=cat`, and `DELTA_PAGER=cat` set and restored across `zsh`, `bash`, `fish`, `pwsh`, and `nu` so `cd`/`export` mutations and exact `$?`/`$status`/`$LASTEXITCODE` exit codes are preserved. | ✓ VERIFIED | `shell_init.rs` defines in-shell `__vox_pager_enter` and `__vox_pager_restore` functions across all 5 shells; `format_agent_command_for_shell` (`agent_exec.rs:78-96`) prepends `__vox_pager_enter; ` without spawning a subshell, and shell prompt hooks capture exit status before calling `__vox_pager_restore`. |
| 13 | 442-03 | When a running agent block stops emitting bytes and either enters `ALT_SCREEN` or ends on a non-newline prompt line matching credential (`password:`, `passphrase`, `[sudo]`, `enter pin`, `otp`, `token:`) or interactive (`[y/n]`, `(yes/no)`, `continue?`, `press enter`) patterns within `1_500 ms` of quiescence, `LocalHost::exec_agent_command` returns `BlockStatus::AwaitingInput` within `< 2 s` instead of hanging until timeout. | ✓ VERIFIED | `stall_detector.rs:14-142` (`DEFAULT_STALL_QUIESCENCE_MS = 1_500`) triggers immediately on `alt_screen` (`credential_prompt: false`) and after `>= 1500ms` quiescence on non-`\n` prompt tails; `LocalHost::exec_agent_command` (`agent_exec.rs:265-435`) polls every `50ms` and transitions the block via `mark_block_awaiting_input`. |
| 14 | 442-03 | Credential prompts set `block.credential_prompt = true`, emit `SessionEvent::BlockAwaitingInput { credential_prompt: true, .. }`, and `agent_send_input` rejects every call on a `credential_prompt` block (`CREDENTIAL_PROMPT_USER_ONLY`) — fail-closed. | ✓ VERIFIED | `session.rs:475-525` sets `credential_prompt` and rejects `Origin::Agent` in `resume_awaiting_block`; `LocalHost::agent_send_input` (`agent_exec.rs:442-510`) also checks `b.credential_prompt` fail-closed before touching the PTY master. |
| 15 | 442-03 | Typing or submitting as `Origin::User` while a session is in `TermControlMode::Autopilot` immediately transitions the session to `TermControlMode::Manual` and emits `SessionEvent::ControlModeChanged`. | ✓ VERIFIED | `LocalHost::{input, submit}` (`host.rs:860-935`) atomically demotes `TermControlMode::Autopilot` to `TermControlMode::Manual` on `Origin::User` activity and broadcasts `SessionEvent::ControlModeChanged`. |
| 16 | 442-03 | `promote_bg` (`Ctrl+B` or `background=true` / timeout handoff) detaches the agent wait loop and leaves the foreground job running in that PTY session without sending `SIGINT`, returning `{ session_id, block_id, status: "running", promoted_bg: true }`. | ✓ VERIFIED | `LocalHost::exec_agent_command` (`agent_exec.rs:292-312,402-432`) returns immediately when `wait_ms == 0` (`WR-02`) and detaches on `promote_bg` notify without sending `SIGINT`; `LocalHost::promote_bg` (`agent_exec.rs:530-555`) notifies waiters and marks the active block promoted. |
| 17 | 442-04 | `vox_terminal_exec`, `vox_terminal_read`, `vox_terminal_send_input`, and `vox_terminal_sessions` are registered in `contracts/operations/catalog.v1.yaml`, generated into `contracts/mcp/tool-registry.canonical.yaml`, exposed with strict JSON Schemas (`additionalProperties: false`), and dispatched through `vox-orchestrator-mcp` without adding a `vox-orchestrator-mcp -> vox-terminal-core` crate dependency. | ✓ VERIFIED | Verified in `catalog.v1.yaml:35682-35765`, `tool-registry.canonical.yaml:3054-3081`, `input_schemas/terminal.rs:1-161`, `dispatch.rs:415,736-745`, and `vox-orchestrator-mcp/Cargo.toml` (zero `vox-terminal-core` dependency; `./target/debug/vox ci operations-verify` and `./target/debug/vox-arch-check` both exit 0). |
| 18 | 442-04 | Before a single byte reaches the PTY, `vox_terminal_exec` evaluates the command through `ExecPolicyV1` + session `SecurityTier` (`L1..L5`) + `TermControlMode` (`manual` always parks, `co_pilot` parks out-of-allowlist / `L3+` mutations, `autopilot` executes within `L1..L3` and parks `L4..L5` or `ExecPolicyV1` blocks) using `ServerState.pending_approvals`, while `floor.rs` enforces non-bypassable `Irreversible` HITL even under `AcceptAll`. | ✓ VERIFIED | `dispatch.rs:233-343` + `terminal_tools.rs:210-335` evaluate `classify_floor`, `PermissionMode`, `ExecPolicyV1`, `SecurityTier`, and `TermControlMode`, threading `outer_hitl_approved` (`CR-05`) so `Ask` mode parks once in `PendingApprovals` and `Manual` mode still parks under `AcceptAll`. Verified by `pending_approvals_tests`, `pending_approvals_floor_tests`, and `floor_entry_points_tests` (`24/24` passed). |
| 19 | 442-04 | `vox_terminal_send_input` refuses `credential_prompt == true` blocks unconditionally (`CREDENTIAL_PROMPT_USER_ONLY` — never even offerable via HITL) and parks non-credential interactive input in `PendingApprovals` whenever `control_mode == Manual`. | ✓ VERIFIED | `handle_terminal_send_input` (`terminal_tools.rs:402-495`) reads target block state first, returns hard `CREDENTIAL_PROMPT_USER_ONLY` error when `credential_prompt` is true before reaching `PendingApprovals`, and parks `Manual` sessions unless `outer_hitl_approved`. |
| 20 | 442-05 | `vox-gui` exposes typed Tauri commands `term_session_set_security`, `term_session_set_control`, `term_session_pin`, `term_session_promote_bg`, and `send_terminal_block_to_agent`, all registered in `crates/vox-gui/src/main.rs` and wrapped via `safeInvoke` in `crates/vox-gui/ui/src/transport.ts` using canonical snake_case wire strings (`l1_read_only`..`l5_unrestricted`, `manual`/`co_pilot`/`autopilot`). | ✓ VERIFIED | Registered in `crates/vox-gui/src/main.rs:199-211`, implemented in `commands/pty.rs:470-561` (`manager.host_for(daemon.inner()).await`) and `commands/console_a2a.rs:102-182`, and wrapped in `transport.ts:62-77,1045-1094,1210-1235` outside the raw-IPC fence. |
| 21 | 442-05 | `send_terminal_block_to_agent` resolves block content on the Rust backend via `BackendCalls::call_tool("vox_terminal_read", ...)` (never trusting unredacted text from the webview), applies 3-stage ANSI + path/token + Clavis scrubbing and `16 KiB` head/tail truncation with `@term:<session_id>#<block_id>:L<start>-<end>` citation headers, and routes through `vox_a2a_send` using `CONSOLE_OPERATOR_AGENT_ID` + `route: "db"`. | ✓ VERIFIED | `console_a2a.rs:102-182` ignores untrusted frontend output bodies, calls `backend.call_tool("vox_terminal_read", ...)`, normalizes single-bound line ranges (`IN-01`), applies `scrub_for_agent_inbox` (`console_a2a.rs:28-46`), and dispatches `vox_a2a_send` with `sender_id: CONSOLE_OPERATOR_AGENT_ID` (`0`) and `route: "db"`. Verified by `commands::console_a2a` (`10/10` passed). |
| 22 | 442-05 | Remote Axis gate tests (`remote::gate::tests`) pass with every new Tauri command explicitly classified (`pty` stays `ModuleClass::Host` local-only; `console_a2a` stays `ModuleClass::Daemon` remote-routed via `BackendCalls`). | ✓ VERIFIED | `remote/manifest.rs` keeps `pty` as `ModuleClass::Host` and `console_a2a` in `REMOTE_ROUTED_DAEMON_MODULES`; `cargo test -p vox-gui --bin vox-gui remote::gate` passed `13/13`. |
| 23 | 442-06 | User chat prompts containing `@term`, `@term:last`, `@term:failed`, `@term:pinned`, `@term:<session_id>`, `@term:<session_id>#<block_id>`, or `@term:<session_id>#<block_id>:L<start>-<end>` resolve against the chat thread's bound terminal session (or explicit `<session_id>`) and append a structured `<terminal_context>` XML envelope to the canonicalized user turn. | ✓ VERIFIED | `term_mentions.rs:58-245,255-425` parses all 7 forms, resolves bound/explicit sessions via `ServerState`, formats `<terminal_context><terminal_block ...>` envelopes escaping embedded closing tags (`WR-04`), and `message.rs:176` attaches the envelope after `canonicalize_prompt`. |
| 24 | 442-06 | File mentions starting with `@term` (such as `@terminal.rs`, `@term_helper.vox`, `@term/mod.rs`) and `@rule:<slug>` continue to resolve as workspace file/rule mentions without false-positive `@term` interception, while genuine `@term...` tokens are removed before `mentions.rs` so they never emit `<missing_file>` warnings. | ✓ VERIFIED | `term_mentions.rs:92-145` requires end-of-token or `:digits`/`#digits`/known selector after `@term`, skipping file extensions/paths (`@terminal.rs`, `@term.rs`, `@term_helper.vox`, `user@term.io`); `message.rs:118` strips valid `@term` tokens before `resolve_mentions_with_rules`, and `mentions.rs:379-381` adds a defensive guard. |
| 25 | 442-06 | Every resolved `@term` block passes through 3-stage scrubbing (ANSI/CR normalization -> `vox_redact` path/key/PEM/URL -> `vox_secrets::redact::scrub_resolved_secrets`), enforces `16 KiB` per-block head-40/tail-160 truncation plus a `32 KiB` total `<terminal_context>` per-turn cap, and never spawns an idle PTY when no terminal session exists. | ✓ VERIFIED | `scrub_term_citation_text` (`term_mentions.rs:248-262`) runs ECMA-48/CR strip + Clavis scrub + `vox_redact`; per-block reads truncate at `16 KiB` and `MAX_TERM_PROMPT_ENVELOPE_BYTES = 32 * 1024` (`term_mentions.rs:12,408-418`) caps total turn injection on UTF-8 char boundaries; missing sessions emit `<terminal_notice reason="no_active_session"/>` without spawning a PTY. |

**Score:** 29/29 must-haves verified (4/4 ROADMAP Success Criteria + 25/25 Plan Must-Have Truths)

#### Must-Have Prohibitions Verified (17/17)

All 17 `<prohibitions>` across `442-01-PLAN.md` through `442-06-PLAN.md` were adversarially verified directly against workspace `Cargo.toml` manifests and Rust/TS source files:

| Plan | Prohibition | Status | Verification Evidence |
| --- | --- | --- | --- |
| 442-01 | Do NOT depend on `vox-container`, `tree-sitter-bash`, or any C-compiler grammar crate from `vox-orchestrator`. | ✓ HELD | `crates/vox-orchestrator/Cargo.toml` has zero `vox-container` or `tree-sitter*` dependencies; tokenizer/parser in `exec_policy.rs` is 100% pure Rust. |
| 442-01 | Do NOT regress single-quoted literal strings containing `$()` or `;` (e.g. `rg 'foo; $(bar)' crates/`) — POSIX single quotes keep contents literal. | ✓ HELD | `exec_policy.rs:242-254` treats `'...'` literally in POSIX mode (`test_single_quoted_command_subst_and_semicolon_allowed`). |
| 442-02 | Do NOT read `std::env::var` directly in `vox-secrets/src/redact.rs` or `vox-terminal-core` — resolve exclusively via `vox_secrets::resolve_secret(id)`. | ✓ HELD | `vox-secrets/src/redact.rs` calls `resolve_secret(spec.id)` exclusively; `./target/debug/vox ci secret-env-guard` reported `0` findings in Phase 442 files. |
| 442-02 | Do NOT retain the narrow `is_ascii_alphabetic()` terminator in `block.rs::strip_ansi` — handle full ECMA-48 `0x40..=0x7E` final bytes and OSC/DCS string terminators. | ✓ HELD | `block.rs:110-198` implements full ECMA-48 `0x40..=0x7E` CSI final-byte scanning plus OSC (`BEL` / `ST`) and DCS/SOS/PM/APC string state machines. |
| 442-03 | Do NOT wrap agent commands in `( ... )` subshells that discard `cd` / `export` / alias mutations across turns. | ✓ HELD | `agent_exec.rs:78-96` (`format_agent_command_for_shell`) runs `__vox_pager_enter; <cmd>` in the current shell environment; verified stateful `cd` + `export` tests pass. |
| 442-03 | Do NOT allow `Origin::Agent` or `agent_send_input` to write to a block flagged `credential_prompt == true` under any security level or control mode. | ✓ HELD | Enforced fail-closed in `agent_exec.rs:468-476`, `session.rs:503-511`, and `terminal_tools.rs:432-446` (`CREDENTIAL_PROMPT_USER_ONLY`). |
| 442-03 | Do NOT register `TermDispatch` or `LocalHostTerminalBridge` on non-loopback `--bind` listeners (`T-441-04` invariant preserved). | ✓ HELD | `bin/vox_orchestrator_d.rs:284-309` gates both `TermDispatch::with_bridge` and `state.set_terminal_bridge` behind `is_loopback_bind(&bind_addr)`. |
| 442-04 | Do NOT add `vox-terminal-core` to `crates/vox-orchestrator-mcp/Cargo.toml` — route strictly through `vox_orchestrator::terminal_bridge::TerminalBridge`. | ✓ HELD | `crates/vox-orchestrator-mcp/Cargo.toml` has no `vox-terminal-core` edge; `./target/debug/vox-arch-check` exited `0`. |
| 442-04 | Do NOT hand-edit `contracts/mcp/tool-registry.canonical.yaml` or `crates/vox-cli/src/commands/ci/operations_catalog/generated_Consumers.rs` — run `operations-sync --write`. | ✓ HELD | Generated registries match `catalog.v1.yaml` cleanly; `./target/debug/vox ci operations-verify` exited `0` (`1426 catalog rows`). |
| 442-04 | Do NOT modify `FROZEN_PREFIX_CORE_TOOLS` in `ast_slice_and_preflight.rs` (prompt-cache prefix stability invariant). | ✓ HELD | `ast_slice_and_preflight.rs:23-36` remains untouched at 12 core tools. |
| 442-04 | Do NOT mark `vox_terminal_exec` or `vox_terminal_send_input` as `http_read_role_eligible: true`. | ✓ HELD | `contracts/mcp/http-read-role-governance.yaml:42-43` includes only `vox_terminal_read` and `vox_terminal_sessions`. |
| 442-05 | Do NOT add raw `@tauri-apps/api/core` `invoke(...)` calls outside the `__VOX_RAW_IPC_BEGIN__..__VOX_RAW_IPC_END__` fence in `transport.ts`. | ✓ HELD | `transport.ts:1045-1094,1210-1235` uses `safeInvoke(...)` exclusively. |
| 442-05 | Do NOT trust raw block output passed from the browser/webview when constructing an A2A message — fetch/verify via `BackendCalls::call_tool("vox_terminal_read", ...)` on the Rust side and scrub before `vox_a2a_send`. | ✓ HELD | `SendTerminalBlockRequest` (`console_a2a.rs:102-182`) takes only `session_id`/`block_id`/`line_start`/`line_end` + optional user note, fetches block output via `vox_terminal_read`, and runs `scrub_for_agent_inbox`. |
| 442-05 | Do NOT expose `pty` commands over Remote Axis (`pty` remains `ModuleClass::Host` local-only). | ✓ HELD | `remote/manifest.rs` keeps `pty` as `ModuleClass::Host`; `remote::gate` tests (`13/13`) verify remote denial with `local_only`. |
| 442-06 | Do NOT mutate `raw_prompt_for_research` in `message.rs` when attaching `<terminal_context>` so Socrates research routing continues to classify the user's actual question rather than 16 KiB of build logs. | ✓ HELD | `message.rs:144` snapshots `raw_prompt_for_research = params.prompt.clone()` from the cleaned user question, and `attach_terminal_context` runs later at `message.rs:176`. |
| 442-06 | Do NOT spawn a new PTY process as a side effect of resolving `@term` when no session exists — return a lightweight `<terminal_notice>` marker instead. | ✓ HELD | `term_mentions.rs:320-352` queries `list_sessions` / `read_Session` without calling `exec` or `open`, emitting `<terminal_notice reason="no_active_session"/>` when empty. |
| 442-06 | Do NOT break `@terminal.rs` or any workspace file path starting with `term`. | ✓ HELD | Unit-tested in `term_mentions.rs` (`does_not_hijack_file_mentions_or_emails`) and `mentions.rs`. |

#### Code Review Findings & Remediation Verification (18/18)

All 6 Critical (`CR-01..06`), 8 Warning (`WR-01..08`), and 4 Info (`IN-01..04`) findings from `442-REVIEW.md` and `442-REVIEW-FIX.md` were verified directly in source:

- **`CR-01..03` (`exec_policy.rs`)**: UTF-8 char-scalar stepping (`next_char_at` — zero panics on multi-byte Unicode), trailing `\` flush + fail-closed `has_trailing_escape`, and PowerShell prefix/slash flag blocking (`-e`..`-encodedcommand`, `-c`..`-command`, `/c`, `/e`, `-f`..`-file`).
- **`CR-04` (`server_state.rs`)**: `terminal_bridge`, `chat_terminal_sessions`, and `ephemeral_terminal_bridge` wrapped in `Arc<PrRwLock<...>>` so `ServerState::clone()` shares live bridge & sticky session state across MCP/HTTP/daemon tasks.
- **`CR-05` (`dispatch.rs`, `terminal_tools.rs`, `agent_exec.rs`, `term_bridge_impl.rs`)**: `outer_hitl_approved: bool` + `hitl_approved: bool` eliminates double-parking under `PermissionMode::Ask` while preserving `TermControlMode::Manual` parking under `AcceptAll`.
- **`CR-06` (`floor.rs`)**: Updated `vox_skill_run` floor unit test assertion to match intentional `SHELL_COMMAND_TOOLS` inclusion.
- **`WR-01..08`**: UTF-8 + ECMA-48 + CR + Clavis + `vox_redact` `scrub_terminal_mcp_str` (`terminal_tools.rs`); `wait_ms == 0` / `background: true` immediate `Running` return without clamping to `100ms` or `SIGINT` (`agent_exec.rs`, `terminal_bridge_ephemeral.rs`, `terminal_tools.rs`); full 7-regex parity in `terminal_bridge_ephemeral.rs`; `ENVELOPE_TAG_BREAKOUT_RE` escaping `</terminal_block>` & `</terminal_context>` (`term_mentions.rs`); pre-truncation `Some(500)` fetch for `pinned_only`/`failed_only` (`terminal_tools.rs`); `vox-term` `BlockStatus::AwaitingInput` + new `SessionEvent` arms (`app.rs`, `ui/blocks.rs`); pre-existing `sess.is_alt_screen()` guard in `LocalHost::exec_agent_command` (`agent_exec.rs`); daemon-boundary `validate_daemon_agent_exec` defense-in-depth (`term_bridge_impl.rs`, `term_dispatch.rs`).
- **`IN-01..04`**: Single-bound `line_start` / `line_end` normalization (`terminal_bridge_ephemeral.rs`, `console_a2a.rs`); Clavis needle scrub run BEFORE regex redaction (`term_bridge_impl.rs`, `terminal_bridge_ephemeral.rs`, `console_a2a.rs`, `term_mentions.rs`); single `resolve_terminal_bridge` call (`terminal_tools.rs`); `TermAttachmentDto` TS field parity (`transport.ts`).

### Required Artifacts

Verified via `gsd-tools.cjs query verify.artifacts --ws axis-warp-terminal-v75` across all 6 plans (`20/20` passed, `issues: []`):

| Artifact | Plan | Expected Provides | Exists | Substantive | Wired | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `crates/vox-orchestrator/src/oversight/exec_policy.rs` | 442-01 | `ExecPolicyV1`, `SecurityTier`, `CommandSegment`, `evaluate_command_ast`, `evaluate_security_tier`, `extract_effective_commands` | ✓ | ✓ (789 lines) | ✓ | ✓ VERIFIED |
| `crates/vox-orchestrator/src/oversight/floor.rs` | 442-01 | Multi-segment `classify_floor` over `SHELL_COMMAND_TOOLS` (`vox_run_shell`, `vox_skill_run`, `vox_terminal_exec`) | ✓ | ✓ (854 lines) | ✓ | ✓ VERIFIED |
| `crates/vox-cli-research/src/check_terminal.rs` | 442-01 | Contract verifier delegating Rust fallback evaluation to `oversight::exec_policy` | ✓ | ✓ (521 lines) | ✓ | ✓ VERIFIED |
| `crates/vox-redact/src/lib.rs` | 442-02 | Cross-OS home path, PEM private key, URL credential, and modern token redaction | ✓ | ✓ (285 lines) | ✓ | ✓ VERIFIED |
| `crates/vox-secrets/src/redact.rs` | 442-02 | `scrub_resolved_secrets` & `scrub_secret_needles` (`aho-corasick` `LeftmostLongest` → `[REDACTED:CLAVIS]`) | ✓ | ✓ (195 lines) | ✓ | ✓ VERIFIED |
| `crates/vox-terminal-core/src/block.rs` | 442-02 | `BlockStatus::AwaitingInput`, `pinned`, `credential_prompt`, full ECMA-48 `strip_ansi_and_normalize` | ✓ | ✓ (251 lines) | ✓ | ✓ VERIFIED |
| `crates/vox-terminal-core/src/corpus/redact.rs` | 442-02 | `scrub_ansi_and_redact`, `truncate_terminal_block` (`16 KiB`, head 40 / tail 160), `slice_lines_1based` | ✓ | ✓ (275 lines) | ✓ | ✓ VERIFIED |
| `crates/vox-terminal-core/src/stall_detector.rs` | 442-03 | `PromptKind`, `StallSignal`, `classify_stall`, `DEFAULT_STALL_QUIESCENCE_MS = 1_500` | ✓ | ✓ (212 lines) | ✓ | ✓ VERIFIED |
| `crates/vox-terminal-core/src/agent_exec.rs` | 442-03 | `LocalHost::{open_with_cwd, exec_agent_command, agent_send_input, pin_block, promote_bg}` + `allows_agent_command_with_approval` | ✓ | ✓ (737 lines) | ✓ | ✓ VERIFIED |
| `crates/vox-orchestrator/src/terminal_bridge.rs` | 442-03 | `TerminalBridge` trait, `OrchDaemonTerminalBridge`, `EphemeralTerminalBridge` (`terminal_bridge_ephemeral.rs`: 553 lines) | ✓ | ✓ (465 lines) | ✓ | ✓ VERIFIED |
| `crates/vox-orchestrator-d/src/term_bridge_impl.rs` | 442-03 | `LocalHostTerminalBridge` sharing `Arc<LocalHost>` + 3-stage Clavis/ANSI/path scrubbing + `validate_daemon_agent_exec` | ✓ | ✓ (414 lines) | ✓ | ✓ VERIFIED |
| `crates/vox-orchestrator-d/src/term_dispatch.rs` | 442-03 | Loopback RPC handlers for `term.{agent_exec,agent_send_input,read,promote_bg,pin,security,control}` | ✓ | ✓ (1,119 lines) | ✓ | ✓ VERIFIED |
| `crates/vox-orchestrator-mcp/src/input_schemas/terminal.rs` | 442-04 | Strict draft-07 JSON Schemas (`additionalProperties: false`) for all 4 `vox_terminal_*` MCP tools | ✓ | ✓ (161 lines) | ✓ | ✓ VERIFIED |
| `crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs` | 442-04 | MCP handlers enforcing `ExecPolicyV1` + `SecurityTier` + `TermControlMode` + `PendingApprovals` + Clavis scrubbing | ✓ | ✓ (602 lines) | ✓ | ✓ VERIFIED |
| `contracts/operations/catalog.v1.yaml` | 442-04 | Canonical operation entries for `vox_terminal_{exec,read,send_input,sessions}` synced across registries | ✓ | ✓ (`1,426` ops) | ✓ | ✓ VERIFIED |
| `crates/vox-gui/src/commands/pty.rs` | 442-05 | `term_session_{set_security,set_control,pin,promote_bg}` routed through `PtyManager::host_for` | ✓ | ✓ (680+ lines) | ✓ | ✓ VERIFIED |
| `crates/vox-gui/src/commands/console_a2a.rs` | 442-05 | Backend-verified, 3-stage scrubbed `send_terminal_block_to_agent` (`vox_terminal_read` → `vox_a2a_send`) | ✓ | ✓ (290+ lines) | ✓ | ✓ VERIFIED |
| `crates/vox-gui/ui/src/transport.ts` | 442-05 | Typed `safeInvoke` wrappers & `TermSecurityLevelWire` / `TermControlModeWire` / `TermAttachmentDto` contracts | ✓ | ✓ | ✓ | ✓ VERIFIED |
| `crates/vox-orchestrator-mcp/src/chat_tools/chat/term_mentions.rs` | 442-06 | 7-selector `@term` resolver, 3-stage scrubber, tag-breakout sanitizer, `32 KiB` `<terminal_context>` builder | ✓ | ✓ (737 lines) | ✓ | ✓ VERIFIED |
| `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs` | 442-06 | Pre-`mentions.rs` `@term` extraction (`L118`) + post-`canonicalize_prompt` envelope attachment (`L176`) | ✓ | ✓ | ✓ | ✓ VERIFIED |

### Key Link Verification

Verified via `gsd-tools.cjs query verify.key-links --ws axis-warp-terminal-v75` across all 6 plans (`13/13` verified):

| From | To | Via | Status | Details |
| --- | --- | --- | --- | --- |
| `crates/vox-orchestrator/src/oversight/floor.rs` | `crates/vox-orchestrator/src/oversight/exec_policy.rs` | `extract_effective_commands` | ✓ WIRED | `floor.rs:375-426` inspects every unwrapped segment across `SHELL_COMMAND_TOOLS`. |
| `crates/vox-cli-research/src/check_terminal.rs` | `crates/vox-orchestrator/src/oversight/exec_policy.rs` | `ExecPolicyV1` + `evaluate_command_ast` | ✓ WIRED | `check_terminal.rs:14-18,175-210` shares the exact runtime AST evaluator. |
| `crates/vox-terminal-core/src/corpus/redact.rs` | `crates/vox-terminal-core/src/block.rs` | `strip_ansi_and_normalize` | ✓ WIRED | `corpus/redact.rs:38` normalizes CR overwrites & ECMA-48 before regex redaction. |
| `crates/vox-secrets/src/lib.rs` | `crates/vox-secrets/src/redact.rs` | `pub use redact::{scrub_resolved_secrets, scrub_secret_needles}` | ✓ WIRED | Downstream crates scrub Clavis secrets without calling `.expose()`. |
| `crates/vox-terminal-core/src/agent_exec.rs` | `crates/vox-orchestrator/src/oversight/exec_policy.rs` | `evaluate_security_tier` + `default_exec_policy_v1` | ✓ WIRED | `agent_exec.rs:34-75` enforces `L1..L5` + `ExecPolicyV1` inside `LocalHost`. |
| `crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs` | `crates/vox-orchestrator-mcp/src/server_state.rs` | `state.set_terminal_bridge(term_bridge.clone())` | ✓ WIRED | `vox_orchestrator_d.rs:284-309` wires one shared `Arc<LocalHost>` on loopback/stdio. |
| `crates/vox-orchestrator-d/src/term_bridge_impl.rs` | `crates/vox-secrets/src/redact.rs` | `scrub_three_stage` → `scrub_resolved_secrets` | ✓ WIRED | `term_bridge_impl.rs:24-34` scrubs Clavis needles before regex/path redaction (`IN-02`). |
| `crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs` | `crates/vox-orchestrator/src/oversight/exec_policy.rs` | `default_exec_policy_v1` + `evaluate_command_ast` + `evaluate_security_tier` | ✓ WIRED | `terminal_tools.rs:230-310` gates commands before HITL / PTY execution. |
| `crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs` | `crates/vox-orchestrator-mcp/src/server_state.rs` | `resolve_terminal_bridge` + `bind_or_resolve_terminal_session` + `pending_approvals` | ✓ WIRED | `terminal_tools.rs:175-335` binds sticky per-thread sessions and parks HITL requests. |
| `crates/vox-gui/src/commands/pty.rs` | `crates/vox-terminal-core/src/host.rs` | `manager.host_for(daemon.inner()).await.{set_security_level,set_control_mode,pin_block,promote_bg}` | ✓ WIRED | `pty.rs:470-561` routes through daemon `DaemonHost` with `LocalHost` fallback. |
| `crates/vox-gui/src/commands/console_a2a.rs` | `vox_terminal_read` + `vox_a2a_send` | `BackendCalls::call_tool` + `scrub_for_agent_inbox` + `CONSOLE_OPERATOR_AGENT_ID` | ✓ WIRED | `console_a2a.rs:102-182` fetches backend block text, scrubs 3 stages, and posts via `"db"`. |
| `crates/vox-orchestrator-mcp/src/chat_tools/chat/term_mentions.rs` | `crates/vox-orchestrator-mcp/src/server_state.rs` | `resolve_terminal_bridge` + `bind_or_resolve_terminal_session` | ✓ WIRED | `term_mentions.rs:312-356` resolves bound/explicit terminal blocks without spawning PTYs. |
| `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs` | `crates/vox-orchestrator-mcp/src/chat_tools/chat/term_mentions.rs` | `expand_term_mentions` (`L118`) + `attach_terminal_context` (`L176`) | ✓ WIRED | `message.rs:118-177` preserves `raw_prompt_for_research` and appends `<terminal_context>` post-canonicalization. |

### Data-Flow Trace (Level 4)

| Artifact Chain | Data Variable | Source | Produces Real Data | Status |
| --- | --- | --- | --- | --- |
| `terminal_tools.rs` → `dispatch.rs` → `term_bridge_impl.rs` → `agent_exec.rs` → `pty.rs` | `TermExecRequest` → `TermExecOutcome` (`block_id`, `status`, `exit_code`, `cwd`, `output`, `stall_reason`, `credential_prompt`) | Real persistent shell PTY (`__vox_pager_enter`, `Osc633Parser`, `VtShadow`, `classify_stall`) scrubbed via 3-stage Clavis/ANSI/path pipeline | Yes — verified over live PTYs and loopback TCP (`orch_daemon_terminal_bridge_exec_read_pin_and_three_stage_scrub_over_tcp`) | ✓ FLOWING |
| `console_a2a.rs` → `vox_terminal_read` → `scrub_for_agent_inbox` → `vox_a2a_send` | `SendTerminalBlockRequest` → `SendTerminalBlockResult { citation, db_message_uuid, truncated }` | Backend `TerminalBridge::read` block lookup + 3-stage scrub + `16 KiB` head/tail truncation → A2A DB inbox | Yes — verified in `commands::console_a2a` (`10/10` unit/integration tests) | ✓ FLOWING |
| `message.rs` → `term_mentions.rs` → `TerminalBridge::read` → `<terminal_context>` | `@term*` tokens → `TermMentionExpansion { cleaned_prompt, context_envelope, citations }` | Live/bound terminal session blocks scrubbed via `scrub_term_citation_text`, XML-escaped, and capped at `32 KiB` | Yes — verified in `term_mentions` and `mentions` test suites | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| --- | --- | --- | --- |
| AST `ExecPolicyV1`, `SecurityTier` (`L1..L5`), wrapper unwrapping, `floor.rs` multi-segment classification, `OrchDaemonTerminalBridge` & `EphemeralTerminalBridge` | `cargo test -p vox-orchestrator --no-default-features --lib --tests` (`task-1004`) | `exit 0` (`100%` passed across `exec_policy`, `floor`, `terminal_bridge`, and integration binaries) | ✓ PASS |
| ECMA-48/CR stripping, `16 KiB` head/tail truncation, 5-shell non-subshell pager shims, `1500ms` stall detector, `credential_prompt` fail-closed guard, `Autopilot -> Manual` demotion, `promote_bg` | `cargo test -p vox-terminal-core --lib --tests` (`task-1009`) + `--features vox-terminal-core/vox-lang --lib` (`task-1081`) | `exit 0` (`67/67` unit + 10 integration binaries passed) | ✓ PASS |
| Loopback `TermDispatch` + `LocalHostTerminalBridge` shared `Arc<LocalHost>`, `validate_daemon_agent_exec`, 3-stage Clavis scrubbing over TCP | `cargo test -p vox-orchestrator-d --lib` (`task-1075`) | `35/35 passed` | ✓ PASS |
| 4 `vox_terminal_*` MCP schemas/handlers, sticky `chat_thread_id` binding, single-park `PendingApprovals` (`CR-05`), `@term` 7-form mentions & XML breakout guard (`WR-04`), static registry parity | `cargo test -p vox-orchestrator-mcp --lib` (`task-1075`) + `--test pending_approvals_tests --test pending_approvals_floor_tests --test floor_entry_points_tests` (`task-1115`) | `1,078 passed` (`--lib`) + `24/24 passed` (HITL/floor suites) | ✓ PASS |
| `vox-gui` `term_session_{set_security,set_control,pin,promote_bg}`, backend-scrubbed `send_terminal_block_to_agent`, Remote Axis `ModuleClass` gate | `cargo test -p vox-gui --bin vox-gui` (`commands::console_a2a`, `commands::pty`, `remote::gate`) | `10/10` + `14/14` + `13/13` passed | ✓ PASS |
| Operations catalog / MCP / CLI / capability registry sync (`1,426` rows) | `./target/debug/vox ci operations-verify` | `operations-verify OK (1426 catalog rows)` (`exit 0`) | ✓ PASS |
| `contracts/terminal/exec-policy.v1.yaml` schema + Rust AST contract fixtures | `./target/debug/vox ci exec-policy-contract` | `exec-policy-contract: rust fallback OK (6 hardcoded + 10 disk)` (`exit 0`) | ✓ PASS |
| Architecture layer, crate-edge, and Clavis secret-env guards | `./target/debug/vox-arch-check` + `./target/debug/vox ci secret-env-guard` | `vox-arch-check` `exit 0`; `0` secret-env findings in Phase 442 files | ✓ PASS |

### Probe Execution

No `probe-*.sh` scripts were declared in `442-01-PLAN.md`..`442-06-PLAN.md` (per repo VoxLang-first / Rust policy, executable verification was authored as Rust unit and integration tests across `vox-orchestrator`, `vox-cli-research`, `vox-redact`, `vox-secrets`, `vox-terminal-core`, `vox-orchestrator-d`, `vox-orchestrator-mcp`, and `vox-gui`, plus CI contract gates `operations-verify`, `exec-policy-contract`, `secret-env-guard`, and `vox-arch-check`).

### Requirements Coverage

Checked against `.planning/workstreams/axis-warp-terminal-v75/REQUIREMENTS.md` (`Phase 442` mapping):

| Requirement | Source Plan(s) | Description | Status | Supporting Evidence |
| --- | --- | --- | --- | --- |
| `TERM-HARNESS-01` | `442-03`, `442-04`, `442-05` | Structured MCP & orchestrator tool suite (`vox_terminal_exec`, `vox_terminal_read`, `vox_terminal_send_input`, `vox_terminal_sessions`) operating over shared `vox-terminal-core` sessions with background/foreground promotion (`Ctrl+B`). | ✓ SATISFIED | `terminal_bridge.rs`, `term_bridge_impl.rs`, `term_dispatch.rs`, `input_schemas/terminal.rs`, `terminal_tools.rs`, `catalog.v1.yaml`, `vox-gui/src/commands/pty.rs`, `transport.ts`. |
| `TERM-HARNESS-02` | `442-03` | Non-interactive environment shaping (`PAGER=cat`, `GIT_PAGER=cat`, `LESS=FRX`, etc.) + interactive stall/prompt detector (`[y/N]`, passphrase/credential prompts, REPL waits, TUI entry) that yields early with `AwaitingInput` status instead of hanging until timeout, and never auto-answers credential prompts. | ✓ SATISFIED | `shell_init.rs` (`__vox_pager_enter` / `__vox_pager_restore` across 5 shells), `stall_detector.rs` (`1,500 ms` quiescence + immediate `ALT_SCREEN`), `agent_exec.rs` & `terminal_tools.rs` (`CREDENTIAL_PROMPT_USER_ONLY`). |
| `TERM-HARNESS-03` | `442-01`, `442-03`, `442-04`, `442-05` | AST-aware shell command parser (pipelines, `;`, `&&`, `||`, `$()`, backticks, redirects, `sh -c`/`env`/`xargs`/`find -exec` wrappers) wired to `contracts/terminal/exec-policy.v1.yaml`, `vox-orchestrator::oversight`, and session control modes (`Manual`, `CoPilot`, `Autopilot`) with HITL approval parking. | ✓ SATISFIED | `oversight/exec_policy.rs`, `oversight/floor.rs`, `check_terminal.rs`, `agent_exec.rs`, `term_bridge_impl.rs`, `terminal_tools.rs`, `dispatch.rs`, `permission_modes.rs`, `vox-gui/src/commands/pty.rs`. |
| `TERM-HARNESS-04` | `442-02`, `442-04`, `442-05`, `442-06` | Bidirectional `@term` chat context (`@term`, `@term:last`, `@term:failed`, `@term:pinned`, `"Send to Agent"` action) with ANSI stripping, Clavis secret + home-path redaction, and head/tail truncation with line-range citations. | ✓ SATISFIED | `vox-redact/src/lib.rs`, `vox-secrets/src/redact.rs`, `block.rs`, `corpus/redact.rs`, `console_a2a.rs`, `term_mentions.rs`, `mentions.rs`, `message.rs`. |

**Orphaned requirements check:** No additional requirements in `REQUIREMENTS.md` are mapped to Phase 442 beyond `TERM-HARNESS-01..04`. Zero orphaned requirements.

### Anti-Patterns Found

Scanned all 19 Phase 442 implementation modules with `rg -n "TBD|FIXME|XXX|TODO|HACK|PLACEHOLDER|unimplemented!|todo!"`:

| File | Line | Pattern | Severity | Impact |
| --- | --- | --- | --- | --- |
| *(all 19 Phase 442 implementation files)* | — | Zero `TBD` / `FIXME` / `XXX` / `TODO` / `HACK` / `PLACEHOLDER` / `unimplemented!` / `todo!` matches | — | Clean — zero stubs, placeholders, or deferred implementation markers. |

### Human Verification Required

None — zero `<human-check>` blocks were declared across `442-01-PLAN.md`..`442-06-PLAN.md`, and `442-VALIDATION.md` confirms all Phase 442 security, PTY stall/pager, MCP HITL, Tauri A2A, and `@term` chat behaviors are covered by deterministic unit, live PTY, and loopback TCP integration suites.

### Gaps Summary

No gaps found. All 4 ROADMAP Success Criteria, 25 plan must-have truths, 17 prohibitions, 18 code-review remediations (`442-REVIEW-FIX.md`), 20 required artifacts, 13 key links, and requirements `TERM-HARNESS-01` through `TERM-HARNESS-04` are implemented, wired end-to-end, and verified.
