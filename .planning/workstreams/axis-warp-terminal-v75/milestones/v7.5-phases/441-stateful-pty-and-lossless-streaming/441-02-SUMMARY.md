---
phase: 441-stateful-pty-and-lossless-streaming
plan: 02
subsystem: terminal
tags: [utf8, osc633, alt-screen, alacritty-terminal, vte, replay-ring, daemon-host, pty]

requires:
  - phase: 441-01
    provides: "Stateful PtyHandle, bounded Osc633Parser + OSC 7, Session CwdChanged, and LocalHost Condvar pump"
provides:
  - "Lossless 3-byte tail `Utf8StreamDecoder` (`new`, `push`, `flush_lossy`) and ANSI-aware `normalize_cr_overwrites` (`\\r` / `CSI K` / `CSI 2K`) in `vox-terminal-core::utf8_stream`"
  - "Live `Osc633Parser` `Output` streaming between `633;C` and `633;D` without hoarding in `output_buf`, plus `Block` `cwd`, `git_branch`, `cancel()`, and `plain_output()` CR normalization"
  - "Zero-scrollback `alacritty_terminal::Term` `VtShadow` (`?1049h`/`?1049l`, `?47h`, `?1047h`, compound DECSET, `\\x1bc` RIS) emitting `SessionEvent::AltScreenChanged` and suppressing `OSC 633` block splitting while active"
  - "Bounded `2 MiB` `VecDeque<OutputFrame>` `after_seq` delta ring (`LocalHost::attach_since`, `resync_required`), widened `SessionHost::attach(id, after_seq)` + `SessionHost::signal(id, sig)`, and `DaemonHost` `term.signal` + automatic `lagged` re-subscribe pump"
affects: [441-03-PLAN, 441-04-PLAN, vox-term, vox-orchestrator-d, vox-cli, vox-gui]

actuals:
  tokens: 30389
  tasks: 3
  commits: 0

tech-stack:
  added: []
  patterns:
    - "`std::str::from_utf8` `valid_up_to()` + `error_len()` 3-byte trailing UTF-8 stream decoder (`Utf8StreamDecoder`) eliminating `String::from_utf8_lossy` split-chunk `U+FFFD` corruption"
    - "Zero-scrollback `alacritty_terminal::Term` shadow grid (`VtShadow`) bridged with a `vte::ansi::Handler` for legacy `?47`/`?1047` DECSET/DECRST modes and causal `ESC`-segmented transition tracking"
    - "`2 MiB` `VecDeque<OutputFrame>` `after_seq` delta replay ring with `truncated_front` guard and bounded `DaemonHost` `lagged` stream auto-resubscription (`MAX_LAG_RESUBSCRIBE_ATTEMPTS = 8`)"

key-files:
  created:
    - crates/vox-terminal-core/src/utf8_stream.rs
    - crates/vox-terminal-core/src/vt_shadow.rs
    - crates/vox-terminal-core/tests/streaming_utf8_and_alt_screen.rs
  modified:
    - crates/vox-terminal-core/src/lib.rs
    - crates/vox-terminal-core/src/osc633.rs
    - crates/vox-terminal-core/src/block.rs
    - crates/vox-terminal-core/src/session.rs
    - crates/vox-terminal-core/src/host.rs
    - crates/vox-terminal-core/src/daemon_host.rs

key-decisions:
  - "Streamed `Osc633Event::Output` chunks live through `Utf8StreamDecoder` both inside and outside `633;C..633;D` (flushing incomplete bytes via `flush_lossy()` only at escape marker boundaries) so long-running builds emit `OutputAppended` immediately before command exit."
  - "Fed `VtShadow` causally inside `Session::apply_osc(Osc633Event::Output)` across `ESC`-segmented sub-slices so nested TUI `OSC 633` markers inside `?1049h..l` are suppressed while an outer `633;D;0` arriving immediately after `?1049l` in the same read buffer closes the outer block cleanly."
  - "Preserved inherent `LocalHost::attach(&self, id)` delegating to `LocalHost::attach_since(id, None)` alongside widened `SessionHost::attach(&self, id, after_seq)` so direct `LocalHost` callers compile without churn ahead of Wave 3."

