---
phase: 442
phase_name: "bidirectional-chat-harness-and-ast-security"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 34
  lessons: 4
  patterns: 20
  surprises: 4
missing_artifacts:
  - "UAT.md"
---

# Phase 442 Learnings: bidirectional-chat-harness-and-ast-security

## Decisions

### Hosted ExecPolicyV1, tokenize_and_split_segments, extract_effective_comm...
Hosted ExecPolicyV1, tokenize_and_split_segments, extract_effective_commands, evaluate_command_ast, and evaluate_security_tier in vox_orchestrator::oversight::exec_policy so vox-terminal-core, vox-orchestrator-mcp, and vox-cli-research share one SSOT with zero new crate-edges

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 442-01-SUMMARY.md

---

### Evaluated L1-L5 SecurityTier directly from AST structure + read-only/wor...
Evaluated L1-L5 SecurityTier directly from AST structure + read-only/workspace allowlists rather than raw exec-policy.v1.yaml command-not-allowed lists so benign shell builtins (cd, pwd, echo) never false-park in L1/L2 while subshells, redirections, and encoded wrappers fail closed

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 442-01-SUMMARY.md

---

### Kept PowerShell System.Management.Automation.Language.Parser AST extract...
Kept PowerShell System.Management.Automation.Language.Parser AST extraction in vox-cli-research::check_terminal layered atop shared evaluate_command_ast for defense-in-depth when pwsh is installed

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 442-01-SUMMARY.md

---

### Anchored re_home() raw regex literal with (?
Anchored re_home() raw regex literal with `(?:` so vox-arch-check [no-hardcoded-abs-path] stays clean while matching /Users/<name>, /home/<name>, and C:\Users\<name> up to path/whitespace delimiters

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 442-02-SUMMARY.md

---

### Combined spec.id.metadata() filtering with is_user_facing_secret(spec.id...
Combined spec.id.metadata() filtering with is_user_facing_secret(spec.id), taxonomy_class_for(spec.id).is_config_only(), and _URL/_EMAIL/_USER_AGENT/_PATH suffix guards in scrub_resolved_secrets so operator config knobs (e.g. VOX_MODEL, GEMINI_TUNING_TEMPERATURE) never cause false-positive redactions

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 442-02-SUMMARY.md

---

### Named the credential stall flag Block.credential_prompt (never password_...
Named the credential stall flag Block.credential_prompt (never password_*/secret_*) so serde serialization passes vox ci secret-env-guard

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 442-02-SUMMARY.md

---

### Upgraded vox-redact (Stage 2)
**Upgraded `vox-redact` (`Stage 2`):** Added `RE_PEM` (`[REDACTED_PEM]`) and `RE_URL_CRED` (`${1}[REDACTED_CRED]@`, applied ahead of `re_email` so `user:pass@host` is never misclassified as an email), expanded `re_token` with `AIza`, `hf_`, `npm_`, `sk-ant-`, and `sk-proj-`/`sk-svcacct-` branches (`[REDACTED_KEY]`), and updated `re_home` to `r"(?:/(?:Users|home)/[^/\s]+|[A-Za-z]:\\Users\\[^\\\s]+)"` (`~[REDACTED_PATH]`) with zero `vox-arch-check` `[no-hardcoded-abs-path]` violations.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 442-02-SUMMARY.md

---

