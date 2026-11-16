---
phase: 441-stateful-pty-and-lossless-streaming
plan: 03
subsystem: terminal
tags: [json-rpc, term-dispatch, daemon-host, after-seq, alt-screen, sigint, pty]

requires:
  - phase: 441-02
    provides: "LocalHost::attach_since, widened SessionHost::attach(id, after_seq) + SessionHost::signal(id, sig), SessionEvent::{AltScreenChanged, CwdChanged}, and DaemonHost term.signal + lag recovery"
provides:
  - "`TermDispatch` `term.open` `120×32` fallback dimensions matching `OpenSpec::default()`"
  - "`TermDispatch` `term.list` and `term.attach` snapshot metadata (`resync_required`, `alt_screen`, `cwd`, `git_branch`) plus `after_seq` delta replay via `LocalHost::attach_since`"
  - "`session_event_json` live streaming frames for `SessionEvent::AltScreenChanged` (`\"type\": \"alt_screen\"`) and `SessionEvent::CwdChanged` (`\"type\": \"cwd_changed\"`)"
  - "Origin-gated `\"term.signal\"` (`vox_terminal_core::daemon_host::SIGNAL`) JSON-RPC handler enforcing `refuse_agent_origin` fail-closed before forwarding `TermSignal` to `LocalHost::signal`"
affects: [441-04-PLAN, 441-05-PLAN, vox-orchestrator-d, vox-term, vox-cli, vox-gui]

actuals:
  tokens: 8529
  tasks: 2
  commits: 0

tech-stack:
  added: []
  patterns:
    - "`after_seq`-cursor delta snapshot framing on `term.attach` (`LocalHost::attach_since(&session, after_seq)`) eliminating redundant `2 MiB` replay transfers across loopback TCP"
    - "Fail-closed wire origin guard (`refuse_agent_origin(p)?`) enforced across `term.input`, `term.submit`, and `term.signal` so unapproved agent origins cannot spoof interactive PTY signals or input"

key-files:
  created: []
  modified:
    - crates/vox-orchestrator-d/src/term_dispatch.rs

key-decisions:
  - "Defaulted omitted or `null` `\"signal\"` JSON-RPC parameter in `term.signal` to `TermSignal::Interrupt` while rejecting malformed non-null variants via `serde_json::from_value`."
  - "Verified `DaemonHost::signal(..., TermSignal::Interrupt)` over loopback TCP using a `(printf ...; exec sleep 30)` foreground subshell so `fork`/`setpgid`/`tcsetpgrp` and `SIG_DFL` disposition are deterministically established before readiness output arrives."

patterns-established:
  - "Shared `host_scenario(&dyn SessionHost)` parity contract exercising both in-process `LocalHost` and loopback TCP `DaemonHost` identically."
  - "Subshell `(printf readiness; exec long_cmd)` synchronization in live PTY signal tests to avoid racing builtin stdout against child process-group placement."

requirements-completed: [TERM-PTY-04]

coverage:
  - id: D1
    description: "`term.open` defaults terminal dimensions to `120×32` when `cols` or `rows` are omitted, matching `OpenSpec::default()`"
    requirement: TERM-PTY-04
    verification:
      - kind: integration
        ref: "crates/vox-orchestrator-d/src/term_dispatch.rs#term_dispatch::tests::attach_after_seq_streams_delta_over_tcp_loopback"
        status: pass
    human_judgment: false
  - id: D2
    description: "`term.list` and `term.attach` snapshot frames expose `alt_screen`, `cwd`, and `git_branch` metadata, and live streams forward `alt_screen` and `cwd_changed` events"
    requirement: TERM-PTY-04
    verification:
      - kind: integration
        ref: "crates/vox-orchestrator-d/src/term_dispatch.rs#term_dispatch::tests::list_and_events_expose_alt_screen_and_cwd_metadata"
        status: pass
    human_judgment: false
  - id: D3
    description: "`term.attach` with `after_seq` calls `LocalHost::attach_since(id, after_seq)` and returns only delta frames with `resync_required: false` over loopback TCP"
    requirement: TERM-PTY-04
    verification:
      - kind: integration
        ref: "crates/vox-orchestrator-d/src/term_dispatch.rs#term_dispatch::tests::attach_after_seq_streams_delta_over_tcp_loopback"
        status: pass
      - kind: integration
        ref: "crates/vox-orchestrator-d/src/term_dispatch.rs#term_dispatch::tests::local_and_daemon_hosts_pass_the_same_scenario"
        status: pass
    human_judgment: false
  - id: D4
    description: "`term.signal` interrupts an active foreground PTY job via `LocalHost::signal` while keeping the underlying session shell alive, and rejects `origin: \"agent\"` fail-closed"
    requirement: TERM-PTY-04
    verification:
      - kind: integration
        ref: "crates/vox-orchestrator-d/src/term_dispatch.rs#term_dispatch::tests::term_signal_interrupts_running_job_over_daemon_host"
        status: pass
      - kind: integration
        ref: "crates/vox-orchestrator-d/src/term_dispatch.rs#term_dispatch::tests::agent_origin_is_refused_on_input_and_submit"
        status: pass
      - kind: unit
        ref: "crates/vox-orchestrator-d/src/term_dispatch.rs#term_dispatch::tests::daemon_host_method_names_match_the_protocol"
        status: pass
    human_judgment: false

