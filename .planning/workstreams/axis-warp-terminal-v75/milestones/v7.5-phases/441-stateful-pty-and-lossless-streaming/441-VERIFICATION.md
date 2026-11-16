---
phase: 441-stateful-pty-and-lossless-streaming
verified: 2026-11-14T15:15:00Z
status: passed
score: 22/22 must-haves verified
covered_files:
  - .planning/workstreams/axis-warp-terminal-v75/REQUIREMENTS.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/441-stateful-pty-and-lossless-streaming/441-01-PLAN.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/441-stateful-pty-and-lossless-streaming/441-01-SUMMARY.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/441-stateful-pty-and-lossless-streaming/441-02-PLAN.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/441-stateful-pty-and-lossless-streaming/441-02-SUMMARY.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/441-stateful-pty-and-lossless-streaming/441-03-PLAN.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/441-stateful-pty-and-lossless-streaming/441-03-SUMMARY.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/441-stateful-pty-and-lossless-streaming/441-04-PLAN.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/441-stateful-pty-and-lossless-streaming/441-04-SUMMARY.md
  - crates/vox-cli/src/cli_dispatch/mod.rs
  - crates/vox-cli/src/command_catalog.rs
  - crates/vox-cli/src/lib.rs
  - crates/vox-gui/src/commands/pty.rs
  - crates/vox-orchestrator-d/src/term_dispatch.rs
  - crates/vox-term/src/app.rs
  - crates/vox-term/src/vt.rs
  - crates/vox-term/tests/headless.rs
  - crates/vox-term/tests/palette.rs
  - crates/vox-term/tests/vt.rs
  - crates/vox-terminal-core/Cargo.toml
  - crates/vox-terminal-core/src/block.rs
  - crates/vox-terminal-core/src/daemon_host.rs
  - crates/vox-terminal-core/src/host.rs
  - crates/vox-terminal-core/src/lib.rs
  - crates/vox-terminal-core/src/osc633.rs
  - crates/vox-terminal-core/src/pty.rs
  - crates/vox-terminal-core/src/session.rs
  - crates/vox-terminal-core/src/shell_init.rs
  - crates/vox-terminal-core/src/utf8_stream.rs
  - crates/vox-terminal-core/src/vt_shadow.rs
  - crates/vox-terminal-core/tests/shell_init_and_stateful_pty.rs
  - crates/vox-terminal-core/tests/streaming_utf8_and_alt_screen.rs
covered_digest: "v1:sha256:b68133e06bc597e1c12213ba7c7cb733359053c71e443fa999e9a3999d1aaf3b"
behavior_unverified: 0
overrides_applied: 0
---

# Phase 441: Stateful PTY & Lossless Streaming Verification Report

**Phase Goal:** Every shell command runs inside a persistent, stateful PTY session with silent startup, clean UTF-8 streaming, alternate-screen safety, and live reattachment across GUI and CLI clients.
**Verified:** 2026-11-14T15:15:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

#### ROADMAP Success Criteria (4/4)

