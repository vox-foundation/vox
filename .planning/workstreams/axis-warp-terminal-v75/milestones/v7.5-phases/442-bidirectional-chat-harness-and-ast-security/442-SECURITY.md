---
phase: "442"
slug: "bidirectional-chat-harness-and-ast-security"
status: verified
threats_open: 0
asvs_level: 1
block_on: high
created: "2026-11-15"
---

# Phase 442 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail for Phase 442 (`442-bidirectional-chat-harness-and-ast-security`).

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| LLM / MCP tool call → `oversight::exec_policy` & `oversight::floor` | Untrusted agent-generated POSIX or PowerShell command string evaluated before reaching any shell or PTY | Raw command strings (`vox_terminal_exec`, `vox_run_shell`, `vox_skill_run`), wrapper argv (`sh -c`, `pwsh -EncodedCommand`, `env`, `xargs`, `find -exec`) |
| PTY stdout/stderr & block history → Redaction pipeline (`vox-terminal-core` → `vox-secrets` → `vox-redact`) | Raw terminal byte stream containing potential ECMA-48 escape sequences, `\r` overwrites, Clavis secrets, PEM keys, URL credentials, API tokens, and user home paths | `Block.raw_output`, `TermExecOutcome.output`, `TermReadResponse`, `@term` citations, A2A inbox payloads |
| Loopback JSON-RPC client → `TermDispatch` & `LocalHostTerminalBridge` (`vox-orchestrator-d`) | Authenticated local daemon RPC calls (`term.agent_exec`, `term.agent_send_input`, `term.read`, `term.input`, `term.submit`, `term.signal`, `term.promote_bg`, `term.pin`) | Session IDs, `Origin` (`User` vs `Agent`), `hitl_approved`, interactive prompt stdin, security/control state |
| MCP caller (`vox-orchestrator-mcp`) → `PendingApprovals` & `TerminalBridge` | Agent tool dispatch crossing `PermissionMode` (`Plan` / `Ask` / `AcceptAll` / `Bypass`), `ExecPolicyV1`, `SecurityTier` (`L1..L5`), and `TermControlMode` (`Manual` / `CoPilot` / `Autopilot`) | `chat_thread_id`, command argv, approval park/resume tokens, HTTP read-role headers |
| `vox-gui` Webview IPC / Remote Axis → Rust Tauri commands (`commands::pty`, `commands::console_a2a`, `remote::gate`) | Untrusted frontend IPC payloads and remote WebSocket clients invoking Tauri command handlers | `session_id`, `block_id`, line ranges, `TermSecurityLevelWire`, `TermControlModeWire`, A2A target agent IDs |
| User chat prompt (`@term*`) → `term_mentions.rs` → LLM `<terminal_context>` turn | Terminal output interpolated into structured XML prompt context sent to an external/local LLM | `@term` selector tokens, `<terminal_block>` XML attributes/bodies, `raw_prompt_for_research` |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-442-01 | Elevation of Privilege / Tampering | `crates/vox-orchestrator/src/oversight/exec_policy.rs` | critical | mitigate | Pure-Rust quote-aware POSIX + PowerShell tokenizer (`tokenize_and_split_segments`, UTF-8 scalar stepping `CR-01`, trailing `\` flush & fail-closed `has_trailing_escape` `CR-02`) splits `;`, `&&`, `||`, `|`, `&`, `\n`, `\r\n`, flags `$()`, backticks, `()`, `<()`, `>()`, and output/heredoc redirects, and recursively unwraps wrappers (`MAX_DEPTH = 6`: `sh`/`bash`/`zsh`/`pwsh`/`cmd`, `env`, `sudo`, `nohup`, `timeout`, `xargs`, `find -exec`/`-delete`) while failing closed on PowerShell `-e..`/`-c..`/`/c`/`-f..` flags (`CR-03`). Verified in `exec_policy` unit tests + `vox ci exec-policy-contract`. | closed |
| T-442-02 | Tampering / Bypass | `crates/vox-orchestrator/src/oversight/floor.rs` | critical | mitigate | `SHELL_COMMAND_TOOLS` (`floor.rs:36-40`) includes `vox_run_shell`, `vox_skill_run`, and `vox_terminal_exec`; `classify_floor` (`floor.rs:375-426`) inspects `"command"` and `"cmd"` across every unwrapped `extract_effective_commands` segment so `echo ok && git push --force` is classified `Irreversible` even under `AcceptAll`. Verified in `floor` and `floor_entry_points_tests`. | closed |
| T-442-03 | Information Disclosure | `crates/vox-secrets/src/redact.rs` | critical | mitigate | `scrub_resolved_secrets` & `scrub_secret_needles` resolve scrubbable Clavis specs (`len >= 6`, skipping config/URL/email/username specs) via `vox_secrets::resolve_secret` (zero `std::env::var` reads) and replace matches via `aho-corasick` `MatchKind::LeftmostLongest` with `"[REDACTED:CLAVIS]"` before regex passes (`IN-02`) without exposing secret values outside `vox-secrets`. | closed |
| T-442-04 | Information Disclosure | `crates/vox-redact/src/lib.rs` | high | mitigate | `vox_redact` orders `RE_PEM` (`[REDACTED_PEM]`) and `RE_URL_CRED` (`[REDACTED_URL_CRED]`) ahead of `RE_EMAIL`, expands `RE_TOKEN` (`AIza`, `hf_`, `npm_`, `ghp_`, `github_pat_`, `glpat-`, `xox[baprs]-`, `sk-ant-`, `sk-proj-`, `sk-`), and scrubs macOS `/Users/<name>`, Linux `/home/<name>`, and Windows `C:\Users\<name>` paths to `~[REDACTED_PATH]`. | closed |
| T-442-05 | Information Disclosure / Evasion | `crates/vox-terminal-core/src/block.rs`, `corpus/redact.rs` | high | mitigate | `strip_ansi_and_normalize` (`block.rs:110-198`) collapses `\r` / `CSI K` overwrites via `normalize_cr_overwrites` and strips full ECMA-48 CSI (`0x40..=0x7E`), OSC (`BEL`/`ST`), DCS/SOS/PM/APC, 2-byte ESC, and C0 controls BEFORE Clavis and regex redaction run (`corpus/redact.rs:34-48`), defeating ANSI-interleaved or carriage-return-masked secret exfiltration. | closed |
| T-442-06 | Elevation of Privilege | `crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs`, `term_dispatch.rs` | critical | mitigate | `TermDispatch::with_bridge` and `state.set_terminal_bridge` are registered strictly when `is_loopback_bind(&bind_addr)` (`vox_orchestrator_d.rs:284-309`), daemon auth token verification precedes dispatch, and `refuse_agent_origin` rejects `Origin::Agent` or missing/unknown origins on `term.input`, `term.submit`, and `term.signal`. | closed |
| T-442-07 | Information Disclosure / Tampering | `crates/vox-terminal-core/src/stall_detector.rs`, `agent_exec.rs`, `session.rs` | critical | mitigate | `stall_detector::classify_stall` identifies password/passphrase/`[sudo]`/OTP/PIN/token prompts within `1_500 ms` of quiescence, sets `block.credential_prompt = true`, and both `LocalHost::agent_send_input` (`agent_exec.rs:468-476`) and `Session::resume_awaiting_block` (`session.rs:503-511`) reject `Origin::Agent` input fail-closed with `CREDENTIAL_PROMPT_USER_ONLY`. | closed |
| T-442-08 | Denial of Service (Pager / TUI Hang) | `crates/vox-terminal-core/src/shell_init.rs`, `agent_exec.rs`, `terminal_bridge_ephemeral.rs` | high | mitigate | Non-subshell `__vox_pager_enter` / `__vox_pager_restore` shims inject `PAGER=cat`, `GIT_PAGER=cat`, `LESS=FRX`, `MANPAGER=cat`, `BAT_PAGER=cat`, `DELTA_PAGER=cat` across `zsh`/`bash`/`fish`/`pwsh`/`nu`; `LocalHost::exec_agent_command` rejects pre-existing `ALT_SCREEN` sessions (`WR-07`), yields `AwaitingInput` within `< 2 s` on TUI/prompt stalls, and honors `wait_ms == 0` / `promote_bg` without hanging (`WR-02`). | closed |
| T-442-09 | Tampering / Race Condition | `crates/vox-terminal-core/src/host.rs`, `agent_exec.rs` | high | mitigate | User `submit` or `input` (`Origin::User`) on a session in `TermControlMode::Autopilot` immediately demotes the session to `TermControlMode::Manual` and emits `SessionEvent::ControlModeChanged` (`host.rs:860-935`), preventing concurrent agent autopilot collisions while the operator is typing. | closed |
| T-442-10 | Elevation of Privilege (HITL Bypass) | `crates/vox-orchestrator-mcp/src/dispatch.rs`, `terminal_tools.rs`, `permission_modes.rs` | critical | mitigate | `vox_terminal_exec` and `vox_terminal_send_input` are classified `Destructive` (`reversible: false`, `operator`); `dispatch.rs` runs `floor_gate` (`Irreversible` floor non-bypassable even under `AcceptAll`) + `PermissionMode` gate and threads `outer_hitl_approved` (`CR-05`) into `terminal_tools.rs`, which enforces `ExecPolicyV1` + `SecurityTier` + `TermControlMode` (`Manual` always parks; `CoPilot` parks out-of-policy / `L3+`; `Autopilot` parks `L4..L5` / policy blocks). | closed |
| T-442-11 | Information Disclosure / Credential Injection | `crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs` | critical | mitigate | `handle_terminal_send_input` (`terminal_tools.rs:432-446`) inspects the target block before `PendingApprovals` and immediately errors with `CREDENTIAL_PROMPT_USER_ONLY` when `credential_prompt == true`, ensuring credential injection can never even be parked for accidental HITL approval; all MCP responses pass through `scrub_terminal_mcp_str` (`WR-01`). | closed |
| T-442-12 | Elevation of Privilege (Read-Role / Plan Mode) | `contracts/mcp/http-read-role-governance.yaml`, `permission-modes.v2.yaml`, `tool_selection.rs` | high | mitigate | Only `vox_terminal_read` and `vox_terminal_sessions` carry `read_only: true` and `http_read_role_eligible: true`; `vox_terminal_exec` and `vox_terminal_send_input` require `operator` role, are denied in `PermissionMode::Plan`, and are omitted from read-only LLM tool selection (`tool_selection.rs:260-267,336-337`). | closed |
| T-442-13 | Elevation of Privilege (Direct Daemon RPC) | `crates/vox-orchestrator-d/src/term_bridge_impl.rs`, `term_dispatch.rs` | high | mitigate | `validate_daemon_agent_exec` (`term_bridge_impl.rs:36-65`, wired in `LocalHostTerminalBridge::exec` and `TermDispatch` `term.agent_exec` at `term_dispatch.rs:320-328`, `WR-08`) enforces `classify_floor` (`Irreversible`) + `ExecPolicyV1` AST validation unless `hitl_approved == true`, closing direct loopback RPC bypasses around MCP. | closed |
| T-442-14 | Information Disclosure / Spoofing | `crates/vox-gui/src/commands/console_a2a.rs` | high | mitigate | `send_terminal_block_to_agent` (`console_a2a.rs:102-182`) never accepts raw terminal output from the untrusted webview; it fetches the authoritative block via `BackendCalls::call_tool("vox_terminal_read", ...)`, applies 3-stage `scrub_for_agent_inbox` (on both block content and operator note), caps at `16 KiB` with `@term:{sid}#{bid}:L{s}-{e}` citations, and attributes sender `CONSOLE_OPERATOR_AGENT_ID = 0` via `route: "db"`. | closed |
| T-442-15 | Elevation of Privilege (Remote Axis) | `crates/vox-gui/src/remote/manifest.rs`, `remote/gate.rs`, `transport.ts` | high | mitigate | `pty` module remains `ModuleClass::Host` so remote WebSocket clients are rejected with `local_only` before reaching any `term_session_*` command; `console_a2a` (`ModuleClass::Daemon`) routes strictly through `BackendCalls`; all TypeScript wrappers in `transport.ts` use `safeInvoke` outside the raw-IPC fence. Verified by `remote::gate` (`13/13` passed). | closed |
| T-442-16 | Information Disclosure (`@term` Context Leak) | `crates/vox-orchestrator-mcp/src/chat_tools/chat/term_mentions.rs` | high | mitigate | `scrub_term_citation_text` (`term_mentions.rs:248-262`) applies ECMA-48/CR normalization, Clavis `scrub_resolved_secrets` (`[REDACTED:CLAVIS]`), and `vox_redact` (`~[REDACTED_PATH]`, PEM, URL creds, API keys) to both block command inputs and outputs before assembling `<terminal_context>`. | closed |
| T-442-17 | Tampering / Prompt Envelope Injection | `crates/vox-orchestrator-mcp/src/chat_tools/chat/term_mentions.rs`, `message.rs` | high | mitigate | `ENVELOPE_TAG_BREAKOUT_RE` (`term_mentions.rs:19-22,265-290`, `WR-04`) neutralizes case-insensitive `</terminal_block>` and `</terminal_context>` closing tags inside untrusted PTY output/commands (`< /terminal_block>`), XML-escapes attribute values, preserves `raw_prompt_for_research` (`message.rs:144`), and appends `<terminal_context>` only after `canonicalize_prompt` (`message.rs:176`). | closed |
| T-442-18 | Denial of Service (Context Bomb / Idle PTY Spawn) | `crates/vox-orchestrator-mcp/src/chat_tools/chat/term_mentions.rs` | medium | mitigate | Individual blocks truncate at `16 KiB` (`head 40 / tail 160`), total per-turn `<terminal_context>` is capped at `MAX_TERM_PROMPT_ENVELOPE_BYTES = 32 * 1024` on UTF-8 char boundaries (`floor_char_boundary`), and resolving `@term` when no session exists emits `<terminal_notice reason="no_active_session"/>` without spawning a PTY. | closed |
| T-442-SC | Supply Chain / Build Surface | Workspace `Cargo.toml` manifests & `layers.toml` | medium | mitigate | Zero new external crates, zero C-compiler / `tree-sitter` grammar dependencies, and zero new crate-graph edges or `layers.toml` exceptions introduced across Phase 442 (`./target/debug/vox-arch-check` exited `0`). | closed |

---

## Accepted Risks Log

No accepted risks.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-11-15 | 19 | 19 | 0 | Antigravity (`gsd-verifier` adversarial source & test verification + `442-REVIEW-FIX`) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-11-15
