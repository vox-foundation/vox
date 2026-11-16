---
phase: 441-stateful-pty-and-lossless-streaming
plan: 04
subsystem: terminal
tags: [vox-term, vox-cli, vox-gui, session-host, daemon-host, utf8-stream, alt-screen, tauri]

requires:
  - phase: 441-02
    provides: "Lossless `Utf8StreamDecoder`, zero-scrollback `VtShadow` `ALT_SCREEN` tracking, `2 MiB` `after_seq` delta replay ring, and widened `SessionHost::{attach, signal}` + `DaemonHost`"
provides:
  - "Live `SessionHost` event loop in `vox-term` (`attach_or_open_session`, numeric `pick_latest_session_id`, `apply_session_event`, `ALT_SCREEN` raw VT pass-through, `Event::Resize`, `Ctrl-C` `TermSignal::Interrupt`, clean detach without `host.close`, and `TryRecvError::Lagged` `after_seq` resync)"
  - "Zero-dimension `1×1` clamping (`T-441-02`), `is_alt_screen()`, and `reset()` on `vox-term::vt::VtGrid`"
  - "Async `Cli::Term { attach, local }` dispatch in `vox-cli` (`vox term [--attach [SESSION_ID]] [--local]`) awaiting `vox_term::app::run_with_args` directly on `vox-cli`'s Tokio runtime without nested-runtime panics or command-catalog path drift"
  - "`vox-gui` `term_session_attach` `after_seq: Option<u64>` delta attach, enriched `TermAttachmentDto` (`resync_required`, `alt_screen`, `cwd`, `git_branch`), lossless `Utf8StreamDecoder` chunk decoding in `pty_spawn` and `term_session_attach`, and `RecvError::Lagged` stream recovery"
affects: [vox-term, vox-cli, vox-gui, phase-442, phase-443]

actuals:
  tokens: 11275
  tasks: 3
  commits: 0

tech-stack:
  added: []
  patterns:
    - "Shared `SessionHost` TUI event loop with `ALT_SCREEN` raw VT byte pass-through (`key_event_to_vt_bytes`) vs structured block line composer, plus `after_seq` delta resync on broadcast lag"
    - "Clap `num_args = 0..=1` + `default_missing_value = \"latest\"` optional-value flag on top-level `Cli::Term` preserving baseline catalog path `[\"term\"]` while sharing `vox-cli`'s active Tokio runtime"
    - "Tauri `term_session_attach` resilient broadcast forwarder using `Utf8StreamDecoder` across split multi-byte UTF-8 chunks and `host.attach(&sid, Some(last_seq)).await` lag recovery (`T-441-03`)"

key-files:
  created: []
  modified:
    - crates/vox-term/src/vt.rs
    - crates/vox-term/src/app.rs
    - crates/vox-term/tests/headless.rs
    - crates/vox-term/tests/palette.rs
    - crates/vox-cli/src/lib.rs
    - crates/vox-cli/src/cli_dispatch/mod.rs
    - crates/vox-cli/src/command_catalog.rs
    - crates/vox-gui/src/commands/pty.rs

key-decisions:
  - "Kept `Cli::Term` as a struct variant with `--attach [SESSION_ID]` and `--local` flags (rather than nested subcommands) so `command_catalog_paths_baseline.txt` remains byte-identical on `[\"term\"]`."
  - "Synced `BlockOpened` and `BlockClosed` metadata in `vox_term::app::apply_session_event` via lightweight `host.attach(sid, Some(last_seq)).await` delta attaches while mutating `OutputAppended` chunks in-place on `blocks_state` so multi-client peers see identical commands, exit codes, and streaming output without quadratic full-replay fetches."
  - "Added `term_attachment_dto_from_attachment` alongside backward-compatible `term_attachment_dto_from_parts` in `vox-gui` so existing call sites stay valid while `term_session_attach` surfaces `resync_required`, `alt_screen`, `cwd`, and `git_branch`."

