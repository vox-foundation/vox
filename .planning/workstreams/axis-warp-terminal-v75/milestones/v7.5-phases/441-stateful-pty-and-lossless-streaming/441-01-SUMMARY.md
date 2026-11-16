---
phase: 441-stateful-pty-and-lossless-streaming
plan: 01
subsystem: terminal
tags: [pty, osc633, osc7, zsh, bash, fish, pwsh, nushell, portable-pty, alacritty-terminal, tempfile]

requires: []
provides:
  - "Non-echo 5-shell (`zsh`, `bash`, `fish`, `pwsh`, `nu`) `tempfile::TempDir` (`0700`) launch shims in `vox-terminal-core::shell_init`"
  - "`PtyHandle` with retained `MasterPty::resize` (`cols.max(1)` / `rows.max(1)` clamping), `TermSignal` (`\x03` ETX interrupt & kill), RAII `TempDir` drop cleanup, and `spawn_pty_with_callback`"
  - "Bounded `Osc633Parser` (`16 KiB` incomplete-OSC cap, 3-byte UTF-8 tail reassembly, `PromptEnd`, `Property`, and percent-decoded `Osc7Cwd`)"
  - "Stateful PTY `Session` + `LocalHost` OS-thread pump with `Condvar` settle, persistent `cd`/`export`/interactive `Running` stdin forwarding, `SessionEvent::CwdChanged`, and `120×32` `OpenSpec` default"
affects: [441-02-PLAN, vox-term, vox-orchestrator-d, vox-cli, vox-gui]

actuals:
  tokens: 30632
  tasks: 2
  commits: 0

tech-stack:
  added:
    - "tempfile 3.27.0 (workspace dependency wired into vox-terminal-core)"
    - "alacritty_terminal 0.26.0 (workspace dependency wired into vox-terminal-core)"
  patterns:
    - "Owner-only `0700` `tempfile::TempDir` shell launch shims (`ZDOTDIR`, `--rcfile`, `--init-command`, `-File`, `--config`) instead of PTY stdin snippet injection"
    - "Dedicated `vox-pty-reader` OS thread pumping directly into `Entry::feed` + `std::sync::Condvar` fast-path settle without `tokio::spawn` runtime starvation"
    - "Poison-tolerant `std::sync::Mutex` locking with explicit pre-`if let` guard drops across Rust 2021 temporary scopes"

key-files:
  created:
    - crates/vox-terminal-core/src/shell_init.rs
    - crates/vox-terminal-core/tests/shell_init_and_stateful_pty.rs
  modified:
    - crates/vox-terminal-core/Cargo.toml
    - Cargo.lock
    - crates/vox-terminal-core/src/pty.rs
    - crates/vox-terminal-core/src/lib.rs
    - crates/vox-terminal-core/src/osc633.rs
    - crates/vox-terminal-core/src/session.rs
    - crates/vox-terminal-core/src/host.rs

key-decisions:
  - "Replaced raw PTY stdin snippet writes in `spawn_pty` with `shell_init::prepare_shell_command` `TempDir` (`0700`) launch shims while preserving `shell_integration_snippet` and its constants for backwards compatibility."
  - "Pinned `__vox_precmd` to index 1 of `precmd_functions` in zsh (`local ec=$?` on line 1) plus a tail `__vox_prompt_wrap` hook and `EXIT` traps in zsh/bash so user `.zshrc`/`.bashrc` themes cannot clobber non-zero `$?` or `exit N` codes."
  - "Bounded `LocalHost::submit_with_origin` synchronous `Condvar` settle at `1500ms` for new commands and `500ms` when forwarding stdin into an active `Running` block so fast commands complete synchronously while long-running/interactive jobs return `Running`."

patterns-established:
  - "Shell bootstrap isolation: write per-session init files into a `0700` `TempDir` owned by `PtyHandle` so hook code never echoes onto the TTY or enters shell history, and unlinks automatically on drop."
  - "Explicit `MutexGuard` extraction before `if let` blocks in `edition = \"2021\"` crates whenever the body re-locks the same `std::sync::Mutex` (e.g. `Condvar::wait_timeout`)."

