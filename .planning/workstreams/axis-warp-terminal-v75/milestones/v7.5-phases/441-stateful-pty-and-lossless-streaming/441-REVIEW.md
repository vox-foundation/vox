---
phase: 441-stateful-pty-and-lossless-streaming
reviewed: 2026-11-14T12:43:00Z
depth: standard
files_reviewed: 22
files_reviewed_list:
  - crates/vox-cli/src/cli_dispatch/mod.rs
  - crates/vox-cli/src/command_catalog.rs
  - crates/vox-cli/src/lib.rs
  - crates/vox-gui/src/commands/pty.rs
  - crates/vox-orchestrator-d/src/term_dispatch.rs
  - crates/vox-term/src/app.rs
  - crates/vox-term/src/vt.rs
  - crates/vox-term/tests/headless.rs
  - crates/vox-term/tests/palette.rs
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
findings:
  critical: 3
  warning: 9
  info: 3
  total: 15
status: issues_found
---

# Phase 441: Code Review Report

**Reviewed:** 2026-11-14T12:43:00Z
**Depth:** standard
**Files Reviewed:** 22
**Status:** issues_found

## Summary

Reviewed all 22 Phase 441 (`stateful-pty-and-lossless-streaming`) files covering non-echo shell launch shims, `Osc633Parser` live streaming, `Utf8StreamDecoder` & CR/CSI-K normalization, `VtShadow` alternate-screen tracking, `LocalHost`/`DaemonHost` `after_seq` delta replay, `vox-orchestrator-d` `term.*` wire dispatch, `vox-gui` PTY/session Tauri commands, and `vox-term` CLI/TUI integration.

The Zsh/Bash `TempDir` launch shims, `Utf8StreamDecoder` tail reassembly, and bounded `LocalHost` replay ring eviction are well-structured, but adversarial analysis surfaced **3 Critical (BLOCKER)** defects, **9 Warnings**, and **3 Info** items:
- **Critical security & shell-shim blockers:** `TermSecurityLevel::allows_agent_command` is trivially bypassed by shell chaining/redirection metacharacters (`!echo ok; rm -rf ...`) now executed directly in a stateful interactive PTY (`CR-01`); the Nushell launch shim uses an invalid 1-arg `{|cmd|}` `pre_execution` closure and broken `$"/(uri_cwd)"` interpolation that fail at runtime/parse time (`CR-02`); and the PowerShell launch shim clobbers `$global:LASTEXITCODE` with `git symbolic-ref`'s exit code (`128` outside git repos) on every prompt while failing to register `PSReadLine` hooks when auto-loaded after `-File` (`CR-03`).
- **Key state & streaming warnings:** Alt-screen transition chunks leak full-screen TUI draws into `Block::plain_output()` (`WR-01`), `vox-term` `apply_session_event` duplicates in-flight streamed output and leaks `DaemonHost` TCP streams/tasks on every block event (`WR-02`), lag recovery drops block/CWD updates and `resync_required` (`WR-03`), `vox-gui` flushes trailing UTF-8 bytes as `U+FFFD` across attach boundaries (`WR-04`), and `PtyHandle` leaks zombie processes and orphaned shells/threads (`WR-05`).

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: Shell command chaining bypasses `TermSecurityLevel::allows_agent_command` and `Agent` submissions can inject stdin into open `User` blocks

