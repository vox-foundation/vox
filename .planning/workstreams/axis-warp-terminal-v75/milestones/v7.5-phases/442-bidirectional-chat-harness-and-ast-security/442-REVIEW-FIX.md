---
phase: 442-bidirectional-chat-harness-and-ast-security
milestone: v7.5
workstream: axis-warp-terminal-v75
fixed_at: 2026-11-15T10:01:00Z
review_path: .planning/workstreams/axis-warp-terminal-v75/phases/442-bidirectional-chat-harness-and-ast-security/442-REVIEW.md
fix_scope: all_critical_warning_and_safe_info
iteration: 1
findings_in_scope: 18
fixed: 18
skipped: 0
status: all_fixed
---

# Phase 442: Code Review Fix Report

**Fixed at:** `2026-11-15T10:01:00Z`
**Source review:** `.planning/workstreams/axis-warp-terminal-v75/phases/442-bidirectional-chat-harness-and-ast-security/442-REVIEW.md`
**Iteration:** `1`

**Summary:**
- Findings in scope: `18` (`CR-01..CR-06`, `WR-01..WR-08`, `IN-01..IN-04`)
- Fixed: `18`
- Skipped: `0`
- New `Cargo.toml` / `Cargo.lock` crate-edges added: `0`
- `god_object` non-blank non-test LoC ceiling (`< 500`): verified across all `11` touched/created `.rs` files (`181`..`488` LoC)

## Fixed Issues

### Critical (`CR-01` .. `CR-06`)