duration: 50min
completed: 2026-11-14
status: complete
---

# Phase 441 Plan 03: `TermDispatch` `120×32` Defaults, `after_seq` Delta Snapshots & Origin-Gated `term.signal` Summary

**Loopback TCP `TermDispatch` JSON-RPC wiring for `120×32` `term.open` defaults, `LocalHost::attach_since` delta snapshots (`resync_required`), `alt_screen`/`cwd`/`git_branch` metadata and live event frames, and fail-closed origin-gated `term.signal` foreground job interruption (`TERM-PTY-04`)**

## Performance

- **Duration:** 50 min
- **Started:** 2026-11-14T09:41:21Z
- **Completed:** 2026-11-14T10:31:00Z
- **Tasks:** 2
- **Files modified:** 1 (`crates/vox-orchestrator-d/src/term_dispatch.rs`)

## Accomplishments

- **`120×32` defaults, `after_seq` delta replay & live `alt_screen`/`cwd_changed` streaming (`TERM-PTY-04`):** Updated `TermDispatch::try_handle` (`term.open` defaulting `cols`/`rows` to `120×32` and `term.list` exposing `alt_screen`, `cwd`, and `git_branch`), upgraded `TermDispatch::try_stream` (`term.attach`) to read optional `after_seq`, invoke `LocalHost::attach_since(&session, after_seq)`, and emit `resync_required`, `alt_screen`, `cwd`, and `git_branch` on `"snapshot"` frames, and extended `session_event_json` with exhaustive `SessionEvent::AltScreenChanged` (`"type": "alt_screen"`) and `SessionEvent::CwdChanged` (`"type": "cwd_changed"`) arms.
- **Origin-gated `term.signal` RPC & `DaemonHost` loopback TCP verification (`TERM-PTY-04`, `T-441-04`):** Wired `vox_terminal_core::daemon_host::SIGNAL` (`"term.signal"`) in `TermDispatch::try_handle` gated fail-closed behind `refuse_agent_origin(p)?` before calling `LocalHost::signal(sid, sig)`, pinned `daemon_host::SIGNAL == "term.signal"` in protocol constant assertions, verified `"origin": "agent"` rejection across `"interrupt"` and `"kill"` payloads, and proved `DaemonHost::signal(..., TermSignal::Interrupt)` interrupts a live `sleep 30` foreground job over loopback TCP while preserving the interactive shell for subsequent commands.

## Task Commits

- `commits: skipped (shared git index protected per workspace constraints — 2,520+ concurrent staged files left untouched for orchestrator commit)`

## Verification & TDD Proof

- **Task 1 (`type="tracer"`, `tdd="true"`) RED → GREEN:**
  - RED: `timeout 900s cargo test -p vox-orchestrator-d --bin vox-orchestrator-d term_dispatch` failed with exit `101` (`error[E0004]: non-exhaustive patterns: vox_terminal_core::SessionEvent::CwdChanged { .. } and vox_terminal_core::SessionEvent::AltScreenChanged { .. } not covered`).
  - GREEN: `timeout 300s cargo test -p vox-orchestrator-d --bin vox-orchestrator-d term_dispatch` exited `0` (`10 passed; 0 failed` in `0.88s`, zero compiler warnings).