**Classification:** BLOCKER
**File:** `crates/vox-terminal-core/src/host.rs:78-148` (and `613-626`)
**Issue:** `TermSecurityLevel::allows_agent_command` is the enforcement gate for `BlockOrigin::Agent` in `LocalHost::submit_with_origin` (`host.rs:598-602`), which in Phase 441 writes `{shell_cmd}\n` directly into a live stateful shell PTY (`host.rs:681`).
1. In `L1ReadOnly` (`lines 104-124`), validation inspects only `lower.split_whitespace().next()` without rejecting shell control, subshell, or redirection operators (`;`, `&&`, `||`, `|`, `` ` ``, `$(`, `\n`, `\r`, `>`, `<`). An agent submission like `"!echo ok; rm -rf src"` or `"!git status && dangerous_cmd"` or `"!echo x > Cargo.toml"` passes `L1ReadOnly` (`first == "echo"` / `"git"`) and executes arbitrary commands in the PTY.
2. In `L2WorkspaceBuild` and `L3StructuredDefault` (`lines 125-144`), space-padded substring checks (`" rm "`, `" sudo "`) are bypassed by `;rm `, `&&rm `, `|rm `, `\nrm `, `rm\t`, or `/bin/rm `.
3. In `submit_with_origin` (`lines 613-626`), if a `BlockOrigin::User` interactive command (`ssh`, `psql`, `python3`, `vim`) is currently open (`active_running_id == Some(running_id)`), an `Agent` call passing `L1ReadOnly` (`"!echo ..."`) writes raw stdin directly into the user's running process and returns `running_id` under `BlockOrigin::User`, erasing agent audit provenance.

**Fix:**
```rust
// In TermSecurityLevel::allows_agent_command (crates/vox-terminal-core/src/host.rs):
if trimmed.contains(['\n', '\r']) {
    return Err(format!("security level {self:?} rejected multi-line agent command"));
}
match self {
    Self::L1ReadOnly => {
        if trimmed.contains([';', '&', '|', '`', '$', '>', '<', '(', ')']) {
            return Err(
                "security level L1ReadOnly forbids shell chaining, subshells, and redirections"
                    .to_string(),
            );
        }
        let first = lower.split_whitespace().next().unwrap_or("");
        // ... existing allowlist check ...
    }
    Self::L2WorkspaceBuild | Self::L3StructuredDefault => {
        // Split on `;`, `|`, `&` and inspect the normalized basename of every segment's command
        for seg in lower.split([';', '|', '&']) {
            let bin = seg
                .split_whitespace()
                .next()
                .unwrap_or("")
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or("");
            // check `bin` against banned mutating/system commands (including `command`/`env` wrappers)
        }
    }
    _ => {}
}

// And in LocalHost::submit_with_origin (lines 613-626), refuse cross-origin stdin injection:
if let Some(running_id) = active_running_id {
    if matches!(origin, BlockOrigin::Agent { .. }) {
        anyhow::bail!("agent cannot inject stdin into an already-running terminal block ({running_id:?})");
    }
    // ...
}
```

### CR-02: Nushell launch shim (`NU_SHIM_CONFIG` and `NU_OSC633`) fails at parse/runtime due to invalid `{|cmd|}` hook parameter and `$"/(uri_cwd)"` command call

**Classification:** BLOCKER
**File:** `crates/vox-terminal-core/src/shell_init.rs:580-597` (and `crates/vox-terminal-core/src/pty.rs:114`)
**Issue:** Three defects break Nushell (`nu`) PTY sessions (undetected on machines without `nu` installed):
1. `shell_init.rs:590` (and `pty.rs:114`): `pre_execution` appends `{|cmd| ... }`. Nushell invokes `pre_execution` hook closures with **zero arguments**, requiring `{|| let cmd = (commandline); ... }`. Declaring a required `cmd` parameter raises `nu::shell::missing_positional` on every command execution so `OSC 633;E` and `OSC 633;C` never fire.
2. `shell_init.rs:580`: `$"/(uri_cwd)"` omits the `$` sigil inside `(...)`. In Nushell string interpolation, `(uri_cwd)` attempts to invoke an external command named `uri_cwd` rather than interpolating `$uri_cwd` (`$"/($uri_cwd)"`), failing static parse or failing at runtime on every Windows prompt (`C:/...`).
3. `shell_init.rs:597`: `PROMPT_INDICATOR` emits `]633;B` (`PromptEnd`) *before* `"> "` instead of after `"> "`, and modern Nushell's default `$env.config.shell_integration.osc633 = true` emits a second overlapping set of `633;A..D` markers unless disabled.

**Fix:**
```nu
const NU_SHIM_CONFIG: &str = r#"# Vox Nushell --config launch shim
$env.__VOX_IN_EXEC = false
$env.config = (
  $env.config
  | upsert shell_integration.osc633 false
  | upsert shell_integration.osc7 false
  | upsert hooks {|c|
      let h = ($c.hooks? | default {})
      $h
      | upsert pre_prompt (
          ($h.pre_prompt? | default []) | append {||
            let ec = ($env.LAST_EXIT_CODE? | default 0)
            if ($env.__VOX_IN_EXEC? | default false) {
              $env.__VOX_IN_EXEC = false
              print -n ([(char -u 1b) "]633;D;" ($ec | into string) (char bel)] | str join)
            }
            let cwd = $env.PWD
            let uri_cwd = ($cwd | str replace -a '\' '/' | url encode)
            let slash_cwd = (if ($uri_cwd | str starts-with '%2F') { $uri_cwd | str replace -a '%2F' '/' } else { $"/($uri_cwd | str replace -a '%2F' '/' | str replace -a '%3A' ':')" })
            let host = (sys host | get hostname | default "localhost")
            let branch = (do -i { ^git symbolic-ref --quiet --short HEAD } | complete | get stdout | str trim)
            print -n ([(char -u 1b) "]7;file://" $host $slash_cwd (char bel)] | str join)
            print -n ([(char -u 1b) "]633;P;Cwd=" $cwd (char bel)] | str join)
            print -n ([(char -u 1b) "]633;P;GitBranch=" $branch (char bel)] | str join)
            print -n ([(char -u 1b) "]633;A" (char bel)] | str join)
          }
        )
      | upsert pre_execution (
          ($h.pre_execution? | default []) | append {||
            $env.__VOX_IN_EXEC = true
            let raw_cmd = (commandline)
            let enc = ($raw_cmd | str replace -a '\' '\x5c' | str replace -a ';' '\x3b' | str replace -a "\n" '\x0a')
            print -n ([(char -u 1b) "]633;E;" $enc (char bel) (char -u 1b) "]633;C" (char bel)] | str join)
          }
        )
    }
)
$env.PROMPT_INDICATOR = {|| (["> " (char -u 1b) "]633;B" (char bel)] | str join) }
"#;
```

### CR-03: PowerShell launch shim (`PWSH_SHIM_INIT`) clobbers `$global:LASTEXITCODE` via `git symbolic-ref` and misses `PSReadLine` hook registration

**Classification:** BLOCKER
**File:** `crates/vox-terminal-core/src/shell_init.rs:522-563`
**Issue:** PowerShell is the canonical Windows shell for this workspace (`default_shell()` returns `"pwsh"` on Windows), and `PWSH_SHIM_INIT` has two critical bugs:
1. **False `Failed (128)` status on PowerShell cmdlets:** Line 533 sets `$global:LASTEXITCODE = 0`, and line 540 *subsequently* executes `git symbolic-ref --quiet --short HEAD 2>$null` (followed by `& $global:__VoxOrigPrompt` on line 545). Outside a git repository (`exit 128`) or on a detached `HEAD` (`exit 1`), `git` sets `$global:LASTEXITCODE = 128` (or `1`). Because PowerShell cmdlets/builtins (`cd`, `echo`/`Write-Output`, `ls`/`Get-ChildItem`, `$env:FOO = ...`) do not touch `$LASTEXITCODE`, the next `prompt` invocation reads `$origLast = 128` on line 524 and emits `]633;D;128` (`line 530-531`), marking every succeeding PowerShell cmdlet block as `BlockStatus::Failed` with `exit_code: Some(128)`.
2. **Skipped `PSReadLine` Enter hook under `-NoExit -File`:** Line 553 checks `if (Get-Module -Name PSReadLine)` without first importing `PSReadLine`. When `pwsh -NoLogo -NoExit -File init.ps1` runs, ConsoleHost normally auto-loads `PSReadLine` *after* `-File` completes unless `$PROFILE` imported it explicitly, so `Get-Module` returns `$null` and `Set-PSReadLineKeyHandler` is skipped (`633;E`, `633;C`, and `$global:__VoxInExec = $true` never fire). In addition, `-Key Enter` binds only `\r`, whereas `LocalHost::submit_with_origin` (`host.rs:681`) sends `{shell_cmd}\n` (`Ctrl+J` on ConPTY).

**Fix:**
```powershell
  function global:prompt {
    $origOk = $?
    $origLast = $global:LASTEXITCODE
    # ... compute $code and emit 633;D;$code when $global:__VoxInExec ...
    # ... run git symbolic-ref and & $global:__VoxOrigPrompt ...
    $out += "$esc]633;B$bel"
    $global:LASTEXITCODE = 0
    return $out
  }

  Import-Module PSReadLine -ErrorAction SilentlyContinue
  if (Get-Module -Name PSReadLine) {
    foreach ($chord in @('Enter', 'Ctrl+j')) {
      Set-PSReadLineKeyHandler -Chord $chord -ScriptBlock {
        $line = $null; $cursor = $null
        [Microsoft.PowerShell.PSConsoleReadLine]::GetBufferState([ref]$line, [ref]$cursor)
        $global:__VoxInExec = $true
        $enc = $line -replace '\\','\x5c' -replace ';','\x3b' -replace "`r",'' -replace "`n",'\x0a'
        [Console]::Write("$([char]27)]633;E;$enc$([char]7)$([char]27)]633;C$([char]7)")
        [Microsoft.PowerShell.PSConsoleReadLine]::AcceptLine()
      }
    }
  }
```

## Warnings

### WR-01: `Session::apply_osc` leaks full-screen TUI draw bytes into `Block.output` on `ALT_SCREEN` entry, exit, and single-chunk round-trips

**Classification:** WARNING
**File:** `crates/vox-terminal-core/src/session.rs:426-448`
**Issue:** Line 438 checks `if !(was_alt && now_alt)` across the entire `Osc633Event::Output(text)` chunk. When `vim`, `less`, or `htop` starts, its initial `write()` bundles `\x1b[?1049h` with up to 4 KiB of full-screen UI drawing (`was_alt == false`, `now_alt == true` -> `!(false && true) == true`). The same happens on teardown (`was_alt == true`, `now_alt == false` -> `true`) and when `?1049h ... ?1049l` both appear in one read (`was_alt == false`, `now_alt == false` -> `true`), polluting `Block::plain_output()` and `redacted_context_bundle` with TUI screen frames.
**Fix:** Have `VtShadow` yield primary-screen byte ranges while advancing through `ESC`-segmented slices so only segments processed while `!self.vt_shadow.is_alt_screen()` are pushed to `Block.output`:
```rust
let mut primary_text = String::new();
let bytes = text.as_bytes();
let mut start = 0;
for idx in 1..=bytes.len() {
    if idx == bytes.len() || bytes[idx] == b'\x1b' {
        let seg = &text[start..idx];
        let was_seg_alt = self.vt_shadow.is_alt_screen();
        if let Some(active) = self.vt_shadow.feed(seg.as_bytes()) {
            let _ = self.tx.send(SessionEvent::AltScreenChanged { active });
        }
        if !was_seg_alt && !self.vt_shadow.is_alt_screen() {
            primary_text.push_str(seg);
        }
        start = idx;
    }
}
if !primary_text.is_empty() && self.in_exec && let Some(id) = self.open_id {
    let chunk = OutputChunk::text(Stream::Stdout, primary_text);
    // append and broadcast OutputAppended ...
}
```

### WR-02: `vox-term` `apply_session_event` duplicates streamed block output and leaks `DaemonHost` TCP streams/tasks on every block event

**Classification:** WARNING
**File:** `crates/vox-term/src/app.rs:175-190` (and `crates/vox-terminal-core/src/daemon_host.rs:220-276`)
**Issue:**
1. **Duplicated output:** `app.rs:448` calls `host.submit(&sid, &line).await` (which blocks up to `1500ms` while output chunks stream into `Session.blocks` and `att.events`) without inserting the new block into `blocks_state`. On the next loop iteration, `apply_session_event` processes `BlockOpened { id }`, calls `host.attach(sid, Some(last_seq)).await`, and sets `*blocks_state = snap.blocks` (which *already* contains all output from the first `1500ms`). The drain loop at `app.rs:316` then immediately processes all buffered `OutputAppended { id, chunk }` events from those same `1500ms`, pushing every chunk onto `block.output` a second time.
2. **Leaked daemon streams:** Calling `DaemonHost::attach` inside `apply_session_event` on every `BlockOpened` and `BlockClosed` spawns a new `subscribe_with_method` TCP stream + `pump_with_resubscribe` task and immediately drops `snap`'s receivers (`app.rs:183, 189`). Because `pump_with_resubscribe` (`daemon_host.rs:221, 273`) parks on `rx.recv().await` and only checks `receiver_count() == 0` *after* another frame arrives, idle sessions leak open TCP sockets and Tokio tasks on every completed command.

**Fix:** Include block metadata directly on `SessionEvent::BlockOpened` (input, origin, cwd, git_branch) and `SessionEvent::BlockClosed` (exit_code, status — note `term_dispatch.rs:120` already serializes `"block"` on `"block_closed"`) so `apply_session_event` updates `blocks_state` in-place without calling `host.attach`. In `daemon_host.rs:220`, race `rx.recv()` against `events.closed()` + `output.closed()` in `tokio::select!` so dropped `Attachment`s abort `stream` immediately.

### WR-03: Broadcast lag recovery in `DaemonHost` and `vox-gui` drops `Block` / `Cwd` updates and ignores `resync_required`

**Classification:** WARNING
**File:** `crates/vox-terminal-core/src/daemon_host.rs:224-267` (and `crates/vox-gui/src/commands/pty.rs:294-338`)
**Issue:**
1. When `term_dispatch.rs` emits `{"type": "lagged"}` (because either `output` or `events` overflowed its 256-item broadcast buffer), `DaemonHost::pump_with_resubscribe` fetches a new `"snapshot"` via `after_seq = last_seen_seq`, but discards `snap["blocks"]`, `snap["cwd"]`, `snap["git_branch"]`, and `snap["resync_required"]`. Clients (`vox-term` and `vox-gui`) never observe `Lagged` locally, leaving blocks stuck in `Running` if `BlockClosed` fell in the lag window and appending a 2 MiB snapshot onto a dirty VT grid without reset when `resync_required == true`. Also, if `events` lagged without `seq` advancing (`snap_seq == last_seen_seq`), `consecutive_lags` is never reset (`line 261`), terminating the stream after 8 event bursts.
2. In `crates/vox-gui/src/commands/pty.rs:336-338`, `Err(RecvError::Lagged(_))` on `att.events` executes `continue;` with zero state recovery, dropping `BlockClosed`/`AltScreenChanged`/`CwdChanged` events even on `LocalHost`.

**Fix:** Add a `SessionEvent::SnapshotResync { blocks, resync_required, alt_screen, cwd, git_branch }` variant emitted by `pump_with_resubscribe` (and reset `consecutive_lags = 0` whenever a valid `"snapshot"` frame is received, with a brief backoff if `snap_seq == last_seen_seq`), and re-attach + emit full state sync on both `output` and `events` `Lagged` branches in `vox-gui/src/commands/pty.rs`.

### WR-04: `vox-gui` `term_session_attach` corrupts multi-byte UTF-8 sequences split across the snapshot-to-live-stream seam

**Classification:** WARNING
**File:** `crates/vox-gui/src/commands/pty.rs:62-67` (and `267-285`)
**Issue:** `term_attachment_dto_from_attachment` calls `decode_replay_bytes(&att.replay)`, which invokes `dec.flush_lossy()` (`line 65`), and `term_session_attach` then creates a separate `Utf8StreamDecoder::new()` (`line 277`) for live frames starting at `att.seq + 1`. If `att.replay` ends mid-UTF-8 codepoint (e.g. bytes `0..2` of a 4-byte emoji at the end of frame `att.seq`), `flush_lossy()` emits `U+FFFD` in `dto.replay_utf8` and the fresh decoder emits `U+FFFD`s for the continuation bytes on frame `att.seq + 1`.
**Fix:** Seed the forwarder's `Utf8StreamDecoder` directly from `att.replay` without calling `flush_lossy()` when attaching to a live session:
```rust
let mut decoder = vox_terminal_core::Utf8StreamDecoder::new();
let replay_utf8 = decoder.push(&att.replay);
let mut dto = term_attachment_dto_from_attachment(&session_id, &att);
dto.replay_utf8 = replay_utf8;
// Move `decoder` into the `tokio::spawn` forwarder loop (and recreate `decoder` on lag reattach if `reattached.resync_required`).
```

### WR-05: `PtyHandle` never reaps killed child shells and keeps `MasterPty` alive in the reader thread, leaking zombies and orphaned shells

**Classification:** WARNING
**File:** `crates/vox-terminal-core/src/pty.rs:322-380` (and `423-445`, `crates/vox-terminal-core/src/host.rs:428-446, 775-786`)
**Issue:**
1. `PtyHandle::kill` (`pty.rs:377`) calls `self.child.kill()` without `wait()`/`try_wait()`. When `LocalHost::close` (`host.rs:779-782`) or `vox-gui`'s `pty_close`/`pty_spawn` (`pty.rs:179, 226`) kills and drops an idle session, `on_eof` skips `try_wait_exit_code` (`needs_exit_code` is `false` when no block is open), leaving an unreaped `<defunct>` zombie child process in `vox-orchestrator-d` and `vox-gui` on Unix for every closed terminal tab.
2. `"vox-pty-reader"` holds `let _keep_master = reader_master;` (`pty.rs:428`), and `PtyHandle` has no `Drop` impl. When `LocalHost` or `PtyHandle` is dropped without `close()` (such as pressing `Ctrl-C` to exit `vox-term --local` at `app.rs:436`), the master PTY FD remains open inside `"vox-pty-reader"`, preventing `SIGHUP`/`EOF` delivery to the child shell and leaking both the OS thread and child process.

**Fix:** Remove `reader_master` from `"vox-pty-reader"` (`reader` already holds its cloned FD), and implement `Drop for PtyHandle` to kill and reap the child process:
```rust
impl Drop for PtyHandle {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
```

### WR-06: `Osc633Parser` swallows following markers when an unterminated OSC sequence is interrupted by a new `ESC`

**Classification:** WARNING
**File:** `crates/vox-terminal-core/src/osc633.rs:240-258`
**Issue:** `extract_osc_payload` scans until `BEL` (`0x07`) or `ST` (`\x1b\\`), ignoring bare `0x1b` bytes followed by anything other than `\` (as well as `CAN`/`SUB` `0x18`/`0x1a`). Under ECMA-48 / VT500, any non-`ST` `ESC` aborts the current OSC string and begins a new escape sequence. If a command or prompt is interrupted mid-OSC (e.g. partial `\x1b]7;file://host/dir` before `SIGINT`), `extract_osc_payload` consumes across the subsequent `\x1b]633;D;130\x07` marker, corrupting the earlier payload and swallowing `Exit(130)` (or hoarding up to `16 KiB` of output if no `BEL` follows).
**Fix:** Detect embedded `0x1b` (when `buf.get(i + 1)` is `Some(&b)` with `b != b'\\'`) or `0x18`/`0x1a` inside `parse()` / `extract_osc_payload`, emit `buf[abs_start..abs_start + i]` as plain output, and restart OSC scanning at `abs_start + i`.

### WR-07: `VtGrid` ignores legacy `?47` / `?1047` alt-screen modes (reverting `alt_screen` on frame 2 in `vox-term`) and misaligns wide UTF-8 characters

**Classification:** WARNING
**File:** `crates/vox-term/src/vt.rs:70-87` (and `108-115`, `crates/vox-term/src/app.rs:290-293`)
**Issue:**
1. `VtShadow` (`vt_shadow.rs:41-63`) maps legacy `?47h`/`?47l` and `?1047h`/`?1047l` onto `TermMode::ALT_SCREEN`, but `VtGrid::feed` (`vt.rs:70-72`) uses a plain `vte::Processor` without `LegacyAltSync`, so `grid.is_alt_screen()` stays `false`. Because `app.rs:292` unconditionally sets `alt_screen = grid.is_alt_screen();` on every `OutputFrame`, any TUI using `?47h` or `?1047h` enters `alt_screen` on `AltScreenChanged(true)` and immediately drops back out to block view on its very next output frame.
2. In `VtGrid::render_lines` (`vt.rs:108-115`), `Flags::WIDE_CHAR_SPACER` trailing cells after 2-column emojis/CJK glyphs (`🦀`, `🚀`) are pushed as `' '`, causing ratatui to advance 3 terminal columns per wide glyph and breaking box-drawing alignment.

**Fix:** Apply the same `LegacyAltSync` handler in `VtGrid::feed` (or move `LegacyAltSync` into `vox-terminal-core::vt_shadow` as a reusable helper), and skip `WIDE_CHAR_SPACER` cells in `VtGrid::render_lines`:
```rust
let cell = &self.term.grid()[Point::new(line, Column(col))];
if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
    continue;
}
```

### WR-08: `vox-term` event loop processes `KeyEventKind::Release` as duplicate keystrokes on Windows

**Classification:** WARNING
**File:** `crates/vox-term/src/app.rs:415-459`
**Issue:** `crossterm::event::read()` on Windows emits both `KeyEventKind::Press` and `KeyEventKind::Release` for every key press. Because `run_with_args` matches `Event::Key(...)` without checking `key.kind`, every character in `InputBox` and `alt_screen` pass-through is typed/sent twice on Windows, `Enter` submits twice, and `Ctrl+C` release can immediately detach `vox-term` if a running block closed on the press event.
**Fix:** Filter out `KeyEventKind::Release` before matching key bindings:
```rust
Event::Key(key) if key.kind == crossterm::event::KeyEventKind::Release => continue,
```

### WR-09: Fish launch shim drops newlines in multi-line `OSC 633;E` commands and `633;P;Cwd` is emitted unescaped across shims

**Classification:** WARNING
**File:** `crates/vox-terminal-core/src/shell_init.rs:478` (and `319, 382, 471, 539, 584`)
**Issue:**
1. In `FISH_SHIM_INIT` (`line 478`), `string replace -a '\\' '\\x5c' -- "$argv[1]" | string replace -a ';' '\\x3b' | string replace -a \n '\\x0a'` pipes multi-line commands across `string replace`, which reads stdin line-by-line without trailing `\n`. As a result, `string replace -a \n '\\x0a'` never matches, command substitution splits the output into a Fish list, and `"$cmd"` joins lines with spaces instead of `\x0a`.
2. In all 5 shims (`lines 319, 382, 471, 539, 584`), `633;P;Cwd=%s` prints raw `$PWD` without escaping control bytes (`\x07`, `\x1b`, `\n`), allowing directory names containing `BEL` or `ESC` to terminate `OSC 633;P` prematurely and inject raw escape sequences (and overwrite the safely URL-decoded `OSC 7` CWD).

**Fix:** In Fish (`line 478`), join multi-line elements explicitly via `string join '\\x0a'`:
```fish
set -l cmd (string split \n -- "$argv[1]" | string replace -a '\\' '\\x5c' | string replace -a ';' '\\x3b' | string join '\\x0a')
```
And either rely on `OSC 7` for `Cwd` or escape `\`, `;`, `\n`, `\x07`, and `\x1b` in `633;P;Cwd` values (matching `decode_command` in `osc633.rs:193`).

## Info

### IN-01: Legacy `shell_integration_snippet` and five `*_OSC633` constants in `pty.rs` are now dead duplicate code

**Classification:** INFO
**File:** `crates/vox-terminal-core/src/pty.rs:33-173`
**Issue:** After migrating `spawn_pty_with_callback` to `shell_init::prepare_shell_command`, `shell_integration_snippet` and `ZSH_OSC633`/`FISH_OSC633`/`NU_OSC633`/`PWSH_OSC633`/`BASH_OSC633` have zero production callers in the workspace (referenced only by tests) and contain stale defects (e.g. unhooked `__vox_trappost` in `ZSH_OSC633` at `line 76` and `{|cmd|}` in `NU_OSC633` at `line 114`).
**Fix:** Retire `shell_integration_snippet` and its duplicate constants (or delegate to `shell_init`) and update the unit tests in `pty.rs` and `vox-gui/src/commands/pty.rs`.

### IN-02: `LocalHost::submit_with_origin` writes `/slash` and `?agent` commands to PTY stdin when a block is running

**Classification:** INFO
**File:** `crates/vox-terminal-core/src/host.rs:613-626`
**Issue:** Because `if let Some(running_id) = active_running_id` runs before `classify(line)`, submitting `/help` or `?ask` while a >1.5s shell command is running writes `"/help\n"` or `"?ask\n"` into the PTY's stdin queue, where the shell executes it after the foreground command exits.
**Fix:** Run `classify(line)` first and dispatch `InputIntent::Command` and `InputIntent::Agent` directly rather than forwarding them to PTY stdin.

### IN-03: `vox term --attach` silently falls back to an empty `LocalHost` when the daemon is unreachable, and `--attach --local` cannot succeed

**Classification:** INFO
**File:** `crates/vox-term/src/app.rs:254-267` (and `crates/vox-cli/src/lib.rs:727-739`)
**Issue:** Combining `--attach` with `--local` (or running `vox term --attach` when `vox-orchestrator-d` is unreachable) creates a brand-new empty `LocalHost` and fails with `"no active terminal sessions to attach to; run vox term without --attach to open one"`, masking that `vox-orchestrator-d` was unreachable or that `--local` sessions cannot be shared across processes.
**Fix:** Mark `--attach` as `conflicts_with = "local"` on `Cli::Term` (`crates/vox-cli/src/lib.rs:730`) and return an explicit error in `run_with_args` when `args.attach.is_some()` and either `args.force_local` is set or `vox-orchestrator-d` could not be reached.

---

_Reviewed: 2026-11-14T12:43:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