| # | Success Criterion | Status | Evidence |
| --- | --- | --- | --- |
| SC-1 | Opening a terminal session in `zsh`, `bash`, `fish`, `pwsh`, or `nu` produces zero echoed shell-hook commands in initial scrollback and accurately reports non-zero exit codes (including subshell `exit 1`). | ✓ VERIFIED | `crates/vox-terminal-core/src/shell_init.rs` builds mode-`0700` `TempDir` launch shims (`ZDOTDIR`, `--rcfile`, `XDG_CONFIG_HOME`, `-NoExit -File`, `--config` + `env.nu` with static `source` splicing); `pty.rs` writes zero bytes to stdin on open; `EXIT` traps emit `\x1b]633;D;$?\x07` on subshell/top-level `exit N`. Live PTY tests `startup_zero_echo_spam_empty_initial_blocks_and_initial_cwd` and `nonzero_exit_codes_false_and_top_level_exit_7` pass. |
| SC-2 | Running `cd` and `export` in one command persists into subsequent commands in the same session, interactive prompts (`read` / `ssh` / `sudo`) receive stdin, and terminal resize + `Ctrl-C` signal delivery propagate to the running process. | ✓ VERIFIED | Single persistent shell child per `LocalHost` session (`host.rs:345-645`); `shell_init.rs` emits `OSC 7` + hex-escaped `OSC 633;P;Cwd` + `OSC 633;P;GitBranch`; `MasterPty::resize` clamps to `1x1` and updates kernel `winsize`; `MasterPty::signal(TermSignal::Interrupt)` writes ETX `\x03` to the PTY master so the foreground job receives `SIGINT` without killing the parent shell. Live PTY tests `stateful_cd_and_export_persist_across_submissions_and_update_cwd`, `interactive_stdin_forwards_into_active_running_block`, and `live_kernel_resize_updates_stty_size_and_sigint_interrupts_foreground_job` pass. |
| SC-3 | Multi-byte UTF-8 glyphs split across arbitrary PTY read chunk boundaries render with zero `U+FFFD` replacement characters, and launching/exiting full-screen TUIs (`vim`/`less`/`htop`) switches cleanly to and from alternate screen without corrupting block history. | ✓ VERIFIED | `Utf8StreamDecoder` (`utf8_stream.rs:14-97`) buffers 1–3 trailing incomplete UTF-8 lead/continuation bytes across chunks in `osc633.rs` and `vox-gui/src/commands/pty.rs`; `VtShadow` (`vt_shadow.rs:34-203`) wraps headless `alacritty_terminal::Term` + `LegacyAltSync` (`?47`/`?1047`/`?1049`) and strips alt-screen bytes via `feed_extracting_primary` so TUI frames never leak into `Block::plain_output`. Verified by `prime_chunk_utf8_reassembly_zero_replacement_chars` and `alt_screen_entry_exit_and_single_chunk_frames_never_leak_into_plain_output`. |
| SC-4 | Closing `vox-gui` or detaching `vox term` while a daemon-hosted command is running and reattaching via `vox term --attach` or reopening `vox-gui` resumes live output and completed block state without killing the process. | ✓ VERIFIED | `LocalHost::attach_since` (`host.rs:789-838`) serves bounded `2 MiB` `after_seq` delta snapshots + live broadcast stream; `DaemonHost` (`daemon_host.rs`) and `term_dispatch.rs` route `after_seq`/`resync_required`/`SnapshotResync`; `vox term --attach [SESSION_ID]` (`vox-term/src/app.rs`, `vox-cli/src/lib.rs`) exits on `q`/`Ctrl-D`/EOF without calling `host.close`; `vox-gui` (`commands/pty.rs`) threads `after_seq` and aborts prior forwarders on reattach without killing daemon sessions. Verified by `term_dispatch` loopback TCP tests, `vox-term` `headless` tests, and `vox-gui` `commands::pty` tests. |

#### Plan Must-Have Truths (18/18)

