# Phase 441: Zero-Spam Stateful PTY Engine, Non-Echo Shell Bootstrap & Lossless VT/OSC Streaming - Research

**Researched:** 2026-11-14
**Domain:** Rust stateful PTY management (`portable-pty`), non-echo multi-shell `OSC 633`/`OSC 7` bootstrap (`tempfile`), lossless UTF-8 & VT shadow streaming (`alacritty_terminal`), and daemon-hosted CLI ↔ GUI session attach (`vox-terminal-core`, `vox-term`, `vox-orchestrator-d`, `vox-cli`, `vox-gui`)
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Shell Launch Shims, Prompt Hygiene & Bootstrap (`TERM-PTY-01`)
- Source user dotfiles first inside a `tempfile::TempDir` launch shim in `crates/vox-terminal-core/src/shell_init.rs`, then install Vox `OSC 633` + `OSC 7` hooks (`ZDOTDIR` `.zshrc` shim sources `~/.zshrc`, `bash --rcfile` sources `~/.bashrc`, `pwsh -NoExit -File` loads `$PROFILE` then shim, `fish --init-command`, `nu --config`) — never write hook source lines to PTY stdin (`ECHO` stays clean and shell history stays unpolluted).
- In the `zsh` `ZDOTDIR/.zshrc` shim: `unsetopt PROMPT_SP` to eliminate macOS `%` inverse startup noise, wrap `633;A` and `633;B` prompt escapes inside `%{...%}` so `zsh` prompt width calculation never drifts, and wire `precmd`/`preexec` via `autoload -Uz add-zsh-hook` so `$?` is captured before any hook resets it (`false` → `1`, `exit 7` → `7`).
- Store `Option<tempfile::TempDir>` directly on `PtyHandle` so RAII drops the shim directory cleanly when the session closes without leaking `/tmp/vox-shell-*` directories or racing subshell startup.
- Emit both standard `OSC 7` (`file://<host>/<cwd>`) and `OSC 633;P;Cwd=<path>` plus lightweight `OSC 633;P;GitBranch=<branch>` on prompt boundaries so `Osc633Parser`, `alacritty_terminal`, and downstream GUI chips stay synchronized.

#### Persistent PTY Command Execution, Signals & Resize Semantics (`TERM-PTY-02`)
- When a `Session` is backed by a live `PtyHandle` (`LocalHost` / `Session::with_pty`), `Session::submit` writes `<line>\n` directly to the persistent PTY master writer and tracks command start/output/completion/exit-code via `OSC 633` (`A`/`B`/`E`/`C`/`D;exit`) so `cd`, `export`, aliases, and virtualenvs persist across commands; retain synchronous `run_shell_capture` ONLY for explicit stateless unit-test constructors (`Session::new_stateless` / `Session::new`).
- Deliver `SIGINT` in `PtyHandle::signal` / `LocalHost::signal` by writing `\x03` (`ETX`) to the PTY master writer so Unix line discipline and Windows ConPTY interrupt the foreground job without killing the interactive shell; use `portable_pty::Child::kill()` for `SIGTERM`/`SIGKILL` or session teardown.
- When a foreground block is already `Running`, forward subsequent `submit` / `send_input` bytes directly to PTY stdin of the active `Running` block (enabling interactive `[y/N]` answers and REPL input) rather than spawning an overlapping `Running` block.
- Honor `OpenSpec { cols, rows }` at `spawn_pty` time (defaulting to `120×32` when unset instead of hardcoded `80×24`), retain `Arc<Mutex<Box<dyn MasterPty + Send>>>` inside `PtyHandle`, and propagate `PtyHandle::resize(cols, rows)` synchronously to both `MasterPty::resize(PtySize { rows, cols, .. })` and the `alacritty_terminal` shadow grid.

#### Lossless Streaming, `ALT_SCREEN` Normalization & `vox term --attach` Continuity (`TERM-PTY-03`, `TERM-PTY-04`)
- Buffer up to 3 trailing incomplete UTF-8 lead/continuation bytes across `read()` boundaries (`utf8_stream.rs` / reader loop) so arbitrary prime-sized byte chunks decode with zero `U+FFFD` replacement characters; emit live raw UTF-8 chunks on `OutputFrame` while normalizing carriage-return overwrites (`\r` without `\n`, e.g. spinners/`cargo build` progress bars) inside `Block` text snapshots.
- Maintain a bounded `seq`-indexed `VecDeque<OutputFrame>` (up to 2 MiB byte budget) in `LocalHost` so `attach(session_id, after_seq: Option<u64>)` replays only frames with `seq > after_seq` (setting `resync_required: true` + full snapshot only when `after_seq` is `None` or older than the oldest retained `seq`), recovering cleanly from `broadcast::RecvError::Lagged`.
- Embed an `alacritty_terminal::Term` shadow state machine (`vt_shadow.rs` in `vox-terminal-core`) fed by the PTY reader stream to track `TermMode::ALT_SCREEN` (`?1049h`/`?1049l`, `?47h`, `?1047h`) transitions, emit `SessionEvent::AltScreenChanged { active: bool }` (and surface `alt_screen: bool` on `SessionInfo` / `Attachment`), and suppress command-block splitting while `alt_screen == true`.
- Add `--attach <Option<String>>` (supporting explicit `<session_id>` or `latest`) and `--local` flags to `Cli::Term` in `crates/vox-cli`, pass `TermCliArgs` into `vox_term::app::run_with_args`, attach to the shared daemon session via `resolve_host(args.force_local)` (updating `daemon_host.rs` and `vox-orchestrator-d` `term.*` handlers for `after_seq` + `alt_screen` + `cwd`/`git_branch` parity), and fail with an explicit actionable error when a requested `<session_id>` does not exist.

### Claude's Discretion
- Exact module split inside `crates/vox-terminal-core/src/` (`shell_init.rs`, `utf8_tail.rs`, `vt_shadow.rs`) provided public re-exports in `lib.rs` remain clean and backward-compatible with `vox-gui` and `vox-term` callers.

### Deferred Ideas (OUT OF SCOPE)
- AST-backed `L1–L5` `ExecPolicyV1` and MCP `vox_terminal_*` tools — Phase 442.
- Frontend `<TerminalSessionPane>` `after_seq` consumer migration and `@xterm/addon-webgl` — Phase 443.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| `TERM-PTY-01` | Operator opens a terminal session in `zsh`, `bash`, `fish`, `pwsh`, or `nu` with zero echoed `OSC 633` startup hook lines, unpolluted shell history, accurate non-zero `$?`/`$LASTEXITCODE` reporting (fixing unhooked `__vox_trappost` and macOS `PROMPT_SP` `%`), and live `OSC 7` / `OSC 633;P;Cwd` directory updates via `tempfile::TempDir` launch shims. | `shell_init.rs` builds per-shell `TempDir` launch shims (`ZDOTDIR`, `--rcfile`, `--init-command`, `-NoLogo -NoExit -File`, `--config`), prepends `$?` capture before user `precmd` hooks + adds `EXIT` trap for top-level `exit N`, unsets `PROMPT_SP`, wraps `zsh` escapes in `%{...%}`, and extends `Osc633Parser` for `633;P;Cwd=`, `633;P;GitBranch=`, `633;B`, and `OSC 7`. |
| `TERM-PTY-02` | Operator and agent shell submissions execute inside the persistent stateful `PtyHandle` (preserving `cd`, exported env vars, virtualenvs, and interactive stdin across commands), with real `MasterPty::resize` (`SIGWINCH` / ConPTY `ResizePseudoConsole`) and structured `SIGINT` (`term.signal`) cancellation. | `PtyHandle` retains `Arc<Mutex<Box<dyn MasterPty + Send>>>` + `Option<tempfile::TempDir>`; `LocalHost` pumps bytes on a dedicated OS reader thread with `Condvar` fast-path settle so `Session::submit` writes `<cmd>\n` to the live PTY, forwards subsequent input into an already-`Running` block, resizes `MasterPty` + `VtShadow`, and handles `TermSignal::Interrupt` (`\x03`) via `term.signal`. |
| `TERM-PTY-03` | Session byte streaming reassembles split multi-byte UTF-8 sequences across 4096-byte PTY reads without `U+FFFD` corruption, tracks `alacritty_terminal` `TermMode::ALT_SCREEN` (`?1049h`/`?1049l`) + `\r` progress-bar line overwrites, and streams `after_seq` deltas with automatic resync on broadcast lag (eliminating full 2 MiB `replay_utf8` re-writes). | `utf8_stream.rs` holds `0..=3` incomplete trailing UTF-8 bytes via `std::str::from_utf8` `error_len() == None`; `Osc633Parser` streams live `Output` chunks between `633;C` and `633;D` instead of hoarding until `D`; `Block` normalizes `\r` column-0 / `\x1b[K` overwrites while preserving split `\r\n`; `vt_shadow.rs` tracks `TermMode::ALT_SCREEN`; `Replay` stores `VecDeque<OutputFrame>` capped at `2 MiB` with `after_seq` delta slicing. |
| `TERM-PTY-04` | Operator attaches to any live daemon-hosted terminal session from CLI `vox term --attach <id>` and Axis GUI simultaneously via `SessionHost` (`resolve_session_host`) with live block and PTY byte continuity. | `Cli::Term { attach, local }` in `vox-cli` awaits `vox_term::app::run_with_args(TermCliArgs)` directly inside `vox-cli`'s active Tokio runtime; `vox-term` connects via `resolve_host`, resolves `<session_id>` or `latest` (with explicit error on missing ID), renders `Attachment` blocks/VT grid live, and `DaemonHost` + `TermDispatch` carry `after_seq`, `resync_required`, `alt_screen`, `cwd`, `git_branch`, and `term.signal`. |
</phase_requirements>

