# Phase 441: Zero-Spam Stateful PTY Engine, Non-Echo Shell Bootstrap & Lossless VT/OSC Streaming - Context

**Gathered:** 2026-11-13
**Status:** Ready for planning

<domain>
## Phase Boundary

Eliminate terminal load echo spam, broken `zsh` exit codes, stateless `Command::new` per-line subprocesses, no-op `resize`, and UTF-8 multi-byte split corruption in `vox-terminal-core`, and wire `vox term --attach` (`vox-cli` + `vox-term` + `vox-orchestrator-d` / `vox-gui` PTY bridge parity) for live CLI ↔ GUI session continuity (`TERM-PTY-01..04`).

</domain>

<decisions>
## Implementation Decisions

### Shell Launch Shims, Prompt Hygiene & Bootstrap (`TERM-PTY-01`)
- Source user dotfiles first inside a `tempfile::TempDir` launch shim in `crates/vox-terminal-core/src/shell_init.rs`, then install Vox `OSC 633` + `OSC 7` hooks (`ZDOTDIR` `.zshrc` shim sources `~/.zshrc`, `bash --rcfile` sources `~/.bashrc`, `pwsh -NoExit -File` loads `$PROFILE` then shim, `fish --init-command`, `nu --config`) — never write hook source lines to PTY stdin (`ECHO` stays clean and shell history stays unpolluted).
- In the `zsh` `ZDOTDIR/.zshrc` shim: `unsetopt PROMPT_SP` to eliminate macOS `%` inverse startup noise, wrap `633;A` and `633;B` prompt escapes inside `%{...%}` so `zsh` prompt width calculation never drifts, and wire `precmd`/`preexec` via `autoload -Uz add-zsh-hook` so `$?` is captured before any hook resets it (`false` → `1`, `exit 7` → `7`).
- Store `Option<tempfile::TempDir>` directly on `PtyHandle` so RAII drops the shim directory cleanly when the session closes without leaking `/tmp/vox-shell-*` directories or racing subshell startup.
- Emit both standard `OSC 7` (`file://<host>/<cwd>`) and `OSC 633;P;Cwd=<path>` plus lightweight `OSC 633;P;GitBranch=<branch>` on prompt boundaries so `Osc633Parser`, `alacritty_terminal`, and downstream GUI chips stay synchronized.

### Persistent PTY Command Execution, Signals & Resize Semantics (`TERM-PTY-02`)
- When a `Session` is backed by a live `PtyHandle` (`LocalHost` / `Session::with_pty`), `Session::submit` writes `<line>\n` directly to the persistent PTY master writer and tracks command start/output/completion/exit-code via `OSC 633` (`A`/`B`/`E`/`C`/`D;exit`) so `cd`, `export`, aliases, and virtualenvs persist across commands; retain synchronous `run_shell_capture` ONLY for explicit stateless unit-test constructors (`Session::new_stateless` / `Session::new`).
- Deliver `SIGINT` in `PtyHandle::signal` / `LocalHost::signal` by writing `\x03` (`ETX`) to the PTY master writer so Unix line discipline and Windows ConPTY interrupt the foreground job without killing the interactive shell; use `portable_pty::Child::kill()` for `SIGTERM`/`SIGKILL` or session teardown.
- When a foreground block is already `Running`, forward subsequent `submit` / `send_input` bytes directly to PTY stdin of the active `Running` block (enabling interactive `[y/N]` answers and REPL input) rather than spawning an overlapping `Running` block.
- Honor `OpenSpec { cols, rows }` at `spawn_pty` time (defaulting to `120×32` when unset instead of hardcoded `80×24`), retain `Arc<Mutex<Box<dyn MasterPty + Send>>>` inside `PtyHandle`, and propagate `PtyHandle::resize(cols, rows)` synchronously to both `MasterPty::resize(PtySize { rows, cols, .. })` and the `alacritty_terminal` shadow grid.

