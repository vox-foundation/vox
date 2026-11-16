//! PTY host — de-Tauri'd from `vox-gui/src/commands/pty.rs`.
//!
//! Pure functions (`default_shell`, `shell_integration_snippet`) and the
//! `ShellBackend` trait are the SSOT. The GUI's Tauri commands now call
//! `PtyHost` and forward the byte stream via their event emitter.
//!
//! **Forward-compat seam for Track 6 (Nushell):** `ShellBackend` is defined
//! here so `Session` depends on `dyn ShellBackend`, not on `PtyShell`
//! concretely. Track 6 adds `NuShell` behind the `nushell` cargo feature.

use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use anyhow::Result;
use portable_pty::{Child, MasterPty, PtySize, native_pty_system};
use tempfile::TempDir;
use tokio::sync::mpsc;

// ── Pure platform helpers (moved verbatim from vox-gui/src/commands/pty.rs) ──

/// The default shell command for the current platform.
pub fn default_shell() -> String {
    if cfg!(windows) {
        "pwsh".to_string()
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string())
    }
}

/// OSC 633 shell-integration init snippet for a given shell.
/// Returns `None` for shells we don't yet integrate (Track 6 adds zsh/fish/nu).
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

/// Prefer `nu` (Nushell) when it is on the PATH; fall back to the default
/// platform shell. Use this instead of `default_shell()` when structured
/// output (block model) is desired.
pub fn detect_ai_shell() -> String {
    if std::process::Command::new("nu")
        .arg("--version")
        .output()
        .is_ok()
    {
        return "nu".to_string();
    }
    default_shell()
}

/// Zsh OSC 633 integration (preexec/precmd hooks).
const ZSH_OSC633: &str = r#"
if [[ -z "${__VOX_OSC633:-}" ]]; then
  __VOX_OSC633=1
  __vox_preexec() {
    local cmd="${1//\\/\\x5c}"; cmd="${cmd//;/\\x3b}"; cmd="${cmd//$'\n'/\\x0a}"
    printf '\e]633;E;%s\a\e]633;C\a' "$cmd"
  }
  __vox_precmd() {
    printf '\e]633;D;%s\a\e]633;A\a' "${__vox_ec:-0}"
    __vox_ec=0
  }
  __vox_trappost() { __vox_ec=$? }
  autoload -Uz add-zsh-hook 2>/dev/null || true
  add-zsh-hook preexec __vox_preexec
  add-zsh-hook precmd __vox_precmd
  PROMPT="${PROMPT}"$'\e]633;B\a'
fi
"#;

/// Fish OSC 633 integration (event handler functions).
const FISH_OSC633: &str = r#"
if not set -q __vox_osc633
  set -g __vox_osc633 1
  function __vox_fish_preexec --on-event fish_preexec
    set -l cmd (string split \n -- "$argv[1]" | string replace -a '\\' '\\x5c' | string replace -a ';' '\\x3b' | string join '\\x0a')
    printf '\e]633;E;%s\a\e]633;C\a' "$cmd"
  end
  function __vox_fish_postexec --on-event fish_postexec
    printf '\e]633;D;%s\a\e]633;A\a' $status
  end
  functions --query fish_prompt && functions --copy fish_prompt __vox_orig_fish_prompt
  function fish_prompt
    __vox_orig_fish_prompt 2>/dev/null
    printf '\e]633;B\a'
  end
end
"#;

/// Nushell OSC 633 integration (config hook upsert).
const NU_OSC633: &str = r#"
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
            print -n ([(char -u 1b) "]633;D;" $ec (char bel) (char -u 1b) "]633;A" (char bel)] | str join)
          }
        )
      | upsert pre_execution (
          ($h.pre_execution? | default []) | append {||
            let raw_cmd = (try { commandline } catch { "" })
            let enc = ($raw_cmd | default "" | str replace -a '\' '\x5c' | str replace -a ';' '\x3b' | str replace -a "\r" '\x0d' | str replace -a "\n" '\x0a')
            print -n ([(char -u 1b) "]633;E;" $enc (char bel) (char -u 1b) "]633;C" (char bel)] | str join)
          }
        )
    }
)
$env.PROMPT_INDICATOR = {|| (["> " (char -u 1b) "]633;B" (char bel)] | str join) }
"#;