## Project Constraints (from CLAUDE.md / AGENTS.md)

- **Zero new external crates & zero new workspace Cargo edges:** `alacritty_terminal = { version = "0.26" }`, `portable-pty = "0.9"`, and `tempfile = "3"` are already declared in root `[workspace.dependencies]` (`[VERIFIED: Cargo.toml:227-228,253]`) and resolved in `Cargo.lock` (`alacritty_terminal 0.26.0`, `portable-pty 0.9.0`, `tempfile 3.27.0` — `[VERIFIED: Cargo.lock:159-160,10460-10461,14496-14497]`). Adding `alacritty_terminal = { workspace = true }` and `tempfile = { workspace = true }` to `crates/vox-terminal-core/Cargo.toml` adds zero new `vox-*` edges in `contracts/ci/crate-graph.v1.json` (`[VERIFIED: contracts/ci/crate-graph.v1.json:931-937]`), though Cargo updates `vox-terminal-core`'s internal `dependencies` list in `Cargo.lock` (`[VERIFIED: Cargo.lock:19240-19256]`) — **any task editing `crates/vox-terminal-core/Cargo.toml` MUST list `Cargo.lock` in its `<files>` block** to comply with `.agents/rules/00-core.md` Rule 1 & Rule 5.
- **Layer & `vox-foundation` boundary:** `vox-terminal-core` is Layer 3 (`vox-terminal-core = { layer = 3, max_loc = 25_000, staleness_exempt = true, orphan_exempt = true }` — `[VERIFIED: docs/src/architecture/layers.toml:272]`) and has **no** dependency on `vox-foundation`. Keep wire-method string constants mirrored in `crates/vox-terminal-core/src/daemon_host.rs` and matched in `crates/vox-orchestrator-d/src/term_dispatch.rs` (the same pattern used by `"term.context"`, `"term.security"`, and `"term.control"` at `[VERIFIED: crates/vox-orchestrator-d/src/term_dispatch.rs:188-212]`) so `L0` `vox-foundation` is untouched.
- **Formatting & Build Discipline (`.agents/rules/00-core.md`):** Format changed Rust files exclusively via `rustfmt --edition 2024 <file.rs>` (never `cargo fmt`). Scope all builds/tests with `timeout 300s cargo test -p <crate>` (never `cargo test --workspace`), and emit one shell command per tool invocation without `|`, `&&`, `;`, or `||`.
- **Wire Security Invariant:** `term.*` methods in `vox-orchestrator-d` are loopback/stdio-only and fail-closed on `origin != "user"` (`refuse_agent_origin` in `[VERIFIED: crates/vox-orchestrator-d/src/term_dispatch.rs:90-97]`). Preserve `refuse_agent_origin` on `term.input`, `term.submit`, and the new `term.signal`.

## Summary

Auditing `crates/vox-terminal-core`, `crates/vox-term`, `crates/vox-orchestrator-d`, `crates/vox-cli`, and `crates/vox-gui` confirms every root cause behind Phase 441's target defects. Today, `spawn_pty` (`[VERIFIED: crates/vox-terminal-core/src/pty.rs:367-372]`) writes multi-line `OSC 633` shell functions directly into the PTY master writer while kernel `ECHO` is active, dumping raw hook code into `replay` and shell history; `ZSH_OSC633` defines `__vox_trappost() { __vox_ec=$? }` (`[VERIFIED: crates/vox-terminal-core/src/pty.rs:73]`) without registering it, forcing `$?` to `0` on every failed command; `PtyHandle::resize` is a literal no-op (`let _ = (cols, rows); Ok(())` — `[VERIFIED: crates/vox-terminal-core/src/pty.rs:319-324]`) because `pair.master` was moved into the reader thread; `Session::submit_with_origin` (`[VERIFIED: crates/vox-terminal-core/src/session.rs:78-88]`) spawns a fresh `std::process::Command` via `run_shell_capture` on every line so `cd` and `export` evaporate; and `Osc633Parser` both corrupts split multi-byte UTF-8 chunks via `String::from_utf8_lossy` (`[VERIFIED: crates/vox-terminal-core/src/osc633.rs:84,112,146]`) and buffers 100% of command output inside `self.output_buf` until `633;D` arrives (`[VERIFIED: crates/vox-terminal-core/src/osc633.rs:80-81,109-110,144-152]`), preventing live streaming during execution.

To solve all four requirements cleanly without breaking existing callers (`vox-gui`, `vox-orchestrator-d`, `vox-term`, and `real_backend_harness`), Phase 441 should introduce three focused modules in `crates/vox-terminal-core/src/` — `shell_init.rs` (`TempDir` non-echo shims for `zsh`, `bash`, `fish`, `pwsh`, `nu`), `utf8_stream.rs` (3-byte trailing UTF-8 reassembler + `\r` line-overwrite normalizer), and `vt_shadow.rs` (`alacritty_terminal::Term` `scrolling_history: 0` shadow state tracking `TermMode::ALT_SCREEN`) — while refactoring `PtyHandle`, `Osc633Parser`, `Session`, `LocalHost`, `DaemonHost`, `TermDispatch`, `Cli::Term`, and `vox_term::app::run_with_args`.