patterns-established:
  - "Numeric suffix ordering (`id.rsplit_once('-')` `u64` sort with lexicographic tie-breaker) in `pick_latest_session_id` so `term-10` beats `term-9` on bare `--attach`."
  - "Resilient Tauri `SessionHost` forwarder pattern: retain `Arc<dyn SessionHost>` + `last_seq` + `Utf8StreamDecoder` in the forwarder task, recover `RecvError::Lagged` via `host.attach(&sid, Some(last_seq))`, and flush trailing UTF-8 bytes on `RecvError::Closed`."

requirements-completed: [TERM-PTY-04]

coverage:
  - id: D1
    description: "`vox-term` and `vox term` resolve `SessionHost` (`DaemonHost` by default, `LocalHost` fallback/`--local`), support bare `--attach` (`latest` by numeric suffix `10 > 9`) and `--attach <id>`, render live `VtGrid` + `ALT_SCREEN` pass-through, forward `Event::Resize` and `Ctrl-C` `TermSignal::Interrupt`, sync peer block events, and leave daemon sessions alive on detach"
    requirement: TERM-PTY-04
    verification:
      - kind: integration
        ref: "crates/vox-term/tests/headless.rs#parse_cli_args_handles_attach_and_local"
        status: pass
      - kind: integration
        ref: "crates/vox-term/tests/headless.rs#attach_or_open_session_numeric_latest_reattach_and_missing_errors"
        status: pass
      - kind: integration
        ref: "crates/vox-term/tests/headless.rs#vt_grid_clamps_zero_dimensions_tracks_alt_screen_and_resets"
        status: pass
      - kind: integration
        ref: "crates/vox-term/tests/headless.rs#apply_session_event_syncs_peer_block_metadata_and_appends_output_in_place"
        status: pass
      - kind: integration
        ref: "crates/vox-term/tests/palette.rs#palette_includes_live_sessions"
        status: pass
    human_judgment: false
  - id: D2
    description: "`vox-cli` `Cli::Term { attach, local }` awaits `vox_term::app::run_with_args` directly on `vox-cli`'s Tokio runtime without nested-runtime panics and preserves `command_catalog_paths_baseline`"
    requirement: TERM-PTY-04
    verification:
      - kind: unit
        ref: "crates/vox-cli/src/command_catalog.rs#tests::term_cli_exposes_attach_and_local_flags"
        status: pass
      - kind: integration
        ref: "crates/vox-cli/tests/command_catalog_paths_baseline.rs"
        status: pass
    human_judgment: false
  - id: D3
    description: "`vox-gui` `term_session_attach` accepts `after_seq: Option<u64>`, returns `resync_required`/`alt_screen`/`cwd`/`git_branch` on `TermAttachmentDto`, decodes chunks losslessly via `Utf8StreamDecoder`, and recovers from `RecvError::Lagged` without dropping the Tauri stream"
    requirement: TERM-PTY-04
    verification:
      - kind: unit
        ref: "crates/vox-gui/src/commands/pty.rs#tests::local_fallback_session_host_lifecycle"
        status: pass
      - kind: unit
        ref: "crates/vox-gui/src/commands/pty.rs#tests::split_utf8_chunks_and_attachment_metadata_roundtrip_losslessly"
        status: pass
      - kind: integration
        ref: "crates/vox-gui/tests/real_backend_harness.rs#real_backend_terminal_session_host_and_command_catalog_roundtrip"
        status: pass
    human_judgment: false

duration: 164min
completed: 2026-11-14
status: complete
---

# Phase 441 Plan 04: `vox-term` TUI, `vox term` CLI & `vox-gui` `SessionHost` Attach Summary

**Live `SessionHost` event loop in `vox-term` (`--attach [SESSION_ID]` / `--local`, `ALT_SCREEN` raw VT pass-through, `Ctrl-C` `TermSignal::Interrupt`, clean detach), async `Cli::Term` dispatch in `vox-cli`, and `after_seq` delta attach + lossless `Utf8StreamDecoder` streaming + lag recovery in `vox-gui`**

