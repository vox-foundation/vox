//! Closed-allowlist base64 non-echo `OSC 633` + `OSC 7` subshell hook injection (`TERM-WARP-04`).

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoxifyShellKind {
    Bash,
    Zsh,
    Fish,
    Pwsh,
    Nu,
}

impl VoxifyShellKind {
    /// Resolve `hint` against a strict closed shell allowlist, defaulting to `Bash`.
    /// Never interpolates caller-supplied text into hook payloads (`T-444-04`).
    pub fn from_hint(hint: Option<&str>) -> Self {
        let Some(raw) = hint else {
            return Self::Bash;
        };
        let trimmed = raw.trim();
        if trimmed.contains([';', '|', '&', '\n', '\r', '`', '$']) {
            return Self::Bash;
        }
        let first = trimmed.split_whitespace().next().unwrap_or("");
        let base = first
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(first)
            .to_ascii_lowercase();
        let name = base.strip_suffix(".exe").unwrap_or(&base);
        match name {
            "zsh" => Self::Zsh,
            "fish" => Self::Fish,
            "pwsh" | "powershell" => Self::Pwsh,
            "nu" | "nushell" => Self::Nu,
            _ => Self::Bash,
        }
    }
}

/// Detect whether `command` enters an interactive subshell (`bash`, `zsh`, `fish`,
/// `pwsh`, `nu`, `ssh user@host`, `docker exec -it ...`, `kubectl exec -it ...`, `sudo -i/-s`).
pub fn detect_subshell_command(command: &str) -> Option<VoxifyShellKind> {
    let trimmed = command.trim();
    let stripped = trimmed.strip_prefix('!').unwrap_or(trimmed).trim();
    if stripped.is_empty() {
        return None;
    }
    let tokens: Vec<&str> = stripped.split_whitespace().collect();
    detect_tokens(&tokens)
}

fn detect_tokens(tokens: &[&str]) -> Option<VoxifyShellKind> {
    let head = *tokens.first()?;
    let base = head
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(head)
        .to_ascii_lowercase();
    let cmd = base.strip_suffix(".exe").unwrap_or(&base);

    if cmd == "env" {
        let idx = tokens
            .iter()
            .skip(1)
            .position(|t| !t.starts_with('-') && !t.contains('='))?;
        return detect_tokens(&tokens[idx + 1..]);
    }

    if let Some(shell) = exact_shell_kind(cmd) {
        let args = &tokens[1..];
        let oneshot = args.iter().any(|a| {
            matches!(
                *a,
                "-c" | "/c" | "/C" | "-Command" | "-command" | "-File" | "-file" | "--commands"
            )
        });
        let has_script_arg = args.iter().any(|a| !a.starts_with('-'));
        return if oneshot || has_script_arg {
            None
        } else {
            Some(shell)
        };
    }

    match cmd {
        "sudo" => {
            let args = &tokens[1..];
            if args
                .iter()
                .any(|a| matches!(*a, "-i" | "--login" | "-s" | "--shell"))
                && !args.contains(&"-c")
            {
                return Some(VoxifyShellKind::Bash);
            }
            let pos: Vec<&str> =
                skip_flag_values(args, &["-u", "-g", "-C", "-D", "-R", "-T", "-p"]);
            if !pos.is_empty() {
                return detect_tokens(&pos);
            }
            None
        }
        "ssh" => {
            let args = &tokens[1..];
            if args
                .iter()
                .any(|a| matches!(*a, "-N" | "-G" | "-V" | "-O" | "-Q"))
            {
                return None;
            }
            let pos = skip_flag_values(
                args,
                &[
                    "-p", "-i", "-l", "-F", "-J", "-L", "-R", "-D", "-o", "-b", "-c", "-E", "-I",
                    "-m", "-S", "-W", "-w",
                ],
            );
            match pos.as_slice() {
                [_dest] => Some(VoxifyShellKind::Bash),
                [_dest, remote @ ..] => detect_tokens(remote),
                _ => None,
            }
        }
        "docker" | "podman" | "kubectl" if tokens.get(1) == Some(&"exec") => {
            let args = &tokens[2..];
            let has_it = args.iter().any(|a| matches!(*a, "-it" | "-ti"))
                || (args.iter().any(|a| matches!(*a, "-i" | "--stdin"))
                    && args.iter().any(|a| matches!(*a, "-t" | "--tty")));
            if !has_it {
                return None;
            }
            if let Some(dd) = args.iter().position(|a| *a == "--") {
                return detect_tokens(&args[dd + 1..]);
            }
            let pos = skip_flag_values(
                args,
                &[
                    "-u",
                    "--user",
                    "-w",
                    "--workdir",
                    "-e",
                    "--env",
                    "--env-file",
                    "-c",
                    "--container",
                    "-n",
                    "--namespace",
                ],
            );
            if pos.len() >= 2 {
                return detect_tokens(&pos[1..]);
            }
            None
        }
        _ => None,
    }
}