**Primary recommendation:** Pump PTY bytes into `LocalHost`'s `Entry` directly on the dedicated OS reader thread (with a `Condvar` fast-path settle in `LocalHost::open` and `LocalHost::submit`) rather than through `tokio::spawn`, so stateful PTY commands preserve `cd`/`export` across calls, stream live `OutputAppended` events, never deadlock single-threaded `#[tokio::test]` callers, and keep every existing `SessionHost` integration test green.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Non-echo 5-shell launch shims (`TempDir`, `OSC 633`, `OSC 7`, `GitBranch`) | `vox-terminal-core` (`L3`: `shell_init.rs`, `pty.rs`) | — | Shell startup, `CommandBuilder` env/argv setup, and `TempDir` RAII lifetime belong exclusively with `PtyHandle`. |
| UTF-8 chunk reassembly, `Osc633Parser` live streaming & `\r` overwrite normalization | `vox-terminal-core` (`L3`: `utf8_stream.rs`, `osc633.rs`, `block.rs`) | — | Ensures both `OutputFrame` consumers (GUI/TUI) and `Block` text snapshots receive valid UTF-8 and progress-normalized lines from one SSOT. |
| `ALT_SCREEN` detection & block-split suppression | `vox-terminal-core` (`L3`: `vt_shadow.rs`, `session.rs`) | `vox-term` (`L4`) / `vox-gui` (`L5`) | Backend shadow `alacritty_terminal::Term` tracks `TermMode::ALT_SCREEN` authoritatively even while zero front-ends are attached. |
| Persistent PTY `submit`, `signal` (`\x03`), `MasterPty::resize`, & `2 MiB` `after_seq` frame ring | `vox-terminal-core` (`L3`: `host.rs`, `daemon_host.rs`) | `vox-orchestrator-d` (`L5`: `term_dispatch.rs`) | `LocalHost` owns the single in-memory session state; `TermDispatch` and `DaemonHost` project it 1:1 over loopback JSON-RPC. |
| CLI `--attach [<id>\|latest]` / `--local` & live ratatui attach loop | `vox-cli` (`L5`) + `vox-term` (`L4`: `app.rs`) | `vox-gui` (`L5`: `commands/pty.rs`) | `vox-cli` parses clap flags and awaits `vox_term::app::run_with_args`; `vox-gui` adapts updated `SessionHost::attach(_, None)` signature cleanly. |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `portable-pty` | `0.9.0` | Cross-platform Unix PTY (`TIOCSWINSZ`) + Windows ConPTY (`ResizePseudoConsole`), `CommandBuilder`, `MasterPty`, `Child` | Already pinned in workspace (`portable-pty = "0.9"` — `[VERIFIED: Cargo.toml:228]`, `version = "0.9.0"` — `[VERIFIED: Cargo.lock:10460-10461]`). |
| `alacritty_terminal` | `0.26.0` | Full VT100/VT220/xterm parser (`vte::ansi::Processor`) + `TermMode::ALT_SCREEN` (`?1049h`/`?1049l`, `?47h`, `?1047h`) state machine | Already pinned in workspace and used in `vox-term` (`alacritty_terminal = { version = "0.26" }` — `[VERIFIED: Cargo.toml:227]`, `version = "0.26.0"` — `[VERIFIED: Cargo.lock:159-160]`). |
| `tempfile` | `3.27.0` | Secure `0700` per-session temporary directory (`TempDir`) with deterministic `Drop` cleanup for shell launch shims | Already pinned in workspace (`tempfile = "3"` — `[VERIFIED: Cargo.toml:253]`, `version = "3.27.0"` — `[VERIFIED: Cargo.lock:14496-14497]`). |
| `tokio` | `1.x` | `broadcast` fan-out channels, `mpsc` daemon stream pump, and async `SessionHost` trait runtime | Already wired in `vox-terminal-core`, `vox-term`, `vox-orchestrator-d`, and `vox-cli` (`[VERIFIED: crates/vox-terminal-core/Cargo.toml:15]`). |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `base64` | `0.22.1` | Binary-safe `replay_b64` and `bytes_b64` framing over JSON-line `term.*` RPCs | Already used in `daemon_host.rs` and `term_dispatch.rs` (`[VERIFIED: crates/vox-terminal-core/Cargo.toml:19]`). |
| `ratatui` + `crossterm` | `0.30` / `0.29` | Interactive TUI rendering and raw terminal event polling for `vox term --attach` | Already wired in `crates/vox-term/Cargo.toml` (`[VERIFIED: crates/vox-term/Cargo.toml:18-19]`). |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `alacritty_terminal::Term` (`scrolling_history: 0`) for `ALT_SCREEN` | Custom regex/byte scanner for `\x1b[?1049h` | Rejected (locked decision): hand-rolled CSI scanners miss compound DECSET sequences (`\x1b[?1049;2004h`), `?47h`, `?1047h`, and `RIS` (`\x1bc`) resets that `alacritty_terminal` handles natively. |
| `tempfile::TempDir` launch shims | Writing `stty -echo` + hooks to PTY stdin | Rejected (locked decision): races fast typeahead, still echoes `stty -echo` itself on initial read, and pollutes shell history. |

**Manifest addition (`crates/vox-terminal-core/Cargo.toml` only — zero external fetches):**
```toml
alacritty_terminal = { workspace = true }
tempfile           = { workspace = true }
```

## Package Legitimacy Audit

Zero new external packages are added to the workspace in Phase 441. All crates wired into `crates/vox-terminal-core/Cargo.toml` are already approved workspace dependencies resolved in `Cargo.lock`:

| Package | Registry | Resolved Version | Workspace Source | Verdict | Disposition |
|---------|----------|------------------|------------------|---------|-------------|
| `portable-pty` | crates.io | `0.9.0` | `Cargo.toml:228` / `Cargo.lock:10460` | `[OK]` | Approved (already in `vox-terminal-core`) |
| `alacritty_terminal` | crates.io | `0.26.0` | `Cargo.toml:227` / `Cargo.lock:159` | `[OK]` | Approved (already in `vox-term`; add `{ workspace = true }` to `vox-terminal-core`) |
| `tempfile` | crates.io | `3.27.0` | `Cargo.toml:253` / `Cargo.lock:14496` | `[OK]` | Approved (already in workspace; add `{ workspace = true }` to `vox-terminal-core`) |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```text
  Cli::Term { --attach, --local }                Axis GUI (pty.rs / Console)
               │                                              │
               ▼                                              ▼
     vox_term::app::run_with_args              term_session_{open,attach,submit,resize}
               │                                              │
               └──────────────► SessionHost ◄─────────────────┘
                                 /       \
                    (--local)   /         \  (TCP/stdio JSON-RPC: term.*)
                               ▼           ▼
                           LocalHost ◄─── TermDispatch (vox-orchestrator-d)
                               │
                 ┌─────────────┴───────────────────────────────────┐
                 │ Entry (Arc)                                     │
                 │  ├─ PtyHandle { writer, master, child, tempdir }│
                 │  │     └─► shell_init.rs TempDir launch shim    │
                 │  ├─ OS Reader Thread ──► Utf8StreamDecoder      │
                 │  ├─ Replay: VecDeque<OutputFrame> (<= 2 MiB)    │
                 │  ├─ VtShadow (alacritty_terminal, hist=0)       │
                 │  │     └─► TermMode::ALT_SCREEN transitions     │
                 │  └─ Session + Osc633Parser                      │
                 │        ├─► 633;A/B/E/C/D + 633;P;Cwd/GitBranch  │
                 │        ├─► OSC 7 Cwd extraction                 │
                 │        └─► Block (\r progress normalization)    │
                 └─────────────────────────────────────────────────┘
```

### Recommended Project Structure

```text
crates/vox-terminal-core/src/
├── lib.rs           # Re-exports (preserve backward-compat + export TermSignal, ShellInit)
├── shell_init.rs    # NEW: 5-shell TempDir launch shim builder (zsh, bash, fish, pwsh, nu + sh fallback)
├── utf8_stream.rs   # NEW: 3-byte incomplete UTF-8 tail buffer + \r / CSI-K line overwrite normalizer
├── vt_shadow.rs     # NEW: alacritty_terminal::Term (scrolling_history: 0) ALT_SCREEN shadow tracker
├── pty.rs           # PtyHandle { writer, master, child, _shim_dir }, MasterPty::resize, signal(\x03)
├── osc633.rs        # Streaming Output between C..D, PromptEnd(B), Property(P;k=v), Osc7Cwd, UTF-8 safe
├── block.rs         # Block \r overwrite normalization state + cwd/git_branch metadata (optional/serde-default)
├── session.rs       # Stateful PTY submit, active-Running stdin forwarding, AltScreenChanged & CwdChanged
├── host.rs          # OpenSpec(120x32), Replay(VecDeque<OutputFrame>), attach(id, after_seq), signal()
└── daemon_host.rs   # SIGNAL="term.signal", attach(id, after_seq), automatic lag delta re-subscribe
```

