---
phase: 441-stateful-pty-and-lossless-streaming
padded_phase: "441"
fixed_at: 2026-11-14T14:32:30Z
review_path: .planning/workstreams/axis-warp-terminal-v75/phases/441-stateful-pty-and-lossless-streaming/441-REVIEW.md
iteration: 1
fix_scope: critical_warning
findings_in_scope: 12
fixed: 12
skipped: 0
commit_hash: "skipped (shared git index)"
status: all_fixed
---

# Phase 441: Code Review Fix Report

**Fixed at:** 2026-11-14T14:32:30Z
**Source review:** `.planning/workstreams/axis-warp-terminal-v75/phases/441-stateful-pty-and-lossless-streaming/441-REVIEW.md`
**Iteration:** 1
**Scope:** `critical_warning` (`CR-01`..`CR-03`, `WR-01`..`WR-09`, plus adjacent `IN-02` cleanly resolved inside `CR-01`)

**Summary:**
- Findings in scope: 12
- Fixed: 12
- Skipped: 0

## Fixed Issues

### CR-01: `TermSecurityLevel::allows_agent_command` and active-block stdin forwarding permit trivial policy bypasses and cross-origin stdin injection

**Files modified:** `crates/vox-terminal-core/src/host.rs`
**Commit:** skipped (shared git index)
**Applied fix:**
- Rejected multi-line (`\n` / `\r`) and empty commands at all `TermSecurityLevel` tiers (`L1`..`L5`) in `TermSecurityLevel::allows_agent_command`.
- Hardened `L1ReadOnly` to reject shell chaining, subshell, expansion, and redirection metacharacters (`;`, `&`, `|`, `` ` ``, `$`, `>`, `<`, `(`, `)`), reject path-qualified executables (e.g. `./ls`), reject `env <cmd>` execution, reject mutating/exec `find` flags (`-exec`, `-execdir`, `-ok`, `-okdir`, `-delete`, `-fprint`, `-fprintf`, `-fls`), and reject mutating `git branch` flags (`-d`, `-m`, `-c`, `--delete`, `--move`, `--copy`).
- Added `extract_segment_basenames` and `normalize_cmd_basename` in `L2WorkspaceBuild` and `L3StructuredDefault` to split shell segments on `;`, `|`, `&`, `` ` ``, `(`, `)`, skip `KEY=VAL` environment assignments, unwrap command wrappers (`env`, `command`, `builtin`, `exec`, `nohup`, `time`, `nice`) and their flags, normalize path basenames (`rsplit(['/', '\\']).next()`, `.exe` suffix stripping), and reject destructive/system/privilege commands across every chained segment (`rm`, `rmdir`, `del`, `sudo`, `su`, `chmod`, `chown`, `mkfs`, `dd`, `shutdown`, `reboot`, etc.).
- Updated `LocalHost::submit_with_origin` to refuse `BlockOrigin::Agent` submissions when `active_running_id.is_some()` (blocking cross-origin stdin injection into running interactive user programs) and classified input before running-block forwarding (`IN-02`) so `/slash` (`InputIntent::Command`) and `?ask` (`InputIntent::Agent`) dispatch directly instead of piping into running PTY stdin.
- Added unit tests `allows_agent_command_blocks_multiline_chaining_path_and_wrapper_bypasses` and `agent_cannot_inject_stdin_into_running_block_and_slash_commands_bypass_stdin`.

### CR-02: Nushell launch shim contains three syntax/runtime defects that break `pre_execution`, `OSC 7`, and `PromptEnd` (`633;B`), plus conflicts with Nushell's built-in `osc633`

**Files modified:** `crates/vox-terminal-core/src/shell_init.rs`, `crates/vox-terminal-core/src/pty.rs`
**Commit:** skipped (shared git index)
**Applied fix:**
- Explicitly disabled Nushell's built-in `shell_integration.osc633 = false` and `shell_integration.osc7 = false` in both `NU_SHIM_CONFIG` (`shell_init.rs`) and `NU_OSC633` (`pty.rs`) so duplicate conflicting `633` markers cannot interleave with Vox's hooks.
- Replaced the invalid 1-parameter `{|cmd| ... }` `pre_execution` closure with a 0-parameter closure `{|| let raw_cmd = (try { commandline } catch { "" }); ... }` matching Nushell 0.89+'s hook contract.
- Added the missing `$` string interpolation sigil on `$"/($uri_cwd | str trim --left --char '/' | url encode)"` so `OSC 7` emits the encoded working directory path rather than literal interpolation syntax.
- Moved `\x1b]633;B\x07` after `"> "` in `$env.PROMPT_INDICATOR` so prompt indicator text renders inside the prompt region (`633;A`..`633;B`) instead of after `PromptEnd`.
- Updated unit tests in `shell_init.rs` verifying `shell_integration.osc633 = false`, `{||`, `$"/($uri_cwd`, and `> (\u{1b}]633;B\u{07})`.

### CR-03: PowerShell launch shim clobbers `$LASTEXITCODE` inside `prompt` when outside a git repo and fails to load/bind `PSReadLine` for `LF`-submitted commands

**Files modified:** `crates/vox-terminal-core/src/shell_init.rs`, `crates/vox-terminal-core/src/pty.rs`
**Commit:** skipped (shared git index)
**Applied fix:**
- Captured `$origOk = $?` and `$origLast = $global:LASTEXITCODE` at the very top of `global:prompt` and moved `$global:LASTEXITCODE = 0` to the end of `global:prompt` (after `git symbolic-ref` and `& $global:__VoxOrigPrompt`) so failed git probes outside git repositories do not poison subsequent prompts with `$LASTEXITCODE = 128`.
- Added `Import-Module PSReadLine -ErrorAction SilentlyContinue` prior to `if (Get-Module -Name PSReadLine)` so `-NoProfile` sessions load `PSReadLine` deterministically when installed.
- Bound `@('Enter', 'Ctrl+j')` via `Set-PSReadLineKeyHandler -Chord` so `\n` (`LF`) submissions from `LocalHost::submit_with_origin` trigger `PSReadLine` `AcceptLine` and emit `633;E` + `633;C`.
- Updated unit tests in `shell_init.rs` asserting ordering and chord bindings.

### WR-01: `Session::apply_osc` leaks alt-screen transition chunks (and single-chunk TUI sessions) into `Block::plain_output()`

**Files modified:** `crates/vox-terminal-core/src/vt_shadow.rs`, `crates/vox-terminal-core/src/session.rs`, `crates/vox-terminal-core/tests/streaming_utf8_and_alt_screen.rs`
**Commit:** skipped (shared git index)
**Applied fix:**
- Added `VtShadow::feed_extracting_primary` which segments incoming output slices at `ESC` (`0x1b`) and CSI/ESC mode-transition terminators (`h`, `l`, `c`), advancing the shadow VT per segment and accumulating only bytes where `!was_alt && !self.alt_screen`.
- Updated `Session::apply_osc(Osc633Event::Output)` to append only `primary_text` to `Block.output` and `SessionEvent::OutputAppended`, preventing TUI entry frames (`?1049h...`), single-chunk TUI sessions (`?1049h...?1049l`), and exit clear sequences from polluting `Block::plain_output()`.
- Added integration test `alt_screen_entry_exit_and_single_chunk_frames_never_leak_into_plain_output` in `crates/vox-terminal-core/tests/streaming_utf8_and_alt_screen.rs`.

### WR-02: `vox-term` `apply_session_event` duplicates already-streamed block output and leaks `DaemonHost` subscription tasks/sockets

**Files modified:** `crates/vox-terminal-core/src/session.rs`, `crates/vox-terminal-core/src/daemon_host.rs`, `crates/vox-orchestrator-d/src/term_dispatch.rs`, `crates/vox-term/src/app.rs`, `crates/vox-term/tests/headless.rs`
**Commit:** skipped (shared git index)
**Applied fix:**
- Enriched externally-tagged `SessionEvent::BlockOpened` (`#[serde(default)]` `kind`, `input`, `origin`, `cwd`, `git_branch`) and `SessionEvent::BlockClosed` (`#[serde(default)]` `exit_code`, `status`), included those fields plus `"block"` on `"block_opened"` / `"block_closed"` wire frames in `term_dispatch.rs::session_event_json`, and parsed them in `daemon_host.rs::route`.
- Updated `vox_term::app::apply_session_event` to construct and close `Block` entries directly in-place without calling `host.attach(...)`, eliminating duplicate `OutputAppended` chunks and per-block RPC storms.
- Updated `DaemonHost::pump_with_resubscribe` to race `rx.recv()` (and `new_rx.recv()`) against `tokio::join!(events.closed(), output.closed())` inside `tokio::select!`, ensuring dropped `Attachment`s cancel their underlying daemon subscription stream immediately even on idle sessions.
- Updated `headless.rs` and `daemon_host.rs` tests to verify zero output duplication and immediate idle-stream teardown.

### WR-03: Lag recovery (`after_seq`) is incomplete across `DaemonHost`, `vox-term`, and `vox-gui`

**Files modified:** `crates/vox-terminal-core/src/session.rs`, `crates/vox-terminal-core/src/daemon_host.rs`, `crates/vox-orchestrator-d/src/term_dispatch.rs`, `crates/vox-term/src/app.rs`, `crates/vox-gui/src/commands/pty.rs`, `crates/vox-term/tests/headless.rs`
**Commit:** skipped (shared git index)
**Applied fix:**
- Added externally-tagged `SessionEvent::SnapshotResync { blocks, resync_required, alt_screen, cwd, git_branch }` emitted by `DaemonHost::pump_with_resubscribe` after recovering from `"lagged"` via `term.session.attach`; raised `MAX_LAG_RESUBSCRIBE_ATTEMPTS` to `64`, reset `consecutive_lags = 0` on `seq` or normal frame progress, and added a `10ms` backoff when `snap_seq <= last_seen_seq` to prevent tight spin loops.
- Handled `SnapshotResync` in `term_dispatch.rs`, `vox-term/src/app.rs` (draining `att.events` prior to `att.output` and calling `grid.reset()` when `resync_required` is `true` before feeding replay bytes), and `vox-gui/src/commands/pty.rs`.
- Updated `vox-gui/src/commands/pty.rs::term_session_attach` to catch `RecvError::Lagged` on BOTH `att.output` and `att.events`, re-attach via `host.attach(&sid, Some(last_seq))`, reset `Utf8StreamDecoder` when `reattached.resync_required` is `true`, and emit `SessionEvent::SnapshotResync` on `TERM_EVENT_EVENT`.

### WR-04: `vox-gui` `term_session_attach` corrupts multi-byte UTF-8 sequences split across the replay-to-live seam

**Files modified:** `crates/vox-gui/src/commands/pty.rs`
**Commit:** skipped (shared git index)
**Applied fix:**
- Extracted `term_attachment_dto_and_decoder` in `crates/vox-gui/src/commands/pty.rs`, decoding `&att.replay` via `decoder.push(&att.replay)` WITHOUT calling `flush_lossy()` so any trailing incomplete 2..4-byte UTF-8 prefix at `att.seq` stays buffered inside `decoder`.
- Moved that same seeded `decoder` into `term_session_attach`'s live forwarder task so the continuation bytes arriving in frame `att.seq + 1` complete the codepoint cleanly with zero `U+FFFD` replacement characters.
- Added unit test `attach_replay_to_live_seam_carries_split_utf8_without_replacement_chars`.

### WR-05: `PtyHandle` and `"vox-pty-reader"` leak zombie child processes and orphaned reader threads when closed or dropped

**Files modified:** `crates/vox-terminal-core/src/pty.rs`, `crates/vox-terminal-core/src/host.rs`
**Commit:** skipped (shared git index)
**Applied fix:**
- Removed `reader_master` (`Arc::clone(&master)`) from the `"vox-pty-reader"` thread so `PtyHandle` is the sole owner of the master PTY descriptor.
- Wrapped `PtyHandle.writer` and `PtyHandle.master` in `Option<...>`, implemented `Drop for PtyHandle` delegating to `self.kill()`, and updated `PtyHandle::kill` / `PtyHandle::signal(Terminate | Kill)` to drop `writer` and `master` FIRST (sending immediate kernel EOF to the slave PTY), invoke `self.child.kill()`, and reap child exit status via bounded `reap_bounded` (`try_wait`).
- Implemented `Drop for LocalHost` to close/kill/reap all open PTY handles on host drop and updated `LocalHost::open`'s `on_eof` callback to reap self-exited child processes even when no active block is open.

### WR-06: `Osc633Parser` swallows subsequent `OSC 633` markers up to 16 KiB when an OSC sequence is interrupted mid-flight (e.g. `SIGINT`)

**Files modified:** `crates/vox-terminal-core/src/osc633.rs`
**Commit:** skipped (shared git index)
**Applied fix:**
- Replaced `extract_osc_payload`'s `Option` return with `OscScan::{Complete, Incomplete, Aborted { resume_at }}`:
  - Returns `OscScan::Incomplete` when `0x1b` is the final buffered byte so chunk-split `\x1b\\` (`ST`) terminators still reassemble seamlessly across `feed()` boundaries.
  - Returns `OscScan::Aborted` on `CAN` (`0x18`), `SUB` (`0x1a`), or bare `0x1b` followed by `Some(b)` where `b != b'\\'` (emitting the aborted prefix as `Osc633Event::Output` and resuming escape scanning directly at `0x1b`).
- Added unit test `interrupted_osc_and_split_st_terminator_preserve_following_markers` verifying both `SIGINT`-aborted OSC payloads followed by `\x1b]633;D;130\x07` and chunk-split `\x1b\\` terminators.

### WR-07: `vox-term` `VtGrid` misses legacy `?47` / `?1047` alt-screen transitions and renders duplicate spaces for wide CJK/emoji glyphs

**Files modified:** `crates/vox-terminal-core/src/vt_shadow.rs`, `crates/vox-term/src/vt.rs`, `crates/vox-term/tests/vt.rs`
**Commit:** skipped (shared git index)
**Applied fix:**
- Exported `LegacyAltSync` and `advance_term_with_legacy_alt` from `vox_terminal_core::vt_shadow` (segmenting at `0x1b` / `h` / `l` so legacy `?47h`/`?47l` and `?1047h`/`?1047l` screen transitions take effect before subsequent draw bytes in the same chunk) and wired `VtGrid::feed` through it.
- Skipped `Flags::WIDE_CHAR_SPACER` trailing cells in `VtGrid::render_lines` so 2-column CJK and emoji glyphs render at their true character count without injected spaces.
- Added integration test `legacy_alt_screen_modes_47_1047_and_wide_char_spacer_width` in `crates/vox-term/tests/vt.rs`.

### WR-08: `vox-term` processes `KeyEventKind::Release` events, duplicating every keystroke and `Enter` submission on Windows (and Kitty-protocol terminals)

**Files modified:** `crates/vox-term/src/app.rs`, `crates/vox-term/tests/headless.rs`
**Commit:** skipped (shared git index)
**Applied fix:**
- Filtered out `Event::Key(key) if key.kind == KeyEventKind::Release` in `vox_term::app::run_with_args` and guarded `pub fn key_event_to_vt_bytes` to return `None` when `key.kind == KeyEventKind::Release`.
- Added integration test `key_event_to_vt_bytes_ignores_release_events` in `crates/vox-term/tests/headless.rs`.

### WR-09: Fish `preexec` breaks `633;E` on multi-line commands, and `633;P;Cwd=` leaves `;` / `\` unescaped across shell shims

**Files modified:** `crates/vox-terminal-core/src/shell_init.rs`, `crates/vox-terminal-core/src/pty.rs`, `crates/vox-terminal-core/src/osc633.rs`
**Commit:** skipped (shared git index)
**Applied fix:**
- Replaced Fish's line-splitting `string replace -a '\n' '\\x0a'` in `FISH_SHIM_INIT` (`shell_init.rs`) and `FISH_OSC633` (`pty.rs`) with `string split \n -- "$argv[1]" | string replace -a '\\' '\\x5c' | string replace -a ';' '\\x3b' | string join '\\x0a'`, preserving multi-line commands as a single `633;E` payload.
- Escaped `\`, `;`, `\r`, `\n`, `\x07`, and `\x1b` inside `633;P;Cwd=` across all five shell shims (Zsh, Bash, Fish, PowerShell, Nushell) and decoded `Cwd` property values via `decode_command` in `crates/vox-terminal-core/src/osc633.rs`.
- Added unit tests in `shell_init.rs` and `osc633.rs` verifying multi-line Fish encoding and escaped semicolon/backslash `Cwd` round-trips.

## Verification

All edits and verification gates ran directly in the **main checkout** (`/Users/brbrainerd/dev/vox`, honoring `.planning/config.json` `workflow.use_worktrees = false`; zero git commands executed due to the shared git index):

1. `rustfmt --edition 2024` across all 14 touched `.rs` files → **PASS** (exit 0)
2. `cargo test -p vox-terminal-core` → **PASS** (`110 passed; 0 failed` across unit + 13 integration test suites, 0 compiler warnings)
3. `cargo test -p vox-orchestrator-d --bin vox-orchestrator-d term_dispatch` → **PASS** (`11 passed; 0 failed`, 0 compiler warnings)
4. `cargo test -p vox-term` → **PASS** (`18 passed; 0 failed; 1 ignored`, 0 compiler warnings)
5. `cargo test -p vox-cli --lib term_cli_exposes_attach_and_local_flags` → **PASS** (`1 passed; 0 failed`)
6. `cargo test -p vox-cli --test command_catalog_paths_baseline` → **PASS** (`1 passed; 0 failed`)
7. `cargo test -p vox-gui --bin vox-gui commands::pty` → **PASS** (`12 passed; 0 failed`, 0 compiler warnings)
8. `cargo run -p vox-arch-check` → **PASS** (exit 0; zero architectural, layer, or `[no-hardcoded-abs-path]` violations)

---

_Fixed: 2026-11-14T14:32:30Z_
_Fixer: Antigravity (gsd-code-fixer)_
_Iteration: 1_
