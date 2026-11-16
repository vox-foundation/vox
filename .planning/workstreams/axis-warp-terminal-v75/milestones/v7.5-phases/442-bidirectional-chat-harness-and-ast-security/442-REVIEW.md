---
phase: 442-bidirectional-chat-harness-and-ast-security
reviewed: 2026-11-15T09:02:00Z
depth: standard
files_reviewed: 41
files_reviewed_list:
  - crates/vox-orchestrator/src/oversight/exec_policy.rs
  - crates/vox-orchestrator/src/oversight/floor.rs
  - crates/vox-orchestrator/src/oversight/mod.rs
  - crates/vox-orchestrator/src/terminal_bridge.rs
  - crates/vox-cli-research/src/check_terminal.rs
  - crates/vox-redact/src/lib.rs
  - crates/vox-secrets/src/redact.rs
  - crates/vox-secrets/src/lib.rs
  - crates/vox-terminal-core/src/block.rs
  - crates/vox-terminal-core/src/corpus/redact.rs
  - crates/vox-terminal-core/src/pty.rs
  - crates/vox-terminal-core/src/shell_init.rs
  - crates/vox-terminal-core/src/stall_detector.rs
  - crates/vox-terminal-core/src/agent_exec.rs
  - crates/vox-terminal-core/src/session.rs
  - crates/vox-terminal-core/src/host.rs
  - crates/vox-terminal-core/src/daemon_host.rs
  - crates/vox-terminal-core/src/lib.rs
  - crates/vox-term/src/app.rs
  - crates/vox-term/src/ui/blocks.rs
  - crates/vox-orchestrator-d/src/term_bridge_impl.rs
  - crates/vox-orchestrator-d/src/term_dispatch.rs
  - crates/vox-orchestrator-d/src/lib.rs
  - crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs
  - crates/vox-orchestrator-mcp/src/server_state.rs
  - crates/vox-orchestrator-mcp/src/input_schemas/terminal.rs
  - crates/vox-orchestrator-mcp/src/input_schemas.rs
  - crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs
  - crates/vox-orchestrator-mcp/src/dispatch_domains/mod.rs
  - crates/vox-orchestrator-mcp/src/dispatch.rs
  - crates/vox-orchestrator-mcp/src/permission_modes.rs
  - crates/vox-orchestrator-mcp/src/dispatch_timeout.rs
  - crates/vox-orchestrator-mcp/src/llm_bridge/tool_selection.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/term_mentions.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/mod.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/mentions.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs
  - crates/vox-gui/src/commands/pty.rs
  - crates/vox-gui/src/commands/console_a2a.rs
  - crates/vox-gui/src/main.rs
  - crates/vox-gui/ui/src/transport.ts
findings:
  critical: 6
  warning: 8
  info: 4
  total: 18
status: issues_found
---

# Phase 442: Code Review Report

**Reviewed:** 2026-11-15T09:02:00Z
**Depth:** standard
**Files Reviewed:** 41
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

Reviewed all 41 Phase 442 source files across 9 crates (`vox-orchestrator`, `vox-cli-research`, `vox-redact`, `vox-secrets`, `vox-terminal-core`, `vox-term`, `vox-orchestrator-d`, `vox-orchestrator-mcp`, and `vox-gui`) against `442-CONTEXT.md`, `442-01..06-PLAN.md`, and workspace contracts (`contracts/orchestration/permission-modes.v2.yaml`, `crates/vox-orchestrator-mcp/tests/fixtures/dev-advertised-tools.default-features.txt`).

**Positive architecture & concurrency verification:**
- Cross-crate trait boundaries (`TerminalBridge`, `OrchDaemonTerminalBridge`, `EphemeralTerminalBridge`, `LocalHostTerminalBridge`) align cleanly, and `dev-advertised-tools.default-features.txt` as well as `permission-modes.v2.yaml` match `RISK_CLASSES` and `SHELL_COMMAND_TOOLS`.
- Lock hygiene across async boundaries is clean: `parking_lot::RwLock` guards in `vox-orchestrator-mcp/src/server_state.rs` are dropped before every `.await` point; `vox-secrets/src/redact.rs` computes Clavis needles outside the `SECRET_NEEDLE_CACHE` `RwLock`; and `vox-terminal-core/src/agent_exec.rs` releases `Session` mutexes properly inside `spawn_blocking` Condvar loops.
- Credential prompt fail-closed guards (`credential_prompt: true`) in `stall_detector.rs`, `agent_exec.rs`, and `terminal_tools.rs` block agent stdin injection before `pty.write_input`.
- Tauri command registrations (`main.rs`), camelCase/snake_case IPC bindings (`transport.ts` <-> `commands/pty.rs` / `commands/console_a2a.rs`), and `remote::manifest` host/daemon classifications match.