## Performance

- **Duration:** 164 min (includes shared Cargo lock & cold incremental compile queue across concurrent Wave 3 builds)
- **Started:** 2026-11-14T09:41:01Z
- **Completed:** 2026-11-14T12:25:00Z
- **Tasks:** 3
- **Files modified:** 8

## Accomplishments

- **Live `SessionHost` TUI loop & `VtGrid` hardening in `vox-term` (`TERM-PTY-04`):** Clamped `VtGrid::new` and `VtGrid::resize` with `cols.max(1)` / `rows.max(1)` (`T-441-02`) and added `is_alt_screen()` + `reset()` in `crates/vox-term/src/vt.rs`. Replaced the disconnected `Session::new("main")` stub in `crates/vox-term/src/app.rs` with `attach_or_open_session`, numeric-suffix `pick_latest_session_id` (`"term-10"` > `"term-9"`), bare `--attach` (`Some("latest")`) parsing, `apply_session_event` peer block sync, `ALT_SCREEN` raw VT rendering and `key_event_to_vt_bytes` pass-through, `Event::Resize` forwarding, `Ctrl-C` `TermSignal::Interrupt` (`T-441-04`), clean detach without `host.close`, and `TryRecvError::Lagged` `after_seq` delta recovery (`T-441-03`).
- **Async `vox term` CLI flags & nested-runtime elimination in `vox-cli` (`TERM-PTY-04`):** Upgraded `Cli::Term` in `crates/vox-cli/src/lib.rs` with `--attach [SESSION_ID]` (`num_args = 0..=1`, `default_missing_value = "latest"`) and `--local`, updated `crates/vox-cli/src/cli_dispatch/mod.rs` to `.await` `vox_term::app::run_with_args(...)` directly on `vox-cli`'s active Tokio runtime instead of calling `vox_term::app::run()` (`Runtime::new()`), and kept `command_catalog_paths_baseline` green.
- **Lossless UTF-8 streaming, `after_seq` delta attach & lag recovery in `vox-gui` (`TERM-PTY-04`):** Extended `TermAttachmentDto` in `crates/vox-gui/src/commands/pty.rs` with `resync_required`, `alt_screen`, `cwd`, and `git_branch`, added `term_attachment_dto_from_attachment`, routed `pty_spawn` and `term_session_attach` output through `Utf8StreamDecoder` (`push` + `flush_lossy`), added optional `after_seq: Option<u64>` to `term_session_attach`, and recovered `broadcast::error::RecvError::Lagged` via `host.attach(&sid, Some(last_seq)).await` (`T-441-03`).

## Task Commits

- `commits: skipped (shared git index protected per workspace constraint and 00-core.md Rule 6 — 2,520+ staged files in concurrent workspace index left untouched for orchestrator commit)`

## Verification & TDD Proof

- **Task 1 (`type="tracer"`, `tdd="true"`) RED → GREEN:**
  - RED: `cargo test -p vox-term --test headless` exited `101` (`E0425` missing `attach_or_open_session`, `pick_latest_session_id`, `apply_session_event`; `E0599` missing `VtGrid::is_alt_screen` and `VtGrid::reset`).
  - GREEN: `timeout 300s cargo test -p vox-term` exited `0` (`Finished test profile in 1m 15s`, `0` warnings; `16 passed; 0 failed; 1 ignored` across `lib` `1/1 + 1 ignored`, `main` `0/0`, `dispatch` `2/2`, `headless` `7/7`, `palette` `4/4`, `vt` `2/2`).
- **Task 2 (`type="auto"`, `tdd="true"`) RED → GREEN:**
  - RED: `cargo test -p vox-cli --lib term_cli_exposes_attach_and_local_flags` exited `101` (`E0026: variant Cli::Term does not have fields named attach, local`).
  - GREEN:
    - `cargo test -p vox-cli --lib term_cli_exposes_attach_and_local_flags` → exit `0` (`1 passed; 0 failed` in `0.12s`).
    - `cargo test -p vox-cli --test command_catalog_paths_baseline` → exit `0` (`Finished test profile in 2m 33s`; `1 passed; 0 failed` in `0.09s`).
