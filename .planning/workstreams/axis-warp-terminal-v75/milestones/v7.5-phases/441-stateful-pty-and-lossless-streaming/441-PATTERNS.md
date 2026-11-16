# Phase 441: Zero-Spam Stateful PTY Engine, Non-Echo Shell Bootstrap & Lossless VT/OSC Streaming - Pattern Map

**Mapped:** 2026-11-14
**Files analyzed:** 22
**Analogs found:** 22 / 22

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/vox-terminal-core/Cargo.toml` (+ `Cargo.lock`) | config | batch | `crates/vox-term/Cargo.toml` | exact |
| `crates/vox-terminal-core/src/lib.rs` | config | transform | `crates/vox-terminal-core/src/lib.rs` | exact |
| `crates/vox-terminal-core/src/shell_init.rs` *(new)* | utility | file-I/O | `crates/vox-terminal-core/src/pty.rs` | role-match |
| `crates/vox-terminal-core/src/utf8_stream.rs` *(new)* | utility | streaming | `crates/vox-terminal-core/src/osc633.rs` | exact |
| `crates/vox-terminal-core/src/vt_shadow.rs` *(new)* | model | streaming | `crates/vox-term/src/vt.rs` | exact |
| `crates/vox-terminal-core/src/pty.rs` | provider | streaming | `crates/vox-terminal-core/src/pty.rs` | exact |
| `crates/vox-terminal-core/src/osc633.rs` | utility | streaming | `crates/vox-terminal-core/src/osc633.rs` | exact |
| `crates/vox-terminal-core/src/block.rs` | model | transform | `crates/vox-terminal-core/src/block.rs` | exact |
| `crates/vox-terminal-core/src/session.rs` | service | event-driven | `crates/vox-terminal-core/src/session.rs` | exact |
| `crates/vox-terminal-core/src/host.rs` | service | pub-sub | `crates/vox-terminal-core/src/host.rs` | exact |
| `crates/vox-terminal-core/src/daemon_host.rs` | provider | streaming | `crates/vox-terminal-core/src/daemon_host.rs` | exact |
| `crates/vox-terminal-core/tests/shell_init_and_stateful_pty.rs` *(new)* | test | event-driven | `crates/vox-terminal-core/tests/session.rs` | exact |
| `crates/vox-terminal-core/tests/streaming_utf8_and_alt_screen.rs` *(new)* | test | streaming | `crates/vox-terminal-core/tests/osc633.rs` | exact |
| `crates/vox-term/src/vt.rs` | component | streaming | `crates/vox-term/src/vt.rs` | exact |
| `crates/vox-term/src/app.rs` | controller | event-driven | `crates/vox-term/src/app.rs` | exact |
| `crates/vox-term/tests/headless.rs` | test | request-response | `crates/vox-term/tests/headless.rs` | exact |
| `crates/vox-term/tests/palette.rs` | test | transform | `crates/vox-term/tests/palette.rs` | exact |
| `crates/vox-orchestrator-d/src/term_dispatch.rs` | controller | streaming | `crates/vox-orchestrator-d/src/term_dispatch.rs` | exact |
| `crates/vox-cli/src/lib.rs` | route | request-response | `crates/vox-cli/src/lib.rs` | exact |
| `crates/vox-cli/src/cli_dispatch/mod.rs` | controller | request-response | `crates/vox-cli/src/cli_dispatch/mod.rs` | exact |
| `crates/vox-cli/src/command_catalog.rs` | service | transform | `crates/vox-cli/src/command_catalog.rs` | exact |
| `crates/vox-gui/src/commands/pty.rs` | controller | pub-sub | `crates/vox-gui/src/commands/pty.rs` | exact |

---

## Pattern Assignments

### `crates/vox-terminal-core/Cargo.toml` (+ `Cargo.lock`) (config, batch)

**Analog:** `crates/vox-term/Cargo.toml` & `crates/vox-terminal-core/Cargo.toml`

**Workspace dependency pattern** (`crates/vox-term/Cargo.toml` lines 16-26):
```toml
[dependencies]
vox-terminal-core  = { workspace = true }
ratatui            = { workspace = true }
crossterm          = { workspace = true, features = ["event-stream"] }
reedline           = { workspace = true }
alacritty_terminal = { workspace = true }
```

**Target manifest section** (`crates/vox-terminal-core/Cargo.toml` lines 13-27):
```toml
[dependencies]
serde        = { workspace = true, features = ["derive"] }
tokio        = { workspace = true, features = ["sync", "rt", "macros"] }
portable-pty = { workspace = true }
anyhow       = { workspace = true }
async-trait  = { workspace = true }
base64       = { workspace = true }
# vox-compiler is ~47k LoC; included directly for now (`vox-lang` feature gating planned).
vox-compiler     = { workspace = true }
vox-journal      = { workspace = true }
vox-orchestrator = { workspace = true }
serde_json       = { workspace = true }
vox-redact       = { workspace = true }
workspace-hack = { version = "0.6", path = "../workspace-hack" }
```

**Validation / Scope rule:** Add `alacritty_terminal = { workspace = true }` and `tempfile = { workspace = true }` to `crates/vox-terminal-core/Cargo.toml`. Both crates are already pinned in root `Cargo.toml` and `Cargo.lock` (zero external network fetches, zero new `vox-*` edges), **and the task's `<files>` list MUST include `Cargo.lock`** because Cargo updates `vox-terminal-core`'s internal `dependencies` array in `Cargo.lock`.

---

### `crates/vox-terminal-core/src/lib.rs` (config, transform)

**Analog:** `crates/vox-terminal-core/src/lib.rs`

**Module & flat re-export pattern** (`crates/vox-terminal-core/src/lib.rs` lines 20-46):
```rust
pub mod agent;
pub mod block;
pub mod commands;
pub mod corpus;
pub mod daemon_host;
pub mod host;
pub mod input;
pub mod osc633;
pub mod pty;
pub mod session;
pub mod transcript;
pub mod vox_interp;