/// PowerShell OSC 633 integration (verbatim from vox-gui).
const PWSH_OSC633: &str = r#"
if (-not $global:__VoxOsc633) {
  $global:__VoxOsc633 = $true
  $global:__VoxOscEsc = [char]27
  $global:__VoxOscBel = [char]7
  $global:__VoxOrigPrompt = $function:prompt
  function global:prompt {
    $origOk = $?
    $origLast = $LASTEXITCODE
    $code = if ($null -ne $origLast -and $origLast -ne 0) { $origLast } elseif (-not $origOk) { 1 } else { 0 }
    $out = "$($global:__VoxOscEsc)]633;D;$code$($global:__VoxOscBel)"
    $out += "$($global:__VoxOscEsc)]633;A$($global:__VoxOscBel)"
    $out += (& $global:__VoxOrigPrompt)
    $out += "$($global:__VoxOscEsc)]633;B$($global:__VoxOscBel)"
    $global:LASTEXITCODE = 0
    return $out
  }
  Import-Module PSReadLine -ErrorAction SilentlyContinue
  if (Get-Module -Name PSReadLine) {
    Set-PSReadLineKeyHandler -Chord @('Enter', 'Ctrl+j') -ScriptBlock {
      $line = $null; $cursor = $null
      [Microsoft.PowerShell.PSConsoleReadLine]::GetBufferState([ref]$line, [ref]$cursor)
      $enc = $line -replace '\\','\x5c' -replace ';','\x3b' -replace "`r",'\x0d' -replace "`n",'\x0a'
      [Console]::Write("$($global:__VoxOscEsc)]633;E;$enc$($global:__VoxOscBel)")
      [Console]::Write("$($global:__VoxOscEsc)]633;C$($global:__VoxOscBel)")
      [Microsoft.PowerShell.PSConsoleReadLine]::AcceptLine()
    }
  }
}
"#;

/// Bash OSC 633 integration (verbatim from vox-gui).
const BASH_OSC633: &str = r#"
if [ -z "${__VOX_OSC633:-}" ]; then
  __VOX_OSC633=1
  __VOX_PREV_PROMPT_COMMAND="${PROMPT_COMMAND:-}"
  __VOX_PREV_DEBUG_TRAP="$(trap -p DEBUG | sed -e "s/^trap -- '//" -e "s/' DEBUG\$//")"
  __vox_preexec() {
    if [ -n "${__VOX_AT_PROMPT:-}" ]; then
      __VOX_AT_PROMPT=
      local cmd="${BASH_COMMAND//\\/\\x5c}"; cmd="${cmd//;/\\x3b}"; cmd="${cmd//$'\n'/\\x0a}"
      printf '\e]633;E;%s\a\e]633;C\a' "$cmd"
    fi
    [ -n "${__VOX_PREV_DEBUG_TRAP:-}" ] && eval "${__VOX_PREV_DEBUG_TRAP}"
  }
  __vox_prompt() {
    local ec=$?
    printf '\e]633;D;%s\a\e]633;A\a' "$ec"
    __VOX_AT_PROMPT=1
    [ -n "${__VOX_PREV_PROMPT_COMMAND:-}" ] && eval "${__VOX_PREV_PROMPT_COMMAND}"
  }
  trap '__vox_preexec' DEBUG
  PROMPT_COMMAND='__vox_prompt'
  PS1="${PS1}\[\e]633;B\a\]"
fi
"#;

/// Run `cmd` as a single line via the platform's default shell, capturing
/// stdout/stderr and exit code. Blocking — callers on a UI event loop should
/// expect this to stall redraws for the command's duration.
///
/// ponytail: raw shell pass-through, no exec-policy AST enforcement. This
/// mirrors `vox shell repl`'s documented non-goal ("no ... policy-enforced
/// passthrough of arbitrary shell lines", `vox-cli/src/commands/runtime/shell/mod.rs`).
/// Upgrade path: route through `vox shell check` (`contracts/terminal/exec-policy.v1.yaml`)
/// before exec once vox-term needs to gate agent-issued or untrusted commands.
pub fn run_shell_capture(cmd: &str) -> (i32, String, String) {
    let shell = default_shell();
    let base = shell
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(&shell)
        .to_ascii_lowercase();
    let flag = if base.starts_with("pwsh") || base.starts_with("powershell") {
        "-Command"
    } else {
        "-c"
    };
    match std::process::Command::new(&shell)
        .arg(flag)
        .arg(cmd)
        .output()
    {
        Ok(out) => (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        ),
        Err(e) => (-1, String::new(), e.to_string()),
    }
}