fn exact_shell_kind(name: &str) -> Option<VoxifyShellKind> {
    match name {
        "bash" | "sh" => Some(VoxifyShellKind::Bash),
        "zsh" => Some(VoxifyShellKind::Zsh),
        "fish" => Some(VoxifyShellKind::Fish),
        "pwsh" | "powershell" => Some(VoxifyShellKind::Pwsh),
        "nu" | "nushell" => Some(VoxifyShellKind::Nu),
        _ => None,
    }
}

fn skip_flag_values<'a>(args: &[&'a str], valued_flags: &[&str]) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut skip_next = false;
    for &arg in args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if valued_flags.contains(&arg) {
            skip_next = true;
            continue;
        }
        if arg.starts_with('-') {
            continue;
        }
        out.push(arg);
    }
    out
}

/// Build a non-echo, leading-space (`HISTCONTROL=ignorespace`) hook bootstrap line
/// from compile-time `OSC 633` + `OSC 7` templates (`T-444-04`).
pub fn build_voxify_payload(shell_hint: Option<&str>) -> String {
    match VoxifyShellKind::from_hint(shell_hint) {
        VoxifyShellKind::Bash => {
            let b64 = B64.encode(BASH_HOOK.as_bytes());
            format!(
                " stty -echo 2>/dev/null; eval \"$(printf '%s' '{b64}' | base64 -d 2>/dev/null || printf '%s' '{b64}' | base64 -D 2>/dev/null)\"; stty echo 2>/dev/null; clear\n"
            )
        }
        VoxifyShellKind::Zsh => {
            let b64 = B64.encode(ZSH_HOOK.as_bytes());
            format!(
                " stty -echo 2>/dev/null; eval \"$(printf '%s' '{b64}' | base64 -d 2>/dev/null || printf '%s' '{b64}' | base64 -D 2>/dev/null)\"; stty echo 2>/dev/null; clear\n"
            )
        }
        VoxifyShellKind::Fish => {
            let b64 = B64.encode(FISH_HOOK.as_bytes());
            format!(
                " stty -echo 2>/dev/null; begin; printf '%s' '{b64}' | base64 -d 2>/dev/null; or printf '%s' '{b64}' | base64 -D 2>/dev/null; end | source; stty echo 2>/dev/null; clear\n"
            )
        }
        VoxifyShellKind::Pwsh => {
            let b64 = B64.encode(PWSH_HOOK.as_bytes());
            format!(
                " [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('{b64}')) | Invoke-Expression; Clear-Host\n"
            )
        }
        VoxifyShellKind::Nu => {
            let b64 = B64.encode(NU_HOOK.as_bytes());
            let inline = NU_HOOK
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
                .join("; ");
            format!(" {inline}; clear # '{b64}'\n")
        }
    }
}

const BASH_HOOK: &str = r#"if [ -z "${__VOX_SUBSHELL_633:-}" ]; then
  __VOX_SUBSHELL_633=1
  __vox_sub_prompt() {
    local ec=$?
    printf '\e]633;D;%d\a\e]7;file://%s%s\a\e]633;P;Cwd=%s\a\e]633;A\a' "$ec" "${HOSTNAME:-localhost}" "$PWD" "$PWD"
    __VOX_SUB_AT_PROMPT=1
  }
  __vox_sub_preexec() {
    if [ -n "${__VOX_SUB_AT_PROMPT:-}" ]; then
      __VOX_SUB_AT_PROMPT=
      local c="${BASH_COMMAND//;/\\x3b}"
      printf '\e]633;E;%s\a\e]633;C\a' "$c"
    fi
  }
  trap '__vox_sub_preexec' DEBUG
  PROMPT_COMMAND="__vox_sub_prompt${PROMPT_COMMAND:+;$PROMPT_COMMAND}"
  PS1="${PS1}\[\e]633;B\a\]"