// Flat re-exports for ergonomic use by front-ends.
pub use agent::{AgentAdapterConfig, translate_event};
pub use block::{Block, BlockId, BlockKind, BlockOrigin, BlockStatus, OutputChunk, Stream};
pub use daemon_host::DaemonHost;
pub use host::{
    Attachment, LocalHost, OpenSpec, OutputFrame, RedactedBlockContext, SessionHost, SessionId,
    SessionInfo, TermControlMode, TermSecurityLevel,
};
pub use input::{InputIntent, classify};
pub use osc633::{Osc633Event, Osc633Parser};
pub use pty::{PtyHandle, ShellBackend, ShellKind, default_shell, spawn_pty};
pub use session::{Session, SessionEvent};
pub use transcript::{JournalSink, NullSink, TranscriptEvent, TranscriptKind, TranscriptSink};
pub use vox_interp::eval_line;
```

**Extension pattern:** Register `pub mod shell_init;`, `pub mod utf8_stream;`, and `pub mod vt_shadow;` alongside existing modules, and re-export `TermSignal` from `pty`, `ShellLaunchShim` / `prepare_shell_command` from `shell_init`, `Utf8StreamDecoder` from `utf8_stream`, and `VtShadow` from `vt_shadow` while keeping every existing `pub use` intact so `vox-gui` and `vox-term` imports stay backward-compatible.

---

### `crates/vox-terminal-core/src/shell_init.rs` *(new)* (utility, file-I/O)

**Analog:** `crates/vox-terminal-core/src/pty.rs` (shell basename normalization + 5-shell OSC 633 scripts) & `crates/vox-cli/src/commands/plugin_bundle/verify.rs` (`tempfile::TempDir` creation + `anyhow::Context`)

**Imports & TempDir pattern** (`crates/vox-cli/src/commands/plugin_bundle/verify.rs` lines 11-18):
```rust
use anyhow::{Context, Result};
use std::path::Path;
// ...
let tmp = tempfile::tempdir().context("creating tempdir for bundle extraction")?;
```

**Shell basename normalization pattern** (`crates/vox-terminal-core/src/pty.rs` lines 30-45):
```rust
pub fn shell_integration_snippet(shell: &str) -> Option<String> {
    let base = shell
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(shell)
        .to_ascii_lowercase();
    let name = base.strip_suffix(".exe").unwrap_or(&base);
    match name {
        "pwsh" | "powershell" => Some(PWSH_OSC633.to_string()),
        "bash" => Some(BASH_OSC633.to_string()),
        "zsh" => Some(ZSH_OSC633.to_string()),
        "fish" => Some(FISH_OSC633.to_string()),
        "nu" => Some(NU_OSC633.to_string()),
        _ => None,
    }
}
```

**Core pattern** (`crates/vox-terminal-core/src/pty.rs` lines 62-170 + `441-RESEARCH.md` Pattern 1):
- Build `tempfile::Builder::new().prefix("vox-shell-").tempdir()` (`0700` owner-only dir on Unix) and return `(portable_pty::CommandBuilder, Option<tempfile::TempDir>)`.
- Configure `CommandBuilder` argv and env per shell without writing any bytes to PTY stdin:
  - `zsh`: set `VOX_ORIG_ZDOTDIR` (if `ZDOTDIR` was set) and `ZDOTDIR=<shim_dir>`; write `.zshenv` (sources user `.zshenv`, keeps `ZDOTDIR=<shim_dir>` while recording user changes in `VOX_USER_ZDOTDIR`), `.zprofile`, and `.zshrc` (restores `ZDOTDIR`, runs `unsetopt PROMPT_SP`, sources user `.zshrc`, prepends `__vox_precmd` with `local ec=$?` on line 1 to `precmd_functions`, installs `EXIT` trap for `exit N`, emits `OSC 7` + `633;P;Cwd=` + `633;P;GitBranch=`, and wraps `633;A`/`633;B` in `%{...%}`).
  - `bash`: pass `--rcfile <shim_dir>/bashrc -i`; source `~/.bashrc`, preserve string/array `PROMPT_COMMAND` with `local ec=$?` captured first, set `__VOX_AT_PROMPT=1` after user `PROMPT_COMMAND`, wrap `PS1` markers in `\[...\]`, and install `EXIT` trap.
  - `fish`: write `<shim_dir>/init.fish` and pass `-i --init-command "source '<escaped_path>'"` (single-quote escaped).
  - `pwsh` / `powershell`: write `<shim_dir>/init.ps1` and pass `-NoLogo -NoExit -File <shim_dir>/init.ps1`; dot-source `$PROFILE` if present, read `$LASTEXITCODE` / `$?` then reset `$global:LASTEXITCODE = 0`, and guard `Set-PSReadLineKeyHandler` behind `if (Get-Module -Name PSReadLine)`.
  - `nu`: write `<shim_dir>/config.nu` and pass `--config <shim_dir>/config.nu` (plus `--env-config` if user `env.nu` exists); probe user `config.nu` on disk in Rust and splice static `source '<escaped_literal_path>'` only when `Path::is_file()` is true.
  - Unknown shells (`sh`, `cmd.exe`): return plain `CommandBuilder::new(shell)` and `None` `TempDir`.

**Security / Validation pattern:** Never use predictable `/tmp/vox-*` paths; single-quote escape paths (`'` → `'\''` for Fish, single-quoted raw string or `''` as appropriate) before splicing into `--init-command` or `config.nu`.

---

### `crates/vox-terminal-core/src/utf8_stream.rs` *(new)* (utility, streaming)

**Analog:** `crates/vox-terminal-core/src/osc633.rs` (stateful byte-ring streaming parser) & `crates/vox-terminal-core/src/block.rs` (`strip_ansi` stateful character scanner)

**Stateful byte-stream struct pattern** (`crates/vox-terminal-core/src/osc633.rs` lines 44-63):
```rust
#[derive(Default)]
pub struct Osc633Parser {
    /// Internal ring buffer — holds partial data across calls.
    buf: Vec<u8>,
    /// True when we are currently inside a `C` (PreExec) marker — capturing output.
    capturing: bool,
    /// Pending output bytes accumulated between a `C` and `D` marker.
    output_buf: Vec<u8>,
}

impl Osc633Parser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed raw PTY bytes, return any events parsed from this batch.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<Osc633Event> {
        self.buf.extend_from_slice(bytes);
        self.parse()
    }
```

**Core pattern** (`441-RESEARCH.md` lines 374-421 + line 199):
- `Utf8StreamDecoder { tail: [u8; 3], tail_len: usize }` using `std::str::from_utf8(rest)`:
  - `Ok(valid)` → push valid slice, done.
  - `Err(err)` → push `&rest[..err.valid_up_to()]`, then match `err.error_len()`:
    - `None` → copy `1..=3` trailing incomplete bytes into `self.tail[..rest.len()]` for the next `push()` call (`0` `U+FFFD` emitted on split multi-byte characters).
    - `Some(bad_len)` → push `\u{FFFD}` and advance by `bad_len` for genuinely invalid UTF-8.
- Stateful carriage-return line-overwrite normalizer (`CrLineNormalizer` or helper used by `Block`): track `pending_cr: bool` across chunk boundaries so split `"\r"` + `"\n"` stays a normal `\n` line break, while `"\r"` followed by non-`\n` printable text or `\x1b[K`/`\x1b[2K` overwrites the current line from column `0` (normalizing spinners and `cargo build` progress bars).

---

### `crates/vox-terminal-core/src/vt_shadow.rs` *(new)* (model, streaming)

**Analog:** `crates/vox-term/src/vt.rs`

**Imports & `Dimensions` / `EventListener` adapter pattern** (`crates/vox-term/src/vt.rs` lines 9-45):
```rust
use alacritty_terminal::{
    Term,
    event::{Event, EventListener},
    grid::Dimensions,
    index::{Column, Line, Point},
    term::{
        Config,
        cell::{Cell, Flags},
    },
    vte::ansi::{Color, NamedColor, Processor, Rgb},
};

struct NoopListener;
impl EventListener for NoopListener {
    fn send_event(&self, _: Event) {}
}

struct GridSize {
    cols: usize,
    lines: usize,
}

impl Dimensions for GridSize {
    fn columns(&self) -> usize {
        self.cols
    }
    fn screen_lines(&self) -> usize {
        self.lines
    }
    fn total_lines(&self) -> usize {
        self.lines
    }
}
```

**Core `Term` + `Processor` feed & resize pattern** (`crates/vox-term/src/vt.rs` lines 48-84):
```rust
pub struct VtGrid {
    term: Term<NoopListener>,
    parser: Processor,
    cols: usize,
    rows: usize,
}

impl VtGrid {
    pub fn new(cols: u16, rows: u16) -> Self {
        let size = GridSize {
            cols: cols as usize,
            lines: rows as usize,
        };
        let term = Term::new(Config::default(), &size, NoopListener);
        Self {
            term,
            parser: Processor::new(),
            cols: cols as usize,
            rows: rows as usize,
        }
    }

    /// Feed raw bytes (PTY output) into the VT grid.
    pub fn feed(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.term, bytes);
    }

    /// Resize the grid.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        let size = GridSize {
            cols: cols as usize,
            lines: rows as usize,
        };
        self.term.resize(size);
        self.cols = cols as usize;
        self.rows = rows as usize;
    }