patterns-established:
  - "Column-indexed ANSI cell line buffer in `normalize_cr_overwrites` so SGR color prefixes attach to cells without advancing column indices during bare `\\r` spinner overwrites or `CSI K` line erases."
  - "Monotonic `seq`-indexed `VecDeque<OutputFrame>` ring setting `resync_required: false` only when `after_seq` is contiguous with retained non-truncated frames, falling back to full retained window (`resync_required: true`) on initial attach or ring eviction."

requirements-completed: [TERM-PTY-03]

coverage:
  - id: D1
    description: "Prime-chunk (`1`, `7`, `13`, `31` bytes) multi-byte UTF-8 box-drawing/emoji/ANSI/OSC stream decoding with zero `U+FFFD` corruption and live `OutputAppended` emission between `633;C` and `633;D`"
    requirement: TERM-PTY-03
    verification:
      - kind: integration
        ref: "crates/vox-terminal-core/tests/streaming_utf8_and_alt_screen.rs#prime_chunk_utf8_reassembly_zero_replacement_chars"
        status: pass
      - kind: integration
        ref: "crates/vox-terminal-core/tests/streaming_utf8_and_alt_screen.rs#osc633_streams_live_output_between_c_and_d_before_exit"
        status: pass
    human_judgment: false
  - id: D2
    description: "`Block::plain_output` `\\r` progress-bar overwrite and `\\x1b[K` / `\\x1b[2K` erase-in-line normalization preserving split `\\r\\n` newlines across chunks"
    requirement: TERM-PTY-03
    verification:
      - kind: integration
        ref: "crates/vox-terminal-core/tests/streaming_utf8_and_alt_screen.rs#cr_progress_overwrite_and_csi_k_normalization_preserves_split_crlf"
        status: pass
    human_judgment: false
  - id: D3
    description: "Zero-scrollback `VtShadow` `TermMode::ALT_SCREEN` tracking (`?1049h`/`?1049l`, `?47h`, `?1047h`, compound DECSET, `\\x1bc` RIS, `1×1` clamping), `SessionEvent::AltScreenChanged`, and `OSC 633` block-split suppression"
    requirement: TERM-PTY-03
    verification:
      - kind: integration
        ref: "crates/vox-terminal-core/tests/streaming_utf8_and_alt_screen.rs#vt_shadow_decset_decrst_ris_and_zero_dimension_clamping"
        status: pass
      - kind: integration
        ref: "crates/vox-terminal-core/tests/streaming_utf8_and_alt_screen.rs#session_alt_screen_emits_events_suppresses_nested_block_splits_and_stamps_cwd"
        status: pass
    human_judgment: false
  - id: D4
    description: "`LocalHost` `2 MiB` `VecDeque<OutputFrame>` `after_seq` delta replay ring (`resync_required`), widened `SessionHost::attach` + `SessionHost::signal`, and `DaemonHost` `term.signal` + bounded `lagged` auto-resubscribe pump"
    requirement: TERM-PTY-03
    verification:
      - kind: integration
        ref: "crates/vox-terminal-core/tests/streaming_utf8_and_alt_screen.rs#localhost_after_seq_delta_slicing_session_info_and_trait_methods"
        status: pass
      - kind: integration
        ref: "crates/vox-terminal-core/tests/streaming_utf8_and_alt_screen.rs#localhost_replay_ring_eviction_and_oversized_frame_cap"
        status: pass
      - kind: integration
        ref: "crates/vox-terminal-core/tests/streaming_utf8_and_alt_screen.rs#daemon_host_wire_routing_maps_alt_screen_cwd_and_signal_constant"
        status: pass
      - kind: unit
        ref: "crates/vox-terminal-core/src/daemon_host.rs#pump_resubscribes_with_after_seq_on_lagged_and_bounds_tight_loops"
        status: pass
    human_judgment: false