fi
"#;

const ZSH_HOOK: &str = r#"if [[ -z "${__VOX_SUBSHELL_633:-}" ]]; then
  __VOX_SUBSHELL_633=1
  __vox_sub_preexec() {
    local c="${1//;/\\x3b}"
    printf '\e]633;E;%s\a\e]633;C\a' "$c"
  }
  __vox_sub_precmd() {
    local ec=$?
    printf '\e]633;D;%d\a\e]7;file://%s%s\a\e]633;P;Cwd=%s\a\e]633;A\a' "$ec" "${HOST:-localhost}" "$PWD" "$PWD"
  }
  autoload -Uz add-zsh-hook 2>/dev/null || true
  add-zsh-hook preexec __vox_sub_preexec
  add-zsh-hook precmd __vox_sub_precmd
  PROMPT="${PROMPT}%{"$'\e]633;B\a'"%}"
fi
"#;

const FISH_HOOK: &str = r#"if not set -q __vox_subshell_633
  set -g __vox_subshell_633 1
  function __vox_sub_preexec --on-event fish_preexec
    set -l cmd (string split \n -- "$argv[1]" | string replace -a '\\' '\\x5c' | string replace -a ';' '\\x3b' | string join '\\x0a')
    printf '\e]633;E;%s\a\e]633;C\a' "$cmd"
  end
  function __vox_sub_postexec --on-event fish_postexec
    printf '\e]633;D;%s\a\e]7;file://%s%s\a\e]633;P;Cwd=%s\a\e]633;A\a' $status (hostname 2>/dev/null) "$PWD" "$PWD"
  end
  functions --query fish_prompt; and functions --copy fish_prompt __vox_sub_orig_prompt
  function fish_prompt
    __vox_sub_orig_prompt 2>/dev/null
    printf '\e]633;B\a'
  end
end
"#;

const PWSH_HOOK: &str = r#"if (-not $global:__VoxSubshell633) {
  $global:__VoxSubshell633 = $true
  $global:__VoxSubOrigPrompt = $function:prompt
  function global:prompt {
    $origQ = $?
    $ec = if (-not $origQ -and -not $LASTEXITCODE) { 1 } elseif ($LASTEXITCODE) { $LASTEXITCODE } else { 0 }
    $esc = [char]27; $bel = [char]7; $cwd = (Get-Location).Path
    $uriPath = if ($cwd.StartsWith('/')) { $cwd } else { "/$($cwd -replace '\\','/')" }
    $out = "$esc]633;D;$ec$bel$esc]7;file://localhost$uriPath$bel$esc]633;P;Cwd=$cwd$bel$esc]633;A$bel"
    $out += (& $global:__VoxSubOrigPrompt)
    $out += "$esc]633;B$bel"
    $global:LASTEXITCODE = 0
    return $out
  }
  if (Get-Module -Name PSReadLine) {
    Set-PSReadLineKeyHandler -Key Enter -ScriptBlock {
      $line = $null; $cur = $null
      [Microsoft.PowerShell.PSConsoleReadLine]::GetBufferState([ref]$line, [ref]$cur)
      $safe = ($line -replace '\\','\x5c' -replace ';','\x3b' -replace "`r`n","`n" -replace "`n",'\x0a')
      [Console]::Write("$([char]27)]633;E;$safe$([char]7)$([char]27)]633;C$([char]7)")
      [Microsoft.PowerShell.PSConsoleReadLine]::AcceptLine()
    }
  }
}
"#;