/// Diagnose POSIX-isms in Nushell input and provide actionable Nushell syntax hints.
pub fn diagnose_nu_posix_isms(script: &str) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if script.contains("&&") {
        diagnostics
            .push("POSIX '&&' is not supported in Nushell; use ';' or 'and' instead".to_string());
    }
    if script.contains("||") {
        diagnostics.push("POSIX '||' is not supported in Nushell; use 'or' instead".to_string());
    }
    if script.contains("2>&1") {
        diagnostics.push(
            "POSIX '2>&1' is not supported in Nushell; use 'out+err>' or 'e>|' instead".to_string(),
        );
    }
    for var in ["$HOME", "$PATH", "$USER"] {
        if script.contains(var) {
            diagnostics.push(format!(
                "POSIX '{var}' should be referenced as '$env.{}' in Nushell",
                &var[1..]
            ));
        }
    }
    diagnostics
}

/// Run pipeline via `nu` with structured JSON capture when available,
/// falling back gracefully with diagnostics.
pub fn run_nu_structured_capture(pipeline: &str) -> (i32, String, String) {
    let warnings = diagnose_nu_posix_isms(pipeline);
    let mut warning_prefix = String::new();
    if !warnings.is_empty() {
        warning_prefix = format!("[nu hint: {}]\n", warnings.join("; "));
    }

    if std::process::Command::new("nu")
        .arg("--version")
        .output()
        .is_ok()
    {
        // Try structured capture via `to json --raw`
        let structured_cmd = format!("do {{ {pipeline} }} | to json --raw");
        if let Ok(out) = std::process::Command::new("nu")
            .arg("--no-config-file")
            .arg("-c")
            .arg(&structured_cmd)
            .output()
            && out.status.success()
        {
            return (
                0,
                format!("{warning_prefix}{}", String::from_utf8_lossy(&out.stdout)),
                String::from_utf8_lossy(&out.stderr).into_owned(),
            );
        }

        // Fall back to direct execution
        match std::process::Command::new("nu")
            .arg("--no-config-file")
            .arg("-c")
            .arg(pipeline)
            .output()
        {
            Ok(out) => (
                out.status.code().unwrap_or(-1),
                format!("{warning_prefix}{}", String::from_utf8_lossy(&out.stdout)),
                String::from_utf8_lossy(&out.stderr).into_owned(),
            ),
            Err(e) => (-1, String::new(), format!("{warning_prefix}{e}")),
        }
    } else {
        let (code, stdout, stderr) = run_shell_capture(pipeline);
        let notice = "[nu not found on PATH; executed via platform shell fallback]\n";
        (code, format!("{notice}{warning_prefix}{stdout}"), stderr)
    }
}

// ── ShellBackend trait (forward-compat seam for Track 6 Nushell) ─────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKind {
    /// A system shell running over a PTY (bash, pwsh, etc.)
    Pty,
    /// Nushell in-process (behind the `nushell` cargo feature, Track 6)
    Nushell,
}

/// Uniform interface over shell backends. `Session` depends on this trait,
/// not on any concrete type, so the default backend can be changed (e.g., to
/// Nushell) without touching session logic.
pub trait ShellBackend: Send + 'static {
    fn kind(&self) -> ShellKind;
    fn write(&mut self, data: &[u8]) -> Result<()>;
    fn kill(&mut self);
}

// ── PtyHost / PtyHandle ───────────────────────────────────────────────────────

/// Structured terminal signal delivered to a live PTY session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TermSignal {
    Interrupt,
    Terminate,
    Kill,
}

fn lock_master(m: &Mutex<Box<dyn MasterPty + Send>>) -> MutexGuard<'_, Box<dyn MasterPty + Send>> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