duration: 68min
completed: 2026-11-14
status: complete
---

# Phase 441 Plan 02: Lossless Streaming UTF-8, `VtShadow` Alternate Screen & `after_seq` Delta Replay Summary

**3-byte tail `Utf8StreamDecoder` with live `633;C..D` streaming, `\r`/`CSI K` progress overwrite normalization, zero-scrollback `alacritty_terminal` `VtShadow` `ALT_SCREEN` tracking, and `2 MiB` `after_seq` delta replay across `LocalHost`, `SessionHost`, and `DaemonHost` in `vox-terminal-core`**

## Performance

- **Duration:** 68 min
- **Started:** 2026-11-14T08:29:00Z
- **Completed:** 2026-11-14T09:37:00Z
- **Tasks:** 3
- **Files modified:** 9 (3 created, 6 modified)

## Accomplishments

- **Lossless streaming UTF-8 & progress overwrite normalization (`TERM-PTY-03`):** Built `crates/vox-terminal-core/src/utf8_stream.rs` exporting `Utf8StreamDecoder` (`new()`, `push(&mut self, bytes: &[u8]) -> String`, `flush_lossy(&mut self) -> String`) and ANSI-aware `normalize_cr_overwrites`, upgraded `Osc633Parser` (`crates/vox-terminal-core/src/osc633.rs`) to stream `Osc633Event::Output` chunks live between `633;C` and `633;D` without `output_buf` hoarding or `String::from_utf8_lossy` corruption, and added `cwd`, `git_branch`, `cancel()`, and `\r`/`CSI K` normalization to `Block` (`crates/vox-terminal-core/src/block.rs`).
- **Zero-scrollback `VtShadow` `ALT_SCREEN` state machine (`TERM-PTY-03`):** Created `crates/vox-terminal-core/src/vt_shadow.rs` wrapping `alacritty_terminal::Term` with `Config { scrolling_history: 0, ..Config::default() }` and `1×1` dimension clamping (`T-441-02`), tracking `?1049h`/`?1049l`, `?47h`/`?47l`, `?1047h`/`?1047l`, compound DECSET, and `\x1bc` RIS transitions, and wired `Session` (`crates/vox-terminal-core/src/session.rs`) to emit `SessionEvent::AltScreenChanged { active }`, stamp `cwd`/`git_branch` onto opened blocks, and suppress nested `OSC 633` block splits while `alt_screen` is active.
- **Bounded `2 MiB` `after_seq` delta ring & `DaemonHost` lag recovery (`TERM-PTY-03`, `L3` contract for `TERM-PTY-04`):** Replaced `LocalHost`'s byte ring in `crates/vox-terminal-core/src/host.rs` with a `seq`-indexed `VecDeque<OutputFrame>` ring (`LocalHost::attach_since`, `resync_required`, `SessionInfo` + `Attachment` metadata fields), widened `SessionHost::attach(id, after_seq)` and `SessionHost::signal(id, sig)`, and upgraded `DaemonHost` (`crates/vox-terminal-core/src/daemon_host.rs`) with `pub const SIGNAL = "term.signal"`, `alt_screen`/`cwd_changed` frame routing, `after_seq` delta attach, and bounded automatic `"lagged"` stream re-subscription (`T-441-04`).

## Task Commits

- `commits: skipped (shared git index protected per 00-core.md Rule 6 — concurrent workspace index left untouched for orchestrator commit)`

## Verification & TDD Proof

- **Task 1 (`type="tracer"`, `tdd="true"`) RED → GREEN:**
  - RED: `timeout 300s cargo test -p vox-terminal-core --test streaming_utf8_and_alt_screen` failed with exit `101` (`E0432` unresolved import `vox_terminal_core::utf8_stream`, `E0599` missing `Block::cancel`, `E0609` missing `Block.cwd` / `Block.git_branch`).
  - GREEN: `timeout 300s cargo test -p vox-terminal-core --test streaming_utf8_and_alt_screen` exited `0` (`3 passed; 0 failed` in `0.01s`).