const NU_HOOK: &str = r#"if (($env.__VOX_SUBSHELL_HOOKED? | default false) == false) { $env.__VOX_SUBSHELL_HOOKED = true; $env.config = ($env.config | upsert hooks.pre_prompt (($env.config.hooks.pre_prompt? | default []) | append {|| let ec = ($env.LAST_EXIT_CODE? | default 0); print -n ([(char -u 1b) "]633;D;" $ec (char bel) (char -u 1b) "]7;file://localhost" $env.PWD (char bel) (char -u 1b) "]633;P;Cwd=" $env.PWD (char bel) (char -u 1b) "]633;A" (char bel)] | str join) }) | upsert hooks.pre_execution (($env.config.hooks.pre_execution? | default []) | append {|| let cmd = (commandline | str replace -a '\' '\x5c' | str replace -a ';' '\x3b' | str replace -a (char nl) '\x0a'); print -n ([(char -u 1b) "]633;E;" $cmd (char bel) (char -u 1b) "]633;C" (char bel)] | str join) })); $env.PROMPT_INDICATOR = {|| (["> " (char -u 1b) "]633;B" (char bel)] | str join) } }
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn extract_single_quoted_b64(payload: &str) -> String {
        payload
            .split('\'')
            .skip(1)
            .step_by(2)
            .max_by_key(|s| s.len())
            .unwrap_or("")
            .to_string()
    }

    #[test]
    fn detects_interactive_subshells_and_ignores_oneshot_commands() {
        assert_eq!(detect_subshell_command("bash"), Some(VoxifyShellKind::Bash));
        assert_eq!(
            detect_subshell_command("/bin/zsh -l"),
            Some(VoxifyShellKind::Zsh)
        );
        assert_eq!(detect_subshell_command("fish"), Some(VoxifyShellKind::Fish));
        assert_eq!(
            detect_subshell_command("pwsh -NoLogo"),
            Some(VoxifyShellKind::Pwsh)
        );
        assert_eq!(detect_subshell_command("nu"), Some(VoxifyShellKind::Nu));
        assert_eq!(
            detect_subshell_command("ssh -p 2222 deploy@prod.example.com"),
            Some(VoxifyShellKind::Bash)
        );
        assert_eq!(
            detect_subshell_command("docker exec -it c1 zsh"),
            Some(VoxifyShellKind::Zsh)
        );
        assert_eq!(
            detect_subshell_command("sudo -i"),
            Some(VoxifyShellKind::Bash)
        );

        assert_eq!(detect_subshell_command("bash -c \"echo hi\""), None);
        assert_eq!(detect_subshell_command("ssh prod.example.com ls -la"), None);
        assert_eq!(detect_subshell_command("docker exec c1 ls"), None);
    }

    #[test]
    fn builds_closed_allowlist_base64_osc633_and_osc7_payloads_for_all_shells() {
        for (hint, is_posix) in [
            (None, true),
            (Some("bash"), true),
            (Some("zsh"), true),
            (Some("fish"), true),
            (Some("pwsh"), false),
            (Some("nu"), false),
            (Some("zsh; rm -rf /"), true),
        ] {
            let payload = build_voxify_payload(hint);
            assert!(
                payload.starts_with(' '),
                "missing ignorespace leading space for {hint:?}"
            );
            assert!(
                payload.ends_with('\n'),
                "missing trailing newline for {hint:?}"
            );
            assert!(
                !payload.contains("rm -rf"),
                "untrusted hint leaked into payload"
            );

            if is_posix {
                assert!(payload.contains("stty -echo"));
                assert!(payload.contains("stty echo"));
            }

            let b64 = extract_single_quoted_b64(&payload);
            let decoded_bytes = B64.decode(b64.as_bytes()).expect("valid base64 segment");
            let decoded = String::from_utf8(decoded_bytes).expect("utf-8 hook script");
            assert!(
                decoded.contains("633;A"),
                "missing 633;A in {hint:?}: {decoded}"
            );
            assert!(
                decoded.contains("633;B"),
                "missing 633;B in {hint:?}: {decoded}"
            );
            assert!(
                decoded.contains("633;D"),
                "missing 633;D in {hint:?}: {decoded}"
            );
            assert!(
                decoded.contains("]7;file://"),
                "missing OSC 7 in {hint:?}: {decoded}"
            );
            assert!(
                decoded.contains("633;C") && decoded.contains("633;E;"),
                "missing preexec 633;E/633;C in {hint:?}: {decoded}"
            );
        }
        assert!(NU_HOOK.contains("__VOX_SUBSHELL_HOOKED"));
        assert!(FISH_HOOK.contains("\\x3b"));
    }
}