| # | Plan | Truth | Status | Evidence |
| --- | --- | --- | --- | --- |
| 1 | 441-01 | Opening a session in `zsh`, `bash`, `fish`, `pwsh`, or `nu` writes zero hook-injection bytes to PTY stdin and produces zero echoed shell-hook commands in initial scrollback or initial `blocks()`. | ✓ VERIFIED | `pty.rs:285-380` (`spawn_pty_with_callback`) injects hooks solely via launch argv/env shims from `shell_init::prepare_shell_command`; `Session::on_osc` (`session.rs:312-320`) keeps `blocks()` empty on initial `PromptStart`. |
| 2 | 441-01 | Commands returning non-zero status (`false`, `sh -c 'exit 42'`, and subshell/top-level `exit 7`) record the exact integer `exit_code` on the closed `Block`. | ✓ VERIFIED | `shell_init.rs` captures `$?` / `$status` / `$LASTEXITCODE` as the very first statement of `precmd`/`PROMPT_COMMAND` and installs `trap ... EXIT` (`zsh`/`bash`) and `fish_exit` handlers. Tested in `nonzero_exit_codes_false_and_top_level_exit_7`. |
| 3 | 441-01 | Running `cd <dir>` and `export FOO=bar` in one submission persists into subsequent submissions in the same session and updates `Session::cwd` + `SessionInfo.cwd`. | ✓ VERIFIED | `session.rs:367-428` updates `Session::cwd` + `git_branch` from `Osc633Event::Property` / `Osc7Cwd` and emits `SessionEvent::CwdChanged`; `host.rs:719-747` mirrors both into `SessionInfo`. |
| 4 | 441-01 | Interactive commands (`read`, `cat`, password/confirmation prompts) receive raw `host.input(...)` bytes on the active PTY while a block is `Running`. | ✓ VERIFIED | `LocalHost::input` (`host.rs:845-866`) writes raw bytes directly to `MasterPty::write_all` while `Origin::User` passes the origin guard. Tested in `interactive_stdin_forwards_into_active_running_block`. |
| 5 | 441-01 | Calling `host.resize(id, cols, rows)` updates kernel PTY `winsize` (`stty size`), and calling `host.signal(id, TermSignal::Interrupt, Origin::User)` interrupts the foreground job without killing the parent shell session. | ✓ VERIFIED | `MasterPty::resize` (`pty.rs:144-160`) invokes `portable_pty::MasterPty::resize` with `cols.max(1)`/`rows.max(1)`; `MasterPty::signal` (`pty.rs:186-214`) sends `\x03` (`ETX`) for `Interrupt` vs `child.kill()` for `Kill`. |
| 6 | 441-02 | Multi-byte UTF-8 sequences (2-, 3-, and 4-byte codepoints) split across arbitrary 1..7-byte PTY read boundaries decode with zero `U+FFFD` replacement characters in both `OutputFrame` stream consumers and `Block::plain_output`. | ✓ VERIFIED | `Utf8StreamDecoder::push` (`utf8_stream.rs:28-80`) retains incomplete trailing sequences up to 3 bytes; `Osc633Parser::feed` (`osc633.rs:116-148`) decodes chunk slices through `self.utf8`. Tested across prime chunks `[1, 2, 3, 5, 7]` over 10,000 multi-byte codepoints. |
| 7 | 441-02 | Command stdout/stderr emitted between `633;C` and `633;D` streams incrementally into the running `Block` on every `feed()` call without waiting for command exit, while `\r` progress-bar overwrites collapse cleanly in `plain_output()`. | ✓ VERIFIED | `osc633.rs` removed `output_buf` hoarding and emits `Osc633Event::CommandOutput` immediately per `feed()`; `block.rs:110-115` runs `normalize_cr_overwrites` (`\r`, `\r\n`, `CSI K`/`0K`/`1K`/`2K`) before `strip_ansi`. |
| 8 | 441-02 | Entering and exiting alternate screen (`CSI ?1049h`/`l`, `?1047h`/`l`, `?47h`/`l`, `ESC c`) flips `Session::is_alt_screen()`, emits `SessionEvent::AltScreenChanged`, suppresses nested block splits while active, and never leaks TUI redraw bytes into `Block::plain_output`. | ✓ VERIFIED | `VtShadow::feed_extracting_primary` (`vt_shadow.rs:125-182`) slices chunks at exact parser transition offsets and excludes alt-screen ranges; `session.rs:289-340` suppresses nested `PromptStart`/`CommandStart` while `alt_screen` is true. |
| 9 | 441-02 | `LocalHost` bounds per-session replay memory with a 2 MiB `VecDeque<OutputFrame>` ring and supports `attach(id, after_seq)` returning only frames with `seq > after_seq` plus `resync_required = true` when `after_seq` was evicted. | ✓ VERIFIED | `MAX_REPLAY_BYTES = 2 * 1024 * 1024` (`host.rs:36`), `push_replay_frame` (`host.rs:299-332`) sets `truncated_front = true` on eviction and caps single frames >2 MiB, and `attach_since` (`host.rs:789-838`) computes `resync_required`. |
| 10 | 441-02 | `SessionHost` trait exposes `attach(id, after_seq: Option<u64>)` and `signal(id, signal: TermSignal, origin: Origin)` with matching `LocalHost` and `DaemonHost` signatures. | ✓ VERIFIED | `SessionHost` (`host.rs:226-267`) defines `attach(&self, id: &SessionId, after_seq: Option<u64>)` and `signal(&self, id: &SessionId, signal: TermSignal, origin: Origin)`, implemented on both `LocalHost` (`host.rs:716-947`) and `DaemonHost` (`daemon_host.rs:255-434`). |
| 11 | 441-03 | `vox-orchestrator-d` `term.open` defaults unspecified dimensions to `120x32` (matching `OpenSpec::default()`) instead of `80x24`. | ✓ VERIFIED | `term_dispatch.rs:141-145` uses `OpenSpec::default()` (`120` cols x `32` rows) when `cols`/`rows` are omitted from `term.open` params. |
| 12 | 441-03 | `term.attach` accepts optional `after_seq: u64`, calls `host.attach_since(&id, after_seq)`, and returns `{ session_id, blocks, replay, resync_required, alt_screen, cwd, git_branch }` before streaming live frames. | ✓ VERIFIED | `term_dispatch.rs:212-265` parses `after_seq`, delegates to `host.attach_since`, serializes full attachment metadata, and streams `SessionEvent` JSON lines (`alt_screen_changed`, `cwd_changed`, `snapshot_resync`, enriched `block_opened`/`block_closed`). |
| 13 | 441-03 | `term.signal` dispatches `Interrupt` or `Kill` to `LocalHost::signal` and fail-closes (`E_AGENT_ORIGIN_FORBIDDEN`) when `origin == Origin::Agent` or `origin` is missing/unknown. | ✓ VERIFIED | `term_dispatch.rs:348-378` parses origin fail-closed via `parse_origin`, enforces `refuse_agent_origin(origin, "term.signal")`, and calls `host.signal(&id, signal, origin)`. Tested over loopback TCP. |
| 14 | 441-03 | When a slow daemon subscriber lags the broadcast ring, the server emits `{"kind":"lagged","skipped":N}` and `DaemonHost` automatically resubscribes with `after_seq` to emit `SessionEvent::SnapshotResync` + delta frames without dropping the client stream (capped so pathological churn terminates cleanly). | ✓ VERIFIED | `daemon_host.rs:479-592` (`pump_with_resubscribe`) intercepts `"lagged"`, caps retries at `MAX_LAG_RESUBSCRIBE_ATTEMPTS = 64`, re-attaches with `after_seq`, emits `SnapshotResync` when `resync_required` is true, and forwards delta `OutputFrame`s. |
| 15 | 441-04 | `vox term` and `vox-term` accept `--attach [SESSION_ID]` (defaulting bare `--attach` to the newest running session using numeric `term-<N>` tie-breaking where `term-10 > term-9`) and `--local`, attaching with preserved blocks, replay, cwd, and alt-screen state. | ✓ VERIFIED | `vox-cli/src/lib.rs:219-231` defines `Cli::Term { attach, local }` with `num_args = 0..=1, default_missing_value = "latest"`; `vox-term/src/app.rs:64-156` implements numeric `term_numeric_key` (`term-10 > term-9`) and `attach_or_open_session`. |
| 16 | 441-04 | Detaching (`q` in Normal mode or `Ctrl-D`/EOF) or closing `vox-term` / `vox-gui` leaves daemon-hosted sessions running so reattaching resumes live output and completed block state without killing the child process. | ✓ VERIFIED | `vox-term/src/app.rs:514-567` breaks out of the UI loop on `Normal` `'q'` or non-TTY EOF without calling `host.close`; `vox-gui/src/commands/pty.rs:475-560` aborts only local Tauri event-forwarder tasks on reattach while `term_session_close` remains explicit. |
| 17 | 441-04 | Pressing `Ctrl-C` in `vox-term` Insert mode sends `host.signal(id, TermSignal::Interrupt, Origin::User)` to interrupt the foreground job inside the PTY instead of tearing down the `vox-term` client. | ✓ VERIFIED | `vox-term/src/app.rs:495-503` intercepts `KeyCode::Char('c')` with `KeyModifiers::CONTROL` in `Mode::Insert` and calls `host.signal(&sid, TermSignal::Interrupt, Origin::User).await`. |
| 18 | 441-04 | `vox-gui` `term_session_attach` accepts `after_seq: Option<u64>`, decodes replay and live `OutputFrame` chunks through a shared per-attachment `Utf8StreamDecoder` (eliminating `String::from_utf8_lossy` boundary `U+FFFD` corruption), and handles broadcast lag via `after_seq` delta resync. | ✓ VERIFIED | `vox-gui/src/commands/pty.rs:256-315` (`term_attachment_dto_and_decoder`) carries the stateful `Utf8StreamDecoder` across the replay-to-live seam, flushes only on `Closed`, and handles `RecvError::Lagged` (`pty.rs:509-546`) via `host.attach(&sid_task, Some(last_seq)).await`. |