- **Task 3 (`type="auto"`, `tdd="true"`) RED → GREEN:**
  - RED: `cargo test -p vox-gui --bin vox-gui commands::pty` exited `101` (`10` errors: `E0061` 1-arg `host.attach`, `E0425` missing `term_attachment_dto_from_attachment` and `decode_stream_chunk`, `E0609` missing `resync_required`/`alt_screen`/`cwd`/`git_branch` on `TermAttachmentDto`).
  - GREEN:
    - `cargo test -p vox-gui --bin vox-gui commands::pty` → exit `0` (`Finished test profile in 2m 46s`, `0` warnings; `11 passed; 0 failed` in `0.32s`).
    - `cargo test -p vox-gui --test real_backend_harness real_backend_terminal_session_host` → exit `0` (`Finished test profile in 2m 50s`, `0` warnings; `1 passed; 0 failed` in `18.15s`).
- **Formatting, Artifact Line Counts & Stub/Threat Scan:**
  - Ran `rustfmt --edition 2024` individually across all 8 modified `.rs` files (exit `0`).
  - `crates/vox-term/src/app.rs` = 476 lines (`>= 240` min_lines), zero `Session::new` references (`rg` exit `1`).
  - `rg -n "TODO|FIXME|XXX|HACK|unimplemented!|todo!"` across all 8 modified `.rs` files returned `0` matches (exit `1`).
  - Threats `T-441-02` (`1×1` resize clamping in `VtGrid`), `T-441-03` (`after_seq` delta resync on `Lagged` in both `vox-term` and `vox-gui`), and `T-441-04` (`SessionHost::signal(..., TermSignal::Interrupt)` + detach leaving sessions open) mitigated and verified.

## Files Created/Modified

- `crates/vox-term/src/vt.rs` - Clamped `cols.max(1)` / `rows.max(1)` in `VtGrid::new` and `VtGrid::resize`, added `is_alt_screen()` and `reset()`
- `crates/vox-term/src/app.rs` - Replaced local `Session::new("main")` stub with `SessionHost` attach/open (`attach_or_open_session`, `pick_latest_session_id`, `apply_session_event`, `key_event_to_vt_bytes`, `ALT_SCREEN` raw VT pass-through, `Event::Resize`, `Ctrl-C` `TermSignal::Interrupt`, clean detach, and `Lagged` `after_seq` resync)
- `crates/vox-term/tests/headless.rs` - Added integration tests for bare `--attach`, numeric `latest` (`term-10` > `term-9`), live PTY detach/reattach block continuity, missing-session errors, `VtGrid` clamping/`is_alt_screen`/`reset`, and `apply_session_event` peer block sync
- `crates/vox-term/tests/palette.rs` - Added `..Default::default()` on `SessionInfo` fixture
- `crates/vox-cli/src/lib.rs` - Converted `Cli::Term` to struct variant with `--attach [SESSION_ID]` and `--local`
- `crates/vox-cli/src/cli_dispatch/mod.rs` - Awaited `vox_term::app::run_with_args(TermCliArgs { attach, force_local: local }).await?` and added `Cli::Term { .. }` arms to `universal_reward_command_path` and `command_verb`
- `crates/vox-cli/src/command_catalog.rs` - Added `term_cli_exposes_attach_and_local_flags` unit test verifying flag shapes and single `["term"]` catalog entry
- `crates/vox-gui/src/commands/pty.rs` - Added `after_seq: Option<u64>` to `term_session_attach`, enriched `TermAttachmentDto` (`resync_required`, `alt_screen`, `cwd`, `git_branch`), added `term_attachment_dto_from_attachment`, wired `Utf8StreamDecoder` into `pty_spawn` and `term_session_attach`, recovered `RecvError::Lagged` via `after_seq` delta attach, and added unit tests