### Pattern 1: Non-Echo `TempDir` Shell Launch Shims (`shell_init.rs`)
**What:** Construct a `tempfile::Builder::new().prefix("vox-shell-").tempdir()?` and configure `portable_pty::CommandBuilder` env vars and CLI flags so the shell loads user dotfiles first and Vox `OSC 633` + `OSC 7` hooks second, without writing a single byte to PTY stdin.
**When to use:** Every `spawn_pty` / `spawn_pty_with_cwd` invocation.
**Per-shell launch matrix:**
- **`zsh`**: Write `<dir>/.zshenv` (sources `${VOX_ORIG_ZDOTDIR:-$HOME}/.zshenv` if present, then re-pins `ZDOTDIR="<dir>"` while saving any user-mutated `ZDOTDIR` in `VOX_USER_ZDOTDIR`), `<dir>/.zprofile` (sources user `.zprofile` if login), and `<dir>/.zshrc`:
  1. Restore `ZDOTDIR` to `VOX_USER_ZDOTDIR` / `VOX_ORIG_ZDOTDIR` (or `unset ZDOTDIR` if originally unset) so child `zsh` processes see the user's real `ZDOTDIR`.
  2. `unsetopt PROMPT_SP` (suppresses macOS inverse `%` + trailing spaces before first prompt).
  3. Source `${ZDOTDIR:-$HOME}/.zshrc` if readable.
  4. Define `__vox_precmd` with `local ec=$?` as its **very first line**, `autoload -Uz add-zsh-hook`, register `add-zsh-hook preexec __vox_preexec` and `add-zsh-hook precmd __vox_precmd`, then **prepend** `__vox_precmd` to `precmd_functions` (`precmd_functions=(__vox_precmd ${precmd_functions:#__vox_precmd})`) so third-party hooks (Starship, Oh-My-Zsh, direnv) never clobber `$?` before Vox reads it.
  5. Install `trap '__vox_ec=$?; if [[ -n "${__VOX_IN_EXEC:-}" ]]; then printf "\e]633;D;%d\a" "$__vox_ec"; fi' EXIT` so top-level `exit 7` emits `633;D;7` before the shell process terminates.
  6. Wrap prompt escapes inside `%{...%}`: `PROMPT="%{"$'\e]633;A\a'"%}${PROMPT}%{"$'\e]633;B\a'"%}"`.