requirements-completed: [TERM-PTY-01, TERM-PTY-02]

coverage:
  - id: D1
    description: "5-shell (`zsh`, `bash`, `fish`, `pwsh`, `nu`) `0700` `TempDir` non-echo launch shims with user dotfile sourcing, quote escaping, Nushell static `source` splicing, and RAII cleanup on `PtyHandle` drop"
    requirement: TERM-PTY-01
    verification:
      - kind: unit
        ref: "crates/vox-terminal-core/src/shell_init.rs#five_shells_generate_tempdir_shims_and_unknown_shells_fallback"
        status: pass
      - kind: unit
        ref: "crates/vox-terminal-core/src/shell_init.rs#nu_shim_splices_static_source_only_when_user_config_exists"
        status: pass
      - kind: integration
        ref: "crates/vox-terminal-core/src/shell_init.rs#live_pty_tracer_zero_echo_resize_and_raii_cleanup"
        status: pass
    human_judgment: false
  - id: D2
    description: "Zero echoed `OSC 633` startup spam, empty initial `blocks`, macOS `unsetopt PROMPT_SP`, and accurate non-zero exit codes (`false` -> `1`, `exit 7` -> `7`) across live `zsh` and `bash` PTY sessions"
    requirement: TERM-PTY-01
    verification:
      - kind: integration
        ref: "crates/vox-terminal-core/tests/shell_init_and_stateful_pty.rs#startup_zero_echo_spam_empty_initial_blocks_and_initial_cwd"
        status: pass
      - kind: integration
        ref: "crates/vox-terminal-core/tests/shell_init_and_stateful_pty.rs#nonzero_exit_codes_false_and_top_level_exit_7"
        status: pass
    human_judgment: false
  - id: D3
    description: "Live `OSC 7` (`file://`), `OSC 633;P;Cwd`, and `OSC 633;P;GitBranch` tracking with `SessionEvent::CwdChanged` and `16 KiB` incomplete-OSC DoS cap"
    requirement: TERM-PTY-01
    verification:
      - kind: integration
        ref: "crates/vox-terminal-core/tests/shell_init_and_stateful_pty.rs#five_shell_shims_openspec_defaults_and_osc_parser_bounds"
        status: pass
      - kind: integration
        ref: "crates/vox-terminal-core/tests/shell_init_and_stateful_pty.rs#stateful_cd_and_export_persist_across_submissions_and_update_cwd"
        status: pass
    human_judgment: false
  - id: D4
    description: "Stateful persistent PTY command execution in `LocalHost::submit` preserving `cd`, exported env vars, and interactive `Running` block stdin forwarding across submissions"
    requirement: TERM-PTY-02
    verification:
      - kind: integration
        ref: "crates/vox-terminal-core/tests/shell_init_and_stateful_pty.rs#stateful_cd_and_export_persist_across_submissions_and_update_cwd"
        status: pass
      - kind: integration
        ref: "crates/vox-terminal-core/tests/shell_init_and_stateful_pty.rs#interactive_stdin_forwards_into_active_running_block"
        status: pass
    human_judgment: false
  - id: D5
    description: "Real kernel `MasterPty::resize` (`stty size` `43 132` + `0×0` clamping) and `\x03` (`ETX`) `TermSignal::Interrupt` foreground job cancellation keeping shell alive"
    requirement: TERM-PTY-02
    verification:
      - kind: integration
        ref: "crates/vox-terminal-core/tests/shell_init_and_stateful_pty.rs#live_kernel_resize_updates_stty_size_and_sigint_interrupts_foreground_job"
        status: pass
    human_judgment: false

duration: 85min
completed: 2026-11-14
status: complete
---

# Phase 441 Plan 01: Stateful PTY & Non-Echo Shell Bootstrap Summary

**5-shell (`zsh`, `bash`, `fish`, `pwsh`, `nu`) `0700` `TempDir` non-echo launch shims, real `MasterPty::resize` + `\x03` `TermSignal::Interrupt`, bounded `OSC 633`/`OSC 7` streaming parser, and persistent stateful `PtyHandle` execution in `vox-terminal-core`**