/// A live PTY child process with an input handle, retained `MasterPty`, and RAII `TempDir`.
pub struct PtyHandle {
    writer: Option<Arc<Mutex<Box<dyn std::io::Write + Send>>>>,
    master: Option<Arc<Mutex<Box<dyn MasterPty + Send>>>>,
    child: Box<dyn Child + Send + Sync>,
    _shim_dir: Option<TempDir>,
    osc633_capable: bool,
    shell_name: String,
}

impl PtyHandle {
    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        use std::io::Write;
        let writer = self
            .writer
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("PTY writer is closed"))?;
        let mut guard = writer.lock().unwrap_or_else(|p| p.into_inner());
        guard.write_all(data)?;
        guard.flush()?;
        Ok(())
    }

    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        let master = self
            .master
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("PTY master is closed"))?;
        lock_master(master).resize(PtySize {
            rows: rows.max(1),
            cols: cols.max(1),
            pixel_width: 0,
            pixel_height: 0,
        })?;
        Ok(())
    }

    pub fn get_size(&self) -> Result<PtySize> {
        let master = self
            .master
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("PTY master is closed"))?;
        lock_master(master).get_size()
    }

    pub fn signal(&mut self, sig: TermSignal) -> Result<()> {
        match sig {
            TermSignal::Interrupt => self.write(b"\x03"),
            TermSignal::Terminate | TermSignal::Kill => {
                drop(self.writer.take());
                drop(self.master.take());
                self.child.kill()?;
                self.reap_bounded();
                Ok(())
            }
        }
    }

    pub fn try_wait_exit_code(&mut self) -> Option<i32> {
        match self.child.try_wait() {
            Ok(Some(status)) => Some(status.exit_code() as i32),
            _ => None,
        }
    }

    pub fn supports_osc633(&self) -> bool {
        self.osc633_capable
    }

    pub fn shell_name(&self) -> &str {
        &self.shell_name
    }

    pub fn shim_path(&self) -> Option<&Path> {
        self._shim_dir.as_ref().map(|d| d.path())
    }

    fn reap_bounded(&mut self) {
        for _ in 0..25 {
            match self.child.try_wait() {
                Ok(Some(_)) | Err(_) => return,
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(2)),
            }
        }
    }

    pub fn kill(&mut self) {
        drop(self.writer.take());
        drop(self.master.take());
        let _ = self.child.kill();
        self.reap_bounded();
    }
}

impl Drop for PtyHandle {
    fn drop(&mut self) {
        self.kill();
    }
}

impl ShellBackend for PtyHandle {
    fn kind(&self) -> ShellKind {
        ShellKind::Pty
    }
    fn write(&mut self, data: &[u8]) -> Result<()> {
        self.write(data)
    }
    fn kill(&mut self) {
        self.kill()
    }
}

fn apply_pty_history_env(cmd: &mut portable_pty::CommandBuilder) {
    cmd.env("HISTCONTROL", "ignorespace");
    cmd.env("HIST_IGNORE_SPACE", "1");
}

/// Spawn a PTY shell using non-echo `TempDir` launch shims and invoke `on_chunk` / `on_eof`
/// directly on a dedicated OS reader thread (`"vox-pty-reader"`).
pub fn spawn_pty_with_callback<F, E>(
    shell: &str,
    cols: u16,
    rows: u16,
    cwd: Option<&Path>,
    mut on_chunk: F,
    on_eof: E,
) -> Result<PtyHandle>
where
    F: FnMut(&[u8]) + Send + 'static,
    E: FnOnce() + Send + 'static,
{
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows: rows.max(1),
        cols: cols.max(1),
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let mut shim = crate::shell_init::prepare_shell_command(shell, cwd)?;
    apply_pty_history_env(&mut shim.cmd);
    let child = pair.slave.spawn_command(shim.cmd)?;
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader()?;
    let writer = Arc::new(Mutex::new(pair.master.take_writer()?));
    let writer_for_reader = Arc::downgrade(&writer);
    let master = Arc::new(Mutex::new(pair.master));

    std::thread::Builder::new()
        .name("vox-pty-reader".into())
        .spawn(move || {
            use std::io::Write;
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let chunk = &buf[..n];
                        if chunk.windows(4).any(|w| w == b"\x1b[6n")
                            && let Some(w) = writer_for_reader.upgrade()
                        {
                            let mut guard = w.lock().unwrap_or_else(|p| p.into_inner());
                            let _ = guard.write_all(b"\x1b[1;1R");
                            let _ = guard.flush();
                        }
                        on_chunk(chunk);
                    }
                }
            }
            on_eof();
        })?;

    Ok(PtyHandle {
        writer: Some(writer),
        master: Some(master),
        child,
        _shim_dir: shim.shim_dir,
        osc633_capable: shim.osc633_capable,
        shell_name: crate::shell_init::normalize_shell_name(shell),
    })
}