- **`bash`**: Pass `--rcfile <dir>/bashrc -i`. Inside `bashrc`, source `~/.bashrc` if present, handle both string and Bash 5.1+ array `PROMPT_COMMAND` (prepending `__vox_prompt` where line 1 is `local ec=$?`), set `__VOX_AT_PROMPT=1` *after* running user `PROMPT_COMMAND` so user prompt hooks don't falsely trigger the `DEBUG` trap, wrap `PS1="\[\e]633;A\a\]${PS1}\[\e]633;B\a\]"`, and install the `EXIT` trap for `exit N`.
- **`fish`**: Write `<dir>/init.fish` and pass `-i --init-command "source '<escaped_path>'"` (Fish loads `~/.config/fish/config.fish` automatically before `--init-command`). Hook `fish_preexec`, `fish_postexec` (`$status`), `fish_exit`, and wrap `fish_prompt` with `633;A` / `633;B`.
- **`pwsh` / `powershell`**: Write `<dir>/init.ps1` and pass `-NoLogo -NoExit -File <dir>/init.ps1`. Dot-source `if (Test-Path $PROFILE) { . $PROFILE }`, wrap `function global:prompt` capturing `$LASTEXITCODE` / `$?` (resetting `$global:LASTEXITCODE = 0` after read so a failed native command doesn't stick to subsequent cmdlets), and guard `Set-PSReadLineKeyHandler` behind `if (Get-Module -Name PSReadLine)`.
- **`nu`**: Write `<dir>/config.nu` and pass `--config <dir>/config.nu` (plus `--env-config <user_env_nu>` if the user's default `env.nu` exists on disk). **Critical Nushell rule:** Nushell `source` requires a parse-time literal path (`source $var` is a compile error), so `shell_init.rs` checks in Rust whether the user's `config.nu` exists on disk and splices `source '<literal_path>'` into `<dir>/config.nu` before appending `$env.config = ($env.config | upsert hooks ...)`.

### Pattern 2: Dedicated OS Reader Thread + `Condvar` Fast-Path Settle (`host.rs`)
**What:** Feed PTY output directly from the `std::thread::spawn` reader loop into `Entry::feed` (protected by `std::sync::Mutex` + `std::sync::Condvar`) rather than hopping through an async `tokio::spawn` `mpsc::Receiver` task.
**Why mandatory:** Existing callers (`[VERIFIED: crates/vox-terminal-core/src/host.rs:488-501]`, `[VERIFIED: crates/vox-terminal-core/tests/session.rs:38-113]`, `[VERIFIED: crates/vox-gui/tests/real_backend_harness.rs:173-226]`) run inside `#[tokio::test]` (single-threaded `current_thread` Tokio executor) and call synchronous `host.submit(&id, "!echo hi")` followed immediately by `host.attach(&id)` / `host.block(&id, bid)`. If `Entry::feed` depended on a `tokio::spawn` task, waiting inside synchronous `host.submit` would starve the single Tokio thread and deadlock! Running `Entry::feed` on the OS reader thread + `Condvar::wait_timeout` (e.g., up to `500ms` fast-path settle for command completion or `alt_screen` entry, returning immediately in `~5ms` for `echo`/`cd`/`export`/`false`, and returning the live `Running` `BlockId` once the short settle window elapses for `sleep 10` / interactive `[y/N]` / REPLs) keeps every synchronous test green AND supports non-blocking long-running/interactive blocks.

### Pattern 3: `OSC 633` Idle-Prompt vs Active-Execution Block Lifecycle (`osc633.rs` + `session.rs`)
**What:** Fix two structural bugs in `osc633.rs` and `session.rs`:
1. **Do NOT open a `Running` `Block` on `633;A` (`PromptStart`)!** Currently `Osc633Event::PromptStart` immediately pushes an empty `Block::new(id, BlockKind::Shell, "")` with `BlockStatus::Running` (`[VERIFIED: crates/vox-terminal-core/src/session.rs:212-226]`). On real PTY startup, the shell prints its initial prompt (`633;A` ... `633;B`) before any command runs — creating a phantom empty `Running` block that breaks `assert!(attached.blocks.is_empty())` (`[VERIFIED: crates/vox-term/tests/headless.rs:90]`) and tricks `Session::submit` into thinking a foreground job is already running! Instead, open a `Running` `Block` ONLY on `Session::submit` (reserving `pending_block_id` with its `BlockOrigin`) OR on `633;E` / `633;C` when typed interactively into the raw PTY.
2. **Stream `Osc633Event::Output` immediately between `633;C` and `633;D`!** Replace `self.output_buf.extend_from_slice(...)` hoarding in `Osc633Parser` (`[VERIFIED: crates/vox-terminal-core/src/osc633.rs:80-81,109-110]`) with immediate UTF-8-aligned `Osc633Event::Output` emission while `self.capturing == true`, and ignore prompt/ZLE echo bytes outside `633;C..633;D` for block output (raw `OutputFrame` still forwards 100% of prompt/echo bytes to VT renderers).

### Pattern 4: Exact `after_seq` Ring Semantics (`host.rs`)
**What:** Store `frames: VecDeque<OutputFrame>` + `total_bytes: usize` + `seq: u64` in `Replay`.
- Evict oldest frames from front while `total_bytes > cap` (and if a single oversized frame exceeds `cap`, slice its trailing `<= cap` bytes on a UTF-8 char boundary).
- On `attach(session_id, after_seq)`:
  - `None` → `resync_required = true`, full retained `replay`, `seq = latest_seq`.
  - `Some(s)` where `s >= latest_seq` → `resync_required = false`, `replay = vec![]`, `seq = latest_seq`.
  - `Some(s)` where `s < latest_seq` and `oldest_seq.is_some_and(|oldest| s + 1 >= oldest)` → `resync_required = false`, `replay` = bytes of frames with `frame.seq > s`, `seq = latest_seq`.
  - `Some(s)` where `s < latest_seq` and `oldest_seq.is_none_or(|oldest| s + 1 < oldest)` → `resync_required = true`, full retained `replay`, `seq = latest_seq`.

### Anti-Patterns to Avoid
- **Calling `Runtime::new()?.block_on(...)` inside `vox_term::app::run_with_args` when invoked from `vox-cli`:** `vox-cli`'s `dispatch_cli_inner` is already an `async fn` running inside Tokio (`[VERIFIED: crates/vox-cli/src/cli_dispatch/mod.rs:155,625]`). Expose `pub async fn run_with_args(args: TermCliArgs) -> Result<()>` for `vox-cli` to `.await`, and keep `pub fn run() -> Result<()>` building a Tokio runtime only for the standalone `vox-term` binary (`[VERIFIED: crates/vox-term/src/main.rs:2-4]`).
- **Lexicographic sorting for `--attach latest`:** Session IDs are formatted as `format!("term-{n}")` (`[VERIFIED: crates/vox-terminal-core/src/host.rs:328]`). Lexicographic comparison orders `"term-9"` after `"term-10"`. Parse the numeric suffix (`id.strip_prefix("term-").and_then(|s| s.parse::<u64>().ok())`) when selecting `latest`.
- **Truncating the current line immediately on seeing `\r` at the end of a chunk:** PTY newlines are `\r\n`, which can split across two 4096-byte reads (`"hello\r"` + `"\n"`). Record `pending_cr = true` on `\r` and only overwrite from column 0 when a non-`\n` visible char or `\x1b[K`/`\x1b[2K` arrives next.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Alternate-screen (`?1049h`/`?1049l`/`?47h`/`?1047h`/`RIS`) state machine | Ad-hoc substring search for `\x1b[?1049h` | `alacritty_terminal::Term` + `vte::ansi::Processor` (`TermMode::ALT_SCREEN`) | Handles compound DECSET params (`\x1b[?1049;2004h`), split escape sequences across chunks, `?47h`/`?1047h`, and terminal full reset `\x1bc`. |
| Temporary shell shim directories & cleanup | Manual `/tmp/vox-shell-{pid}` `fs::create_dir` / `remove_dir_all` | `tempfile::TempDir` stored on `PtyHandle` | Cryptographically random names, `0700` Unix permissions (prevents local symlink/injection races), and automatic RAII `Drop` cleanup on close or panic. |
| Incomplete UTF-8 lead/continuation detection | Manual bitmask table on `0xC0..0xF7` | `std::str::from_utf8(buf)` (`Utf8Error::valid_up_to()` + `Utf8Error::error_len()`) | Rust's stdlib validator handles overlong encodings, surrogate halves, and `> U+10FFFF` bounds per RFC 3629: `error_len() == None` means `1..=3` incomplete trailing bytes to retain; `Some(len)` means genuine invalid bytes to emit `U+FFFD` and advance past. |
| Cross-platform PTY window-size ioctl | Raw `libc::ioctl(fd, TIOCSWINSZ)` / Win32 `ResizePseudoConsole` FFI | `MasterPty::resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })` | `portable-pty 0.9` implements both Unix `TIOCSWINSZ` (`SIGWINCH`) and Windows ConPTY `ResizePseudoConsole` behind `MasterPty::resize`. |

**Key insight:** Every complex protocol edge in Phase 441 (UTF-8 RFC 3629 state, DECSET VT modes, kernel PTY ioctls, secure temp dirs) is already solved by stdlib + workspace-pinned crates; custom byte-twiddling was the exact source of the baseline bugs.

## Runtime State Inventory

Phase 441 refactors `PtyHandle`, `Session`, `Attachment`, `SessionInfo`, and `term.*` wire payloads. Explicit inventory across all five runtime-state categories:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| **Stored data** | None — terminal sessions, replay buffers, and block lists are strictly in-memory inside `LocalHost` (`docs/src/architecture/terminal-session-host-design-2026.md:197-198`: replay buffer is never persisted; `vox-journal` / `CorpusWriter` consumes `Block::plain_output()` strings unchanged). | Code edit only; no database or file migration. |
| **Live service config** | `vox-orchestrator-d` loopback daemon hosts `TermDispatch` in-process (`crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs:291-293`). All new JSON fields (`after_seq`, `resync_required`, `alt_screen`, `cwd`, `git_branch`) use `#[serde(default)]` / `.unwrap_or_default()`. | Restarting `vox-orchestrator-d` picks up new `term.*` fields; `#[serde(default)]` preserves wire tolerance. |
| **OS-registered state** | Per-session temporary shim directories (`/tmp/vox-shell-*` / `%TEMP%\vox-shell-*`) created at `spawn_pty` time. | Store `Option<tempfile::TempDir>` inside `PtyHandle` so `LocalHost::close` / `PtyHandle` drop removes them automatically. |
| **Secrets/env vars** | `VOX_TERM_NO_AUTOSPAWN` (`crates/vox-terminal-core/src/daemon_host.rs:67`), `VOX_ORCHESTRATOR_DAEMON_SOCKET` (`crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs:3`), `ZDOTDIR` (restored inside `.zshrc` shim). | Preserve `VOX_TERM_NO_AUTOSPAWN` and `VOX_ORCHESTRATOR_DAEMON_SOCKET` unchanged; restore `ZDOTDIR` immediately inside `.zshrc` before sourcing user `~/.zshrc`. |
| **Build artifacts** | `Cargo.lock` workspace-member `[[package]] name = "vox-terminal-core"` dependency list (`Cargo.lock:19240-19256`); `crates/vox-cli/tests/fixtures/command_catalog_paths_baseline.txt` (`term` row at line 623 tracks subcommands only, not flags). | Include `Cargo.lock` in the `<files>` list of the task updating `crates/vox-terminal-core/Cargo.toml`; verify `command_catalog_paths_baseline` test stays green. |

## Common Pitfalls

### Pitfall 1: `zsh` `ZDOTDIR` Leakage into Subshells & Third-Party `precmd` Clobbering `$?`
**What goes wrong:** Setting `ZDOTDIR=/tmp/vox-shell-xyz` on `CommandBuilder` leaks into child `zsh` processes, or if `.zshenv` resets `ZDOTDIR=$HOME` too early, `zsh` skips `/tmp/vox-shell-xyz/.zshrc` entirely. Meanwhile, `add-zsh-hook precmd __vox_precmd` appends to `precmd_functions`, so Oh-My-Zsh or Starship runs first and resets `$?` to `0` before `__vox_precmd` executes.
**How to avoid:** Keep `ZDOTDIR` pointing to the shim dir through `.zshenv`, restore `ZDOTDIR` to `VOX_ORIG_ZDOTDIR` (or `unset ZDOTDIR`) at the **top of `.zshrc`**, source user `~/.zshrc`, and then force `__vox_precmd` to index 1 of `precmd_functions`: `precmd_functions=(__vox_precmd ${precmd_functions:#__vox_precmd})`. Also add an `EXIT` trap guarded by `__VOX_IN_EXEC=1` so top-level `exit 7` emits `\e]633;D;7\a` before process exit.
**Warning signs:** `false` reports `exit_code: Some(0)` when `~/.zshrc` loads Starship, or `exit 7` closes the block with `exit_code: Some(1)` / `None` instead of `Some(7)`.

### Pitfall 2: Struct-Literal Compilation Breaks Across Downstream Crates (`vox-term`, `vox-gui`, `vox-orchestrator-d`)
**What goes wrong:** Adding fields to `SessionInfo`, `OpenSpec`, or changing `SessionHost::attach` breaks downstream crates if every call site isn't updated in lockstep.
**Exact verified call sites across the entire workspace:**
- `SessionInfo` struct literals (3 total):
  1. `crates/vox-terminal-core/src/host.rs:344-349` (`LocalHost::list`)
  2. `crates/vox-terminal-core/src/daemon_host.rs:224-229` (`DaemonHost::list`)
  3. `crates/vox-term/tests/palette.rs:37-42` (`palette_includes_live_sessions` test!)
- `OpenSpec` non-`..Default::default()` struct literals (3 total):
  1. `crates/vox-gui/src/commands/pty.rs:188` (`OpenSpec { cols, rows, shell }`)
  2. `crates/vox-gui/src/commands/pty.rs:376-380` (`OpenSpec { cols: 80, rows: 24, shell: None }`)
  3. `crates/vox-orchestrator-d/src/term_dispatch.rs:136-140`
  *(Keeping `OpenSpec`'s existing 3 fields `{ cols, rows, shell }` while changing `OpenSpec::default()` from `80×24` to `120×32` avoids breaking any `OpenSpec` struct literal!)*
- `SessionHost::attach` trait callers (4 total):
  1. `crates/vox-gui/src/commands/pty.rs:210` (`host.attach(&session_id, None).await`)
  2. `crates/vox-term/tests/headless.rs:89` (`host.attach(&id, None).await`)
  3. `crates/vox-orchestrator-d/src/term_dispatch.rs:413` (`host.attach(&id, None).await`)
  4. `crates/vox-term/src/app.rs` (new live attach loop)
  *(Retaining inherent `LocalHost::attach(&self, id: &str)` delegating to `LocalHost::attach_since(&self, id: &str, None)` keeps `crates/vox-gui/tests/real_backend_harness.rs:205` and internal `host.rs`/`term_dispatch.rs` tests compiling with zero churn!)*

### Pitfall 3: Single-Threaded `#[tokio::test]` Deadlock on Stateful PTY `submit`
**What goes wrong:** `LocalHost::open` currently spawns a `tokio::spawn` task (`[VERIFIED: crates/vox-terminal-core/src/host.rs:332-336]`) to drain `rx.recv().await` and call `pump.feed(&bytes)`. In `#[tokio::test]` (default `current_thread` runtime), blocking synchronous `LocalHost::submit` while waiting for `OSC 633;D` prevents `tokio::spawn` from ever being polled, hanging the test until the `300s` timeout.
**How to avoid:** Have the OS reader `std::thread::spawn` invoke the `Entry::feed` callback directly (after `Utf8StreamDecoder`), and wake synchronous waiters via `std::sync::Condvar`. `broadcast::Sender::send` and `std::sync::Mutex` are 100% runtime-independent.

### Pitfall 4: Nushell Dynamic `source` Syntax Error
**What goes wrong:** Writing `if ($user_cfg | path exists) { source $user_cfg }` in `config.nu` fails to parse in Nushell because `source` is a parse-time keyword requiring a constant string literal.
**How to avoid:** Probe the user's Nushell config file path in Rust inside `shell_init.rs` when generating `<tempdir>/config.nu`, and emit `source '/literal/path/to/config.nu'` only when `Path::is_file()` is true.

### Pitfall 5: `alacritty_terminal` Default 10,000-Line Scrollback Overhead
**What goes wrong:** `alacritty_terminal::term::Config::default()` allocates `scrolling_history: 10000` (`[VERIFIED: /Users/brbrainerd/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/alacritty_terminal-0.26.0/src/term/mod.rs:356-360]`), duplicating megabytes of cell structs already covered by `LocalHost`'s `2 MiB` byte ring.
**How to avoid:** Initialize `VtShadow` with `Config { scrolling_history: 0, ..Config::default() }`.

## Code Examples

### 1. Verbatim In-Repo Baseline Types & Signatures (Verified via `Read`)

From `crates/vox-terminal-core/src/pty.rs` (`[VERIFIED: crates/vox-terminal-core/src/pty.rs:306-329]`):
```rust
pub struct PtyHandle {
    writer: Box<dyn std::io::Write + Send>,
    child: Box<dyn Child + Send + Sync>,
}

impl PtyHandle {
    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        use std::io::Write;
        self.writer.write_all(data)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        // portable-pty doesn't expose resize through Box<dyn Child>; keep the
        // master in PtyHost for resize. This is a best-effort no-op for now.
        let _ = (cols, rows);
        Ok(())
    }

    pub fn kill(&mut self) {
        let _ = self.child.kill();
    }
}
```

From `portable-pty 0.9.0` (`[VERIFIED: /Users/brbrainerd/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/portable-pty-0.9.0/src/lib.rs:88-103]`):
```rust
pub trait MasterPty: Downcast + Send {
    fn resize(&self, size: PtySize) -> Result<(), Error>;
    fn get_size(&self) -> Result<PtySize, Error>;
    fn try_clone_reader(&self) -> Result<Box<dyn std::io::Read + Send>, Error>;
    fn take_writer(&self) -> Result<Box<dyn std::io::Write + Send>, Error>;
```

From `alacritty_terminal 0.26.0` (`[VERIFIED: /Users/brbrainerd/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/alacritty_terminal-0.26.0/src/term/mod.rs:55-70,334-337,709-711]`):
```rust
    pub struct TermMode: u32 {
        const NONE                    = 0;
        // ...
        const ALT_SCREEN              = 1 << 12;
    }
// ...
pub struct Config {
    /// The maximum amount of scrolling history.
    pub scrolling_history: usize,
// ...
    pub fn mode(&self) -> &TermMode {
        &self.mode
    }
```

From `crates/vox-terminal-core/src/host.rs` (`[VERIFIED: crates/vox-terminal-core/src/host.rs:23-38,158-165,193-213]`):
```rust
#[derive(Debug, Clone)]
pub struct OpenSpec {
    pub cols: u16,
    pub rows: u16,
    pub shell: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SessionInfo {
    pub id: SessionId,
    pub blocks: usize,
    #[serde(default)]
    pub security_level: TermSecurityLevel,
    #[serde(default)]
    pub control_mode: TermControlMode,
}

pub struct Attachment {
    pub blocks: Vec<Block>,
    /// Last `replay_cap` bytes of raw PTY output.
    pub replay: Vec<u8>,
    /// `seq` of the newest frame included in `replay` (0 = none yet).
    pub seq: u64,
    pub events: broadcast::Receiver<SessionEvent>,
    pub output: broadcast::Receiver<OutputFrame>,
}

#[async_trait]
pub trait SessionHost: Send + Sync {
    async fn open(&self, spec: OpenSpec) -> Result<SessionId>;
    async fn list(&self) -> Result<Vec<SessionInfo>>;
    async fn attach(&self, id: &str) -> Result<Attachment>;
    async fn submit(&self, id: &str, line: &str) -> Result<BlockId>;
    async fn input(&self, id: &str, bytes: &[u8]) -> Result<()>;
    async fn resize(&self, id: &str, cols: u16, rows: u16) -> Result<()>;
    async fn close(&self, id: &str) -> Result<()>;
}
```

From `crates/vox-cli/src/lib.rs` (`[VERIFIED: crates/vox-cli/src/lib.rs:723-728]`) & `crates/vox-cli/src/cli_dispatch/mod.rs` (`[VERIFIED: crates/vox-cli/src/cli_dispatch/mod.rs:625-627]`):
```rust
    Term,
// ...
        Cli::Term => {
            vox_term::app::run()?;
        }
```

### 2. Recommended Target Patterns

#### UTF-8 Tail Reassembler (`crates/vox-terminal-core/src/utf8_stream.rs`)
```rust
#[derive(Default)]
pub struct Utf8StreamDecoder {
    tail: [u8; 3],
    tail_len: usize,
}

impl Utf8StreamDecoder {
    /// Reassemble valid UTF-8 strings across arbitrary byte-chunk boundaries,
    /// retaining 1..=3 incomplete trailing UTF-8 bytes for the next call.
    pub fn push(&mut self, incoming: &[u8]) -> String {
        let mut combined = Vec::with_capacity(self.tail_len + incoming.len());
        combined.extend_from_slice(&self.tail[..self.tail_len]);
        combined.extend_from_slice(incoming);
        self.tail_len = 0;

        let mut out = String::with_capacity(combined.len());
        let mut rest: &[u8] = &combined;
        while !rest.is_empty() {
            match std::str::from_utf8(rest) {
                Ok(valid) => {
                    out.push_str(valid);
                    break;
                }
                Err(err) => {
                    let valid_len = err.valid_up_to();
                    if valid_len > 0 {
                        // Safety: `valid_up_to()` guarantees valid UTF-8 prefix.
                        out.push_str(std::str::from_utf8(&rest[..valid_len]).unwrap_or_default());
                    }
                    rest = &rest[valid_len..];
                    match err.error_len() {
                        None => {
                            // Incomplete 2..=4-byte UTF-8 sequence at end of chunk (1..=3 bytes).
                            self.tail_len = rest.len();
                            self.tail[..self.tail_len].copy_from_slice(rest);
                            break;
                        }
                        Some(bad_len) => {
                            out.push('\u{FFFD}');
                            rest = &rest[bad_len..];
                        }
                    }
                }
            }
        }
        out
    }
}
```

#### `VtShadow` `ALT_SCREEN` Tracker (`crates/vox-terminal-core/src/vt_shadow.rs`)
```rust
use alacritty_terminal::{
    Term,
    event::{Event, EventListener},
    grid::Dimensions,
    term::{Config, TermMode},
    vte::ansi::Processor,
};

struct NoopListener;
impl EventListener for NoopListener {
    fn send_event(&self, _: Event) {}
}

struct ShadowSize {
    cols: usize,
    lines: usize,
}

impl Dimensions for ShadowSize {
    fn columns(&self) -> usize { self.cols }
    fn screen_lines(&self) -> usize { self.lines }
    fn total_lines(&self) -> usize { self.lines }
}

pub struct VtShadow {
    term: Term<NoopListener>,
    parser: Processor,
    alt_screen: bool,
}

impl VtShadow {
    pub fn new(cols: u16, rows: u16) -> Self {
        let size = ShadowSize {
            cols: cols.max(1) as usize,
            lines: rows.max(1) as usize,
        };
        let cfg = Config {
            scrolling_history: 0,
            ..Config::default()
        };
        Self {
            term: Term::new(cfg, &size, NoopListener),
            parser: Processor::new(),
            alt_screen: false,
        }
    }

    /// Feed raw PTY bytes; returns `Some(active)` iff `TermMode::ALT_SCREEN` toggled.
    pub fn feed(&mut self, bytes: &[u8]) -> Option<bool> {
        self.parser.advance(&mut self.term, bytes);
        let now = self.term.mode().contains(TermMode::ALT_SCREEN);
        if now != self.alt_screen {
            self.alt_screen = now;
            Some(now)
        } else {
            None
        }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.term.resize(ShadowSize {
            cols: cols.max(1) as usize,
            lines: rows.max(1) as usize,
        });
    }

    pub fn is_alt_screen(&self) -> bool {
        self.alt_screen
    }
}
```

#### Retained `MasterPty` + `TempDir` + `TermSignal` on `PtyHandle` (`crates/vox-terminal-core/src/pty.rs`)
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TermSignal {
    Interrupt,
    Terminate,
    Kill,
}

pub struct PtyHandle {
    writer: Box<dyn std::io::Write + Send>,
    master: std::sync::Arc<std::sync::Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
    child: Box<dyn Child + Send + Sync>,
    _shim_dir: Option<tempfile::TempDir>,
}

impl PtyHandle {
    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        let guard = self.master.lock().unwrap_or_else(|p| p.into_inner());
        guard.resize(PtySize {
            rows: rows.max(1),
            cols: cols.max(1),
            pixel_width: 0,
            pixel_height: 0,
        })?;
        Ok(())
    }

    pub fn signal(&mut self, sig: TermSignal) -> Result<()> {
        match sig {
            TermSignal::Interrupt => self.write(b"\x03"),
            TermSignal::Terminate | TermSignal::Kill => {
                self.child.kill()?;
                Ok(())
            }
        }
    }
}
```

## State of the Art

| Old Approach (`vox-terminal-core` baseline) | Current Approach (`v7.5` Phase 441) | When Changed | Impact |
|---------------------------------------------|-------------------------------------|--------------|--------|
| Write `OSC 633` snippets to PTY `writer` stdin | Per-session `tempfile::TempDir` launch shims (`ZDOTDIR`, `--rcfile`, `--init-command`, `-File`, `--config`) | Phase 441 | Zero echoed startup spam; zero history pollution; accurate `$?` across all 5 shells. |
| Per-command `std::process::Command::new(shell).arg("-c")` (`run_shell_capture`) | Persistent `PtyHandle` stdin write + `OSC 633` lifecycle tracking | Phase 441 | `cd`, `export`, aliases, virtualenvs, and interactive `[y/N]` stdin persist across commands. |
| Dropped `MasterPty` (`resize` no-op) | `Arc<Mutex<Box<dyn MasterPty + Send>>>` + `VtShadow::resize` | Phase 441 | Real `TIOCSWINSZ` (`SIGWINCH`) & ConPTY `ResizePseudoConsole` updates `tput cols` / `$COLUMNS`. |
| `String::from_utf8_lossy` per 4096B read + full `2 MiB` `VecDeque<u8>` replay | 3-byte `Utf8StreamDecoder` + `VecDeque<OutputFrame>` `after_seq` deltas + `\r` normalization | Phase 441 | Zero `U+FFFD` corruption on split emojis/box-drawing; zero `2 MiB` lag re-write storms; clean spinner blocks. |

**Deprecated/outdated:**
- Writing `shell_integration_snippet` to `writer` inside `spawn_pty`: remove from `spawn_pty` (keep `pub fn shell_integration_snippet(shell: &str) -> Option<String>` intact as a pure helper so existing tests in `crates/vox-terminal-core/tests/pty.rs:9-34` and `crates/vox-gui/src/commands/pty.rs:320-369` continue passing!).

## Assumptions Log

All technical claims, dependency versions, file paths, and struct/trait signatures in this research were verified directly against repository source files and `~/.cargo/registry/src/` crate sources in this session.

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| — | None — zero `[ASSUMED]` claims required. | — | — |

## Open Questions (RESOLVED)

1. **Fast-path settle timeout duration in synchronous `LocalHost::submit`:** — RESOLVED: Use a `1500ms` `Condvar::wait_timeout` fast-path settle in `LocalHost::submit_with_origin` waiting for *either* `BlockClosed` *or* `AltScreenChanged { active: true }` (or PTY EOF), waking in `<5ms` via `Condvar` as soon as `633;D` arrives, and returning the live `Running` `BlockId` cleanly if the command remains running after `1500ms` (or after `500ms` when forwarding stdin into an already-`Running` block).

## Environment Availability

Verified on target workspace host (`macOS` `arm64-apple-darwin25`) during this research session:

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo` (build-broker) | All Rust builds & tests | ✓ | `cargo 1.98.1 (797e8a9bc 2026-08-05)` | — |
| `zsh` | `TERM-PTY-01..02` live PTY tests | ✓ | `zsh 5.9 (arm64-apple-darwin25.0)` (`/bin/zsh`) | — |
| `bash` | `TERM-PTY-01..02` live PTY tests | ✓ | `GNU bash 5.3.20(1)-release` (`/opt/homebrew/bin/bash`) | `/bin/bash` (`3.2` macOS base) |
| `sh` | `openspec_custom_shell` test | ✓ | POSIX `/bin/sh` | — |
| `fish` | `TERM-PTY-01` Fish shim | ✗ | — | Test generated `init.fish` content + `CommandBuilder` argv/env deterministically; gate live `fish` spawn test behind binary probe. |
| `pwsh` | `TERM-PTY-01` PowerShell shim | ✗ | — | Test generated `init.ps1` content + `-NoLogo -NoExit -File` argv deterministically; gate live `pwsh` spawn test behind binary probe (runs live on Windows CI). |
| `nu` | `TERM-PTY-01` Nushell shim | ✗ | — | Test generated `config.nu` content + static `source` splicing + `--config` argv deterministically; gate live `nu` spawn test behind binary probe. |

**Missing dependencies with no fallback:**
- None.

**Missing dependencies with fallback:**
- `fish`, `pwsh`, `nu` are not installed on this macOS dev box: plans MUST write unconditional deterministic unit tests inspecting the generated `TempDir` files and `CommandBuilder` argv/env for all 5 shells, unconditional live PTY spawn/state/exit-code/resize/SIGINT tests against `zsh` and `bash` (on Unix) / `pwsh` (on Windows), and runtime `which`-gated live PTY tests for optional shells (`fish`, `nu`, `pwsh` on Unix).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` (`libtest` + `tokio::test` 1.x) |
| Config file | `crates/vox-terminal-core/Cargo.toml`, `crates/vox-term/Cargo.toml`, `crates/vox-orchestrator-d/Cargo.toml`, `crates/vox-cli/Cargo.toml`, `crates/vox-gui/Cargo.toml` |
| Quick run command | `timeout 300s cargo test -p vox-terminal-core` |
| Full suite command | `timeout 300s cargo test -p vox-terminal-core -p vox-term -p vox-orchestrator-d --lib --tests` (plus `timeout 300s cargo check -p vox-cli -p vox-gui`) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| `TERM-PTY-01` | 5-shell `TempDir` launch shims (`zsh`, `bash`, `fish`, `pwsh`, `nu`), zero echoed `OSC 633` lines in `replay` & history, `zsh` `PROMPT_SP` `%` suppressed, `false`→`1` & `exit 7`→`7`, live `OSC 7` + `633;P;Cwd` + `633;P;GitBranch`, `TempDir` RAII cleanup on close | unit + integration (real PTY) | `timeout 300s cargo test -p vox-terminal-core --test shell_init_and_stateful_pty` | ❌ Wave 0 (new test target) |
| `TERM-PTY-02` | Persistent `PtyHandle` `Session::submit` (`cd` + `export` persist across blocks), interactive stdin forwarding into `Running` block, `PtyHandle::resize` updating real kernel `tput cols` + `VtShadow`, `term.signal` (`\x03` `SIGINT`) interrupting `sleep` without killing shell | integration (real PTY) | `timeout 300s cargo test -p vox-terminal-core --test shell_init_and_stateful_pty` | ❌ Wave 0 (new test target) |
| `TERM-PTY-03` | Prime-chunked (`7`, `13`, `31` bytes) multi-byte UTF-8 box-drawing/emoji/ANSI stream with `0` `U+FFFD`, `VtShadow` `TermMode::ALT_SCREEN` (`?1049h`/`?1049l`) transitions & block-split suppression, `\r` progress overwrite normalization (preserving split `\r\n`), `after_seq` delta attach & lag resync | unit + integration | `timeout 300s cargo test -p vox-terminal-core --test streaming_utf8_and_alt_screen` | ❌ Wave 0 (new test target) |
| `TERM-PTY-04` | `Cli::Term { attach, local }` clap parsing, `vox term --attach <id\|latest>` live shared-session attach over `DaemonHost` ↔ `TermDispatch` (`after_seq`, `alt_screen`, `cwd`, `git_branch`, `term.signal`), actionable error on missing `<session_id>`, and `vox-gui` `pty.rs` compile/test parity | integration | `timeout 300s cargo test -p vox-term --test headless` & `timeout 300s cargo test -p vox-orchestrator-d term_dispatch` & `timeout 300s cargo test -p vox-gui --lib commands::pty` | ✅ (extend existing + add attach parity tests) |

### Sampling Rate
- **Per task verification:**
  - `timeout 300s cargo test -p vox-terminal-core`
  - `timeout 300s cargo test -p vox-term`
  - `timeout 300s cargo test -p vox-orchestrator-d --lib term_dispatch`
- **Per wave gate:**
  - `timeout 300s cargo test -p vox-terminal-core`
  - `timeout 300s cargo test -p vox-term`
  - `timeout 300s cargo test -p vox-orchestrator-d --bin vox-orchestrator-d term_dispatch`
  - `timeout 300s cargo test -p vox-cli --test command_catalog_paths_baseline`
  - `timeout 300s cargo test -p vox-gui --lib commands::pty`
  - `timeout 300s cargo test -p vox-gui --test real_backend_harness real_backend_terminal_session_host`
- **Phase gate:** All 6 commands above pass with `0` errors + `timeout 300s cargo run -p vox-arch-check` green before `/gsd:verify-work`.

### Wave 0 Gaps
- [ ] `crates/vox-terminal-core/tests/shell_init_and_stateful_pty.rs` — TDD RED→GREEN suite covering `TERM-PTY-01` and `TERM-PTY-02` (5-shell shim generation, zero echo spam, `zsh`/`bash` `$?` `false` & `exit 7`, `cd`/`export` persistence, `tput cols` / `stty size` resize verification, `\x03` `SIGINT` cancellation, `TempDir` drop cleanup).
- [ ] `crates/vox-terminal-core/tests/streaming_utf8_and_alt_screen.rs` — TDD RED→GREEN suite covering `TERM-PTY-03` (`7`/`13`/`31`-byte prime chunking over emojis + box drawing + split `OSC 633`/`OSC 7`, `\r` spinner normalization vs split `\r\n`, `?1049h`/`?1049l` `AltScreenChanged` + block suppression, `after_seq` delta vs `resync_required` eviction).
- [ ] Extend `crates/vox-term/tests/headless.rs` & `crates/vox-orchestrator-d/src/term_dispatch.rs` — TDD RED→GREEN tests for `TERM-PTY-04` (`--attach <id>`, `--attach latest`, nonexistent `<id>` error, `term.signal`, `after_seq` delta snapshot over TCP loopback).

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes | Existing `OrchDaemonClient` token auth gate on `vox-orchestrator-d` (`requests_without_the_daemon_token_are_rejected` — `[VERIFIED: crates/vox-orchestrator-d/src/term_dispatch.rs:473-482]`). |
| V3 Session Management | yes | `tempfile::TempDir` (`0700` mode) bound to `PtyHandle` lifetime; explicit session isolation by `SessionId` and clean child kill + `TempDir` unlink on `close`. |
| V4 Access Control | yes | Loopback/stdio-only registration (`[VERIFIED: crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs:288-296]`) + fail-closed `refuse_agent_origin` on `term.input`, `term.submit`, and `term.signal` (`[VERIFIED: crates/vox-orchestrator-d/src/term_dispatch.rs:90-97]`). |
| V5 Input Validation | yes | Safe single-quote path escaping in `fish`/`nu` launch shims; `16 KiB` cap on incomplete `OSC` buffering in `Osc633Parser` to prevent unbounded buffer growth on malformed binary output; `cols.max(1)` / `rows.max(1)` clamping before `MasterPty::resize` and `VtShadow::resize`. |
| V6 Cryptography | no | No cryptographic operations in Phase 441. |

### Known Threat Patterns for Terminal PTY & Launch Shims

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Predictable `/tmp` shell init path hijacking (symlink race) | Elevation of Privilege / Tampering | Use `tempfile::TempDir` (`0700` owner-only directory via `mkdtemp`), never predictable `/tmp/vox-zshrc-$PID` filenames. |
| Wire client spoofing `origin: "agent"` on `term.signal` / `term.submit` | Spoofing | Call `refuse_agent_origin(p)?` on every mutating `term.*` RPC (`term.input`, `term.submit`, `term.signal`). |
| Unbounded `Osc633Parser` memory growth on unterminated `\x1b]633;` in binary `cat` output | Denial of Service | Flush incomplete OSC buffer as plain output if `buf.len() - abs_start > 16 * 1024` without seeing BEL (`\x07`) or ST (`\x1b\\`). |
| Zero-dimension `0×0` PTY/grid panic (`VtShadow` / `MasterPty::resize`) | Denial of Service | Clamp `cols.max(1)` and `rows.max(1)` (defaulting `OpenSpec` `0` dimensions to `120×32`). |

## Sources

### Primary (HIGH confidence)
- `crates/vox-terminal-core/src/{lib,pty,osc633,session,host,daemon_host,block}.rs` — inspected verbatim (`[VERIFIED]`)
- `crates/vox-term/src/{lib,app,vt,main}.rs`, `crates/vox-term/src/ui/{blocks,palette}.rs`, `crates/vox-term/tests/{headless,dispatch,vt,palette}.rs` — inspected verbatim (`[VERIFIED]`)
- `crates/vox-orchestrator-d/src/term_dispatch.rs` & `crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs` — inspected verbatim (`[VERIFIED]`)
- `crates/vox-cli/src/lib.rs`, `crates/vox-cli/src/cli_dispatch/mod.rs`, `crates/vox-cli/src/command_catalog.rs` — inspected verbatim (`[VERIFIED]`)
- `crates/vox-gui/src/commands/pty.rs` & `crates/vox-gui/tests/real_backend_harness.rs` — inspected verbatim (`[VERIFIED]`)
- `portable-pty 0.9.0` crate source (`lib.rs`, `cmdbuilder.rs`) & `alacritty_terminal 0.26.0` crate source (`lib.rs`, `term/mod.rs`) in local Cargo registry — inspected verbatim (`[VERIFIED]`)
- `docs/src/architecture/warp-roman-terminal-v75-research-2026.md` & `docs/src/architecture/terminal-session-host-design-2026.md` — architecture SSOTs (`[VERIFIED]`)

## Metadata

**Confidence breakdown:**
- Standard stack: **HIGH** — All crates (`portable-pty 0.9.0`, `alacritty_terminal 0.26.0`, `tempfile 3.27.0`) are already in `Cargo.toml` and `Cargo.lock` and verified at source level.
- Architecture: **HIGH** — Every caller across `vox-terminal-core`, `vox-term`, `vox-orchestrator-d`, `vox-cli`, and `vox-gui` was read and mapped line-by-line.
- Pitfalls: **HIGH** — Uncovered concrete execution blockers (`#[tokio::test]` single-thread pump deadlock, nested Tokio runtime in `vox-cli` → `vox-term`, `Osc633Parser` output hoarding, `PromptStart` phantom block, Nushell static `source`, `palette.rs` struct literal, `Cargo.lock` `<files>` scope) before planning.

**Research date:** 2026-11-14
**Valid until:** 2026-12-14 (30 days — pinned local workspace crates)