## Performance

- **Duration:** 85 min
- **Started:** 2026-11-14T07:01:00Z
- **Completed:** 2026-11-14T08:26:00Z
- **Tasks:** 2
- **Files modified:** 9 (2 created, 7 modified)

## Accomplishments

- **Non-echo 5-shell `TempDir` bootstrap (`TERM-PTY-01`):** Created `crates/vox-terminal-core/src/shell_init.rs` with owner-only `0700` `tempfile::TempDir` launch shims for `zsh` (`ZDOTDIR` chain restoring user `ZDOTDIR`, `unsetopt PROMPT_SP`, `%{...%}` prompt wrapping, `precmd_functions` index-1 `$?` capture, `EXIT` trap), `bash` (`--rcfile` chaining string/array `PROMPT_COMMAND` before `__VOX_AT_PROMPT=1` `DEBUG` trap + `EXIT` trap), `fish` (`--init-command` with single-quote escaping), `pwsh` (`-NoLogo -NoExit -File` guarding `PSReadLine`), and `nu` (Rust-side `Path::is_file()` probe splicing a literal `source '...'` line before `$env.config` hooks).
- **Live `MasterPty::resize`, `TermSignal`, and RAII cleanup (`TERM-PTY-01`, `TERM-PTY-02`):** Upgraded `PtyHandle` in `crates/vox-terminal-core/src/pty.rs` to retain `Arc<Mutex<Box<dyn MasterPty + Send>>>` and `Option<TempDir>`, implementing real `MasterPty::resize` (`cols.max(1)`, `rows.max(1)`), `get_size`, `TermSignal` (`Interrupt` -> `\x03` ETX, `Terminate | Kill` -> `child.kill()`), `try_wait_exit_code`, and `spawn_pty_with_callback` on a dedicated `vox-pty-reader` OS thread.
- **Bounded `OSC 633` + `OSC 7` streaming parser (`TERM-PTY-01`):** Extended `crates/vox-terminal-core/src/osc633.rs` with `Osc633Event::PromptEnd` (`633;B`), `Osc633Event::Property { key, value }` (`633;P;Cwd` / `633;P;GitBranch`), percent-decoded `Osc633Event::Osc7Cwd` (`OSC 7` `file://`), 3-byte UTF-8 split-chunk tail reassembly, and a `16 KiB` (`MAX_INCOMPLETE_OSC_BYTES`) incomplete-OSC flush guard (`T-441-03`).
- **Stateful PTY execution & `Condvar` pump (`TERM-PTY-02`):** Upgraded `Session` (`crates/vox-terminal-core/src/session.rs`) and `LocalHost` (`crates/vox-terminal-core/src/host.rs`) to `120×32` `OpenSpec::default()`, eliminated idle-prompt phantom blocks on `633;A`, pumped PTY bytes directly from `vox-pty-reader` into `Entry::feed` + `settled_cv.notify_all()`, and routed shell submissions through the persistent `PtyHandle` so `cd`, `export`, and interactive `Running` stdin (`read`) persist across submissions.

## Task Commits

- `commits: skipped (shared git index protected per 00-core.md Rule 6 — 2,520 staged files in concurrent workspace index left untouched for orchestrator commit)`

## Verification & TDD Proof

- **Task 1 RED → GREEN:**
  - RED: `timeout 300s cargo test -p vox-terminal-core --lib shell_init` failed against initial stub (`0 passed; 5 failed`).
  - GREEN: `timeout 300s cargo test -p vox-terminal-core` exited `0` (`48 passed; 0 failed` in lib unit tests + all existing integration suites passing).