- **Task 2 (`type="auto"`, `tdd="true"`) RED → GREEN:**
  - RED: `timeout 300s cargo test -p vox-terminal-core --test streaming_utf8_and_alt_screen` failed with exit `101` (`E0432` unresolved import `vox_terminal_core::vt_shadow`, `E0599` missing `Session::with_size` and `SessionEvent::AltScreenChanged`).
  - GREEN: `timeout 300s cargo test -p vox-terminal-core --test streaming_utf8_and_alt_screen` exited `0` (`5 passed; 0 failed` in `0.03s`).
- **Task 3 (`type="auto"`, `tdd="true"`) RED → GREEN:**
  - RED: `timeout 300s cargo test -p vox-terminal-core --test streaming_utf8_and_alt_screen` failed with exit `101` (33 compile errors on missing `daemon_host::SIGNAL`, `route` 3-arg signature, `attach_since`, `resync_required`, `alt_screen`, `SessionHost::signal`, etc.).
  - GREEN:
    - `timeout 300s cargo test -p vox-terminal-core --test streaming_utf8_and_alt_screen` → **exit `0` (`8 passed; 0 failed` in `0.01s`, zero compiler warnings)**:
      - `prime_chunk_utf8_reassembly_zero_replacement_chars ... ok`
      - `osc633_streams_live_output_between_c_and_d_before_exit ... ok`
      - `cr_progress_overwrite_and_csi_k_normalization_preserves_split_crlf ... ok`
      - `vt_shadow_decset_decrst_ris_and_zero_dimension_clamping ... ok`
      - `session_alt_screen_emits_events_suppresses_nested_block_splits_and_stamps_cwd ... ok`
      - `localhost_after_seq_delta_slicing_session_info_and_trait_methods ... ok`
      - `localhost_replay_ring_eviction_and_oversized_frame_cap ... ok`
      - `daemon_host_wire_routing_maps_alt_screen_cwd_and_signal_constant ... ok`
    - All 14 `vox-terminal-core` test targets compiled (`Finished test profile [unoptimized + debuginfo] target(s)` with `0` warnings) and passed (`106 passed; 0 failed` total: `lib` 49/49, `agent` 3/3, `block` 5/5, `commands` 2/2, `host` 1/1, `input` 10/10, `osc633` 5/5, `pty` 5/5, `session` 7/7, `shell_init_and_stateful_pty` 6/6, `smoke` 1/1, `streaming_utf8_and_alt_screen` 8/8, `transcript` 2/2, `vox_interp` 2/2).
- **Formatting, Artifact Line Counts & Stub/Threat Scan:**
  - Ran `rustfmt --edition 2024` across all 9 created/modified `.rs` files (exit `0`).
  - Artifact line thresholds verified: `utf8_stream.rs` = 232 lines (`>= 90`), `vt_shadow.rs` = 154 lines (`>= 65`), `streaming_utf8_and_alt_screen.rs` = 418 lines (`>= 160`).
  - `rg -n "TODO|FIXME|XXX|HACK|unimplemented!|todo!"` across all 9 created/modified `.rs` files returned `0` matches (exit `1`).
  - Threats `T-441-02` (`1×1` clamping + `scrolling_history: 0`), `T-441-03` (`2 MiB` `Replay` cap with oversized-frame tail slicing + `16 KiB` incomplete OSC cap), and `T-441-04` (authenticated `OrchDaemonClient` reuse + `MAX_LAG_RESUBSCRIBE_ATTEMPTS = 8` tight-loop guard) mitigated and verified by tests.

## Files Created/Modified

