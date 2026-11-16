---
phase: "442"
slug: "bidirectional-chat-harness-and-ast-security"
status: validated
nyquist_compliant: true
wave_0_complete: true
created: "2026-11-14"
---

# Phase 442 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (`libtest` + `tokio::test` 1.x) + `vox-arch-check` + `vox-cli ci` contract gates |
| **Config file** | `crates/{vox-redact,vox-secrets,vox-orchestrator,vox-cli-research,vox-terminal-core,vox-orchestrator-d,vox-orchestrator-mcp,vox-gui}/Cargo.toml` |
| **Quick run command** | `timeout 300s cargo test -p vox-orchestrator --lib exec_policy` |
| **Full suite command** | Per-crate scoped runs (`vox-redact`, `vox-secrets`, `vox-orchestrator`, `vox-cli-research`, `vox-terminal-core`, `vox-orchestrator-d`, `vox-orchestrator-mcp`, `vox-gui --bin vox-gui`) |
| **Estimated runtime** | ~45 seconds per crate |

---

## Sampling Rate

- **After every task:** Run the task's scoped `timeout 300s cargo test -p <crate> ...` command
- **After every plan wave:** Run wave-touched crate suites + `timeout 300s cargo run -p vox-arch-check`
- **Before `/gsd-verify-work`:** All 8 crate test targets + `operations-verify` + `exec-policy-contract` + `secret-env-guard` + `vox-arch-check` green
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 442-01-01 | 01 | 1 | TERM-HARNESS-03 | T-442-01, T-442-02 | Pure-Rust POSIX + PowerShell quote-aware `ExecPolicyV1` + `SecurityTier` (`L1..L5`) blocks `$()`, backticks, `<()`, `L1` redirects, `pwsh -EncodedCommand`, `bash -c`, `find -exec`/`-delete`, `env`/`xargs` wrappers | unit | `timeout 300s cargo test -p vox-orchestrator --lib exec_policy` | ✅ | ✅ green |
| 442-01-02 | 01 | 1 | TERM-HARNESS-03 | T-442-01 | Multi-segment `classify_shell_command` in `floor.rs` catches history-rewriting git verbs after `;`/`&&`/`||`/`|` and `check_terminal.rs` delegates cleanly to `exec_policy` | unit + contract | `timeout 300s cargo test -p vox-cli-research --lib check_terminal` | ✅ | ✅ green |
| 442-02-01 | 02 | 1 | TERM-HARNESS-04 | T-442-03, T-442-04 | Upgraded `vox-redact` (`re_home` macOS/Linux/Windows, PEM keys, URL `user:pass@host`, `AIza`/`hf_`/`npm_`/`ghp_`/`sk-`) + Clavis `scrub_resolved_secrets` (`>= 6` bytes → `[REDACTED:CLAVIS]`, config-only skipped) | unit + contract | `timeout 300s cargo test -p vox-secrets --lib redact` | ✅ | ✅ green |
| 442-02-02 | 02 | 1 | TERM-HARNESS-04 | T-442-03, T-442-05 | `BlockStatus::AwaitingInput`, `Block.{pinned,credential_prompt}`, full ECMA-48 `strip_ansi_and_normalize`, and UTF-8-safe head-40/tail-160 (`16 KiB`) `truncate_terminal_block` | unit | `timeout 300s cargo test -p vox-terminal-core --lib` | ✅ | ✅ green |
| 442-03-01 | 03 | 2 | TERM-HARNESS-02, TERM-HARNESS-03 | T-442-06, T-442-07 | Non-subshell 5-shell `__vox_pager_enter` (`PAGER=cat`, `GIT_PAGER=cat`, `LESS=FRX`, `BAT_PAGER=cat`, `MANPAGER=cat`, `DELTA_PAGER=cat`), `ALT_SCREEN` + `1_500 ms` quiescence `stall_detector.rs`, and `agent_exec.rs` | unit | `timeout 300s cargo test -p vox-terminal-core --lib` | ✅ | ✅ green |
| 442-03-02 | 03 | 2 | TERM-HARNESS-01, TERM-HARNESS-02, TERM-HARNESS-03 | T-442-06, T-442-07, T-442-08 | Stateful `LocalHost::exec_agent_command` + fail-closed `credential_prompt` `agent_send_input`, `Ctrl+B` `promote_bg`, `pin_block`, `DaemonHost` parity, and `vox_orchestrator::terminal_bridge` | unit + integration | `timeout 300s cargo test -p vox-terminal-core` | ✅ | ✅ green |
| 442-03-03 | 03 | 2 | TERM-HARNESS-01, TERM-HARNESS-03 | T-442-08, T-442-09, T-442-13 | `ServerState` `terminal_bridge` slot, `LocalHostTerminalBridge` 3-stage scrubbing + `validate_daemon_agent_exec`, `TermDispatch` `term.{agent_exec,agent_send_input,read,promote_bg,pin}`, and shared `Arc<LocalHost>` in `vox-orchestrator-d` | unit + integration | `timeout 300s cargo test -p vox-orchestrator-d --lib` | ✅ | ✅ green |
| 442-04-01 | 04 | 3 | TERM-HARNESS-01, TERM-HARNESS-03, TERM-HARNESS-04 | T-442-10, T-442-11 | `input_schemas/terminal.rs` + `dispatch_domains/terminal_tools.rs` with sticky `chat_thread_id` binding, `TermControlMode` (`manual`/`co_pilot`/`autopilot`) × `SecurityTier` × `ExecPolicyV1` single-park HITL (`CR-05`), and Clavis scrubbing | unit + integration | `timeout 300s cargo test -p vox-orchestrator-mcp --lib terminal_tools` | ✅ | ✅ green |
| 442-04-02 | 04 | 3 | TERM-HARNESS-01, TERM-HARNESS-03 | T-442-10, T-442-12 | Sorted `catalog.v1.yaml` + `http-read-role-governance.yaml` + `operations-sync` + `operations-verify`, `floor.rs` `SHELL_COMMAND_TOOLS` (`vox_skill_run` preserved) + `permission-modes.v2.yaml` lockstep, `150s` timeout, and `tool_selection.rs` | integration + contract | `timeout 300s cargo run -p vox-cli -- ci operations-verify` | ✅ | ✅ green |
| 442-05-01 | 05 | 3 | TERM-HARNESS-03, TERM-HARNESS-04 | T-442-14, T-442-15 | `commands::pty` `term_session_{set_security,set_control,pin,promote_bg}` (`manager.host_for(daemon.inner()).await`) and `commands::console_a2a` backend-resolved 3-stage scrubbed `send_terminal_block_to_agent` (`FakeBackend`, `--bin vox-gui`) | unit + integration | `timeout 300s cargo test -p vox-gui --bin vox-gui commands::console_a2a` | ✅ | ✅ green |
| 442-05-02 | 05 | 3 | TERM-HARNESS-03, TERM-HARNESS-04 | T-442-15 | `main.rs` `axis_invoke_handler!` registrations (`remote::gate` classified) + `transport.ts` canonical `TermSecurityLevelWire`/`TermControlModeWire` & typed `safeInvoke` wrappers | unit + guard | `timeout 300s cargo test -p vox-gui --bin vox-gui remote::gate` | ✅ | ✅ green |
| 442-06-01 | 06 | 3 | TERM-HARNESS-04 | T-442-16, T-442-17 | `chat_tools/chat/term_mentions.rs` (`TERM_MENTION_RE` 7 locked forms, `@terminal.rs` non-interference, 3-stage Clavis/path/ANSI scrubbing, XML tag breakout escaping `WR-04`, `32 KiB` UTF-8 turn cap) | unit + integration | `timeout 300s cargo test -p vox-orchestrator-mcp --lib term_mentions` | ✅ | ✅ green |
| 442-06-02 | 06 | 3 | TERM-HARNESS-04 | T-442-18 | `mentions.rs` `term`/`term:*` guard and `message.rs` pre-`resolve_mentions_with_rules` (`L118`) extraction + post-`canonicalize_prompt` (`L176`) `<terminal_context>` attachment | unit + integration | `timeout 300s cargo test -p vox-orchestrator-mcp --lib mentions` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] Every task embeds TDD RED→GREEN test coverage inside existing workspace crates (`vox-redact`, `vox-secrets`, `vox-orchestrator`, `vox-cli-research`, `vox-terminal-core`, `vox-orchestrator-d`, `vox-orchestrator-mcp`, `vox-gui`) — no separate pre-wave test-scaffold plan required.

---

## Manual-Only Verifications

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 60s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-11-15