- **Task 2 RED → GREEN:**
  - RED: `timeout 300s cargo test -p vox-terminal-core --test shell_init_and_stateful_pty` against stateless `LocalHost` baseline failed as expected (`1 passed; 5 failed` — `OpenSpec` `80 != 120`, `session_cwd == None`, `read` `Failed != Running`, `stty: stdin isn't a terminal`, stateless `cd` non-persistence).
  - GREEN:
    - `timeout 300s cargo test -p vox-terminal-core --test shell_init_and_stateful_pty` → **exit `0` (`6 passed; 0 failed` in `2.38s`, zero compiler warnings)**.
    - All 13 `vox-terminal-core` test targets verified (`97 passed; 0 failed` total: `lib` 48/48, `agent` 3/3, `block` 5/5, `commands` 2/2, `host` 1/1, `input` 10/10, `osc633` 5/5, `pty` 5/5, `session` 7/7, `shell_init_and_stateful_pty` 6/6, `smoke` 1/1, `transcript` 2/2, `vox_interp` 2/2).
- **Formatting & Stub/Threat Scan:**
  - Ran `rustfmt --edition 2024` on all 7 modified/created `.rs` files (exit `0`).
  - `rg -n "TODO|FIXME|XXX|HACK|unimplemented!|todo!"` across all modified/created `.rs` files returned `0` matches.
  - Threats `T-441-01` (`0700` `TempDir` + quote escaping + RAII drop), `T-441-02` (`cols.max(1)` / `rows.max(1)` clamping + `120×32` defaults), and `T-441-03` (`16 KiB` incomplete OSC buffer cap) implemented and covered by automated assertions.

## Files Created/Modified

- `crates/vox-terminal-core/Cargo.toml` - Wired workspace `alacritty_terminal` and `tempfile` dependencies
- `Cargo.lock` - Recorded `vox-terminal-core` dependency edges to workspace-pinned `alacritty_terminal` and `tempfile`
- `crates/vox-terminal-core/src/shell_init.rs` - 5-shell (`zsh`, `bash`, `fish`, `pwsh`, `nu`) `0700` `TempDir` non-echo launch shim builder and unit/tracer tests
- `crates/vox-terminal-core/src/pty.rs` - `TermSignal`, retained `MasterPty::resize`/`get_size`, RAII `TempDir`, and `spawn_pty_with_callback`
- `crates/vox-terminal-core/src/lib.rs` - Exported `pub mod shell_init` and re-exported `TermSignal`, `spawn_pty_with_callback`, `ShellLaunchShim`, and `prepare_shell_command`
- `crates/vox-terminal-core/src/osc633.rs` - `PromptEnd`, `Property`, `Osc7Cwd` percent-decoder, UTF-8 split-byte tail buffer, and `16 KiB` incomplete-OSC cap
- `crates/vox-terminal-core/src/session.rs` - `SessionEvent::CwdChanged`, `cwd`/`git_branch` tracking, `begin_pty_submission`, `finish_open_block_on_eof`, and non-phantom `PromptStart` handling
- `crates/vox-terminal-core/src/host.rs` - `120×32` `OpenSpec::default()`, OS-thread `Entry::feed` + `Condvar` settle, stateful PTY `submit_with_origin`, `signal`, `session_cwd`, and `session_git_branch`
- `crates/vox-terminal-core/tests/shell_init_and_stateful_pty.rs` - 6-test integration suite verifying 5-shell shims, zero startup spam, `false`/`exit 7` exit codes, `cd`/`export` + `GitBranch` persistence, interactive `read` stdin forwarding, `resize` (`stty size`), and `TermSignal::Interrupt`

## Decisions Made

- Kept `shell_integration_snippet` and all 5 `*_OSC633` string constants in `pty.rs` intact while migrating `spawn_pty` / `spawn_pty_with_callback` onto `shell_init::prepare_shell_command` so existing `tests/pty.rs` and `vox-gui` callers continue compiling without breakage ahead of Plan `441-02`.
- Waited up to `2500ms` via `tokio::task::spawn_blocking` on `LocalHost::open` for `session.is_at_prompt()` (`633;B`) on `osc633_capable` shells (waking immediately in ~300ms once the prompt renders) so initial `CwdChanged` and prompt hooks settle cleanly before `attach` or `submit`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Prevented Rust 2021 `if let` temporary `MutexGuard` self-deadlock on active-block stdin forwarding**
- **Found during:** Task 2 (`interactive_stdin_forwards_into_active_running_block`)
- **Issue:** In Rust 2021 edition, `if let Some(running_id) = lock(&entry.session).open_block_id() { ... }` held the temporary `MutexGuard<'_, Session>` across `entry.wait_until(...)`, self-deadlocking `entry.session` when forwarding stdin into an active `Running` block.
- **Fix:** Bound `let active_running_id = lock(&entry.session).open_block_id();` (and audited `pump_eof` and `close` for the same pattern) so the guard drops before `pty.write(...)` and `entry.wait_until(...)`.
- **Files modified:** `crates/vox-terminal-core/src/host.rs`
- **Verification:** `interactive_stdin_forwards_into_active_running_block` passed deterministically in `<2s`.