```

**Adaptation for `VtShadow` in `vox-terminal-core`:**
- Omit `ratatui` imports (since `vox-terminal-core` is UI-agnostic `L3`).
- Use `Config { scrolling_history: 0, ..Config::default() }` to avoid 10,000-line scrollback duplication.
- Clamp dimensions with `cols.max(1) as usize` and `rows.max(1) as usize` to prevent zero-size grid panics.
- Check `self.term.mode().contains(TermMode::ALT_SCREEN)` after `self.parser.advance(&mut self.term, bytes)` and return `Option<bool>` when `alt_screen` toggles.

---

### `crates/vox-terminal-core/src/pty.rs` (provider, streaming)

**Analog:** `crates/vox-terminal-core/src/pty.rs` (self — targeted refactor)

**Imports pattern** (`crates/vox-terminal-core/src/pty.rs` lines 11-16):
```rust
use std::io::Read;

use anyhow::Result;
use portable_pty::{Child, CommandBuilder, PtySize, native_pty_system};
use tokio::sync::mpsc;
```

**Core `PtyHandle` & `spawn_pty` baseline to modify** (`crates/vox-terminal-core/src/pty.rs` lines 306-394):
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
// ...
pub fn spawn_pty(
    shell: &str,
    cols: u16,
    rows: u16,
) -> Result<(PtyHandle, mpsc::Receiver<Vec<u8>>)> {
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let cmd = CommandBuilder::new(shell);
    let child = pair.slave.spawn_command(cmd)?;
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader()?;
    let mut writer = pair.master.take_writer()?;

    // Inject OSC 633 integration (best-effort; ignore write errors)
    if let Some(snippet) = shell_integration_snippet(shell) {
        use std::io::Write;
        let _ = writer.write_all(format!("{snippet}\n").as_bytes());
        let _ = writer.flush();
    }
```

**Modification & Preservation rules:**
- **Preserve** `pub fn shell_integration_snippet(shell: &str) -> Option<String>` (lines 30-45) and `ZSH_OSC633`/`FISH_OSC633`/`NU_OSC633`/`PWSH_OSC633`/`BASH_OSC633` constants (fixing `ZSH_OSC633` hook order if desired while keeping all existing unit tests in `crates/vox-terminal-core/tests/pty.rs:9-34` and `crates/vox-gui/src/commands/pty.rs:320-369` passing).
- **Remove** lines 367-372 (`writer.write_all` stdin snippet injection) from `spawn_pty`; instead build `(cmd, shim_dir)` via `crate::shell_init::prepare_shell_command(shell, cwd)`.
- **Retain** `master: Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>` and `_shim_dir: Option<tempfile::TempDir>` inside `PtyHandle`.
- **Implement** real `PtyHandle::resize(&self, cols, rows)` calling `master.lock()...resize(PtySize { rows: rows.max(1), cols: cols.max(1), pixel_width: 0, pixel_height: 0 })` and `PtyHandle::signal(&mut self, sig: TermSignal)` (`Interrupt` → `self.write(b"\x03")`, `Terminate | Kill` → `self.child.kill()`).
- Provide `spawn_pty_with_callback` (or equivalent callback hook for `LocalHost`) so `LocalHost` can pump bytes directly on the OS reader thread without hopping through `tokio::spawn`, while keeping `pub fn spawn_pty(shell, cols, rows) -> Result<(PtyHandle, mpsc::Receiver<Vec<u8>>)>` signature intact for `vox-gui`'s legacy `pty_spawn` (`crates/vox-gui/src/commands/pty.rs:136`).

---

### `crates/vox-terminal-core/src/osc633.rs` (utility, streaming)

**Analog:** `crates/vox-terminal-core/src/osc633.rs` (self — targeted refactor)