### Clavis Exact-Value Scrubber (Stage 3)
**Clavis Exact-Value Scrubber (`Stage 3`):** Implemented `scrub_secret_needles` and `scrub_resolved_secrets` in `crates/vox-secrets/src/redact.rs` (re-exported from `crates/vox-secrets/src/lib.rs`) using `AhoCorasick` `MatchKind::LeftmostLongest`, `>= 6` UTF-8 byte minimum threshold, `[REDACTED:CLAVIS]` replacement, and strict config/tuning/URL/username/delegation exclusion via Clavis `resolve_secret` (zero `std::env::var` reads; `.expose()` confined inside `vox-secrets`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 442-02-SUMMARY.md

---

### ECMA-48 Normalization, Block Model & Truncation (Stage 1 + Budgets)
**ECMA-48 Normalization, Block Model & Truncation (`Stage 1` + Budgets):** Added `BlockStatus::AwaitingInput` and `#[serde(default)] pub pinned: bool` + `#[serde(default)] pub credential_prompt: bool` on `Block`; exposed `strip_ansi_and_normalize` stripping CSI, OSC, DCS/SOS/PM/APC, charset nF sequences, 2-byte ESC, and stray C0/DEL control bytes after `normalize_cr_overwrites`; and added `scrub_ansi_and_redact`, `scrub_terminal_text`, `truncate_terminal_block` (head-40 + tail-160 lines, `16_384` UTF-8 byte cap with `is_char_boundary` slicing and `vox_terminal_read` pagination hint), `truncate_head_tail`, and `slice_lines_1based`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 442-02-SUMMARY.md

---

### Used (?: prefix on re_home() (r"(?:/(?:Users|home)/[^/\s]+|[A-Za-z]:\\Us...
Used `(?:` prefix on `re_home()` (`r"(?:/(?:Users|home)/[^/\s]+|[A-Za-z]:\\Users\\[^\\\s]+)"`) and `// vox-arch-check: allow abs-path` on test fixtures so `vox-arch-check` passes with zero `[no-hardcoded-abs-path]` violations.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 442-02-SUMMARY.md

---

### When truncate_terminal_block hits the 16_384-byte cap on multi-line or s...
When `truncate_terminal_block` hits the `16_384`-byte cap on multi-line or single-line inputs, head/tail byte slices are computed directly from `text` on UTF-8 character boundaries (`is_char_boundary`) around a single pagination marker so markers are never duplicated or mid-codepoint-split.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 442-02-SUMMARY.md

---

### Kept OpenSpec { cols, rows, shell } unchanged for downstream vox-gui/vox...
Kept OpenSpec { cols, rows, shell } unchanged for downstream vox-gui/vox-term compatibility and added LocalHost::open_with_cwd(&self, spec: OpenSpec, cwd: Option<&Path>) while accepting optional params.cwd on term.open

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 442-03-SUMMARY.md

---

### Wrapped ServerState::terminal_bridge, chat_terminal_sessions, and epheme...
Wrapped ServerState::terminal_bridge, chat_terminal_sessions, and ephemeral_terminal_bridge in Arc<parking_lot::RwLock<...>> across all 3 constructors so set_terminal_bridge after DaemonBootCtx propagates immediately to every pre-existing ServerState clone

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 442-03-SUMMARY.md

---

### Kept all vox_secrets::scrub_resolved_secrets Stage-3 calls in crates tha...
Kept all vox_secrets::scrub_resolved_secrets Stage-3 calls in crates that already depend on vox-secrets (vox-orchestrator, vox-orchestrator-d) so zero Cargo.toml or crate-edges changes were required

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 442-03-SUMMARY.md

---

### Retained shell_name on PtyHandle (crates/vox-terminal-core/src/pty.rs).
Retained `shell_name` on `PtyHandle` (`crates/vox-terminal-core/src/pty.rs`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 442-03-SUMMARY.md

---

### Added non-subshell __vox_pager_enter and __vox_pager_restore across all ...
Added non-subshell `__vox_pager_enter` and `__vox_pager_restore` across all 5 shell shims (`zsh`, `bash`, `fish`, `pwsh`, `nu` in `crates/vox-terminal-core/src/shell_init.rs`) covering `PAGER`, `GIT_PAGER`, `SYSTEMD_PAGER`, `BAT_PAGER`, `MANPAGER`, and `GH_PAGER`, restoring user pager settings immediately after `$?`/`$status`/`$LASTEXITCODE`/`$env.LAST_EXIT_CODE` capture and before `OSC 633;D` emission so `cd`/`export` persist in the current shell while human interactive commands retain their normal pagers.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 442-03-SUMMARY.md

---

### Created crates/vox-terminal-core/src/stall_detector.rs (DEFAULT_STALL_QU...
Created `crates/vox-terminal-core/src/stall_detector.rs` (`DEFAULT_STALL_QUIESCENCE_MS = 1_500`, `PromptKind`, `StallSignal`, `classify_stall`) detecting `ALT_SCREEN` immediately and classifying credential (`Password:`, `passphrase`, `[sudo]`, `Enter PIN`, `verification code`, `2FA`/`OTP`), yes/no, press-enter, selection, and interactive prompts only on unterminated non-newline tails after `>= quiescence_ms` silence (never false-firing on newline-terminated `rustc` `error[E0308]:` diagnostics).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 442-03-SUMMARY.md

---

### Created crates/vox-terminal-core/src/agent_exec.rs (AgentExecRequest, Ag...
Created `crates/vox-terminal-core/src/agent_exec.rs` (`AgentExecRequest`, `AgentExecStatus`, `AgentExecOutcome`, `allows_agent_command` delegating to `vox_orchestrator::oversight::exec_policy`, `format_agent_command_for_shell`) and re-exported both modules in `crates/vox-terminal-core/src/lib.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 442-03-SUMMARY.md

---

### Replaced ~230 lines of inline heuristic parsing in crates/vox-terminal-c...
Replaced ~230 lines of inline heuristic parsing in `crates/vox-terminal-core/src/host.rs` by delegating `TermSecurityLevel::allows_agent_command` to `crate::agent_exec::allows_agent_command`, shrinking `host.rs` and upgrading `redacted_context_bundle` to `scrub_ansi_and_redact` + `truncate_terminal_block`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 442-03-SUMMARY.md

---

### Added promote_bg_on_host(host, session_id, block_id
Added promote_bg_on_host(host, session_id, block_id: Option<u64>) -> Result<bool, String> in commands::pty so term_session_promote_bg supports both Ctrl+B session-level promotion (resolving the active Running/AwaitingInput block via zero-replay attach(Some(u64::MAX)) and returning Ok(false) when idle) and explicit block-level promotion against SessionHost::promote_bg(&self, id: &str, block_id: BlockId) -> Result<()>

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 442-05-SUMMARY.md

---

### Validated session_id and parse_receiver(&agent_id) in send_terminal_bloc...
Validated session_id and parse_receiver(&agent_id) in send_terminal_block_to_agent_impl prior to calling vox_terminal_read so empty sessions and reserved/invalid agent IDs fail fast with zero daemon/gateway round-trips

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 442-05-SUMMARY.md

---

### Added parse_security_level ("l1_read_only".."l5_autopilot_floor" plus nu...
Added `parse_security_level` (`"l1_read_only"`..`"l5_autopilot_floor"` plus numeric/short/legacy serde aliases `"1"`..`"5"`, `"L1"`..`"L5"`, `"read_only"`, `"workspace_build"`, `"safe_exec"`, `"structured_default"`, `"workspace_write"`, `"host_gated"`, `"system_exec"`, `"autopilot_floor"`), `parse_control_mode` (`"manual"`, `"co_pilot"` / `"copilot"`, `"autopilot"`), and `promote_bg_on_host` in `crates/vox-gui/src/commands/pty.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 442-05-SUMMARY.md

---

### Implemented term_session_set_security, term_session_set_control, term_se...
Implemented `term_session_set_security`, `term_session_set_control`, `term_session_pin`, and `term_session_promote_bg` in `commands::pty` (`ModuleClass::Host`) resolving the host via `manager.host_for(daemon.inner()).await`, plus unit tests covering alias parsing, rejection of empty/invalid inputs, and `LocalHost` round-trips.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 442-05-SUMMARY.md

---

### Added scrub_for_agent_inbox (scrub_ansi_and_redact + vox_secrets::scrub_...
Added `scrub_for_agent_inbox` (`scrub_ansi_and_redact` + `vox_secrets::scrub_resolved_secrets`), `send_terminal_block_to_agent_impl`, and `#[tauri::command] pub async fn send_terminal_block_to_agent` in `crates/vox-gui/src/commands/console_a2a.rs` (`ModuleClass::Daemon`), resolving `{ session_id, block_id, line_start, line_end }` via `"vox_terminal_read"`, applying 3-stage scrubbing and head-40/tail-160 (`16 KiB`) truncation to `input`, `output`, and optional `note` (as well as legacy free-form `send_to_agent` bodies), and dispatching `"vox_a2a_send"` with `CONSOLE_OPERATOR_AGENT_ID` (`0`), `parse_receiver`, and `"route": "db"`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 442-05-SUMMARY.md

---

### Registered commands::pty::term_session_set_security, commands::pty::term...
Registered `commands::pty::term_session_set_security`, `commands::pty::term_session_set_control`, `commands::pty::term_session_pin`, `commands::pty::term_session_promote_bg`, and `commands::console_a2a::send_terminal_block_to_agent` in `axis_invoke_handler!` in `crates/vox-gui/src/main.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 442-05-SUMMARY.md

---

### Updated crates/vox-gui/ui/src/transport.ts strictly outside the // __VOX...
Updated `crates/vox-gui/ui/src/transport.ts` strictly outside the `// __VOX_RAW_IPC_BEGIN__`..`// __VOX_RAW_IPC_END__` guard region: widened `TermBlockStatus` with `'AwaitingInput'`, exported canonical `TermSecurityLevelWire` and `TermControlModeWire`, extended `TermBlockDto` (`pinned?`, `credential_prompt?`) and `TermSessionInfoDto` (`security_level?`, `control_mode?`, `alt_screen?`, `cwd?`, `git_branch?`), and added `termSessionSetSecurity`, `termSessionSetControl`, `termSessionPin`, `termSessionPromoteBg`, `SendTerminalBlockToAgentArgs`, and `sendTerminalBlockToAgent` calling `safeInvoke`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 442-05-SUMMARY.md

---

### Combined word-boundary @term\b capture in TERM_MENTION_RE with explicit ...
Combined word-boundary @term\b capture in TERM_MENTION_RE with explicit trailing delimiter validation so workspace file mentions (@terminal.rs, @term_helper.vox, @term.rs, @term/mod.rs) and malformed suffixes (@term:sess_1#42:L0-5) never partially match

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 442-06-SUMMARY.md

---

### Replaced matched @term spans by exact byte offsets in a single left-to-r...
Replaced matched @term spans by exact byte offsets in a single left-to-right pass rather than String::replace so overlapping prefixes (@term vs @term:last) never corrupt neighboring citations

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 442-06-SUMMARY.md

---

### Checked bridge.list_sessions() emptiness before calling ServerState::bin...
Checked bridge.list_sessions() emptiness before calling ServerState::bind_or_resolve_terminal_session for implicit @term/@term:last/@term:failed/@term:pinned selectors so typing @term with zero open terminals never spawns an idle PTY shell

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 442-06-SUMMARY.md

---

### Created crates/vox-orchestrator-mcp/src/chat_tools/chat/term_mentions.rs...
Created `crates/vox-orchestrator-mcp/src/chat_tools/chat/term_mentions.rs` (`~370` non-blank non-test LoC, `8` fns, `0` new Cargo edges) and exported `pub mod term_mentions;` from `crates/vox-orchestrator-mcp/src/chat_tools/chat/mod.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 442-06-SUMMARY.md

---

### Implemented TERM_MENTION_RE and parse_term_mentions supporting @term, @t...
Implemented `TERM_MENTION_RE` and `parse_term_mentions` supporting `@term`, `@term:last`, `@term:failed`, `@term:pinned`, `@term:<sid>`, `@term:<sid>#<id>`, and 1-indexed inclusive `@term:<sid>#<id>:L<a-b>` (rejecting `a == 0` or `b < a`), replacing matched byte spans with `[ref @term...]` in left-to-right appearance order while deduplicating repeated selectors and leaving `@terminal.rs`, `@term_helper.vox`, `@term.rs`, `user@term.io`, and `@rule:tdd` untouched.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 442-06-SUMMARY.md

---

### Implemented scrub_term_citation_text performing full ECMA-48 CSI/OSC/DCS...
Implemented `scrub_term_citation_text` performing full ECMA-48 CSI/OSC/DCS/SOS/PM/APC/nF/2-byte ESC stripping + `\r` and `CSI K` progress-bar overwrite normalization (`strip_ecma48_and_normalize_cr`), followed by `vox_redact::redact_owned` and `vox_secrets::scrub_resolved_secrets` with zero dependency on `vox-terminal-core`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 442-06-SUMMARY.md

---

### Implemented expand_term_mentions resolving citations over ServerState::r...
Implemented `expand_term_mentions` resolving citations over `ServerState::resolve_terminal_bridge()`, checking `bridge.list_sessions()` before `bind_or_resolve_terminal_session` so empty sessions emit `<terminal_block status="empty" selector="..." />` without spawning idle PTY shells, and capping `<terminal_context>...</terminal_context>` at `MAX_TERM_PROMPT_ENVELOPE_BYTES = 32_768` bytes via `str::floor_char_boundary` with an intact closing `</terminal_context>` tag.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 442-06-SUMMARY.md

---

### Added if filename == "term" || filename.starts_with("term:") { continue;...
Added `if filename == "term" || filename.starts_with("term:") { continue; }` inside `resolve_mentions_with_rules` (`crates/vox-orchestrator-mcp/src/chat_tools/chat/mentions.rs`) so direct callers and `[ref @term:...]` markers never misclassify terminal citations as workspace files.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 442-06-SUMMARY.md

## Lessons

### Sibling Wave 1 (442-01) compile fix needed before Wave 2
**Sibling Wave 1 (`442-01`) compile fix needed before Wave 2:** `crates/vox-orchestrator/src/oversight/exec_policy.rs:448` and `:465` need `in_w!(*a, ...)` (dereferencing `a: &&str`); once applied, re-run `timeout 300s cargo test -p vox-terminal-core` and `timeout 300s cargo run -p vox-cli -- ci secret-env-guard` for full workspace linkage confirmation.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 442-02-SUMMARY.md

---

### Exhaustive BlockStatus match in vox-term (outside 442-02 files_modified)
**Exhaustive `BlockStatus` match in `vox-term` (outside `442-02` `files_modified`):** `rg "BlockStatus" crates/` identified one exhaustive match without a `_ =>` wildcard at `crates/vox-term/src/ui/blocks.rs:19-24` (`match block.status`). Per parallel wave file-scope rules it was not edited here; add `BlockStatus::AwaitingInput => Color::Cyan,` (or `Color::Yellow`) in `crates/vox-term/src/ui/blocks.rs` during orchestrator Wave 1 integration.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 442-02-SUMMARY.md

---

### Adapted to Verified Wave 2 (442-03) Async Signatures
**Adapted to Verified Wave 2 (`442-03`) Async Signatures:** `ServerState::resolve_terminal_bridge(&self)` and `ServerState::bind_or_resolve_terminal_session(&self, chat_thread_id, explicit_session_id)` in `server_state.rs` are `async fn`s returning `Arc<dyn TerminalBridge>` and `anyhow::Result<(Arc<dyn TerminalBridge>, String)>`, and `TerminalBridge::send_input` returns `anyhow::Result<()>` (followed by `bridge.read(...)` for the updated block snapshot). Wired `terminal_tools.rs` directly against those exact signatures.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 442-04-SUMMARY.md

---

### No gaps found
No gaps found. All 4 ROADMAP Success Criteria, 25 plan must-have truths, 17 prohibitions, 18 code-review remediations (`442-REVIEW-FIX.md`), 20 required artifacts, 13 key links, and requirements `TERM-HARNESS-01` through `TERM-HARNESS-04` are implemented, wired end-to-end, and verified.

**Context:** Verified in phase acceptance checks.
**Source:** 442-VERIFICATION.md

## Patterns

### All terminal and shell tool command strings pass through multi-segment w...
All terminal and shell tool command strings pass through multi-segment wrapper-unwrapping AST inspection before tier classification or PTY execution

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 442-01-SUMMARY.md

---

### Pure-Rust quote-aware POSIX + PowerShell lexical segmenter handling |, |...
Pure-Rust quote-aware POSIX + PowerShell lexical segmenter handling |, ||, &&, ;, newlines, backticks, $(), ${}, > / >> / 2> redirections, and fail-closed unclosed quotes

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 442-01-SUMMARY.md

---

### Recursive wrapper binary unwrapping (sudo, env, nohup, timeout, nice, xa...
Recursive wrapper binary unwrapping (sudo, env, nohup, timeout, nice, xargs, watch, sh/bash/zsh -c, pwsh/powershell -Command/-EncodedCommand, cmd /c, find -exec/-execdir/-delete)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 442-01-SUMMARY.md

---

### Effective oversight floor = max(ActionFloor across all extracted segment...
Effective oversight floor = max(ActionFloor across all extracted segments and unwrapped inner commands)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 442-01-SUMMARY.md

---

### Strip ECMA-48 CSI/OSC/DCS/SOS/PM/APC/nF/Fp sequences and normalize CR ov...
Strip ECMA-48 CSI/OSC/DCS/SOS/PM/APC/nF/Fp sequences and normalize CR overwrites strictly before running regex or Aho-Corasick secret redaction

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 442-02-SUMMARY.md

---

### Confine ResolvedSecret::expose() inside crates/vox-secrets/src/redact.rs...
Confine ResolvedSecret::expose() inside crates/vox-secrets/src/redact.rs and expose only scrub_resolved_secrets(text: &str) -> String to downstream crates

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 442-02-SUMMARY.md

---

### 3-stage terminal output scrubbing order
3-stage terminal output scrubbing order: (1) normalize_cr_overwrites + full ECMA-48 strip_ansi_and_normalize -> (2) vox_redact::redact_owned -> (3) vox_secrets::scrub_resolved_secrets

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 442-02-SUMMARY.md

---

### UTF-8 boundary-safe head-40 + tail-160 line / 16 KiB byte truncation wit...
UTF-8 boundary-safe head-40 + tail-160 line / 16 KiB byte truncation with vox_terminal_read pagination hints

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 442-02-SUMMARY.md

---

### Agent PTY commands execute via __vox_pager_enter; <cmd> in the live shel...
Agent PTY commands execute via __vox_pager_enter; <cmd> in the live shell rather than a subshell so cd and export persist while git/man/bat/gh/systemd pagers never hang

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 442-03-SUMMARY.md

---

### User terminal submissions and raw PTY keystrokes automatically demote Te...
User terminal submissions and raw PTY keystrokes automatically demote TermControlMode::Autopilot to TermControlMode::Manual and emit SessionEvent::ControlModeChanged

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 442-03-SUMMARY.md

---

### Non-subshell current-shell pager guard (__vox_pager_enter saves/exports ...
Non-subshell current-shell pager guard (__vox_pager_enter saves/exports 6 pager vars to cat; precmd/prompt __vox_pager_restore restores exact prior state immediately after exit-code capture)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 442-03-SUMMARY.md

---

### ALT_SCREEN + 1_500 ms quiescence stall classifier pausing blocks in Bloc...
ALT_SCREEN + 1_500 ms quiescence stall classifier pausing blocks in BlockStatus::AwaitingInput and failing closed on credential_prompt == true (CREDENTIAL_PROMPT_USER_ONLY)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 442-03-SUMMARY.md

---

### Single shared Arc<LocalHost> across TermDispatch and ServerState::termin...
Single shared Arc<LocalHost> across TermDispatch and ServerState::terminal_bridge inside vox-orchestrator-d on loopback/stdio transports with 3-stage output scrubbing

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 442-03-SUMMARY.md

---

### All terminal-to-agent A2A notes (both structured send_terminal_block_to_...
All terminal-to-agent A2A notes (both structured send_terminal_block_to_agent citations and free-form send_to_agent bodies) pass through scrub_for_agent_inbox (scrub_ansi_and_redact + vox_secrets::scrub_resolved_secrets) and truncate_terminal_block (head-40/tail-160, 16 KiB) before calling vox_a2a_send with route: "db"

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 442-05-SUMMARY.md

---

### All new frontend terminal IPC helpers in transport.ts route exclusively ...
All new frontend terminal IPC helpers in transport.ts route exclusively through safeInvoke outside the marked raw-IPC region

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 442-05-SUMMARY.md

---

### Backend-resolved SendToAgent citations
Backend-resolved SendToAgent citations: webview sends { agentId, sessionId, blockId, lineStart, lineEnd, note } and Rust resolves output via vox_terminal_read before running 3-stage ANSI + vox-redact + Clavis scrubbing and 16 KiB head-40/tail-160 truncation

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 442-05-SUMMARY.md

---

### ModuleClass boundary preservation
ModuleClass boundary preservation: PTY session controls stay in commands::pty (ModuleClass::Host) while durable A2A terminal citations live in commands::console_a2a (ModuleClass::Daemon)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 442-05-SUMMARY.md

---

### External runtime context blocks (<terminal_context>) bypass prompt_canon...
External runtime context blocks (<terminal_context>) bypass prompt_canonical::canonicalize_prompt and raw_prompt_for_research while still contributing to chat_grounding_score via mention_count

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 442-06-SUMMARY.md

---

### Pre-MENTION_RE @term span replacement ([ref @term:...]) + post-canonical...
Pre-MENTION_RE @term span replacement ([ref @term:...]) + post-canonicalize_prompt <terminal_context> attachment preserving raw_prompt_for_research and stack-trace fidelity

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 442-06-SUMMARY.md

---

### Non-spawning implicit session resolution
Non-spawning implicit session resolution: consult bridge.list_sessions() before bind_or_resolve_terminal_session so @term on idle workspaces emits <terminal_block status="empty" /> instead of spawning a PTY

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 442-06-SUMMARY.md

## Surprises

### Estimate variance on 442-01
Estimate variance on 442-01: planned 40000 tokens vs 8420 actual (0.21x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 442-01-PLAN.md, 442-01-SUMMARY.md

---

### Estimate variance on 442-02
Estimate variance on 442-02: planned 38000 tokens vs 7884 actual (0.21x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 442-02-PLAN.md, 442-02-SUMMARY.md

---

### Estimate variance on 442-05
Estimate variance on 442-05: planned 36000 tokens vs 5278 actual (0.15x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 442-05-PLAN.md, 442-05-SUMMARY.md

---

### Estimate variance on 442-06
Estimate variance on 442-06: planned 34000 tokens vs 5844 actual (0.17x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 442-06-PLAN.md, 442-06-SUMMARY.md