**Key concerns requiring remediation (`6 Critical`, `8 Warning`, `4 Info`):**
1. **UTF-8 char-boundary panics (`CR-01`, `CR-04`):** Byte-indexed slicing `&s[i..]` / `&raw[i..]` in `exec_policy::lex_all` and `terminal_tools::scrub_terminal_mcp_str` panics unconditionally on multi-byte UTF-8 characters.
2. **AST security / floor bypasses (`CR-02`, `CR-03`):** A trailing backslash `\` in `lex_all` skips the final segment flush (`pwd && git push --force origin main \` evaluates as `[pwd]`), and PowerShell prefix-abbreviated flags (`pwsh -enco <base64>`, `pwsh -com ...`) bypass `ENC_W` and `SHELL_C`.
3. **Broken `Manual`-mode HITL override in production (`CR-05`):** After human approval in `terminal_tools.rs`, `LocalHost::exec_agent_command` and `LocalHost::agent_send_input` still unconditionally `bail!` on `TermControlMode::Manual` because no `hitl_approved` flag crosses `TerminalBridge` (masked by `MockBridge` in unit tests).
4. **Wave 1 vs Wave 3 unit-test regression (`CR-06`):** `oversight::floor::tests::shell_command_tools_route_through_history_rewrite_floor` still asserts the 2-element Wave 1 slice after Wave 3 added `"vox_terminal_exec"`.

## Critical Issues

### CR-01: `lex_all` panics on any multi-byte UTF-8 character in shell commands

**File:** `crates/vox-orchestrator/src/oversight/exec_policy.rs:137`
**Issue:** `lex_all` indexes `b = s.as_bytes()` and advances `i` by `1` or `2` raw bytes inside single quotes (`L184`), backslash escapes (`L185`), double quotes (`L208`), and unquoted words (`L214`). On the next loop iteration, `L137` evaluates `let (ch, r) = b.get(i).map_or((b'\n', ""), |&c| (c, &s[i..]));`. Whenever a command contains any multi-byte UTF-8 codepoint (e.g. `echo "🦀"`, `rg "═" src/`, `cd "/Users/café"`), `i` lands inside a multi-byte UTF-8 sequence and `&s[i..]` panics at runtime (`byte index N is not a char boundary`), crashing `vox_terminal_exec`, `classify_floor`, `LocalHost::exec_agent_command`, and `vox shell check`.
**Fix:**
```rust
while i <= b.len() {
    if i == b.len() {
        push(&mut out, &mut seg, &mut cur, &mut cur_has, None);
        break;
    }
    let ch = b[i];
    if q == 0 {
        if let Some(op) = ["&&", "||", "|&", ";", "|", "&", "\n"]
            .into_iter()
            .find(|op| b[i..].starts_with(op.as_bytes()))
        {
            push(&mut out, &mut seg, &mut cur, &mut cur_has, Some(op));
            i += op.len();
            continue;
        }
    }
    // Advance by UTF-8 codepoint width when consuming literal text:
    // let step = s[i..].chars().next().map_or(1, char::len_utf8);
    // cur.push_str(&s[i..i + step]); i += step;
}
```
(Apply the same `&b[i..]` byte-prefix check or UTF-8 char-width step inside `scan_bal` at `exec_policy.rs:101-118` if `&s[..]` slicing is ever added there.)

### CR-02: Trailing backslash `\` in `lex_all` drops the final command segment, bypassing `ExecPolicyV1`, `SecurityTier`, and `classify_floor`

**File:** `crates/vox-orchestrator/src/oversight/exec_policy.rs:134-142,185`
**Issue:** `lex_all` only flushes the final command segment when `i == b.len()` (`L140`). When the last byte of `s.trim()` is `\` (`i == b.len() - 1`), arm `(0 | b'"', b'\\')` at `L185` sets `cur_has = true; i += 2;` (`i == b.len() + 1`). The loop condition `while i <= b.len()` (`L134`) immediately terminates without executing the `i == b.len()` flush! Consequently:
- `"pwd && git push --force origin main \\"` lexes as a single segment `[["pwd"]]`, bypassing `classify_floor` (`history_rewrite` floor), `DENY_RULES`, and `evaluate_security_tier` (`T1` instead of `T4`/`Deny`).
- Meanwhile `EphemeralTerminalBridge::exec` (`crates/vox-orchestrator/src/terminal_bridge.rs:458`) interpolates `{trimmed}\n__vox_ec=$?` (POSIX line-continuation) or `LocalHost::exec_agent_command` writes the command into the live PTY.
**Fix:**
```rust
(0 | b'"', b'\\') => {
    if i + 1 >= b.len() {
        return Err("trailing-backslash-escape".into());
    }
    let next_len = s[i + 1..].chars().next().map_or(1, char::len_utf8);
    cur.push_str(&s[i + 1..i + 1 + next_len]);
    cur_has = true;
    i += 1 + next_len;
}
```
Also assert post-loop invariants or flush unconditionally if `cur_has || !seg.is_empty()` after the `while` loop so no segment can ever be silently dropped.

### CR-03: PowerShell prefix-abbreviated `-EncodedCommand` and `-Command` flags bypass `ENC_W` and `SHELL_C`

**File:** `crates/vox-orchestrator/src/oversight/exec_policy.rs:16-18,273-287`
**Issue:** PowerShell (`pwsh`, `powershell`, `powershell.exe`) natively accepts any unambiguous case-insensitive prefix of `-EncodedCommand` (`-e`, `-ec`, `-en`, `-enc`, `-enco`, `-encod`, `-encode`, `-encoded`, `-encodedc`, `-encodedco`, ..., plus `/` prefixes on Windows PowerShell) and `-Command` (`-c`, `-co`, `-com`, `-comm`, `-comma`, `-comman`, `-command`). `ENC_W` (`L18`) only lists `["-e", "-ec", "-enc", "-encoded", "-encodedcommand", "--encodedcommand"]`, and `SHELL_C` (`L16`) only lists `["-c", "-command", "--command"]`. As a result:
- `pwsh -enco <base64>` hits `t.starts_with('-') => wi += 1` (`L286`), treats `<base64>` as an unknown command name (`SecurityTier::T3`), and auto-executes under default `CoPilot` + `L3StructuredDefault` without HITL approval, bypassing `DENY_RULES` and `classify_floor`.
- `pwsh -com "git push --force origin main"` skips recursive AST parsing of the inner script string.
**Fix:**
```rust
fn is_pwsh_bin(b0: &str) -> bool {
    matches!(b0, "pwsh" | "powershell")
}