**2. [Rule 1 - Bug] Preserved Bash `$?` across `DEBUG` trap before `PROMPT_COMMAND` and `EXIT` trap**
- **Found during:** Task 2 (`nonzero_exit_codes_false_and_top_level_exit_7`)
- **Issue:** Bash runs `trap '__vox_preexec' DEBUG` before `PROMPT_COMMAND` and `EXIT` traps; returning `0` from `__vox_preexec` when `__VOX_AT_PROMPT` is unset overwrote non-zero `$?` unless `$?` was saved and returned (`return $__vox_ec`).
- **Fix:** Captured `local __vox_ec=$?` on line 1 of `__vox_preexec` in `BASH_SHIM_RC` and returned `$__vox_ec` whenever `extdebug` is off.
- **Files modified:** `crates/vox-terminal-core/src/shell_init.rs`
- **Verification:** Both `false` (`exit_code == Some(1)`) and `exit 7` (`exit_code == Some(7)`) passed on live `/bin/zsh` and `bash` PTYs.

---

**Total deviations:** 2 auto-fixed (2 Rule 1 bugs)
**Impact on plan:** Both fixes were directly required for `TERM-PTY-01` / `TERM-PTY-02` correctness under live `zsh` and `bash` PTY execution. Zero scope creep.

## Issues Encountered

- Synthetic `.git/HEAD` test fixture in `stateful_cd_and_export_persist_across_submissions_and_update_cwd` required `.git/objects` and `.git/refs/heads` directories for `git symbolic-ref --quiet --short HEAD` to recognize the temp directory as a Git repo; added both directories to the test fixture.
- Shared workspace Cargo build directory lock contention from concurrent workstream builds intermittently queued `cargo test` invocations; all 13 test targets in `vox-terminal-core` (`97/97` tests) compiled cleanly with zero warnings and passed.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `vox-terminal-core` exports `TermSignal`, `ShellLaunchShim`, `prepare_shell_command`, `spawn_pty_with_callback`, `SessionEvent::CwdChanged`, `LocalHost::signal`, `LocalHost::session_cwd`, and `LocalHost::session_git_branch`, ready for Plan `441-02` (`TERM-PTY-03`, `TERM-PTY-04`, `TERM-TUI-02`) to wire lossless base64 byte-frame streaming and resize/signal RPCs across `vox-orchestrator-d`, `vox-term`, `vox-cli`, and `vox-gui`.

## Self-Check: PASSED

- Verified on disk:
  - `crates/vox-terminal-core/Cargo.toml` — FOUND
  - `Cargo.lock` — FOUND
  - `crates/vox-terminal-core/src/shell_init.rs` — FOUND (923 lines)
  - `crates/vox-terminal-core/src/pty.rs` — FOUND
  - `crates/vox-terminal-core/src/lib.rs` — FOUND
  - `crates/vox-terminal-core/src/osc633.rs` — FOUND
  - `crates/vox-terminal-core/src/session.rs` — FOUND
  - `crates/vox-terminal-core/src/host.rs` — FOUND
  - `crates/vox-terminal-core/tests/shell_init_and_stateful_pty.rs` — FOUND (396 lines)
  - `.planning/workstreams/axis-warp-terminal-v75/phases/441-stateful-pty-and-lossless-streaming/441-01-SUMMARY.md` — FOUND
- Verified tests: 97/97 tests across all `vox-terminal-core` lib and integration test suites passed (`0` failed, `0` warnings).

---
*Phase: 441-stateful-pty-and-lossless-streaming*
*Completed: 2026-11-14*