**Score:** 22/22 truths verified (4/4 ROADMAP Success Criteria + 18/18 Plan Must-Have Truths)

> **Implementation Refinement Note (`441-04`):** `441-04-PLAN.md` Task 1 initially suggested re-calling `host.attach` on `BlockOpened`/`BlockClosed` in `vox-term`. During implementation, `SessionEvent::{BlockOpened, BlockClosed}` were enriched in `session.rs`, `daemon_host.rs`, and `term_dispatch.rs` to carry full `kind`, `input`, `origin`, `cwd`, `git_branch`, `exit_code`, and `status` directly on the event stream. `vox-term::app::apply_session_event` (`app.rs:159-250`) updates peer block metadata in-place with zero redundant `host.attach` RPC round-trips (preventing duplicate replay emission and orphaned daemon subscriptions), while `SnapshotResync` and `TryRecvError::Lagged` perform `host.attach(&sid, Some(last_seq))` delta resyncs. Covered by `apply_session_event_syncs_peer_block_metadata_and_appends_output_in_place`.

### Required Artifacts

Verified via `gsd-tools.cjs query verify.artifacts --ws axis-warp-terminal-v75` across all 4 plans (`15/15` passed):

| Artifact | Plan | Expected Provides | Exists | Substantive | Wired | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `crates/vox-terminal-core/src/shell_init.rs` | 441-01 | Per-shell launch shims (`zsh`, `bash`, `fish`, `pwsh`, `nu`) emitting `OSC 633` + `OSC 7` + `GitBranch` with zero stdin echo (`prepare_shell_command`) | ✓ | ✓ (1,004 lines) | ✓ | ✓ VERIFIED |
| `crates/vox-terminal-core/src/pty.rs` | 441-01 | `portable-pty` spawn via `ShellLaunchShim`, `MasterPty::resize`, `MasterPty::get_size`, `MasterPty::signal` (`TermSignal`), RAII `TempDir` retention | ✓ | ✓ (592 lines) | ✓ | ✓ VERIFIED |
| `crates/vox-terminal-core/src/session.rs` | 441-01 | Stateful `Session` tracking `cwd` + `git_branch` from `OSC 633;P` / `OSC 7`, non-phantom initial `PromptStart`, `begin_pty_submission` | ✓ | ✓ (641 lines) | ✓ | ✓ VERIFIED |
| `crates/vox-terminal-core/src/host.rs` | 441-01 / 441-02 | `OpenSpec` `120x32` defaults, `2 MiB` `VecDeque<OutputFrame>` replay ring, `attach_since`, widened `SessionHost::{attach, signal}` | ✓ | ✓ (1,249 lines) | ✓ | ✓ VERIFIED |
| `crates/vox-terminal-core/tests/shell_init_and_stateful_pty.rs` | 441-01 | Integration tests for `TERM-PTY-01` and `TERM-PTY-02` | ✓ | ✓ (396 lines, 6 tests) | ✓ | ✓ VERIFIED |
| `crates/vox-terminal-core/src/utf8_stream.rs` | 441-02 | `Utf8StreamDecoder` + cell-indexed ANSI-aware `normalize_cr_overwrites` | ✓ | ✓ (232 lines) | ✓ | ✓ VERIFIED |
| `crates/vox-terminal-core/src/vt_shadow.rs` | 441-02 | `VtShadow` wrapping `alacritty_terminal::Term` (`scrolling_history: 0`) + `LegacyAltSync` + `feed_extracting_primary` | ✓ | ✓ (204 lines) | ✓ | ✓ VERIFIED |
| `crates/vox-terminal-core/src/daemon_host.rs` | 441-02 | `DaemonHost` client forwarding `after_seq`, `term.signal`, `AltScreenChanged`, `CwdChanged`, `SnapshotResync`, lag resubscribe | ✓ | ✓ (731 lines) | ✓ | ✓ VERIFIED |
| `crates/vox-terminal-core/tests/streaming_utf8_and_alt_screen.rs` | 441-02 | Integration + property-style chunking tests for `TERM-PTY-03` and `L3` `TERM-PTY-04` | ✓ | ✓ (448 lines, 9 tests) | ✓ | ✓ VERIFIED |
| `crates/vox-orchestrator-d/src/term_dispatch.rs` | 441-03 | `term.open` (`120x32`), `term.list` (`cwd`/`git_branch`/`alt_screen`), `term.attach` (`after_seq` + `resync_required`), `term.signal` (`Origin::User`-only) | ✓ | ✓ (923 lines, 11 tests) | ✓ | ✓ VERIFIED |
| `crates/vox-term/src/app.rs` | 441-04 | `TermCliArgs`, `run_with_args`, `attach_or_open_session`, numeric `"latest"`, `SnapshotResync`/lag handling, `Ctrl-C` `TermSignal::Interrupt`, clean detach | ✓ | ✓ (752 lines) | ✓ | ✓ VERIFIED |
| `crates/vox-term/src/vt.rs` | 441-04 | `VtGrid` `1x1` clamping, `LegacyAltSync`, `WIDE_CHAR_SPACER` skip, `is_alt_screen()`, `reset()` | ✓ | ✓ (227 lines) | ✓ | ✓ VERIFIED |
| `crates/vox-cli/src/lib.rs` | 441-04 | `Cli::Term { attach: Option<String>, local: bool }` clap flags | ✓ | ✓ | ✓ | ✓ VERIFIED |
| `crates/vox-gui/src/commands/pty.rs` | 441-04 | `term_session_attach` with `after_seq`, shared `Utf8StreamDecoder`, `SnapshotResync` + lag delta recovery, forwarder cleanup | ✓ | ✓ (627 lines, 12 tests) | ✓ | ✓ VERIFIED |