fn is_pwsh_encoded_flag(flag: &str) -> bool {
    let f = flag.strip_prefix("--").or_else(|| flag.strip_prefix('-')).or_else(|| flag.strip_prefix('/'));
    matches!(f, Some(s) if s == "ec" || (s.len() >= 1 && "encodedcommand".starts_with(s)))
}

fn is_pwsh_command_flag(flag: &str) -> bool {
    let f = flag.strip_prefix("--").or_else(|| flag.strip_prefix('-')).or_else(|| flag.strip_prefix('/'));
    matches!(f, Some(s) if !s.is_empty() && "command".starts_with(s))
}
```
Check `is_pwsh_bin(b0.as_str()) && is_pwsh_encoded_flag(&t)` before `SHELL_C` / `t.starts_with('-')` in `unwrap_argv`.

### CR-04: `scrub_terminal_mcp_str` panics on `\x1b` before multi-byte UTF-8, leaks non-CSI/OSC escapes, and inverts Stage 2 / Stage 3 redaction order

**File:** `crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs:26-69`
**Issue:** Three bugs exist in `scrub_terminal_mcp_str`:
1. **UTF-8 boundary panic (`L54`, `L58`):** When `0x1b` is followed by any byte other than `[` or `]` (e.g., `\x1b` followed by multi-byte UTF-8 `\xf0\x9f\xa6\x80`), `_ => i += usize::from(i < b.len())` advances `i` by `1` byte into the middle of the multi-byte character, causing `raw[i..].chars().next()` at `L58` to panic (`byte index is not a char boundary`).
2. **Incomplete ECMA-48 stripping:** Charset designators (`\x1b(B`) leave `B` in `plain` (turning `sk-\x1b(Bproj-...` into `sk-Bproj-...`, evading regex/Clavis redaction while rendering cleanly in a terminal), and DCS/APC/PM/SOS sequences (`\x1bP...\x1b\\`) leak unstripped. Bare trailing `\r` (`"secret\r"`) also wipes the entire line buffer even when nothing overwrites it.
3. **Stage 2 / Stage 3 inversion (`L68`):** `vox_redact::redact_owned(&vox_secrets::scrub_resolved_secrets(&plain))` runs Stage 3 (`vox_secrets`) before Stage 2 (`vox_redact`), violating the locked Stage 1 -> Stage 2 -> Stage 3 contract in `442-CONTEXT.md`.
**Fix:**
```rust
fn scrub_terminal_mcp_str(raw: &str) -> String {
    let stage1 = crate::chat_tools::chat::term_mentions::strip_ecma48_and_normalize_cr(raw);
    let stage2 = vox_redact::redact_owned(&stage1);
    vox_secrets::scrub_resolved_secrets(&stage2)
}
```
(Make `strip_ecma48_and_normalize_cr` `pub(crate)` or extract it into `vox-redact` so all 4 crates share one verified implementation.)

### CR-05: Human-approved HITL overrides in `Manual` control mode unconditionally fail in `LocalHost`

**File:** `crates/vox-terminal-core/src/agent_exec.rs:233-238,426-428` (and `crates/vox-orchestrator/src/terminal_bridge.rs:51-77`, `crates/vox-orchestrator-d/src/term_bridge_impl.rs:76-124`, `crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs:208-238,289-336`)
**Issue:** When a terminal session is in `TermControlMode::Manual`, `terminal_tools.rs` (`L208-214`, `L289-298`) parks on `PendingApprovals` (or accepts `outer_hitl_approved` under `PermissionMode::Ask`) and, once the human operator approves, calls `bridge.exec(req)` / `bridge.send_input(send_req)`. Unit tests in `terminal_tools.rs:474-495` pass only because `MockBridge` ignores `control_mode`. In production, `LocalHostTerminalBridge` calls `LocalHost::exec_agent_command` (`agent_exec.rs:233-238`) and `LocalHost::agent_send_input` (`agent_exec.rs:426-428`), both of which **unconditionally `anyhow::bail!`** whenever `*lock(&entry.control_mode) == TermControlMode::Manual` because no `hitl_approved` boolean is carried on `TermExecRequest`, `AgentExecRequest`, or `TermSendInputRequest`. Additionally, in `vox_terminal_send_input` (`terminal_tools.rs:289-325`), `park_hitl` runs *before* checking `t.credential_prompt`, asking the human to approve a password prompt only to reject it afterward!
**Fix:**
1. Move the `bridge.read` + `credential_prompt` check in `vox_terminal_send_input` *above* `park_hitl` so credential prompts fail closed immediately without showing a useless HITL prompt.
2. Add `#[serde(default)] pub hitl_approved: bool` to `TermExecRequest`, `AgentExecRequest`, and `TermSendInputRequest` (default `false`), set `hitl_approved = outer_hitl_approved || approved_via_park_hitl` in `terminal_tools.rs`, forward it in `LocalHostTerminalBridge`, and gate the `Manual` rejection in `agent_exec.rs`:
```rust
if *lock(&entry.control_mode) == TermControlMode::Manual && !req.hitl_approved {
    anyhow::bail!(
        "session `{}` is in Manual control mode; explicit HITL approval is required",
        req.session_id
    );
}
```

### CR-06: Stale `SHELL_COMMAND_TOOLS` assertion in `oversight::floor::tests` breaks `vox-orchestrator` unit tests

**File:** `crates/vox-orchestrator/src/oversight/floor.rs:89,628`
**Issue:** Wave 3 (`442-04`) updated `SHELL_COMMAND_TOOLS` at `floor.rs:89` to `&["vox_run_shell", "vox_skill_run", "vox_terminal_exec"]`, but left the Wave 1 (`442-01`) test assertion at `floor.rs:628` unchanged (`assert_eq!(SHELL_COMMAND_TOOLS, &["vox_run_shell", "vox_skill_run"]);`). Because Wave 3 deferred Cargo execution under `target/.cargo-lock` contention, this deterministic test failure was missed.
**Fix:**
```rust
assert_eq!(
    SHELL_COMMAND_TOOLS,
    &["vox_run_shell", "vox_skill_run", "vox_terminal_exec"]
);
assert_eq!(
    classify_floor(&FloorInput::tool(
        "vox_terminal_exec",
        &serde_json::json!({ "command": "git push --force origin main" })
    )),
    Some(FloorCategory::HistoryRewrite)
);
```

## Warnings

### WR-01: Fail-open `list_sessions().await.unwrap_or_default()` silently downgrades `Manual` / `L1ReadOnly` sessions to `CoPilot` + `L3StructuredDefault`

**File:** `crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs:188-206,285-288`
**Issue:** In `vox_terminal_exec` (`L188-206`), if `bridge.list_sessions().await` returns an IPC error or does not contain `sid`, `.unwrap_or_default()` and `.map_or(("co_pilot", SecurityTier::L3StructuredDefault), ...)` silently assume `CoPilot` + `L3StructuredDefault`. Under `PermissionMode::AcceptAll`, this skips HITL parking for sessions the operator restricted to `Manual`, `L1ReadOnly`, or `L2WorkspaceBuild`. Likewise, `vox_terminal_send_input` (`L285-288`) defaults `manual` to `false` when `list_sessions()` errors.
**Fix:** Propagate `bridge.list_sessions().await` errors fail-closed (`let sessions = match bridge.list_sessions().await { Ok(s) => s, Err(e) => return err_j(&e) };`), and when `sid` is `Some(...)` and not ephemeral, fail closed or default unknown sessions to `("manual", SecurityTier::L1ReadOnly)`.

### WR-02: `background: true` / `wait_ms: 0` clamps to `50ms` (and kills ephemeral subprocesses), while `vox_terminal_send_input` ignores `wait_ms`

**File:** `crates/vox-terminal-core/src/agent_exec.rs:34,317` (and `crates/vox-orchestrator/src/terminal_bridge.rs:424,492`, `crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs:215-219,334-346`)
**Issue:**
1. `terminal_tools.rs:215` maps `background: true` to `wait_ms = 0`, expecting immediate `status: "running"` return per `442-CONTEXT.md`. However, `LocalHost::exec_agent_command` (`agent_exec.rs:317`) clamps `req.wait_ms.clamp(50, 120_000)`, blocking at least 50ms, and `EphemeralTerminalBridge::exec` (`terminal_bridge.rs:424,492`) clamps to 50ms and sets `cmd.kill_on_drop(true)` — killing the "background" command as soon as the 50ms timeout fires!
2. Conversely, `input_schemas/terminal.rs:141-147` advertises `wait_ms` (default `5000`) on `vox_terminal_send_input`, but `terminal_tools.rs:334-346` never reads `args.get("wait_ms")`, calling `bridge.read` immediately (`0ms`) before the PTY child can process stdin or emit output.
**Fix:** Allow `wait_ms == 0` in `LocalHost::exec_agent_command` to return `AgentExecStatus::Running` immediately after writing the wrapped command; reject `background: true` with an actionable error on `EphemeralTerminalBridge` (no persistent daemon PTY); and honor `wait_ms` in `vox_terminal_send_input` before the readback.

### WR-03: `EphemeralTerminalBridge` uses `PiiFilter` instead of `vox-redact` and skips CR overwrite / full ECMA-48 normalization

**File:** `crates/vox-orchestrator/src/terminal_bridge.rs:329-363`
**Issue:** When standalone `vox-mcp` uses `EphemeralTerminalBridge` (or any non-MCP caller reads directly from `EphemeralTerminalBridge`), `scrub_ephemeral_text`:
1. Drops `\r` instead of normalizing carriage-return overwrites (`"10%\r100%"` becomes `"10%100%"`).
2. Strips only CSI/OSC sequences, leaving 2-byte ESC/charset/DCS escapes intact (`sk-\x1b7proj-...` evades token regexes).
3. Calls `PiiFilter::redact` (`crates/vox-orchestrator/src/pii_filter.rs:9-35`), which only matches email, IPv4, and `(sk|gsk|pk|ak)-[a-zA-Z0-9]{20+}` — missing `sk-proj-...` (hyphenated), `ghp_`/`github_pat_`, `AKIA`, `AIza`, `Bearer`, URL `user:pass@host`, PEM private keys, and local home-directory paths covered by `vox-redact`.
**Fix:** Add `vox-redact` to `vox-orchestrator` (leaf crate with zero workspace deps) and run Stage 1 ECMA-48 + CR normalization -> `vox_redact::redact_owned` -> `vox_secrets::scrub_resolved_secrets` in `scrub_ephemeral_text`.

### WR-04: Unescaped `</terminal_block>` / `</terminal_context>` tags in `@term` expansion enable XML envelope breakout after prompt canonicalization

**File:** `crates/vox-orchestrator-mcp/src/chat_tools/chat/term_mentions.rs:256-279` (and `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs:175-176`)
**Issue:** `message.rs:175-176` runs `expand_term_mentions` *after* `prompt_canonical::canonicalize_prompt` so stack traces are not split into numbered objectives. However, `format_resolved_block` (`term_mentions.rs:262-278`) only escapes XML attributes (`xml_attr_escape`) and embeds `cmd` and `out` verbatim inside `<terminal_block ...>...\n</terminal_block>`. Any terminal output containing literal `</terminal_block>` or `</terminal_context>` breaks out of `<terminal_context>` and injects raw uncanonicalized text directly into the user turn.
**Fix:** Escape `<` and `&` (or replace `</terminal_block` and `</terminal_context` case-insensitively with `&lt;/terminal_block` / `&lt;/terminal_context`) in `cmd` and `out` inside `format_resolved_block`.

### WR-05: `vox_terminal_read` with `pinned_only: true` or `failed_only: true` slices to the last 10 blocks before filtering

**File:** `crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs:247-267` (and `crates/vox-orchestrator-d/src/term_bridge_impl.rs:168-170`)
**Issue:** When `tail_blocks` is omitted on `vox_terminal_read`, `terminal_tools.rs:252` passes `max_blocks: None`. `LocalHostTerminalBridge::read` (`term_bridge_impl.rs:168`) and `EphemeralTerminalBridge::read` (`terminal_bridge.rs:548`) default `max_blocks` to `10` *before* returning to `terminal_tools.rs:259-266`, where `.retain(|b| b.pinned)` and `.retain(|b| failed)` run. Any pinned or failed block older than the 10 most recent blocks is silently omitted.
**Fix:** When `pinned_only` or `failed_only` is `true`, pass `max_blocks: Some(500)` (`MAX_BLOCKS_PER_SESSION`) in `TermReadRequest`, run `.retain(...)`, and apply `tail_blocks` truncation *after* filtering.

### WR-06: `vox-term` `Ctrl+C` exits the TUI and empty `Enter` is ignored when a block is `BlockStatus::AwaitingInput`

**File:** `crates/vox-term/src/app.rs:516-536`
**Issue:** Once the stall detector transitions a running block from `BlockStatus::Running` to `BlockStatus::AwaitingInput` (e.g. `[sudo] password:` or `Continue? [Y/n]`):
1. `Ctrl+C` (`L516-524`) checks `b.status == BlockStatus::Running`; because status is `AwaitingInput`, pressing `Ctrl+C` exits `vox-term` (`return Ok(true)`) instead of sending `TermSignal::Interrupt` to cancel the stalled prompt.
2. `Enter` with an empty input line (`L531-536`) also checks `b.status == BlockStatus::Running` and drops the keypress instead of sending `\n` to accept default interactive prompts (`[Y/n]`, `Press Enter to continue`).
**Fix:** Replace `b.status == BlockStatus::Running` with `matches!(b.status, BlockStatus::Running | BlockStatus::AwaitingInput)` at `app.rs:517` and `app.rs:533`.

### WR-07: `LocalHost::exec_agent_command` does not reject sessions in `ALT_SCREEN` when `open_block_id` is `None`

**File:** `crates/vox-terminal-core/src/agent_exec.rs:241-250`
**Issue:** `exec_agent_command` checks `sess.open_block_id()` (`L243`) to ensure the shell is idle, but never checks `sess.is_alt_screen()`. If a session entered alternate screen (`\x1b[?1049h`, e.g. `vim`, `less`, `htop`) without an open OSC 633 block (for example in a subshell or non-OSC633 session), `exec_agent_command` injects `__vox_pager_enter; <command>\n` directly as live keystrokes into the TUI application.
**Fix:** Add `if sess.is_alt_screen() { anyhow::bail!("session `{}` is in alternate-screen TUI mode; exit the TUI or use another session", req.session_id); }` inside the session lock block at `agent_exec.rs:242-250`.

### WR-08: Daemon `term.agent_exec` RPC lacks defense-in-depth `ExecPolicyV1` / floor enforcement and splits `PendingApprovals` in standalone `vox-mcp`

**File:** `crates/vox-orchestrator-d/src/term_dispatch.rs:320-329` (and `crates/vox-orchestrator-d/src/term_bridge_impl.rs:76-112`, `crates/vox-orchestrator-mcp/src/server_state.rs:299-310`)
**Issue:** `term_dispatch.rs` blocks agent access to `term.input`/`term.submit`/`term.signal` via `refuse_agent_origin`, yet exposes `term.agent_exec` (`L320-324`) without running `classify_floor` or `evaluate_command_ast` in `LocalHostTerminalBridge::exec`. Any local client holding the auto-discovered daemon token can invoke `term.agent_exec` bypassing `DENY_RULES` and `history_rewrite` floor checks. Additionally, when standalone `vox-mcp` connects via `OrchDaemonTerminalBridge` (`server_state.rs:299`), HITL parking happens on standalone `vox-mcp`'s in-memory `PendingApprovals` rather than `vox-orchestrator-d`'s GUI-visible `PendingApprovals`, leaving GUI operators unable to see or approve parked requests from standalone MCP processes.
**Fix:** Enforce `classify_floor` and `evaluate_command_ast` (`DENY_RULES` hard-reject) inside `LocalHostTerminalBridge::exec` as defense-in-depth, and document or proxy standalone `vox-mcp` HITL parking through `vox-orchestrator-d` when daemon-backed.

## Info

### IN-01: `send_terminal_block_to_agent_impl` emits unsupported `:L<start>` single-line `@term` syntax and skips single-line slicing when `line_end` is `None`

**File:** `crates/vox-gui/src/commands/console_a2a.rs:116-143`
**Issue:** When `line_start` is `Some(s)` and `line_end` is `None`, `console_a2a.rs:140` formats `@term:{sid}#{block_id}:L{s}` (which does not match `TERM_MENTION_RE`'s required `:L<start>-<end>` syntax in `term_mentions.rs:23`), and `term_bridge_impl.rs:139` only slices lines when both `line_start` and `line_end` are `Some`.
**Fix:** Normalize `let line_end = line_end.or(line_start);` at the top of `send_terminal_block_to_agent_impl` and format `:L{s}-{e}`.

### IN-02: Stage-2 regex redaction before Stage-3 Clavis needle scrubbing can fragment compound secrets

**File:** `crates/vox-orchestrator-d/src/term_bridge_impl.rs:23-26`
**Issue:** When a Clavis-managed secret (such as a `DATABASE_URL` or webhook URL) contains an internal email, IPv4 address, or token substring, Stage 2 (`vox_redact::redact_owned`) replaces the inner substring first, preventing Stage 3 (`vox_secrets::scrub_resolved_secrets`) from matching the full secret needle and leaving custom suffix/query fragments unredacted.
**Fix:** Consider running `vox_secrets::scrub_resolved_secrets` immediately after Stage 1 ECMA-48 normalization *and* (or before) Stage 2 `vox_redact::redact_owned` so exact Clavis secrets are scrubbed whole before heuristic regexes run.

### IN-03: Four duplicate ECMA-48 / CR stripping implementations across crates

**File:** `crates/vox-terminal-core/src/block.rs:120-200` (and `crates/vox-orchestrator-mcp/src/chat_tools/chat/term_mentions.rs:138-231`, `crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs:30-67`, `crates/vox-orchestrator/src/terminal_bridge.rs:333-360`)
**Issue:** Phase 442 introduced four separate ANSI/CR strippers across four crates, directly causing the UTF-8 panic and escape-leak divergences in `CR-04` and `WR-03`.
**Fix:** Consolidate `strip_ansi_and_normalize` into leaf crate `vox-redact` (`0` workspace dependencies) and call it everywhere Stage 1 normalization is needed.

### IN-04: `TermAttachmentDto` TypeScript interface omits optional fields serialized by Rust

**File:** `crates/vox-gui/ui/src/transport.ts:913-918`
**Issue:** Rust `TermAttachmentDto` (`crates/vox-gui/src/commands/pty.rs:30-44`) serializes `resync_required: boolean`, `alt_screen: boolean`, `cwd?: string | null`, and `git_branch?: string | null`, which are omitted from `TermAttachmentDto` in `transport.ts:913-918`.
**Fix:** Add `resync_required?: boolean; alt_screen?: boolean; cwd?: string | null; git_branch?: string | null;` to `TermAttachmentDto` in `transport.ts` ahead of Phase 443 frontend consumption.

---

_Reviewed: 2026-11-15T09:02:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