- **`CR-01` (`crates/vox-orchestrator/src/oversight/exec_policy.rs`):** Replaced byte-cast `cur.push(b[i] as char)` in `scan_bal` and `lex_all` with UTF-8 scalar decoding (`s[i..].chars().next()`, advancing by `ch.len_utf8()`), eliminating UTF-8 corruption and byte-index slice panics. Added multi-byte UTF-8 test (`"echo 'Salvē, Rōma — 🚀'"`).
- **`CR-02` (`crates/vox-orchestrator/src/oversight/exec_policy.rs`):** Fixed trailing `\` at EOF in `lex_all` (`trailing_esc` flag) so final command segments are unconditionally flushed for `extract_effective_commands` / `classify_floor` while `tokenize_and_split_segments`, `evaluate_command_ast`, and `evaluate_security_tier` fail closed with `trailing-backslash-escape`. Added regression test `"pwd && git push --force origin main \\"`.
- **`CR-03` (`crates/vox-orchestrator/src/oversight/exec_policy.rs`):** Added case-insensitive PowerShell prefix matching (`is_pwsh_encoded_flag` for `-e`..`-encodedcommand` / `-ea`..`-encodedarguments` and `/` forms; `is_pwsh_command_flag` for `-c`..`-command` and `/` forms; plus `-ex`/`-w`/`-config` option pairs) in `unwrap_argv`. Added regression tests for `pwsh -enco ...` and `pwsh -com ...`.
- **`CR-04` (`crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs`, `term_mentions.rs`):** Replaced the byte-walking ANSI stripper in `scrub_terminal_mcp_str` with `term_mentions::strip_ecma48_and_normalize_cr`, providing 100% char-safe UTF-8 handling (`\x1b` before multi-byte scalars never panics), full ECMA-48 (`CSI`, `OSC`, `DCS`, `SOS`, `PM`, `APC`, `nF`/`Fp` including `\x1b(B`), and stateful CR column-0 overwrite (preserving trailing `\r` lines at EOF).
- **`CR-05` (`terminal_bridge.rs`, `terminal_bridge_ephemeral.rs`, `agent_exec.rs`, `term_bridge_impl.rs`, `term_dispatch.rs`, `terminal_tools.rs`):**
  - Threaded `#[serde(default)] pub hitl_approved: bool` through `TermExecRequest`, `AgentExecRequest`, `TermSendInputRequest`, `LocalHostTerminalBridge`, and `LocalHost::{exec_agent_command, agent_send_input}` (`allows_agent_command_with_approval`) so human-approved HITL overrides in `Manual` and `CoPilot` modes execute cleanly up to `L5AutopilotFloor` while still unconditionally blocking hard AST hazards.
  - Moved `bridge.read` + `credential_prompt` (`CREDENTIAL_PROMPT_USER_ONLY`) check BEFORE `list_sessions()` and `park_hitl` in `vox_terminal_send_input`, preventing wasted HITL approvals and eliminating the 300s approval timeout on credential prompts in `Manual` sessions.
- **`CR-06` (`crates/vox-orchestrator/src/oversight/floor.rs`):** Updated `safety_floor_unwraps_compound_and_wrapped_history_rewrites` assertion to `&["vox_run_shell", "vox_skill_run", "vox_terminal_exec"]` and added floor coverage for `vox_terminal_exec`, trailing `\`, and `pwsh -com`.

### Warnings (`WR-01` .. `WR-08`)

- **`WR-01` (`crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs`):** Propagated `list_sessions()` errors fail-closed (`err_j`) and defaulted missing/unlisted `session_id` or unrecognized `control_mode`/`security_level` values to `("manual", SecurityTier::L1ReadOnly)`.
- **`WR-02` (`agent_exec.rs`, `terminal_bridge_ephemeral.rs`, `terminal_tools.rs`):**
  - Preserved `req.wait_ms == 0` (`background: true`) in `LocalHost::exec_agent_command` so background commands return `AgentExecStatus::Running` immediately after PTY submission without clamping to `50ms`.
  - Rejected `wait_ms == 0` explicitly in `EphemeralTerminalBridge::exec` instead of spawning and killing a child process after `50ms`.
  - Honored `wait_ms` (`default: 5000`, `0..=120000`) in `vox_terminal_send_input` via bounded post-input polling until `!running` or deadline.
- **`WR-03` (`crates/vox-orchestrator/src/terminal_bridge.rs`, `terminal_bridge_ephemeral.rs`):** Replaced `EphemeralTerminalBridge`'s incomplete byte-level ANSI stripper and `PiiFilter`-only pass with char-safe ECMA-48 + CR overwrite normalization, Clavis `scrub_resolved_secrets` first, and regex redaction for PEM blocks, URL credentials, emails, Bearer/API keys, OpenAI/Anthropic/GitHub/HF/NPM/Slack/AWS/Google/JWT tokens, RFC1918/loopback IPv4, and Unix/Windows home paths — with `0` new `Cargo.toml` dependencies. Extracted `terminal_bridge_ephemeral.rs` so both files stay well below the 500-LoC `god_object` cap.
- **`WR-04` (`crates/vox-orchestrator-mcp/src/chat_tools/chat/term_mentions.rs`):** Escaped `<terminal_block` / `</terminal_block` and `<terminal_context` / `</terminal_context` (case-insensitive) to `&lt;...` inside resolved `@term` command/output payloads, with unit test coverage.
- **`WR-05` (`crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs`):** Widened `max_blocks` fetch to `Some(500)` when `pinned_only` or `failed_only` is set without `block_id`, applying `.retain(...)` filtering BEFORE `tail_blocks` truncation.
- **`WR-06` (`crates/vox-term/src/app.rs`):** Updated `Ctrl+C` (`TermSignal::Interrupt`) and empty `Enter` (`host.submit`) guards to treat `BlockStatus::AwaitingInput` identically to `BlockStatus::Running`.
- **`WR-07` (`crates/vox-terminal-core/src/agent_exec.rs`):** Added `sess.is_alt_screen()` guard inside `LocalHost::exec_agent_command` before opening an agent block so active fullscreen TUIs (`?1049h`) reject agent shell injection fail-closed.
- **`WR-08` (`crates/vox-orchestrator-d/src/term_bridge_impl.rs`, `term_dispatch.rs`):** Added `validate_daemon_agent_exec` enforcing `ExecPolicyV1` AST validation and `classify_floor("vox_terminal_exec", ...)` (unless `hitl_approved`) at both `term.agent_exec` RPC dispatch and `LocalHostTerminalBridge::exec`.

### Info (`IN-01` .. `IN-04`)

- **`IN-01` (`console_a2a.rs`, `term_bridge_impl.rs`, `terminal_bridge_ephemeral.rs`):** Normalized single-bound line ranges (`line_end = line_end.or(line_start)` in `console_a2a.rs` emitting `@term:{sid}#{block_id}:L{s}-{s}`, and `(Some(ls), None) => (ls, ls)` / `(None, Some(le)) => (1, le)` in `term_bridge_impl.rs` and `terminal_bridge_ephemeral.rs`).
- **`IN-02` (`term_bridge_impl.rs`, `term_mentions.rs`, `console_a2a.rs`, `terminal_tools.rs`, `terminal_bridge.rs`):** Standardized 3-stage scrubbing order across all terminal surfaces to `ECMA-48 + CR normalize` → `vox_secrets::scrub_resolved_secrets` (exact Aho-Corasick needles) → `vox_redact` regex pass so compound Clavis secrets are never fragmented before exact matching.
- **`IN-03` (`terminal_tools.rs`, `term_mentions.rs`, `terminal_bridge.rs`):** Deduplicated MCP scrubbing onto `term_mentions::strip_ecma48_and_normalize_cr` and aligned `terminal_bridge.rs`'s local normalizer to the identical char-safe ECMA-48 + CR state machine while keeping `0` new crate edges.
- **`IN-04` (`crates/vox-gui/ui/src/transport.ts`):** Added optional `resync_required?`, `alt_screen?`, `cwd?`, and `git_branch?` fields to `TermAttachmentDto`.

## Verification Performed in Fixer Pass

- `rustfmt --edition 2024` run individually on all `11` touched/created `.rs` files (all exit `0`).
- Non-blank non-test LoC (`god_object` check) verified `< 500` on every touched/created `.rs` file.
- `timeout 120s pnpm --dir crates/vox-gui/ui exec vitest run src/guards`: `10 passed (10)`, `40 passed (40)`.
- Unified Cargo check/test pass deferred to parent orchestrator per `<strict_rules>`.