/// Spawn a PTY shell and return a handle + a receiver for raw output bytes.
///
/// A blocking OS thread reads the PTY and pushes chunks into `tx`. The caller
/// (GUI Tauri command, TUI render loop, or Session) owns the receiver.
pub fn spawn_pty(
    shell: &str,
    cols: u16,
    rows: u16,
) -> Result<(PtyHandle, mpsc::Receiver<Vec<u8>>)> {
    let (tx, rx) = mpsc::channel::<Vec<u8>>(256);
    let handle = spawn_pty_with_callback(
        shell,
        cols,
        rows,
        None,
        move |chunk| {
            let _ = tx.blocking_send(chunk.to_vec());
        },
        || {},
    )?;
    Ok((handle, rx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_shell_is_nonempty() {
        assert!(!default_shell().is_empty());
    }

    #[test]
    fn pwsh_and_bash_snippets_are_some() {
        assert!(shell_integration_snippet("pwsh").is_some());
        assert!(shell_integration_snippet("bash").is_some());
    }

    #[test]
    fn zsh_snippet_is_some() {
        assert!(shell_integration_snippet("zsh").is_some());
    }

    #[test]
    fn fish_snippet_is_some() {
        assert!(shell_integration_snippet("fish").is_some());
    }

    #[test]
    fn nu_snippet_is_some() {
        assert!(shell_integration_snippet("nu").is_some());
    }

    #[test]
    fn detect_ai_shell_is_nonempty() {
        // nu may or may not be installed; either way we get a nonempty string.
        assert!(!detect_ai_shell().is_empty());
    }

    #[test]
    fn run_shell_capture_returns_stdout_and_zero_exit() {
        let (code, stdout, _stderr) = run_shell_capture("echo hello-vox-term");
        assert_eq!(code, 0);
        assert!(
            stdout.contains("hello-vox-term"),
            "unexpected stdout: {stdout:?}"
        );
    }

    #[test]
    fn run_shell_capture_reports_nonzero_exit() {
        let (code, _stdout, _stderr) = run_shell_capture("exit 3");
        assert_eq!(code, 3);
    }

    #[test]
    fn nu_snippet_emits_osc633_markers() {
        let s = shell_integration_snippet("nu").unwrap();
        assert!(s.contains("]633;A"));
        assert!(s.contains("]633;B"));
        assert!(s.contains("]633;C"));
        assert!(s.contains("]633;D;"));
        assert!(s.contains("]633;E;"));
    }

    #[test]
    fn diagnose_nu_posix_isms_catches_common_mistakes() {
        let diag = diagnose_nu_posix_isms("echo $HOME && ls 2>&1 || true");
        assert!(diag.iter().any(|d| d.contains("&&")));
        assert!(diag.iter().any(|d| d.contains("||")));
        assert!(diag.iter().any(|d| d.contains("2>&1")));
        assert!(diag.iter().any(|d| d.contains("$env.HOME")));
    }

    #[test]
    fn run_nu_structured_capture_executes_or_falls_back_cleanly() {
        let (code, stdout, _stderr) = run_nu_structured_capture("echo hello-nu-structured");
        assert_eq!(code, 0);
        assert!(stdout.contains("hello-nu-structured"));
    }

    #[test]
    fn spawned_pty_sets_histcontrol_ignorespace() {
        let mut cmd = portable_pty::CommandBuilder::new("sh");
        apply_pty_history_env(&mut cmd);
        assert_eq!(
            cmd.get_env("HISTCONTROL").and_then(|v| v.to_str()),
            Some("ignorespace")
        );
        assert_eq!(
            cmd.get_env("HIST_IGNORE_SPACE").and_then(|v| v.to_str()),
            Some("1")
        );
    }
}