### Key Link Verification

Verified via `gsd-tools.cjs query verify.key-links --ws axis-warp-terminal-v75` across all 4 plans (`11/11` verified):

| From | To | Via | Status | Details |
| --- | --- | --- | --- | --- |
| `crates/vox-terminal-core/src/pty.rs` | `crates/vox-terminal-core/src/shell_init.rs` | `prepare_shell_command(&spec.shell, spec.cwd.as_deref(), &spec.env)` | ✓ WIRED | `pty.rs:297` builds `ShellLaunchShim` and retains `_shim_dir` on `MasterPty` (`pty.rs:117`). |
| `crates/vox-terminal-core/src/host.rs` | `crates/vox-terminal-core/src/pty.rs` | `spawn_pty_with_callback(&spec, ...)` | ✓ WIRED | `host.rs:459` spawns persistent PTY reader thread feeding `Session::on_pty_bytes`. |
| `crates/vox-terminal-core/src/session.rs` | `crates/vox-terminal-core/src/osc633.rs` | `Osc633Event::{Property, Osc7Cwd, PromptEnd}` | ✓ WIRED | `session.rs:367-428` updates `cwd`/`git_branch` and emits `SessionEvent::CwdChanged`. |
| `crates/vox-terminal-core/src/osc633.rs` | `crates/vox-terminal-core/src/utf8_stream.rs` | `Utf8StreamDecoder` | ✓ WIRED | `osc633.rs:88,125` decodes split UTF-8 chunks losslessly and flushes on EOF (`osc633.rs:153`). |
| `crates/vox-terminal-core/src/session.rs` | `crates/vox-terminal-core/src/vt_shadow.rs` | `VtShadow::feed_extracting_primary` | ✓ WIRED | `session.rs:220` tracks `alt_screen` transitions and filters alt-screen bytes before `Osc633Parser::feed`. |
| `crates/vox-terminal-core/src/daemon_host.rs` | `crates/vox-terminal-core/src/host.rs` | `SessionHost::{attach(after_seq), signal}` | ✓ WIRED | `daemon_host.rs:314,395` implements `attach` with `after_seq` and `signal` over `term.attach`/`term.signal`. |
| `crates/vox-orchestrator-d/src/term_dispatch.rs` | `crates/vox-terminal-core/src/host.rs` | `LocalHost::{attach_since, signal}` | ✓ WIRED | `term_dispatch.rs:222,374` routes daemon TCP RPCs into `LocalHost`. |
| `crates/vox-orchestrator-d/src/term_dispatch.rs` | `crates/vox-terminal-core/src/daemon_host.rs` | Loopback TCP wire-protocol parity | ✓ WIRED | `term_dispatch.rs:607-921` exercises real `DaemonHost` against `term_dispatch::dispatch`. |
| `crates/vox-cli/src/cli_dispatch/mod.rs` | `crates/vox-term/src/app.rs` | `vox_term::app::run_with_args(TermCliArgs { attach, force_local: local }).await` | ✓ WIRED | `cli_dispatch/mod.rs:394-398` awaits async `run_with_args` directly without nested Tokio runtime panic. |
| `crates/vox-term/src/app.rs` | `crates/vox-terminal-core/src/host.rs` | `attach_or_open_session` -> `SessionHost::{list, attach, open, signal}` | ✓ WIRED | `app.rs:106-156,360-567` attaches/resyncs/signals via trait object `Arc<dyn SessionHost>`. |
| `crates/vox-gui/src/commands/pty.rs` | `crates/vox-terminal-core/src/utf8_stream.rs` | `Utf8StreamDecoder::{push, flush_lossy}` | ✓ WIRED | `pty.rs:265,495,500` shares one decoder across replay and live stream frames, flushing on `Closed`. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| --- | --- | --- | --- | --- |
| `shell_init.rs` -> `pty.rs` -> `session.rs` | `Osc633Event::{Property("Cwd"\|"GitBranch"), CommandFinished { exit_code }}` | Child shell `precmd` / `PROMPT_COMMAND` / `fish_prompt` / `env.nu` hooks over kernel PTY master reader | Yes — live shell cwd, git branch, and `$?` exit codes verified in `shell_init_and_stateful_pty` | ✓ FLOWING |
| `pty.rs` -> `vt_shadow.rs` -> `osc633.rs` -> `block.rs` | Primary-screen UTF-8 text -> `Block::raw_output` -> `Block::plain_output()` | `VtShadow::feed_extracting_primary` + `Utf8StreamDecoder::push` + `normalize_cr_overwrites` | Yes — live incremental chunks, zero `U+FFFD`, zero alt-screen TUI frame leakage | ✓ FLOWING |
| `host.rs` -> `term_dispatch.rs` -> `daemon_host.rs` -> `vox-term` / `vox-gui` | `Attachment { blocks, replay, resync_required, alt_screen, cwd, git_branch }` + `SessionEvent` stream | `LocalHost` `2 MiB` `VecDeque<OutputFrame>` ring + `tokio::sync::broadcast` -> TCP JSON-RPC stream -> TUI `VtGrid` & GUI `TermOutputPayload` | Yes — verified end-to-end over TCP loopback and across replay-to-live UTF-8 boundaries | ✓ FLOWING |