- `crates/vox-terminal-core/src/utf8_stream.rs` - 3-byte trailing `Utf8StreamDecoder` (`new`, `push`, `flush_lossy`) and ANSI-aware `normalize_cr_overwrites` (`\r`, `CSI K`, `CSI 2K`)
- `crates/vox-terminal-core/src/vt_shadow.rs` - Zero-scrollback `alacritty_terminal::Term` `VtShadow` (`new`, `feed`, `feed_with_transitions`, `resize`, `is_alt_screen`) with legacy `?47`/`?1047` DECSET/DECRST sync
- `crates/vox-terminal-core/src/lib.rs` - Exported `pub mod utf8_stream`, `pub mod vt_shadow`, `Utf8StreamDecoder`, `normalize_cr_overwrites`, and `VtShadow`
- `crates/vox-terminal-core/src/osc633.rs` - Replaced `output_buf` hoarding and `from_utf8_lossy` with live `Utf8StreamDecoder` `Output` streaming and fixed multi-byte UTF-8 `decode_command`
- `crates/vox-terminal-core/src/block.rs` - Added `cwd`, `git_branch`, `Block::cancel()`, and `normalize_cr_overwrites` in `Block::plain_output()`
- `crates/vox-terminal-core/src/session.rs` - Added `SessionEvent::AltScreenChanged`, `Session::with_size`, `is_alt_screen`, `resize`, `cwd`/`git_branch` block stamping, and `ALT_SCREEN` block-split suppression
- `crates/vox-terminal-core/src/host.rs` - Upgraded `Replay` to a `2 MiB` `VecDeque<OutputFrame>` ring, added `LocalHost::attach_since`, `open_synthetic`, `feed_raw_output`, `SessionInfo`/`Attachment` metadata, and widened `SessionHost::{attach, signal}`
- `crates/vox-terminal-core/src/daemon_host.rs` - Added `pub const SIGNAL = "term.signal"`, `alt_screen`/`cwd_changed` wire routing, `after_seq` delta attach, `SessionHost::signal`, and bounded `"lagged"` auto-resubscribe pump
- `crates/vox-terminal-core/tests/streaming_utf8_and_alt_screen.rs` - 8-test integration suite covering prime-chunk UTF-8 reassembly, live `633;C..D` streaming, CR/`CSI K` normalization, `VtShadow` transitions, `Session` `ALT_SCREEN` suppression, `after_seq` delta slicing, ring eviction, and `DaemonHost` wire routing

## Decisions Made

- Kept `LocalHost::attach(&self, id)` as an inherent convenience method delegating to `self.attach_since(id, None)` while widening `SessionHost::attach(&self, id, after_seq: Option<u64>)` so direct `LocalHost` callers (`vox-gui` harnesses, unit tests) require zero changes until Wave 3 migrates `SessionHost` trait callers.
- Tracked `truncated_front: bool` on `Replay` so if a single `OutputFrame` exceeds `replay_cap` and retains only its trailing `replay_cap` bytes, a client attaching with `after_seq == seq - 1` receives `resync_required: true` rather than mistaking the truncated tail for a gap-free delta.
- Woke `Entry::settled_cv` in `LocalHost::submit_with_origin` as soon as `s.is_alt_screen()` becomes `true` so launching a full-screen TUI (`vim`, `htop`, `less`) returns immediately instead of waiting out the `1500ms` synchronous command settle budget.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed multi-byte UTF-8 corruption in `osc633::decode_command`**
- **Found during:** Task 1 (`prime_chunk_utf8_reassembly_zero_replacement_chars`)
- **Issue:** `osc633::decode_command(enc: &str)` previously pushed raw bytes via `bytes[i] as char`, corrupting multi-byte UTF-8 characters inside `633;E;<cmd>` markers (such as `echo 🦀╭─╮`) into Latin-1 code points.
- **Fix:** Accumulated decoded bytes into a `Vec<u8>` buffer and converted via `String::from_utf8` (with `from_utf8_lossy` fallback for non-UTF-8 hex escapes).
- **Files modified:** `crates/vox-terminal-core/src/osc633.rs`
- **Verification:** `prime_chunk_utf8_reassembly_zero_replacement_chars` passed across `1`-, `7`-, `13`-, and `31`-byte chunk boundaries with exact multi-byte `633;E` command reconstruction.