- **Task 2 (`type="auto"`, `tdd="true"`) RED → GREEN:**
  - RED: `timeout 300s cargo test -p vox-orchestrator-d --bin vox-orchestrator-d term_dispatch` failed with exit `101` (`9 passed; 2 failed`: `agent_origin_is_refused_on_input_and_submit` and `term_signal_interrupts_running_job_over_daemon_host` both failed with `unknown terminal method 'term.signal'`).
  - GREEN: `timeout 300s cargo test -p vox-orchestrator-d --bin vox-orchestrator-d term_dispatch` → **exit `0` (`11 passed; 0 failed` in `1.63s`, zero compiler warnings)**:
    - `term_dispatch::tests::daemon_host_method_names_match_the_protocol ... ok`
    - `term_dispatch::tests::list_and_events_expose_alt_screen_and_cwd_metadata ... ok`
    - `term_dispatch::tests::requests_without_the_daemon_token_are_rejected_and_create_nothing ... ok`
    - `term_dispatch::tests::agent_command_opens_block_with_agent_origin_and_enforces_floor ... ok`
    - `term_dispatch::tests::agent_origin_is_refused_on_input_and_submit ... ok`
    - `term_dispatch::tests::term_context_rpc_returns_redacted_blocks ... ok`
    - `term_dispatch::tests::origin_is_fail_closed_and_resize_needs_dimensions ... ok`
    - `term_dispatch::tests::term_signal_interrupts_running_job_over_daemon_host ... ok`
    - `term_dispatch::tests::open_submit_list_then_attach_snapshots_and_streams ... ok`
    - `term_dispatch::tests::local_and_daemon_hosts_pass_the_same_scenario ... ok`
    - `term_dispatch::tests::attach_after_seq_streams_delta_over_tcp_loopback ... ok`
- **Formatting, Artifact Line Counts & Stub/Threat Scan:**
  - Ran `rustfmt --edition 2024 crates/vox-orchestrator-d/src/term_dispatch.rs` (exit `0`).
  - Artifact line threshold verified: `crates/vox-orchestrator-d/src/term_dispatch.rs` = 881 lines (`>= 640`), contains `"term.signal"`, `attach_since`, and `DaemonHost`.
  - `rg -n "TODO|FIXME|XXX|HACK|unimplemented!|todo!" crates/vox-orchestrator-d/src/term_dispatch.rs` returned `0` matches (exit `1`).
  - Threats `T-441-02` (`dim` `120×32` defaults on `term.open` + `req_dim` validation on `term.resize`) and `T-441-04` (daemon auth token gate + fail-closed `refuse_agent_origin(p)?` at the top of `term.input`, `term.submit`, and `term.signal`) mitigated and verified by loopback TCP tests.

## Files Created/Modified

- `crates/vox-orchestrator-d/src/term_dispatch.rs` - Extended `TermDispatch` with `120×32` `term.open` defaults, `term.list` and `term.attach` `alt_screen`/`cwd`/`git_branch` metadata, `after_seq` delta snapshots via `LocalHost::attach_since`, `AltScreenChanged`/`CwdChanged` live stream serialization, origin-gated `term.signal` dispatch, and loopback TCP integration tests (`881` lines).

## Decisions Made

- Defaulted omitted or `null` `"signal"` parameters on `term.signal` to `TermSignal::Interrupt` while strictly validating explicit values via `serde_json::from_value::<TermSignal>` so invalid signal names fail fast rather than silently sending `SIGINT`.
- Structured `term_signal_interrupts_running_job_over_daemon_host` to launch `(printf '__sleep_%s__\n' 'ready'; exec sleep 30)` in a foreground subshell so `fork`, `setpgid`, `tcsetpgrp`, and `SIG_DFL` disposition are already active when `__sleep_ready__` arrives on the attached PTY stream.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- Concurrent Wave 3 Cargo builds contended on the workspace `target/` build directory lock and `vox-corpus` `build.rs` rebuilds during initial compilation; once the build lock cleared, the `vox-orchestrator-d` binary test target compiled with zero warnings and executed all 11 `term_dispatch` tests in `1.63s`.
- Initial `printf ... && sleep 30` readiness probe in `term_signal_interrupts_running_job_over_daemon_host` flushed shell-builtin `printf` output before interactive `zsh` forked `sleep 30` into its foreground process group; wrapping the probe in `(printf ...; exec sleep 30)` eliminated the process-group setup race cleanly.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `vox-orchestrator-d` `TermDispatch` now serves the complete Phase 441 daemon wire contract (`120×32` `term.open` defaults, `after_seq` delta replay with `resync_required`, `alt_screen`/`cwd`/`git_branch` metadata and live events, and origin-gated `term.signal`), ready for Wave 3/4 `vox-term`, `vox-cli`, and `vox-gui` clients.

## Self-Check: PASSED

- Verified on disk:
  - `crates/vox-orchestrator-d/src/term_dispatch.rs` — FOUND (881 lines, `>= 640` min_lines)
  - `.planning/workstreams/axis-warp-terminal-v75/phases/441-stateful-pty-and-lossless-streaming/441-03-SUMMARY.md` — FOUND
- Verified tests: `timeout 300s cargo test -p vox-orchestrator-d --bin vox-orchestrator-d term_dispatch` exited `0` (`11 passed; 0 failed` in `1.63s`, `0` warnings).

---
*Phase: 441-stateful-pty-and-lossless-streaming*
*Completed: 2026-11-14*