### Behavioral Spot-Checks

All 6 behavioral test suites were executed directly in this verification session:

| Behavior | Command | Result | Status |
| --- | --- | --- | --- |
| 5-shell shims, zero startup echo, exact exit codes (`false`->1, `exit 7`->7), stateful `cd`/`export`, interactive `read` stdin, live `stty size` resize + `Ctrl-C` `SIGINT` | `timeout 300s cargo test -p vox-terminal-core --test shell_init_and_stateful_pty -- --nocapture` | `6 passed; 0 failed` (2.06s) | ✓ PASS |
| Prime-chunk UTF-8 reassembly (0 `U+FFFD`), live `633;C..D` streaming, CR/`CSI K` overwrite normalization, `VtShadow` alt-screen isolation, `2 MiB` `after_seq` delta ring | `timeout 300s cargo test -p vox-terminal-core --test streaming_utf8_and_alt_screen` | `9 passed; 0 failed` (0.04s) | ✓ PASS |
| Daemon `120x32` defaults, `term.attach` `after_seq` delta + `resync_required`, `alt_screen`/`cwd` events, `Origin::Agent` refusal on `term.signal`, live `SIGINT` over TCP | `timeout 300s cargo test -p vox-orchestrator-d --bin vox-orchestrator-d term_dispatch` | `11 passed; 0 failed` (1.20s) | ✓ PASS |
| `--attach` / `--local` CLI parsing, numeric `"latest"` (`term-10 > term-9`), detach/reattach continuity, `VtGrid` clamping/alt-screen/reset, in-place peer block sync | `timeout 300s cargo test -p vox-term --test headless` | `8 passed; 0 failed` (1.91s) | ✓ PASS |
| `vox term --attach [SESSION_ID] --local` clap surface and command catalog registration | `timeout 300s cargo test -p vox-cli --lib term_cli_exposes_attach_and_local_flags` | `1 passed; 0 failed` (0.10s) | ✓ PASS |
| `vox-gui` `after_seq` attachment DTO, stateful `Utf8StreamDecoder` across replay-to-live seam, local fallback session lifecycle | `timeout 300s cargo test -p vox-gui --bin vox-gui commands::pty` | `12 passed; 0 failed` (0.48s) | ✓ PASS |