## Decisions Made

- Kept `Cli::Term` as a flag-bearing top-level command rather than introducing nested subcommands (`vox term attach`) so `crates/vox-cli/tests/fixtures/command_catalog_paths_baseline.txt` required zero changes.
- Used `Option<u64>` without `#[serde(default)]` on the `term_session_attach` Tauri command argument so Tauri v2's IPC deserializer natively accepts existing frontend `{ sessionId }` payloads (`after_seq == None`) as well as `{ sessionId, afterSeq }` reconnect calls.
- Kept `term_attachment_dto_from_parts` (`#[allow(dead_code)]`) setting `resync_required: true` and decoding through `Utf8StreamDecoder` for backward compatibility alongside `term_attachment_dto_from_attachment`.

## Deviations from Plan

None — plan implementation executed as specified. Note on verification invocation: `vox-gui` is a binary-only crate (`src/main.rs` with no `[lib]` target in `crates/vox-gui/Cargo.toml`), so unit tests in `crates/vox-gui/src/commands/pty.rs` were executed via `cargo test -p vox-gui --bin vox-gui commands::pty` (`11/11` passed) rather than `--lib`.

## Issues Encountered

- Cold incremental compilation of `vox-orchestrator-mcp` (`features = ["news-publish", "gui-visual-review", "heavy-browser"]`) under concurrent multi-agent CPU load (`load avg ~22`) combined with `crates/vox-corpus/build.rs`'s `cargo:rerun-if-changed=../vox-mcp/src/tools/` invalidated downstream crates on initial `vox-gui` build. Warming `vox-orchestrator-mcp` once populated `target/debug/incremental/`, after which `vox-gui` unit (`2m 46s`), `vox-gui` `real_backend_harness` (`2m 50s`), `vox-cli` (`2m 33s`), and `vox-term` (`1m 15s`) test builds all completed cleanly under `300s`.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 441 (`TERM-PTY-01`..`TERM-PTY-04`) terminal attach/stream plumbing is wired end-to-end across `vox-terminal-core`, `vox-term`, `vox-cli`, and `vox-gui`, ready for Phase 442 structured block grid / xterm frontend hydration (`resync_required`, `alt_screen`, `cwd`, `git_branch`, `afterSeq`) and Phase 443 remote/workspace sessions.

## Self-Check: PASSED

- Verified on disk:
  - `crates/vox-term/src/vt.rs` — FOUND (187 lines)
  - `crates/vox-term/src/app.rs` — FOUND (476 lines)
  - `crates/vox-term/tests/headless.rs` — FOUND (254 lines)
  - `crates/vox-term/tests/palette.rs` — FOUND (51 lines)
  - `crates/vox-cli/src/lib.rs` — FOUND (922 lines)
  - `crates/vox-cli/src/cli_dispatch/mod.rs` — FOUND (700 lines)
  - `crates/vox-cli/src/command_catalog.rs` — FOUND (1286 lines)
  - `crates/vox-gui/src/commands/pty.rs` — FOUND (553 lines)
  - `.planning/workstreams/axis-warp-terminal-v75/phases/441-stateful-pty-and-lossless-streaming/441-04-SUMMARY.md` — FOUND
- Verified tests:
  - `vox-term`: `16 passed; 0 failed; 1 ignored` across all 6 targets (`lib`, `main`, `dispatch`, `headless`, `palette`, `vt`), `0` warnings
  - `vox-cli`: `term_cli_exposes_attach_and_local_flags` (`1/1` passed) + `command_catalog_paths_baseline` (`1/1` passed)
  - `vox-gui`: `commands::pty` (`11/11` passed, `0` warnings) + `real_backend_harness::real_backend_terminal_session_host_and_command_catalog_roundtrip` (`1/1` passed, `0` warnings)

---
*Phase: 441-stateful-pty-and-lossless-streaming*
*Completed: 2026-11-14*