### Lossless Streaming, `ALT_SCREEN` Normalization & `vox term --attach` Continuity (`TERM-PTY-03`, `TERM-PTY-04`)
- Buffer up to 3 trailing incomplete UTF-8 lead/continuation bytes across `read()` boundaries (`utf8_stream.rs` / reader loop) so arbitrary prime-sized byte chunks decode with zero `U+FFFD` replacement characters; emit live raw UTF-8 chunks on `OutputFrame` while normalizing carriage-return overwrites (`\r` without `\n`, e.g. spinners/`cargo build` progress bars) inside `Block` text snapshots.
- Maintain a bounded `seq`-indexed `VecDeque<OutputFrame>` (up to 2 MiB byte budget) in `LocalHost` so `attach(session_id, after_seq: Option<u64>)` replays only frames with `seq > after_seq` (setting `resync_required: true` + full snapshot only when `after_seq` is `None` or older than the oldest retained `seq`), recovering cleanly from `broadcast::RecvError::Lagged`.
- Embed an `alacritty_terminal::Term` shadow state machine (`vt_shadow.rs` in `vox-terminal-core`) fed by the PTY reader stream to track `TermMode::ALT_SCREEN` (`?1049h`/`?1049l`, `?47h`, `?1047h`) transitions, emit `SessionEvent::AltScreenChanged { active: bool }` (and surface `alt_screen: bool` on `SessionInfo` / `Attachment`), and suppress command-block splitting while `alt_screen == true`.
- Add `--attach <Option<String>>` (supporting explicit `<session_id>` or `latest`) and `--local` flags to `Cli::Term` in `crates/vox-cli`, pass `TermCliArgs` into `vox_term::app::run_with_args`, attach to the shared daemon session via `resolve_host(args.force_local)` (updating `daemon_host.rs` and `vox-orchestrator-d` `term.*` handlers for `after_seq` + `alt_screen` + `cwd`/`git_branch` parity), and fail with an explicit actionable error when a requested `<session_id>` does not exist.

### the agent's Discretion
- Exact module split inside `crates/vox-terminal-core/src/` (`shell_init.rs`, `utf8_tail.rs`, `vt_shadow.rs`) provided public re-exports in `lib.rs` remain clean and backward-compatible with `vox-gui` and `vox-term` callers.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/vox-terminal-core/src/pty.rs`: `PtyHandle`, `ShellKind`, `ShellBackend`, `default_shell()`, `spawn_pty()` (`portable-pty 0.9` already wired).
- `crates/vox-terminal-core/src/osc633.rs`: `Osc633Parser`, `Osc633Event`, `decode_command()` for `OSC 633` frame extraction.
- `crates/vox-terminal-core/src/host.rs` & `daemon_host.rs`: `SessionHost` trait, `LocalHost`, `DaemonHost`, `OpenSpec`, `Attachment`, `OutputFrame`, `SessionInfo`.
- `crates/vox-term/src/app.rs` & `vt.rs`: `TermCliArgs` (`--attach`, `--local` already parsed!), `resolve_host()`, and `VtGrid` (`alacritty_terminal 0.26` wrapper).
- Workspace dependencies already in root `Cargo.toml` & `Cargo.lock`: `portable-pty = "0.9"`, `alacritty_terminal = "0.26"`, `tempfile = "3"` (zero lockfile fetches, zero new workspace crate edges).

### Established Patterns
- `vox-terminal-core` is an `L3` crate consumed by `vox-term` (`L4`), `vox-orchestrator-d` (`L5`), and `vox-gui` (`L5`).
- JSON-RPC `term.*` methods in `vox-orchestrator-d` (`term.open`, `term.attach`, `term.write`, `term.submit`, `term.resize`, `term.signal`, `term.list`, `term.close`) mirror `SessionHost` methods 1:1, and `crates/vox-gui/src/commands/pty.rs` wraps `SessionHost` for Tauri IPC.

### Integration Points
- `crates/vox-terminal-core/Cargo.toml`: add workspace references `alacritty_terminal = { workspace = true }` and `tempfile = { workspace = true }` (both already in root `[workspace.dependencies]` and `Cargo.lock`, zero new workspace crate edges).
- `crates/vox-cli/src/lib.rs` (or CLI enum definition) + `crates/vox-cli/src/cli_dispatch/mod.rs`: extend `Cli::Term` with `--attach` and `--local` forwarding to `vox_term::app::run_with_args`.
- `crates/vox-orchestrator-d` & `crates/vox-gui/src/commands/pty.rs`: pass `after_seq: Option<u64>` to `SessionHost::attach` (defaulting `None` where callers haven't migrated yet in Phase 443) so workspace builds stay 100% green.

</code_context>

<specifics>
## Specific Ideas

- Eliminate every trace of `ZSH_OSC633` / `BASH_OSC633` raw echo on startup (`console-before.png` root cause) and verify with deterministic unit/integration tests feeding prime-sized byte chunks (`7`, `13`, `31` bytes) and real PTY spawns (`zsh`/`bash`/`sh`).

</specifics>

<deferred>
## Deferred Ideas

- AST-backed `L1–L5` `ExecPolicyV1` and MCP `vox_terminal_*` tools — Phase 442.
- Frontend `<TerminalSessionPane>` `after_seq` consumer migration and `@xterm/addon-webgl` — Phase 443.

</deferred>