### Probe Execution

No `probe-*.sh` scripts were declared in `441-01-PLAN.md`..`441-04-PLAN.md` (per repo VoxLang/Rust policy, executable TDD verification was authored as Rust integration tests in `crates/vox-terminal-core/tests/`, `crates/vox-orchestrator-d/src/term_dispatch.rs`, `crates/vox-term/tests/headless.rs`, `crates/vox-cli/src/command_catalog.rs`, and `crates/vox-gui/src/commands/pty.rs`, all executed in Behavioral Spot-Checks above).

### Requirements Coverage

Checked against `.planning/workstreams/axis-warp-terminal-v75/REQUIREMENTS.md` (`Phase 441` mapping):

| Requirement | Source Plan(s) | Description | Status | Supporting Evidence |
| --- | --- | --- | --- | --- |
| `TERM-PTY-01` | `441-01` | Silent per-shell startup integration (`zsh`, `bash`, `fish`, `pwsh`, `nu`) injecting `OSC 633` (`A`/`B`/`C`/`D`/`E`/`P`) and `OSC 7` without echoing hook code into scrollback, preserving user RC hooks and accurate non-zero exit codes (including subshell `exit`). | ✓ SATISFIED | `shell_init.rs`, `pty.rs`, `osc633.rs`, `session.rs`; `shell_init_and_stateful_pty` (`five_shell_shims_openspec_defaults_and_osc_parser_bounds`, `startup_zero_echo_spam_empty_initial_blocks_and_initial_cwd`, `nonzero_exit_codes_false_and_top_level_exit_7`). |
| `TERM-PTY-02` | `441-01`, `441-03`, `441-04` | Stateful persistent shell session per tab (`cd`, `export`, aliases, interactive `read`/`ssh`/`sudo` stdin, `SIGWINCH` resize, `Ctrl-C`/`SIGINT` signal delivery). | ✓ SATISFIED | `pty.rs` (`MasterPty::{resize, get_size, signal}`), `host.rs` (`LocalHost::{input, submit, resize, signal}`), `term_dispatch.rs` (`term.signal`), `vox-term/src/app.rs` (`Ctrl-C` -> `TermSignal::Interrupt`). |
| `TERM-PTY-03` | `441-02`, `441-04` | Lossless streaming byte pipeline with stateful UTF-8 boundary reassembly (zero `U+FFFD` corruption across arbitrary PTY read chunks) and alternate-screen (`?1049h`/`?1049l`) detection + isolation for full-screen TUIs (`vim`, `less`, `htop`, `git log`). | ✓ SATISFIED | `utf8_stream.rs`, `vt_shadow.rs`, `osc633.rs`, `block.rs`, `vox-term/src/vt.rs`, `vox-gui/src/commands/pty.rs`; `streaming_utf8_and_alt_screen` (9 tests) + `commands::pty` seam tests. |
| `TERM-PTY-04` | `441-02`, `441-03`, `441-04` | Daemon-backed session persistence and reattach (`vox term --attach`, GUI window close/reopen) with bounded ring-buffer replay and delta sequence numbers (`after_seq`). | ✓ SATISFIED | `host.rs` (`2 MiB` ring + `attach_since`), `daemon_host.rs` (`pump_with_resubscribe`), `term_dispatch.rs` (`term.attach` `after_seq`), `vox-cli` + `vox-term` (`--attach`), `vox-gui` (`term_session_attach`). |

