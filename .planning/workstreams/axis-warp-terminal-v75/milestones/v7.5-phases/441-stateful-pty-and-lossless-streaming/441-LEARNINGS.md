---
phase: 441
phase_name: "stateful-pty-and-lossless-streaming"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 30
  lessons: 7
  patterns: 19
  surprises: 2
missing_artifacts:
  - "UAT.md"
---

# Phase 441 Learnings: stateful-pty-and-lossless-streaming

## Decisions

### Replaced raw PTY stdin snippet writes in spawn_pty with shell_init::prep...
Replaced raw PTY stdin snippet writes in `spawn_pty` with `shell_init::prepare_shell_command` `TempDir` (`0700`) launch shims while preserving `shell_integration_snippet` and its constants for backwards compatibility.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 441-01-SUMMARY.md

---

### Pinned __vox_precmd to index 1 of precmd_functions in zsh (local ec=$? o...
Pinned `__vox_precmd` to index 1 of `precmd_functions` in zsh (`local ec=$?` on line 1) plus a tail `__vox_prompt_wrap` hook and `EXIT` traps in zsh/bash so user `.zshrc`/`.bashrc` themes cannot clobber non-zero `$?` or `exit N` codes.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 441-01-SUMMARY.md

---

### Bounded LocalHost::submit_with_origin synchronous Condvar settle at 1500...
Bounded `LocalHost::submit_with_origin` synchronous `Condvar` settle at `1500ms` for new commands and `500ms` when forwarding stdin into an active `Running` block so fast commands complete synchronously while long-running/interactive jobs return `Running`.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 441-01-SUMMARY.md

---

### Non-echo 5-shell TempDir bootstrap (TERM-PTY-01)
**Non-echo 5-shell `TempDir` bootstrap (`TERM-PTY-01`):** Created `crates/vox-terminal-core/src/shell_init.rs` with owner-only `0700` `tempfile::TempDir` launch shims for `zsh` (`ZDOTDIR` chain restoring user `ZDOTDIR`, `unsetopt PROMPT_SP`, `%{...%}` prompt wrapping, `precmd_functions` index-1 `$?` capture, `EXIT` trap), `bash` (`--rcfile` chaining string/array `PROMPT_COMMAND` before `__VOX_AT_PROMPT=1` `DEBUG` trap + `EXIT` trap), `fish` (`--init-command` with single-quote escaping), `pwsh` (`-NoLogo -NoExit -File` guarding `PSReadLine`), and `nu` (Rust-side `Path::is_file()` probe splicing a literal `source '...'` line before `$env.config` hooks).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 441-01-SUMMARY.md

---

### Live MasterPty::resize, TermSignal, and RAII cleanup (TERM-PTY-01, TERM-...
**Live `MasterPty::resize`, `TermSignal`, and RAII cleanup (`TERM-PTY-01`, `TERM-PTY-02`):** Upgraded `PtyHandle` in `crates/vox-terminal-core/src/pty.rs` to retain `Arc<Mutex<Box<dyn MasterPty + Send>>>` and `Option<TempDir>`, implementing real `MasterPty::resize` (`cols.max(1)`, `rows.max(1)`), `get_size`, `TermSignal` (`Interrupt` -> `\x03` ETX, `Terminate | Kill` -> `child.kill()`), `try_wait_exit_code`, and `spawn_pty_with_callback` on a dedicated `vox-pty-reader` OS thread.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 441-01-SUMMARY.md

---