**2. [Rule 1 - Bug] Bridged legacy `?47` and `?1047` alternate-screen modes in `VtShadow`**
- **Found during:** Task 2 (`vt_shadow_decset_decrst_ris_and_zero_dimension_clamping`)
- **Issue:** `vte 0.15.0` maps mode `1049` to `NamedPrivateMode::SwapScreenAndSetRestoreCursor`, but surfaces legacy xterm alternate-screen modes `47` and `1047` as `PrivateMode::Unknown(47 | 1047)`, which `alacritty_terminal::Term::set_private_mode` ignores.
- **Fix:** Added a 15-line `LegacyAltSync` `vte::ansi::Handler` in `crates/vox-terminal-core/src/vt_shadow.rs` forwarding `PrivateMode::Unknown(47 | 1047)` set/unset transitions onto `Term`'s `SwapScreenAndSetRestoreCursor` mode.
- **Files modified:** `crates/vox-terminal-core/src/vt_shadow.rs`
- **Verification:** `vt_shadow_decset_decrst_ris_and_zero_dimension_clamping` passed for `?1049h/l`, split `?10`+`49h`, compound `?1049;2004h`, `?47h/l`, `?1047h/l`, and `\x1bc` RIS.

---

**Total deviations:** 2 auto-fixed (2 Rule 1 bugs)
**Impact on plan:** Both fixes were required to satisfy Task 1 and Task 2 behavioral specs cleanly without adding dependencies or touching files outside `files_modified`.

## Issues Encountered

- Upstream `crates/vox-corpus/build.rs` watches `../vox-mcp/src/tools/` (which triggers Cargo rebuilds of `vox-corpus` → `vox-populi` → `vox-orchestrator` on `cargo test -p vox-terminal-core` runs under heavy multi-agent CPU contention). Once `cargo test -p vox-terminal-core` finished linking all 14 `test` profile targets cleanly (`0` warnings), both `cargo test -p vox-terminal-core --test streaming_utf8_and_alt_screen` (`8/8` in `0.01s`) and all 14 compiled `vox-terminal-core` test binaries (`106/106` tests) executed to completion and passed.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `vox-terminal-core` (`L3`) now exports `Utf8StreamDecoder` (`new`, `push`, `flush_lossy`), `normalize_cr_overwrites`, `VtShadow`, `SessionEvent::AltScreenChanged`, `LocalHost::attach_since`, widened `SessionHost::attach(id, after_seq)` and `SessionHost::signal(id, sig)`, and `DaemonHost` (`SIGNAL = "term.signal"`, `after_seq` delta attach, automatic `"lagged"` resync), ready for Wave 3 Plans `441-03` (`vox-orchestrator-d` `TermDispatch` + `vox-term` TUI) and `441-04` (`vox-cli` + `vox-gui` WS/HTTP terminal routes and React UI).

## Self-Check: PASSED

- Verified on disk:
  - `crates/vox-terminal-core/src/utf8_stream.rs` — FOUND (232 lines)
  - `crates/vox-terminal-core/src/vt_shadow.rs` — FOUND (154 lines)
  - `crates/vox-terminal-core/src/lib.rs` — FOUND (55 lines)
  - `crates/vox-terminal-core/src/osc633.rs` — FOUND (258 lines)
  - `crates/vox-terminal-core/src/block.rs` — FOUND (161 lines)
  - `crates/vox-terminal-core/src/session.rs` — FOUND (558 lines)
  - `crates/vox-terminal-core/src/host.rs` — FOUND (952 lines)
  - `crates/vox-terminal-core/src/daemon_host.rs` — FOUND (609 lines)
  - `crates/vox-terminal-core/tests/streaming_utf8_and_alt_screen.rs` — FOUND (418 lines)
  - `.planning/workstreams/axis-warp-terminal-v75/phases/441-stateful-pty-and-lossless-streaming/441-02-SUMMARY.md` — FOUND
- Verified tests: 106/106 tests across all 14 `vox-terminal-core` lib and integration test targets passed (`0` failed, `0` warnings).

---
*Phase: 441-stateful-pty-and-lossless-streaming*
*Completed: 2026-11-14*