**Event enum & parser baseline** (`crates/vox-terminal-core/src/osc633.rs` lines 8-16, 72-162):
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Osc633Event {
    PromptStart,
    PromptEnd,
    CommandLine(String),
    PreExec,
    Exit(i32),
    Output(String),
}
```
```rust
    fn process_marker(
        &mut self,
        payload: &str,
        events: &mut Vec<Osc633Event>,
    ) -> Option<Osc633Event> {
        // payload is everything after "633;"
        if let Some(rest) = payload.strip_prefix("633;") {
            match rest {
                "A" => return Some(Osc633Event::PromptStart),
                "B" => return None, // no-op per osc633.ts
                "C" => {
                    self.capturing = true;
                    self.output_buf.clear();
                    return Some(Osc633Event::PreExec);
                }
                _ if rest.starts_with("E;") => {
                    let cmd = decode_command(&rest[2..]);
                    return Some(Osc633Event::CommandLine(cmd));
                }
                _ if rest.starts_with("D;") || rest == "D" => {
                    if self.capturing {
                        // Flush captured output
                        let s = String::from_utf8_lossy(&self.output_buf);
                        // ...
```

**Modification rules:**
- Replace `String::from_utf8_lossy` and `self.output_buf` hoarding until `D` with `Utf8StreamDecoder` so multi-byte UTF-8 split across chunks emits zero `U+FFFD` and `Osc633Event::Output` streams **live** between `633;C` and `633;D`.
- Emit `Osc633Event::PromptEnd` on `"B"`.
- Parse `633;P;<Key>=<Value>` (`Property { key, value }`, e.g. `Cwd=...`, `GitBranch=...`) and `OSC 7` (`\x1b]7;file://<host>/<path>\x07` or `\x1b\\`, URL-percent-decoded into `Osc7Cwd(String)`), while keeping `non_633_osc_passes_through_as_output` (`crates/vox-terminal-core/tests/osc633.rs:41-54`) passing for other OSC sequences (`OSC 1`, `OSC 2`, etc.).
- Enforce a `16 KiB` cap on incomplete OSC sequence buffering in `parse()` so malformed binary streams containing `\x1b]633;` without BEL/ST flush safely as output rather than growing unboundedly.

---

### `crates/vox-terminal-core/src/block.rs` (model, transform)

**Analog:** `crates/vox-terminal-core/src/block.rs` (self — targeted extension)

**Core `Block` struct & `plain_output` baseline** (`crates/vox-terminal-core/src/block.rs` lines 56-105):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub id: BlockId,
    pub kind: BlockKind,
    #[serde(default)]
    pub origin: BlockOrigin,
    pub input: String,
    pub output: Vec<OutputChunk>,
    pub status: BlockStatus,
    pub exit_code: Option<i32>,
}
// ...
    pub fn plain_output(&self) -> String {
        let raw: String = self.output.iter().map(|c| c.text.as_str()).collect();
        strip_ansi(&raw)
    }
```

**Modification rules:**
- Add optional `#[serde(default, skip_serializing_if = "Option::is_none")] pub cwd: Option<String>` and `#[serde(default, skip_serializing_if = "Option::is_none")] pub git_branch: Option<String>` if surfaced on `Block` (using `#[serde(default)]` so existing serialized `Block` fixtures deserialize cleanly).
- Normalize `\r` progress-bar / spinner line overwrites (`\r` without `\n`, plus `\x1b[K` / `\x1b[2K` erase-in-line) in `plain_output()` / block text projection while preserving split `\r\n` newlines across `OutputChunk`s.
- Add `Block::cancel(&mut self)` (setting `self.status = BlockStatus::Cancelled` and `self.exit_code = Some(130)` when interrupted).

---

### `crates/vox-terminal-core/src/session.rs` (service, event-driven)

**Analog:** `crates/vox-terminal-core/src/session.rs` (self — targeted refactor)

**Event enum & `apply_osc` baseline** (`crates/vox-terminal-core/src/session.rs` lines 15-30, 210-256):
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SessionEvent {
    BlockOpened { id: BlockId },
    OutputAppended { id: BlockId, chunk: OutputChunk },
    BlockClosed { id: BlockId },
    AgentMessage { text: String },
}

pub struct Session {
    pub id: String,
    blocks: Vec<Block>,
    next_id: u64,
    open_id: Option<BlockId>,
    parser: Osc633Parser,
    tx: broadcast::Sender<SessionEvent>,
}
```

**Modification rules:**
- Extend `SessionEvent` with `AltScreenChanged { active: bool }` and `CwdChanged { cwd: String, git_branch: Option<String> }` (serde-compatible enum variants).
- Embed `VtShadow` inside `Session` (fed in `on_pty_bytes(bytes)`): when `VtShadow::feed` reports a toggle, update `self.alt_screen` and emit `SessionEvent::AltScreenChanged { active }`. While `self.alt_screen == true`, suppress opening/splitting new command `Block`s on `OSC 633` markers.
- **Fix idle-prompt phantom block bug:** Do NOT open a `Running` `Block` merely on `Osc633Event::PromptStart` (`633;A`)! Instead:
  - Keep `Session::new` stateless (`run_shell_capture`) so all synchronous unit tests (`crates/vox-terminal-core/src/session.rs:264-334` and `crates/vox-terminal-core/tests/session.rs:5-36`) keep working. Note `crates/vox-terminal-core/tests/session.rs:30-36` (`block_ids_are_unique` feeds `633;A` then `633;D;0` without `E`/`C`): handle `633;D` after `633;A` in stateless/raw OSC mode or open block on `633;E` / `633;C` (and on `633;D` if `saw_prompt_start && open_id.is_none()` in synthetic non-PTY feeds, or distinguish PTY-backed vs synthetic `Session::new` cleanly — verify `crates/vox-terminal-core/tests/session.rs` AND `crates/vox-term/tests/headless.rs:90` `assert!(attached.blocks.is_empty())` both pass!).
  - For PTY-backed sessions (`Session::new_pty` / `LocalHost`), initial shell prompt `633;A ... 633;B` must leave `blocks` **empty** (`attached.blocks.is_empty() == true`). When `Session` prepares a stateful PTY shell submission (`begin_pty_submission(cmd, origin)`), reserve/open the `Running` `Block` (or attach to `633;E`/`633;C`) and write `<cmd>\n` to `PtyHandle`; if a foreground `Block` is already `Running`, forward `<line>\n` directly to PTY stdin without opening a second overlapping `Running` block!

---

### `crates/vox-terminal-core/src/host.rs` (service, pub-sub)

**Analog:** `crates/vox-terminal-core/src/host.rs` (self — targeted refactor)

**Lock order & `Entry::feed` pattern** (`crates/vox-terminal-core/src/host.rs` lines 240-290):
```rust
struct Replay {
    buf: VecDeque<u8>,
    seq: u64,
    cap: usize,
}

struct Entry {
    session: Mutex<Session>,
    pty: Mutex<Option<PtyHandle>>,
    replay: Mutex<Replay>,
    out_tx: broadcast::Sender<OutputFrame>,
    security_level: Mutex<TermSecurityLevel>,
    control_mode: Mutex<TermControlMode>,
}
// ...
    /// Record raw output (ring + seq), fan it out, and feed the block parser.
    /// Lock order is replay → session everywhere, so attach() sees both consistently.
    fn feed(&self, bytes: &[u8]) {
        let mut r = lock(&self.replay);
        r.seq += 1;
        r.buf.extend(bytes);
        let over = r.buf.len().saturating_sub(r.cap);
        r.buf.drain(..over);
        let _ = self.out_tx.send(OutputFrame {
            seq: r.seq,
            bytes: bytes.to_vec(),
        });
        lock(&self.session).on_pty_bytes(bytes);
    }

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}
```

**`SessionHost` trait & `OpenSpec` / `SessionInfo` / `Attachment` baseline** (`crates/vox-terminal-core/src/host.rs` lines 23-38, 157-238):
```rust
impl Default for OpenSpec {
    fn default() -> Self {
        Self {
            cols: 80,
            rows: 24,
            shell: None,
        }
    }
}
// ...
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SessionInfo {
    pub id: SessionId,
    pub blocks: usize,
    #[serde(default)]
    pub security_level: TermSecurityLevel,
    #[serde(default)]
    pub control_mode: TermControlMode,
}
// ...
pub struct Attachment {
    pub blocks: Vec<Block>,
    /// Last `replay_cap` bytes of raw PTY output.
    pub replay: Vec<u8>,
    /// `seq` of the newest frame included in `replay` (0 = none yet).
    pub seq: u64,
    pub events: broadcast::Receiver<SessionEvent>,
    pub output: broadcast::Receiver<OutputFrame>,
}
```

**Modification rules:**
- Change `OpenSpec::default()` to `cols: 120, rows: 32, shell: None` (and default `0` cols/rows in `open` to `120×32`).
- Add `#[serde(default)] pub alt_screen: bool`, `#[serde(default)] pub cwd: Option<String>`, and `#[serde(default)] pub git_branch: Option<String>` to `SessionInfo`, and `pub resync_required: bool`, `pub alt_screen: bool`, `pub cwd: Option<String>`, `pub git_branch: Option<String>` to `Attachment`.
- Refactor `Replay` to store `frames: VecDeque<OutputFrame>` + `total_bytes: usize` + `seq: u64` + `cap: usize` (`DEFAULT_REPLAY_CAP = 2 * 1024 * 1024`), supporting `attach_since(&self, id: &str, after_seq: Option<u64>) -> Result<Attachment>` while keeping inherent `pub fn attach(&self, id: &str) -> Result<Attachment>` delegating to `self.attach_since(id, None)` so `crates/vox-gui/tests/real_backend_harness.rs:205` and internal unit tests compile unchanged!
- Update `SessionHost` trait:
  - `async fn attach(&self, id: &str, after_seq: Option<u64>) -> Result<Attachment>;`
  - `async fn signal(&self, id: &str, sig: crate::pty::TermSignal) -> Result<()>;`
- Pump PTY bytes directly on the OS reader thread into `Entry::feed` and notify a `std::sync::Condvar` on `Entry` so `LocalHost::open` (awaiting initial prompt readiness or short timeout) and synchronous `LocalHost::submit` (writing `<cmd>\n` to the live PTY for `Shell` / shell-fallback `VoxNative`, settling via `Condvar::wait_timeout` without deadlocking single-threaded `#[tokio::test]` executors!) execute against the stateful PTY when `pty.is_some()` (and fall back cleanly if the shell has no OSC 633 hooks, e.g., `sh` in `openspec_custom_shell_and_structured_shell_submission` at `crates/vox-terminal-core/tests/session.rs:116-130` — note that test submits `"!nu echo structured-test"` which hits `InputIntent::StructuredShell`, whereas `sh` without OSC 633 should also work if someone submits a plain command to `sh` by either emitting POSIX `PS1` markers or sentinel/fallback!).
- Propagate `LocalHost::resize` to both `pty.resize(cols, rows)` and `session.resize(cols, rows)` (`VtShadow`).

---

### `crates/vox-terminal-core/src/daemon_host.rs` (provider, streaming)

**Analog:** `crates/vox-terminal-core/src/daemon_host.rs` (self — targeted extension)

**Wire method constants & `route` / `attach` pattern** (`crates/vox-terminal-core/src/daemon_host.rs` lines 20-27, 129-175, 234-276):
```rust
pub const OPEN: &str = "term.open";
pub const LIST: &str = "term.list";
pub const ATTACH: &str = "term.attach";
pub const INPUT: &str = "term.input";
pub const SUBMIT: &str = "term.submit";
pub const RESIZE: &str = "term.resize";
pub const CLOSE: &str = "term.close";
```
```rust
    async fn attach(&self, id: &str) -> Result<Attachment> {
        let (tx, mut rx) = mpsc::channel::<Value>(256);
        let client = self.client.clone();
        let params = json!({"session_id": id});
        let stream =
            tokio::spawn(async move { client.subscribe_with_method(ATTACH, params, tx).await });
```

**Modification rules:**
- Add `pub const SIGNAL: &str = "term.signal";` (keep `vox-foundation` `L0` untouched; `term_dispatch.rs` matches `"term.signal"` alongside `"term.context"`, `"term.security"`, `"term.control"`).
- Update `SessionHost::list` deserialization (`lines 210-232`) to read `alt_screen`, `cwd`, and `git_branch` with `#[serde(default)]` / `.unwrap_or_default()`.
- Update `SessionHost::attach(&self, id: &str, after_seq: Option<u64>)` to send `"after_seq": after_seq` in `params`, parse `resync_required`, `alt_screen`, `cwd`, and `git_branch` from `snap`, and in `pump`: when `frame["type"] == "lagged"`, automatically re-subscribe with `after_seq = Some(last_seen_seq)` on `self.client.clone()` instead of dropping the client stream!
- Route `"alt_screen"` → `SessionEvent::AltScreenChanged { active }` and `"cwd_changed"` → `SessionEvent::CwdChanged { cwd, git_branch }` in `route()`.
- Implement `async fn signal(&self, id: &str, sig: TermSignal) -> Result<()>` calling `self.client.call(SIGNAL, json!({"session_id": id, "signal": sig})).await.map(drop)`.

---

### `crates/vox-terminal-core/tests/shell_init_and_stateful_pty.rs` *(new)* (test, event-driven)

**Analog:** `crates/vox-terminal-core/tests/session.rs` & `crates/vox-terminal-core/tests/pty.rs`

**Imports & async `LocalHost` test pattern** (`crates/vox-terminal-core/tests/session.rs` lines 1-4, 38-60):
```rust
use vox_terminal_core::block::{BlockKind, BlockOrigin, BlockStatus};
use vox_terminal_core::host::{LocalHost, OpenSpec};
use vox_terminal_core::session::Session;
// ...
#[tokio::test]
async fn submit_with_agent_origin_tags_block() {
    let host = LocalHost::default();
    let id = host.open(OpenSpec::default()).await.unwrap();
    let block_id = host
        .submit_with_origin(
            &id,
            "!echo agent-hi",
            BlockOrigin::Agent {
                agent_id: "agent-7".to_string(),
            },
        )
        .unwrap();
    let b = host.block(&id, block_id).expect("block exists");
    assert_eq!(
        b.origin,
        BlockOrigin::Agent {
            agent_id: "agent-7".to_string()
        }
    );
    assert!(b.plain_output().contains("agent-hi"));
    host.close(&id).unwrap();
}
```

**Required test cases to include (`TERM-PTY-01`, `TERM-PTY-02`):**
1. Deterministic unit assertions over `prepare_shell_command` for all 5 shells (`zsh`, `bash`, `fish`, `pwsh`, `nu`): verify `TempDir` exists, `0700` permissions on Unix, user dotfile sourcing precedes Vox hooks, `unsetopt PROMPT_SP` + `%{...%}` in `.zshrc`, static `source '...'` splicing in `config.nu` when a temp user `config.nu` exists, and RAII removal of `TempDir` path when `PtyHandle` / `LocalHost::close` drops.
2. Live PTY startup hygiene (`zsh` and `bash` on Unix, `pwsh` when on `PATH`): open session, inspect `attach(&id).replay` — assert **zero** occurrences of `__vox_preexec`, `__VOX_OSC633`, `add-zsh-hook`, or inverse `%` `PROMPT_SP` spam, and `attached.blocks.is_empty()`.
3. Non-zero exit codes in `zsh` and `bash`: submit `!false` → `exit_code == Some(1)`, `status == BlockStatus::Failed`; submit `!(exit 7)` / top-level `exit 7` → `exit_code == Some(7)`.
4. Stateful persistence & `OSC 7` / `633;P;Cwd`: submit `!cd <tempdir>` + `!export VOX_STATE_VAR=persisted_441`, then in a second `submit` run `!pwd && echo $VOX_STATE_VAR` → verify both `cwd` and `VOX_STATE_VAR` persisted across commands and `SessionInfo.cwd` updated via `OSC 7` / `633;P;Cwd`.
5. Interactive stdin into `Running` block: submit `!read VOX_REPLY && echo "got:$VOX_REPLY"`, verify block is `Running`, send `"yes\n"` via `submit` / `input`, verify same block finishes `Ok` with `"got:yes"`.
6. Real kernel `resize` & `TermSignal::Interrupt`: resize session to `132×43`, run `!stty size` or `!tput cols` → verify `132` columns; start `!sleep 30`, call `host.signal(&id, TermSignal::Interrupt)`, verify foreground sleep interrupts without killing the shell (`!echo still-alive` succeeds afterwards).

---

### `crates/vox-terminal-core/tests/streaming_utf8_and_alt_screen.rs` *(new)* (test, streaming)

**Analog:** `crates/vox-terminal-core/tests/osc633.rs` & `crates/vox-terminal-core/src/host.rs` (`mod tests`)

**Parser chunk-feeding test pattern** (`crates/vox-terminal-core/tests/osc633.rs` lines 1-31):
```rust
use vox_terminal_core::osc633::{Osc633Event, Osc633Parser};

#[test]
fn marker_split_across_feeds() {
    // Partial ESC sequence fed across two calls — must not lose data
    let mut p = Osc633Parser::new();
    let mut evs = vec![];
    // split "\x1b]633;A\x07" after ESC
    evs.extend(p.feed(b"\x1b"));
    evs.extend(p.feed(b"]633;A\x07"));
    assert!(evs.contains(&Osc633Event::PromptStart));
}
```

**Replay `seq` test pattern** (`crates/vox-terminal-core/src/host.rs` lines 516-534):
```rust
    #[tokio::test]
    async fn replay_keeps_only_the_newest_bytes_and_tracks_seq() {
        let host = LocalHost::with_replay_cap(8);
        // No PTY: a real shell's startup output would race the exact assertions.
        let id = "t".to_string();
        let entry = Entry::new(&id, None, host.replay_cap);
        lock(&host.sessions).insert(id.clone(), Arc::clone(&entry));
        entry.feed(b"hello ");
        entry.feed(b"world!!!");
```

**Required test cases to include (`TERM-PTY-03`):**
1. Prime-sized chunking (`7`, `13`, `31`, and `1` byte chunks) of a payload mixing 3-byte box-drawing (`╭─╮│╰─╯`), 4-byte emojis (`🦀🚀🧪`), ANSI SGR sequences, `OSC 7`, and `OSC 633` markers through `Utf8StreamDecoder` and `Osc633Parser` → assert `0` `\u{FFFD}` characters and byte-exact reconstruction.
2. Live streaming between `633;C` and `633;D`: feed `633;C` + `"chunk-1\n"` without `633;D` → assert `Osc633Event::Output` / `SessionEvent::OutputAppended` fires immediately before `633;D` arrives.
3. `\r` spinner / progress-bar overwrite normalization: feed `"Compiling 1/10\rCompiling 2/10\rCompiling 10/10\r\nDone\r\n"` (including split `"\r"` + `"\n"` across chunks) → assert `block.plain_output()` equals `"Compiling 10/10\nDone\n"`.
4. `VtShadow` `ALT_SCREEN` transitions (`\x1b[?1049h`, `\x1b[?1049l`, `\x1b[?47h`, `\x1b[?1049;2004h`, and `\x1bc` RIS reset): assert `SessionEvent::AltScreenChanged { active: true }` then `{ active: false }`, and assert `OSC 633` markers emitted while `alt_screen == true` do NOT split or open new `Block`s.
5. `after_seq` delta ring & eviction resync: feed frames `seq 1..=5`, assert `attach_since(&id, Some(2))` returns only `seq 3..=5` bytes with `resync_required == false`, `attach_since(&id, Some(5))` returns empty `replay` with `resync_required == false`, and after evicting `seq 1..=3` past `replay_cap`, `attach_since(&id, Some(1))` sets `resync_required == true` and returns the full retained window.

---

### `crates/vox-term/src/vt.rs` (component, streaming)

**Analog:** `crates/vox-term/src/vt.rs` (self — minor enhancement)

**Core pattern** (`crates/vox-term/src/vt.rs` lines 48-85):
```rust
pub struct VtGrid {
    term: Term<NoopListener>,
    parser: Processor,
    cols: usize,
    rows: usize,
}
```

**Modification rules:**
- Clamp `cols.max(1)` / `rows.max(1)` in `VtGrid::new` and `VtGrid::resize`.
- Add `pub fn is_alt_screen(&self) -> bool` (`self.term.mode().contains(alacritty_terminal::term::TermMode::ALT_SCREEN)`) and `pub fn reset(&mut self)` (re-initializing `Term` + `Processor` when handling `Attachment { resync_required: true, .. }` so full-screen/lagged resyncs repaint cleanly).

---

### `crates/vox-term/src/app.rs` (controller, event-driven)

**Analog:** `crates/vox-term/src/app.rs` (self — wire `SessionHost` attach loop)

**CLI args & entry-point baseline** (`crates/vox-term/src/app.rs` lines 33-75, 98-120):
```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TermCliArgs {
    pub attach: Option<String>,
    pub force_local: bool,
}
// ...
pub async fn resolve_host(force_local: bool) -> std::sync::Arc<dyn SessionHost> {
    resolve_session_host(force_local).await
}
// ...
pub fn run() -> Result<()> {
    let _cli_args = TermCliArgs::from_env();
    if is_headless() {
        return run_plain();
    }
```

**Modification rules:**
- Support bare `--attach` (defaulting to `"latest"` in `TermCliArgs::parse_iter` when `--attach` is at end of argv or next token starts with `--`).
- Expose `pub async fn run_with_args(args: TermCliArgs) -> Result<()>` (awaited directly by `vox-cli`'s async `dispatch_cli_inner` — **never** build a nested `tokio::runtime::Runtime` inside `run_with_args`!). Keep synchronous `pub fn run() -> Result<()>` for standalone `crates/vox-term/src/main.rs` (`Runtime::new()?.block_on(run_with_args(TermCliArgs::from_env()))`, checking `is_headless()` when `args.attach.is_none()` so existing `crates/vox-term/tests/headless.rs:52-68` piped-stdin tests stay 100% green!).
- Extract a pure/testable async helper `pub async fn attach_or_open_session(host: &dyn SessionHost, attach: Option<&str>, cols: u16, rows: u16) -> Result<(String, vox_terminal_core::Attachment)>`:
  - `Some("latest")` → call `host.list().await?`, pick highest numeric `"term-{n}"` suffix (`id.strip_prefix("term-").and_then(|s| s.parse::<u64>().ok())`), or bail with an actionable error (`"no live terminal sessions to attach to; start one with 'vox term'"`) if empty.
  - `Some(target_id)` → call `host.attach(target_id, None).await` and surface an explicit actionable error naming `target_id` and available session IDs if not found.
  - `None` → call `host.open(OpenSpec { cols, rows, shell: None }).await?` then `host.attach(&id, None).await?`.
- Drive the TUI render/input loop against `SessionHost` (`Attachment.blocks`, `Attachment.replay` → `VtGrid`, `Attachment.output`, `Attachment.events`, `Event::Resize(c, r)` → `host.resize` + `grid.resize`, `Ctrl-C` → `host.signal(&id, TermSignal::Interrupt)` when a block is running or `alt_screen` is active, and `TryRecvError::Lagged` → `host.attach(&id, Some(last_seq))` delta resync).

---

### `crates/vox-term/tests/headless.rs` (test, request-response)

**Analog:** `crates/vox-term/tests/headless.rs` (self — extend existing test file)

**Baseline `SessionHost::attach` test** (`crates/vox-term/tests/headless.rs` lines 70-92):
```rust
#[test]
fn parse_cli_args_handles_attach_and_local() {
    let args =
        vox_term::app::TermCliArgs::parse_iter(["vox-term", "--attach", "term-2", "--local"]);
    assert_eq!(args.attach.as_deref(), Some("term-2"));
    assert!(args.force_local);
// ...
}

#[tokio::test]
async fn resolve_host_falls_back_to_local_when_daemon_unreachable() {
    let host = vox_term::app::resolve_host(true).await;
    let id = host
        .open(vox_terminal_core::OpenSpec::default())
        .await
        .expect("open local session");
    let attached = host.attach(&id).await.expect("attach local session");
    assert!(attached.blocks.is_empty());
    host.close(&id).await.expect("close local session");
}
```

**Modification rules:**
- Update line 89 `host.attach(&id).await` → `host.attach(&id, None).await` (keep `assert!(attached.blocks.is_empty());` on line 90 intact!).
- Add tests for:
  1. Bare `--attach` parsing (`["vox-term", "--attach"]` and `["vox-term", "--attach", "--local"]` → `attach == Some("latest")`).
  2. `attach_or_open_session` with explicit `<session_id>`, `"latest"` (verifying `"term-10"` beats `"term-9"` numerically!), and nonexistent `"term-999"` returning an explicit actionable error.

---

### `crates/vox-term/tests/palette.rs` (test, transform)

**Analog:** `crates/vox-term/tests/palette.rs` (self — struct literal update)

**Exact struct literal site** (`crates/vox-term/tests/palette.rs` lines 34-49):
```rust
#[test]
fn palette_includes_live_sessions() {
    let mut palette = Palette::new(default_commands());
    palette.with_sessions(&[vox_terminal_core::SessionInfo {
        id: "term-1".into(),
        blocks: 3,
        security_level: Default::default(),
        control_mode: Default::default(),
    }]);
```

**Modification rules:** Populate the new `SessionInfo` fields (`alt_screen: false, cwd: None, git_branch: None`, or derive `Default` on `SessionInfo` and use `..Default::default()`) so `cargo test -p vox-term` compiles cleanly.

---

### `crates/vox-orchestrator-d/src/term_dispatch.rs` (controller, streaming)

**Analog:** `crates/vox-orchestrator-d/src/term_dispatch.rs` (self — targeted RPC & stream extension)

**Imports & wire origin guard pattern** (`crates/vox-orchestrator-d/src/term_dispatch.rs` lines 15-22, 90-97):
```rust
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use serde_json::{Value, json};
use vox_foundation::protocol::{DispatchPayload, DispatchRequest, DispatchResponse, term_method};
use vox_orchestrator::orch_daemon::{ExtraDispatch, FrameSink, response_err, response_result};
// ...
fn refuse_agent_origin(p: &Value) -> anyhow::Result<()> {
    match p.get("origin").and_then(Value::as_str) {
        None | Some("user") => Ok(()),
        Some(o) => anyhow::bail!(
            "origin `{o}` is refused over the wire (only `user`); agents drive terminals via the approval-gated term_run tool"
        ),
    }
}
```

**Custom `"term.*"` dispatch & `try_stream` snapshot pattern** (`crates/vox-orchestrator-d/src/term_dispatch.rs` lines 134-156, 188-215, 223-248):
```rust
                term_method::OPEN => {
                    let spec = OpenSpec {
                        cols: dim(p, "cols", 80),
                        rows: dim(p, "rows", 24),
                        shell: p.get("shell").and_then(Value::as_str).map(String::from),
                    };
                    Ok(json!({"session_id": self.host.open(spec).await?}))
                }
                term_method::LIST => Ok(Value::Array(
                    self.host
                        .list()
                        .into_iter()
                        .map(|s| {
                            json!({
                                "session_id": s.id,
                                "blocks": s.blocks,
                                "security_level": s.security_level,
                                "control_mode": s.control_mode,
                            })
                        })
                        .collect(),
                )),
// ...
        let mut a = match self.host.attach(&session) {
            Ok(a) => a,
            Err(e) => return Some(out.send(&response_err(&req.id, e.to_string())).await),
        };
        let snapshot = json!({
            "type": "snapshot",
            "blocks": a.blocks,
            "replay_b64": B64.encode(&a.replay),
            "seq": a.seq,
        });
```

**Modification rules:**
- In `term_method::OPEN`: default `cols` to `120` and `rows` to `32` (`dim(p, "cols", 120)`, `dim(p, "rows", 32)`).
- In `term_method::LIST`: include `"alt_screen": s.alt_screen`, `"cwd": s.cwd`, `"git_branch": s.git_branch`.
- Add `"term.signal"` arm in `try_handle` (matching the string-literal style of `"term.context"`, `"term.security"`, `"term.control"` on lines 188-212): call `refuse_agent_origin(p)?;`, parse `"signal"` (`TermSignal`, defaulting to `TermSignal::Interrupt` when omitted or matching `"interrupt"`/`"terminate"`/`"kill"`), call `self.host.signal(sid, sig)?`, return `Ok(json!({}))`.
- In `session_event_json` (lines 113-124): serialize `SessionEvent::AltScreenChanged { active }` → `json!({"type": "alt_screen", "active": active})` and `SessionEvent::CwdChanged { cwd, git_branch }` → `json!({"type": "cwd_changed", "cwd": cwd, "git_branch": git_branch})`.
- In `try_stream` (`term.attach`, lines 231-245): read `let after_seq = req.params.get("after_seq").and_then(Value::as_u64);`, call `self.host.attach_since(&session, after_seq)`, and include `"resync_required": a.resync_required`, `"alt_screen": a.alt_screen`, `"cwd": a.cwd`, `"git_branch": a.git_branch` in `snapshot`.
- In `mod tests`: update `host_scenario` line 413 (`host.attach(&id, None).await.unwrap()`) and line 432 (`host.attach("nope", None).await.is_err()`), extend `agent_origin_is_refused_on_input_and_submit` to also assert `"term.signal"` refuses `origin: "agent"`, and add loopback TCP tests for `after_seq` delta snapshots + `term.signal` (`SIGINT`).

---

### `crates/vox-cli/src/lib.rs` (route, request-response)

**Analog:** `crates/vox-cli/src/lib.rs` (self — clap `Cli` variant extension)

**Neighboring `Cli` variant pattern** (`crates/vox-cli/src/lib.rs` lines 704-728):
```rust
        #[arg(long)]
        id: Option<String>,
    },
// ...
    /// Headless-capable ratatui terminal UI — block-model shell + AI agent strip.
    ///
    /// Runs under a real terminal, over SSH, or with TERM=dumb (plain-text output).
    /// Ctrl-C exits cleanly.
    Term,
}
```

**Modification rules:**
- Change `Term` from a unit variant to struct fields supporting `--attach [<SESSION_ID>]` (where bare `--attach` defaults to `"latest"` via `#[arg(long, value_name = "SESSION_ID", num_args = 0..=1, default_missing_value = "latest")]`) and `--local` (`#[arg(long)] local: bool`):
```rust
    Term {
        /// Attach to an existing live daemon terminal session (`<session_id>` or `latest`).
        #[arg(long, value_name = "SESSION_ID", num_args = 0..=1, default_missing_value = "latest")]
        attach: Option<String>,
        /// Force an in-process local PTY session instead of connecting to `vox-orchestrator-d`.
        #[arg(long)]
        local: bool,
    },
```

---

### `crates/vox-cli/src/cli_dispatch/mod.rs` (controller, request-response)

**Analog:** `crates/vox-cli/src/cli_dispatch/mod.rs` (self — async subcommand dispatch)

**Async dispatch pattern** (`crates/vox-cli/src/cli_dispatch/mod.rs` lines 616-628):
```rust
        Cli::Rollback { id } => {
            crate::commands::rollback::run(id).await?;
        }
        Cli::Workflow { cmd } => {
            crate::commands::workflow::run(cmd).await?;
        }
        Cli::Dispatch { cmd } => {
            crate::commands::dispatch::run(cmd).await?;
        }
        Cli::Term => {
            vox_term::app::run()?;
        }
```

**Modification rules:**
- Match `Cli::Term { attach, local }` and `.await` `vox_term::app::run_with_args(vox_term::app::TermCliArgs { attach, force_local: local }).await?;`.
- Add `"term"` arms to `universal_reward_command_path` (`Cli::Term { .. } => Some("term")`, around line 66) and `command_verb` (`Cli::Term { .. } => "term"`, around line 150) if telemetry verb parity is desired.

---

### `crates/vox-cli/src/command_catalog.rs` (service, transform)

**Analog:** `crates/vox-cli/src/command_catalog.rs` (self — clap reflection & catalog test verification)

**Clap argument introspection pattern** (`crates/vox-cli/src/command_catalog.rs` lines 416-445, 681-696):
```rust
        arguments: cmd
            .get_arguments()
            .map(|arg| {
                let action = arg.get_action();
                let value_kind = match action {
                    clap::ArgAction::SetTrue | clap::ArgAction::SetFalse => ArgValueKind::Flag,
                    clap::ArgAction::Count => ArgValueKind::Count,
                    _ => ArgValueKind::Value,
                };
                CommandCatalogArgument {
                    name: arg.get_id().to_string(),
                    short: arg.get_short(),
                    long: arg.get_long().map(|s| s.to_string()),
// ...
    #[test]
    fn build_catalog_has_expected_top_level_commands() {
        let catalog = run_on_big_stack(build_catalog);
```

**Modification rules:**
- `build_catalog()` automatically reflects `--attach` and `--local` from `VoxCliRoot::command()` without changing `path == ["term"]` (so `crates/vox-cli/tests/fixtures/command_catalog_paths_baseline.txt` stays untouched!).
- Add a unit test in `crates/vox-cli/src/command_catalog.rs` (`mod tests`, using `run_on_big_stack(build_catalog)`) asserting the `"term"` catalog entry exposes `--attach` (`takes_value: true`, `required: false`) and `--local` (`ArgValueKind::Flag`), and parses `vox term`, `vox term --attach`, `vox term --attach term-2`, and `vox term --local` cleanly via `VoxCliRoot::try_parse_from`.

---

### `crates/vox-gui/src/commands/pty.rs` (controller, pub-sub)

**Analog:** `crates/vox-gui/src/commands/pty.rs` (self — Tauri IPC `SessionHost` bridge)

**`term_session_attach` & DTO pattern** (`crates/vox-gui/src/commands/pty.rs` lines 29-62, 202-215):
```rust
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct TermAttachmentDto {
    pub session_id: String,
    pub blocks: Vec<Block>,
    pub replay_utf8: String,
    pub seq: u64,
}
// ...
#[tauri::command]
pub async fn term_session_attach(
    app: tauri::AppHandle,
    manager: tauri::State<'_, PtyManager>,
    daemon: tauri::State<'_, Arc<super::daemon::PersistentDaemon>>,
    session_id: String,
) -> Result<TermAttachmentDto, String> {
    let host = manager.host_for(daemon.inner()).await;
    let mut att = host.attach(&session_id).await.map_err(|e| e.to_string())?;
    let dto = term_attachment_dto_from_parts(&session_id, att.blocks, &att.replay, att.seq);
```

**Modification rules:**
- Update line 210 to pass `after_seq: Option<u64>` (either adding `#[serde(default)] after_seq: Option<u64>` parameter to `term_session_attach` or passing `None` until Phase 443 frontend migration, plus surfacing `#[serde(default)] pub resync_required: bool` and `#[serde(default)] pub alt_screen: bool` on `TermAttachmentDto` if helpful while keeping `term_attachment_dto_from_parts` unit test at line 385 compiling).
- Optionally replace `String::from_utf8_lossy(&c.bytes)` inside the forwarder loop (`line 227` and `line 151`) with `Utf8StreamDecoder` so live Tauri `vox://term-output` and `vox://pty-output` events also never corrupt split multi-byte UTF-8 chunks!

---

## Shared Patterns

### 1. Poison-Tolerant `std::sync::Mutex` Helper
**Source:** `crates/vox-terminal-core/src/host.rs` lines 287-289 (mirrored in `crates/vox-gui/src/commands/pty.rs` lines 81-83)
**Apply to:** `crates/vox-terminal-core/src/pty.rs`, `crates/vox-terminal-core/src/host.rs`
```rust
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}
```
Always maintain strict lock acquisition order `replay` → `session` inside `Entry::feed` and `LocalHost::attach_since` so no `OutputFrame` or `SessionEvent` can slip between the snapshot and the live `broadcast::Receiver` subscription.

### 2. Wire-Origin Fail-Closed Security Guard
**Source:** `crates/vox-orchestrator-d/src/term_dispatch.rs` lines 90-97, 157-166
**Apply to:** Every mutating `term.*` RPC in `crates/vox-orchestrator-d/src/term_dispatch.rs` (`term.input`, `term.submit`, and the new `term.signal`)
```rust
fn refuse_agent_origin(p: &Value) -> anyhow::Result<()> {
    match p.get("origin").and_then(Value::as_str) {
        None | Some("user") => Ok(()),
        Some(o) => anyhow::bail!(
            "origin `{o}` is refused over the wire (only `user`); agents drive terminals via the approval-gated term_run tool"
        ),
    }
}
```

### 3. Layer-Safe `term.*` Method String Constants (`L3` ↔ `L5` without touching `L0`)
**Source:** `crates/vox-terminal-core/src/daemon_host.rs` lines 17-27 & `crates/vox-orchestrator-d/src/term_dispatch.rs` lines 188-212
**Apply to:** `crates/vox-terminal-core/src/daemon_host.rs` (`pub const SIGNAL: &str = "term.signal";`) and `crates/vox-orchestrator-d/src/term_dispatch.rs` (`"term.signal" => { ... }`)
```rust
// In crates/vox-terminal-core/src/daemon_host.rs:
pub const SIGNAL: &str = "term.signal";

// In crates/vox-orchestrator-d/src/term_dispatch.rs:
"term.signal" => {
    refuse_agent_origin(p)?;
    let sid = str_param(p, "session_id")?;
    // ...
}
```
`vox-terminal-core` (`L3`) has no dependency edge to `vox-foundation` (`L0`), so keeping `"term.signal"` mirrored between `daemon_host.rs` and `term_dispatch.rs` avoids touching `vox-foundation`.

### 4. Tolerant Wire Deserialization (`#[serde(default)]` + Base64 Framing)
**Source:** `crates/vox-terminal-core/src/host.rs` lines 157-165 & `crates/vox-terminal-core/src/daemon_host.rs` lines 8, 136-142, 216-229
**Apply to:** `SessionInfo`, `Block`, `DaemonHost::list`, `DaemonHost::attach`, `TermDispatch::try_stream`
```rust
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
// Always annotate newly added wire struct fields with #[serde(default)] and read JSON Value
// fields via .get("...").and_then(...).unwrap_or_default() so rolling daemon restarts never fail.
```

### 5. Single-Threaded `#[tokio::test]` Safe OS-Thread Pumping
**Source:** `crates/vox-terminal-core/src/pty.rs` lines 378-391 & `crates/vox-terminal-core/src/host.rs` lines 487-501
**Apply to:** `crates/vox-terminal-core/src/pty.rs`, `crates/vox-terminal-core/src/host.rs`, and all `#[tokio::test]` integration tests
- Never block a synchronous `LocalHost::submit` call waiting on a `tokio::spawn` task: `#[tokio::test]` uses a single-threaded `current_thread` executor by default and will deadlock! Pump PTY reader chunks directly into `Entry::feed` on the `std::thread::spawn` OS reader thread and wake synchronous `Condvar` waiters directly from `Entry::feed`.

---

## No Analog Found

All 22 new and modified files map directly to git-tracked workspace analogs. For the two net-new protocol algorithms inside `crates/vox-terminal-core/src/shell_init.rs` (5-shell `TempDir` rcfile/ZDOTDIR/config.nu contents) and `crates/vox-terminal-core/src/utf8_stream.rs` (`std::str::from_utf8` 3-byte tail reassembler + `pending_cr` overwrite state machine), combine the structural analogs above with the verbatim reference implementations in `441-RESEARCH.md` (`## Architecture Patterns` Pattern 1 and `## Code Examples` §2).

## Metadata

**Analog search scope:** `crates/vox-terminal-core/`, `crates/vox-term/`, `crates/vox-orchestrator-d/`, `crates/vox-cli/`, `crates/vox-gui/`
**Files scanned:** 26 git-tracked files
**Pattern extraction date:** 2026-11-14