### Bounded OSC 633 + OSC 7 streaming parser (TERM-PTY-01)
**Bounded `OSC 633` + `OSC 7` streaming parser (`TERM-PTY-01`):** Extended `crates/vox-terminal-core/src/osc633.rs` with `Osc633Event::PromptEnd` (`633;B`), `Osc633Event::Property { key, value }` (`633;P;Cwd` / `633;P;GitBranch`), percent-decoded `Osc633Event::Osc7Cwd` (`OSC 7` `file://`), 3-byte UTF-8 split-chunk tail reassembly, and a `16 KiB` (`MAX_INCOMPLETE_OSC_BYTES`) incomplete-OSC flush guard (`T-441-03`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 441-01-SUMMARY.md

---

### Stateful PTY execution & Condvar pump (TERM-PTY-02)
**Stateful PTY execution & `Condvar` pump (`TERM-PTY-02`):** Upgraded `Session` (`crates/vox-terminal-core/src/session.rs`) and `LocalHost` (`crates/vox-terminal-core/src/host.rs`) to `120×32` `OpenSpec::default()`, eliminated idle-prompt phantom blocks on `633;A`, pumped PTY bytes directly from `vox-pty-reader` into `Entry::feed` + `settled_cv.notify_all()`, and routed shell submissions through the persistent `PtyHandle` so `cd`, `export`, and interactive `Running` stdin (`read`) persist across submissions.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 441-01-SUMMARY.md

---

### Kept shell_integration_snippet and all 5 *_OSC633 string constants in pt...
Kept `shell_integration_snippet` and all 5 `*_OSC633` string constants in `pty.rs` intact while migrating `spawn_pty` / `spawn_pty_with_callback` onto `shell_init::prepare_shell_command` so existing `tests/pty.rs` and `vox-gui` callers continue compiling without breakage ahead of Plan `441-02`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 441-01-SUMMARY.md

---

### Streamed Osc633Event::Output chunks live through Utf8StreamDecoder both ...
Streamed `Osc633Event::Output` chunks live through `Utf8StreamDecoder` both inside and outside `633;C..633;D` (flushing incomplete bytes via `flush_lossy()` only at escape marker boundaries) so long-running builds emit `OutputAppended` immediately before command exit.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 441-02-SUMMARY.md

---

### Fed VtShadow causally inside Session::apply_osc(Osc633Event::Output) acr...
Fed `VtShadow` causally inside `Session::apply_osc(Osc633Event::Output)` across `ESC`-segmented sub-slices so nested TUI `OSC 633` markers inside `?1049h..l` are suppressed while an outer `633;D;0` arriving immediately after `?1049l` in the same read buffer closes the outer block cleanly.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 441-02-SUMMARY.md

---

### Preserved inherent LocalHost::attach(&self, id) delegating to LocalHost:...
Preserved inherent `LocalHost::attach(&self, id)` delegating to `LocalHost::attach_since(id, None)` alongside widened `SessionHost::attach(&self, id, after_seq)` so direct `LocalHost` callers compile without churn ahead of Wave 3.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 441-02-SUMMARY.md

---

### Lossless streaming UTF-8 & progress overwrite normalization (TERM-PTY-03...
**Lossless streaming UTF-8 & progress overwrite normalization (`TERM-PTY-03`):** Built `crates/vox-terminal-core/src/utf8_stream.rs` exporting `Utf8StreamDecoder` (`new()`, `push(&mut self, bytes: &[u8]) -> String`, `flush_lossy(&mut self) -> String`) and ANSI-aware `normalize_cr_overwrites`, upgraded `Osc633Parser` (`crates/vox-terminal-core/src/osc633.rs`) to stream `Osc633Event::Output` chunks live between `633;C` and `633;D` without `output_buf` hoarding or `String::from_utf8_lossy` corruption, and added `cwd`, `git_branch`, `cancel()`, and `\r`/`CSI K` normalization to `Block` (`crates/vox-terminal-core/src/block.rs`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 441-02-SUMMARY.md

---

### Zero-scrollback VtShadow ALT_SCREEN state machine (TERM-PTY-03)
**Zero-scrollback `VtShadow` `ALT_SCREEN` state machine (`TERM-PTY-03`):** Created `crates/vox-terminal-core/src/vt_shadow.rs` wrapping `alacritty_terminal::Term` with `Config { scrolling_history: 0, ..Config::default() }` and `1×1` dimension clamping (`T-441-02`), tracking `?1049h`/`?1049l`, `?47h`/`?47l`, `?1047h`/`?1047l`, compound DECSET, and `\x1bc` RIS transitions, and wired `Session` (`crates/vox-terminal-core/src/session.rs`) to emit `SessionEvent::AltScreenChanged { active }`, stamp `cwd`/`git_branch` onto opened blocks, and suppress nested `OSC 633` block splits while `alt_screen` is active.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 441-02-SUMMARY.md

---

### Bounded 2 MiB after_seq delta ring & DaemonHost lag recovery (TERM-PTY-0...
**Bounded `2 MiB` `after_seq` delta ring & `DaemonHost` lag recovery (`TERM-PTY-03`, `L3` contract for `TERM-PTY-04`):** Replaced `LocalHost`'s byte ring in `crates/vox-terminal-core/src/host.rs` with a `seq`-indexed `VecDeque<OutputFrame>` ring (`LocalHost::attach_since`, `resync_required`, `SessionInfo` + `Attachment` metadata fields), widened `SessionHost::attach(id, after_seq)` and `SessionHost::signal(id, sig)`, and upgraded `DaemonHost` (`crates/vox-terminal-core/src/daemon_host.rs`) with `pub const SIGNAL = "term.signal"`, `alt_screen`/`cwd_changed` frame routing, `after_seq` delta attach, and bounded automatic `"lagged"` stream re-subscription (`T-441-04`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 441-02-SUMMARY.md

---

### Kept LocalHost::attach(&self, id) as an inherent convenience method dele...
Kept `LocalHost::attach(&self, id)` as an inherent convenience method delegating to `self.attach_since(id, None)` while widening `SessionHost::attach(&self, id, after_seq: Option<u64>)` so direct `LocalHost` callers (`vox-gui` harnesses, unit tests) require zero changes until Wave 3 migrates `SessionHost` trait callers.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 441-02-SUMMARY.md

---

### Tracked truncated_front
Tracked `truncated_front: bool` on `Replay` so if a single `OutputFrame` exceeds `replay_cap` and retains only its trailing `replay_cap` bytes, a client attaching with `after_seq == seq - 1` receives `resync_required: true` rather than mistaking the truncated tail for a gap-free delta.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 441-02-SUMMARY.md

---

### Defaulted omitted or null "signal" JSON-RPC parameter in term.signal to ...
Defaulted omitted or `null` `"signal"` JSON-RPC parameter in `term.signal` to `TermSignal::Interrupt` while rejecting malformed non-null variants via `serde_json::from_value`.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 441-03-SUMMARY.md

---

### Verified DaemonHost::signal(..., TermSignal::Interrupt) over loopback TC...
Verified `DaemonHost::signal(..., TermSignal::Interrupt)` over loopback TCP using a `(printf ...; exec sleep 30)` foreground subshell so `fork`/`setpgid`/`tcsetpgrp` and `SIG_DFL` disposition are deterministically established before readiness output arrives.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 441-03-SUMMARY.md

---

### 120×32 defaults, after_seq delta replay & live alt_screen/cwd_changed st...
**`120×32` defaults, `after_seq` delta replay & live `alt_screen`/`cwd_changed` streaming (`TERM-PTY-04`):** Updated `TermDispatch::try_handle` (`term.open` defaulting `cols`/`rows` to `120×32` and `term.list` exposing `alt_screen`, `cwd`, and `git_branch`), upgraded `TermDispatch::try_stream` (`term.attach`) to read optional `after_seq`, invoke `LocalHost::attach_since(&session, after_seq)`, and emit `resync_required`, `alt_screen`, `cwd`, and `git_branch` on `"snapshot"` frames, and extended `session_event_json` with exhaustive `SessionEvent::AltScreenChanged` (`"type": "alt_screen"`) and `SessionEvent::CwdChanged` (`"type": "cwd_changed"`) arms.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 441-03-SUMMARY.md

---

### Origin-gated term.signal RPC & DaemonHost loopback TCP verification (TER...
**Origin-gated `term.signal` RPC & `DaemonHost` loopback TCP verification (`TERM-PTY-04`, `T-441-04`):** Wired `vox_terminal_core::daemon_host::SIGNAL` (`"term.signal"`) in `TermDispatch::try_handle` gated fail-closed behind `refuse_agent_origin(p)?` before calling `LocalHost::signal(sid, sig)`, pinned `daemon_host::SIGNAL == "term.signal"` in protocol constant assertions, verified `"origin": "agent"` rejection across `"interrupt"` and `"kill"` payloads, and proved `DaemonHost::signal(..., TermSignal::Interrupt)` interrupts a live `sleep 30` foreground job over loopback TCP while preserving the interactive shell for subsequent commands.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 441-03-SUMMARY.md

---

### Defaulted omitted or null "signal" parameters on term.signal to TermSign...
Defaulted omitted or `null` `"signal"` parameters on `term.signal` to `TermSignal::Interrupt` while strictly validating explicit values via `serde_json::from_value::<TermSignal>` so invalid signal names fail fast rather than silently sending `SIGINT`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 441-03-SUMMARY.md

---

### Structured term_signal_interrupts_running_job_over_daemon_host to launch...
Structured `term_signal_interrupts_running_job_over_daemon_host` to launch `(printf '__sleep_%s__\n' 'ready'; exec sleep 30)` in a foreground subshell so `fork`, `setpgid`, `tcsetpgrp`, and `SIG_DFL` disposition are already active when `__sleep_ready__` arrives on the attached PTY stream.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 441-03-SUMMARY.md

---

### Kept Cli::Term as a struct variant with --attach [SESSION_ID] and --loca...
Kept `Cli::Term` as a struct variant with `--attach [SESSION_ID]` and `--local` flags (rather than nested subcommands) so `command_catalog_paths_baseline.txt` remains byte-identical on `["term"]`.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 441-04-SUMMARY.md

---

### Synced BlockOpened and BlockClosed metadata in vox_term::app::apply_sess...
Synced `BlockOpened` and `BlockClosed` metadata in `vox_term::app::apply_session_event` via lightweight `host.attach(sid, Some(last_seq)).await` delta attaches while mutating `OutputAppended` chunks in-place on `blocks_state` so multi-client peers see identical commands, exit codes, and streaming output without quadratic full-replay fetches.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 441-04-SUMMARY.md

---

### Added term_attachment_dto_from_attachment alongside backward-compatible ...
Added `term_attachment_dto_from_attachment` alongside backward-compatible `term_attachment_dto_from_parts` in `vox-gui` so existing call sites stay valid while `term_session_attach` surfaces `resync_required`, `alt_screen`, `cwd`, and `git_branch`.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 441-04-SUMMARY.md

---

### Live SessionHost TUI loop & VtGrid hardening in vox-term (TERM-PTY-04)
**Live `SessionHost` TUI loop & `VtGrid` hardening in `vox-term` (`TERM-PTY-04`):** Clamped `VtGrid::new` and `VtGrid::resize` with `cols.max(1)` / `rows.max(1)` (`T-441-02`) and added `is_alt_screen()` + `reset()` in `crates/vox-term/src/vt.rs`. Replaced the disconnected `Session::new("main")` stub in `crates/vox-term/src/app.rs` with `attach_or_open_session`, numeric-suffix `pick_latest_session_id` (`"term-10"` > `"term-9"`), bare `--attach` (`Some("latest")`) parsing, `apply_session_event` peer block sync, `ALT_SCREEN` raw VT rendering and `key_event_to_vt_bytes` pass-through, `Event::Resize` forwarding, `Ctrl-C` `TermSignal::Interrupt` (`T-441-04`), clean detach without `host.close`, and `TryRecvError::Lagged` `after_seq` delta recovery (`T-441-03`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 441-04-SUMMARY.md

---

### Async vox term CLI flags & nested-runtime elimination in vox-cli (TERM-P...
**Async `vox term` CLI flags & nested-runtime elimination in `vox-cli` (`TERM-PTY-04`):** Upgraded `Cli::Term` in `crates/vox-cli/src/lib.rs` with `--attach [SESSION_ID]` (`num_args = 0..=1`, `default_missing_value = "latest"`) and `--local`, updated `crates/vox-cli/src/cli_dispatch/mod.rs` to `.await` `vox_term::app::run_with_args(...)` directly on `vox-cli`'s active Tokio runtime instead of calling `vox_term::app::run()` (`Runtime::new()`), and kept `command_catalog_paths_baseline` green.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 441-04-SUMMARY.md

---

### Lossless UTF-8 streaming, after_seq delta attach & lag recovery in vox-g...
**Lossless UTF-8 streaming, `after_seq` delta attach & lag recovery in `vox-gui` (`TERM-PTY-04`):** Extended `TermAttachmentDto` in `crates/vox-gui/src/commands/pty.rs` with `resync_required`, `alt_screen`, `cwd`, and `git_branch`, added `term_attachment_dto_from_attachment`, routed `pty_spawn` and `term_session_attach` output through `Utf8StreamDecoder` (`push` + `flush_lossy`), added optional `after_seq: Option<u64>` to `term_session_attach`, and recovered `broadcast::error::RecvError::Lagged` via `host.attach(&sid, Some(last_seq)).await` (`T-441-03`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 441-04-SUMMARY.md

---

### Kept Cli::Term as a flag-bearing top-level command rather than introduci...
Kept `Cli::Term` as a flag-bearing top-level command rather than introducing nested subcommands (`vox term attach`) so `crates/vox-cli/tests/fixtures/command_catalog_paths_baseline.txt` required zero changes.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 441-04-SUMMARY.md

---

### Used Option<u64> without #[serde(default)] on the term_session_attach Ta...
Used `Option<u64>` without `#[serde(default)]` on the `term_session_attach` Tauri command argument so Tauri v2's IPC deserializer natively accepts existing frontend `{ sessionId }` payloads (`after_seq == None`) as well as `{ sessionId, afterSeq }` reconnect calls.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 441-04-SUMMARY.md

## Lessons

### Synthetic .git/HEAD test fixture in stateful_cd_and_export_persist_acros...
Synthetic `.git/HEAD` test fixture in `stateful_cd_and_export_persist_across_submissions_and_update_cwd` required `.git/objects` and `.git/refs/heads` directories for `git symbolic-ref --quiet --short HEAD` to recognize the temp directory as a Git repo; added both directories to the test fixture.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 441-01-SUMMARY.md

---

### Shared workspace Cargo build directory lock contention from concurrent w...
Shared workspace Cargo build directory lock contention from concurrent workstream builds intermittently queued `cargo test` invocations; all 13 test targets in `vox-terminal-core` (`97/97` tests) compiled cleanly with zero warnings and passed.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 441-01-SUMMARY.md

---

### Upstream crates/vox-corpus/build.rs watches ../vox-mcp/src/tools/ (which...
Upstream `crates/vox-corpus/build.rs` watches `../vox-mcp/src/tools/` (which triggers Cargo rebuilds of `vox-corpus` → `vox-populi` → `vox-orchestrator` on `cargo test -p vox-terminal-core` runs under heavy multi-agent CPU contention). Once `cargo test -p vox-terminal-core` finished linking all 14 `test` profile targets cleanly (`0` warnings), both `cargo test -p vox-terminal-core --test streaming_utf8_and_alt_screen` (`8/8` in `0.01s`) and all 14 compiled `vox-terminal-core` test binaries (`106/106` tests) executed to completion and passed.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 441-02-SUMMARY.md

---

### Concurrent Wave 3 Cargo builds contended on the workspace target/ build ...
Concurrent Wave 3 Cargo builds contended on the workspace `target/` build directory lock and `vox-corpus` `build.rs` rebuilds during initial compilation; once the build lock cleared, the `vox-orchestrator-d` binary test target compiled with zero warnings and executed all 11 `term_dispatch` tests in `1.63s`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 441-03-SUMMARY.md

---

### Initial printf ..
Initial `printf ... && sleep 30` readiness probe in `term_signal_interrupts_running_job_over_daemon_host` flushed shell-builtin `printf` output before interactive `zsh` forked `sleep 30` into its foreground process group; wrapping the probe in `(printf ...; exec sleep 30)` eliminated the process-group setup race cleanly.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 441-03-SUMMARY.md

---

### Cold incremental compilation of vox-orchestrator-mcp (features = ["news-...
Cold incremental compilation of `vox-orchestrator-mcp` (`features = ["news-publish", "gui-visual-review", "heavy-browser"]`) under concurrent multi-agent CPU load (`load avg ~22`) combined with `crates/vox-corpus/build.rs`'s `cargo:rerun-if-changed=../vox-mcp/src/tools/` invalidated downstream crates on initial `vox-gui` build. Warming `vox-orchestrator-mcp` once populated `target/debug/incremental/`, after which `vox-gui` unit (`2m 46s`), `vox-gui` `real_backend_harness` (`2m 50s`), `vox-cli` (`2m 33s`), and `vox-term` (`1m 15s`) test builds all completed cleanly under `300s`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 441-04-SUMMARY.md

---

### No gaps found
No gaps found. All 4 ROADMAP Success Criteria, 18 plan must-have truths, 15 required artifacts, 11 key links, and requirements `TERM-PTY-01` through `TERM-PTY-04` are implemented, wired end-to-end, and behaviorally verified against live tests.

**Context:** Verified in phase acceptance checks.
**Source:** 441-VERIFICATION.md

## Patterns

### Shell bootstrap isolation
Shell bootstrap isolation: write per-session init files into a `0700` `TempDir` owned by `PtyHandle` so hook code never echoes onto the TTY or enters shell history, and unlinks automatically on drop.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 441-01-SUMMARY.md

---

### Explicit MutexGuard extraction before if let blocks in edition = "2021" ...
Explicit `MutexGuard` extraction before `if let` blocks in `edition = "2021"` crates whenever the body re-locks the same `std::sync::Mutex` (e.g. `Condvar::wait_timeout`).

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 441-01-SUMMARY.md

---

### Owner-only 0700 tempfile::TempDir shell launch shims (ZDOTDIR, --rcfile,...
Owner-only `0700` `tempfile::TempDir` shell launch shims (`ZDOTDIR`, `--rcfile`, `--init-command`, `-File`, `--config`) instead of PTY stdin snippet injection

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 441-01-SUMMARY.md

---

### Dedicated vox-pty-reader OS thread pumping directly into Entry::feed + s...
Dedicated `vox-pty-reader` OS thread pumping directly into `Entry::feed` + `std::sync::Condvar` fast-path settle without `tokio::spawn` runtime starvation

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 441-01-SUMMARY.md

---

### Poison-tolerant std::sync::Mutex locking with explicit pre-if let guard ...
Poison-tolerant `std::sync::Mutex` locking with explicit pre-`if let` guard drops across Rust 2021 temporary scopes

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 441-01-SUMMARY.md

---

### Column-indexed ANSI cell line buffer in normalize_cr_overwrites so SGR c...
Column-indexed ANSI cell line buffer in `normalize_cr_overwrites` so SGR color prefixes attach to cells without advancing column indices during bare `\r` spinner overwrites or `CSI K` line erases.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 441-02-SUMMARY.md

---

### Monotonic seq-indexed VecDeque<OutputFrame> ring setting resync_required
Monotonic `seq`-indexed `VecDeque<OutputFrame>` ring setting `resync_required: false` only when `after_seq` is contiguous with retained non-truncated frames, falling back to full retained window (`resync_required: true`) on initial attach or ring eviction.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 441-02-SUMMARY.md

---

### std::str::from_utf8 valid_up_to() + error_len() 3-byte trailing UTF-8 st...
`std::str::from_utf8` `valid_up_to()` + `error_len()` 3-byte trailing UTF-8 stream decoder (`Utf8StreamDecoder`) eliminating `String::from_utf8_lossy` split-chunk `U+FFFD` corruption

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 441-02-SUMMARY.md

---

### Zero-scrollback alacritty_terminal::Term shadow grid (VtShadow) bridged ...
Zero-scrollback `alacritty_terminal::Term` shadow grid (`VtShadow`) bridged with a `vte::ansi::Handler` for legacy `?47`/`?1047` DECSET/DECRST modes and causal `ESC`-segmented transition tracking

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 441-02-SUMMARY.md

---

### 2 MiB VecDeque<OutputFrame> after_seq delta replay ring with truncated_f...
`2 MiB` `VecDeque<OutputFrame>` `after_seq` delta replay ring with `truncated_front` guard and bounded `DaemonHost` `lagged` stream auto-resubscription (`MAX_LAG_RESUBSCRIBE_ATTEMPTS = 8`)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 441-02-SUMMARY.md

---

### Shared host_scenario(&dyn SessionHost) parity contract exercising both i...
Shared `host_scenario(&dyn SessionHost)` parity contract exercising both in-process `LocalHost` and loopback TCP `DaemonHost` identically.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 441-03-SUMMARY.md

---

### Subshell (printf readiness; exec long_cmd) synchronization in live PTY s...
Subshell `(printf readiness; exec long_cmd)` synchronization in live PTY signal tests to avoid racing builtin stdout against child process-group placement.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 441-03-SUMMARY.md

---

### after_seq-cursor delta snapshot framing on term.attach (LocalHost::attac...
`after_seq`-cursor delta snapshot framing on `term.attach` (`LocalHost::attach_since(&session, after_seq)`) eliminating redundant `2 MiB` replay transfers across loopback TCP

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 441-03-SUMMARY.md

---

### Fail-closed wire origin guard (refuse_agent_origin(p)?) enforced across ...
Fail-closed wire origin guard (`refuse_agent_origin(p)?`) enforced across `term.input`, `term.submit`, and `term.signal` so unapproved agent origins cannot spoof interactive PTY signals or input

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 441-03-SUMMARY.md

---

### Numeric suffix ordering (id.rsplit_once('-') u64 sort with lexicographic...
Numeric suffix ordering (`id.rsplit_once('-')` `u64` sort with lexicographic tie-breaker) in `pick_latest_session_id` so `term-10` beats `term-9` on bare `--attach`.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 441-04-SUMMARY.md

---

### Resilient Tauri SessionHost forwarder pattern
Resilient Tauri `SessionHost` forwarder pattern: retain `Arc<dyn SessionHost>` + `last_seq` + `Utf8StreamDecoder` in the forwarder task, recover `RecvError::Lagged` via `host.attach(&sid, Some(last_seq))`, and flush trailing UTF-8 bytes on `RecvError::Closed`.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 441-04-SUMMARY.md

---

### Shared SessionHost TUI event loop with ALT_SCREEN raw VT byte pass-throu...
Shared `SessionHost` TUI event loop with `ALT_SCREEN` raw VT byte pass-through (`key_event_to_vt_bytes`) vs structured block line composer, plus `after_seq` delta resync on broadcast lag

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 441-04-SUMMARY.md

---

### Clap num_args = 0..=1 + default_missing_value = "latest" optional-value ...
Clap `num_args = 0..=1` + `default_missing_value = "latest"` optional-value flag on top-level `Cli::Term` preserving baseline catalog path `["term"]` while sharing `vox-cli`'s active Tokio runtime

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 441-04-SUMMARY.md

---

### Tauri term_session_attach resilient broadcast forwarder using Utf8Stream...
Tauri `term_session_attach` resilient broadcast forwarder using `Utf8StreamDecoder` across split multi-byte UTF-8 chunks and `host.attach(&sid, Some(last_seq)).await` lag recovery (`T-441-03`)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 441-04-SUMMARY.md

## Surprises

### Estimate variance on 441-03
Estimate variance on 441-03: planned 26000 tokens vs 8529 actual (0.33x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 441-03-PLAN.md, 441-03-SUMMARY.md

---

### Estimate variance on 441-04
Estimate variance on 441-04: planned 44000 tokens vs 11275 actual (0.26x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 441-04-PLAN.md, 441-04-SUMMARY.md