**Orphaned requirements check:** No additional requirements in `REQUIREMENTS.md` are mapped to Phase 441 beyond `TERM-PTY-01..04`. Zero orphaned requirements.

### Anti-Patterns Found

Scanned all 22 modified/created implementation and test files with `rg -n "TBD|FIXME|XXX|TODO|HACK|PLACEHOLDER|unimplemented!|todo!"`:

| File | Line | Pattern | Severity | Impact |
| --- | --- | --- | --- | --- |
| *(all 22 Phase 441 source & test files)* | — | Zero `TODO` / `FIXME` / `HACK` / `PLACEHOLDER` / `todo!` / `unimplemented!` matches | — | Clean. Two documented `ponytail:` architecture notes exist (`pty.rs:192` explaining cross-platform ETX `\x03` PTY line-discipline `SIGINT` delivery; `host.rs:348` noting single-shell-per-session invariants) — both are intentional design rationale comments, not stubs or deferred Phase 441 requirements. |

### Human Verification Required

None — zero `<human-check>` blocks were declared in `441-01-PLAN.md`..`441-04-PLAN.md`, and `441-VALIDATION.md` confirms all Phase 441 behaviors have automated PTY, loopback TCP, and headless TUI/GUI unit/integration coverage.

### Gaps Summary

No gaps found. All 4 ROADMAP Success Criteria, 18 plan must-have truths, 15 required artifacts, 11 key links, and requirements `TERM-PTY-01` through `TERM-PTY-04` are implemented, wired end-to-end, and behaviorally verified against live tests.
