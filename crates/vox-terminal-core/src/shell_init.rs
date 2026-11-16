//! Non-echo 5-shell `tempfile::TempDir` launch shims (`zsh`, `bash`, `fish`, `pwsh`, `nu`).
//!
//! Instead of writing OSC 633 hook functions to PTY stdin while kernel `ECHO` is active
//! (which dumps hook source lines into initial replay and pollutes shell history), this
//! module creates an owner-only (`0700` on Unix) temporary launch directory and wires
//! shell-specific startup arguments/env vars so user dotfiles load first and Vox `OSC 633`
//! + `OSC 7` hooks load second.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use portable_pty::CommandBuilder;
use tempfile::TempDir;

/// Prepared shell launch command and optional RAII temporary shim directory.
pub struct ShellLaunchShim {
    pub cmd: CommandBuilder,
    pub shim_dir: Option<TempDir>,
    pub osc633_capable: bool,
}

/// Optional explicit user home / Nushell config directory overrides for deterministic testing.
#[derive(Debug, Clone, Default)]
pub struct ShellInitDirs<'a> {
    pub home_dir: Option<&'a Path>,
    pub nu_config_dir: Option<&'a Path>,
}

/// Normalize a shell path or binary name (`"bin/pwsh.exe"` -> `"pwsh"`).
pub fn normalize_shell_name(shell: &str) -> String {
    let base = shell
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(shell)
        .to_ascii_lowercase();
    base.strip_suffix(".exe").unwrap_or(&base).to_string()
}

/// Escape single quotes for Fish single-quoted strings (`'` -> `\'` with `\` -> `\\`).
pub fn escape_fish_single_quoted(raw: &str) -> String {
    raw.replace('\\', "\\\\").replace('\'', "\\'")
}

/// Format a parse-time static `source` line for Nushell `config.nu`.
pub fn format_nu_source_line(path: &Path) -> String {
    let raw = path.to_string_lossy();
    if raw.contains('\'') {
        let escaped = raw.replace('\\', "\\\\").replace('"', "\\\"");
        format!("source \"{escaped}\"")
    } else {
        format!("source '{raw}'")
    }
}

/// Resolve default user home directory from explicit override or environment.
fn resolve_home_dir(override_home: Option<&Path>) -> Option<PathBuf> {
    if let Some(h) = override_home {
        return Some(h.to_path_buf());
    }
    if let Some(h) = std::env::var_os("HOME")
        && !h.is_empty()
    {
        return Some(PathBuf::from(h));
    }
    #[cfg(windows)]
    if let Some(up) = std::env::var_os("USERPROFILE")
        && !up.is_empty()
    {
        return Some(PathBuf::from(up));
    }
    None
}

/// Resolve default Nushell config directory on the host when not explicitly overridden.
pub fn default_nu_config_dir(home: Option<&Path>) -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME")
        && !xdg.is_empty()
    {
        return Some(PathBuf::from(xdg).join("nushell"));
    }
    #[cfg(target_os = "macos")]
    if let Some(h) = home {
        let app_support = h.join("Library/Application Support/nushell");
        if app_support.exists() {
            return Some(app_support);
        }
    }
    #[cfg(windows)]
    if let Some(appdata) = std::env::var_os("APPDATA")
        && !appdata.is_empty()
    {
        return Some(PathBuf::from(appdata).join("nushell"));
    }
    home.map(|h| h.join(".config/nushell"))
}

fn create_secure_shim_dir() -> Result<TempDir> {
    let dir = tempfile::Builder::new()
        .prefix("vox-shell-")
        .tempdir()
        .context("creating temporary shell launch shim directory")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700))
            .context("setting 0700 permissions on shell launch shim directory")?;
    }
    Ok(dir)
}

/// Prepare a non-echo `CommandBuilder` and RAII `TempDir` launch shim for `shell`.
pub fn prepare_shell_command(shell: &str, cwd: Option<&Path>) -> Result<ShellLaunchShim> {
    prepare_shell_command_with_dirs(shell, cwd, ShellInitDirs::default())
}

/// Testable inner builder accepting explicit home/config directory overrides.
pub fn prepare_shell_command_with_dirs(
    shell: &str,
    cwd: Option<&Path>,
    dirs: ShellInitDirs<'_>,
) -> Result<ShellLaunchShim> {
    let mut cmd = CommandBuilder::new(shell);
    if let Some(dir) = cwd {
        cmd.cwd(dir);
    }
    if let Some(home) = dirs.home_dir {
        cmd.env("HOME", home);
    }

    let name = normalize_shell_name(shell);
    match name.as_str() {
        "zsh" => build_zsh_shim(&mut cmd, dirs).map(|shim_dir| ShellLaunchShim {
            cmd,
            shim_dir: Some(shim_dir),
            osc633_capable: true,
        }),
        "bash" => build_bash_shim(&mut cmd).map(|shim_dir| ShellLaunchShim {
            cmd,
            shim_dir: Some(shim_dir),
            osc633_capable: true,
        }),
        "fish" => build_fish_shim(&mut cmd).map(|shim_dir| ShellLaunchShim {
            cmd,
            shim_dir: Some(shim_dir),
            osc633_capable: true,
        }),
        "pwsh" | "powershell" => build_pwsh_shim(&mut cmd).map(|shim_dir| ShellLaunchShim {
            cmd,
            shim_dir: Some(shim_dir),
            osc633_capable: true,
        }),
        "nu" => build_nu_shim(&mut cmd, dirs).map(|shim_dir| ShellLaunchShim {
            cmd,
            shim_dir: Some(shim_dir),
            osc633_capable: true,
        }),
        _ => Ok(ShellLaunchShim {
            cmd,
            shim_dir: None,
            osc633_capable: false,
        }),
    }
}

fn build_zsh_shim(cmd: &mut CommandBuilder, _dirs: ShellInitDirs<'_>) -> Result<TempDir> {
    let shim_dir = create_secure_shim_dir()?;

    if let Some(orig) = cmd.get_env("ZDOTDIR").map(|s| s.to_os_string())
        && !orig.is_empty()
    {
        cmd.env("VOX_ORIG_ZDOTDIR", orig);
        cmd.env("VOX_HAD_ZDOTDIR", "1");
    } else {
        cmd.env_remove("VOX_ORIG_ZDOTDIR");
        cmd.env("VOX_HAD_ZDOTDIR", "0");
    }
    cmd.env("ZDOTDIR", shim_dir.path());
    cmd.arg("-i");

    fs::write(shim_dir.path().join(".zshenv"), ZSH_SHIM_ZSHENV)
        .context("writing zsh .zshenv shim")?;
    fs::write(shim_dir.path().join(".zprofile"), ZSH_SHIM_ZPROFILE)
        .context("writing zsh .zprofile shim")?;
    fs::write(shim_dir.path().join(".zshrc"), ZSH_SHIM_ZSHRC).context("writing zsh .zshrc shim")?;

    Ok(shim_dir)
}

fn build_bash_shim(cmd: &mut CommandBuilder) -> Result<TempDir> {
    let shim_dir = create_secure_shim_dir()?;
    let bashrc_path = shim_dir.path().join("bashrc");
    fs::write(&bashrc_path, BASH_SHIM_RC).context("writing bash rcfile shim")?;
    cmd.arg("--rcfile");
    cmd.arg(&bashrc_path);
    cmd.arg("-i");
    Ok(shim_dir)
}

fn build_fish_shim(cmd: &mut CommandBuilder) -> Result<TempDir> {
    let shim_dir = create_secure_shim_dir()?;
    let init_path = shim_dir.path().join("init.fish");
    fs::write(&init_path, FISH_SHIM_INIT).context("writing fish init shim")?;
    let escaped = escape_fish_single_quoted(&init_path.to_string_lossy());
    cmd.arg("-i");
    cmd.arg("--init-command");
    cmd.arg(format!("source '{escaped}'"));
    Ok(shim_dir)
}

fn build_pwsh_shim(cmd: &mut CommandBuilder) -> Result<TempDir> {
    let shim_dir = create_secure_shim_dir()?;
    let init_path = shim_dir.path().join("init.ps1");
    fs::write(&init_path, PWSH_SHIM_INIT).context("writing pwsh init.ps1 shim")?;
    cmd.arg("-NoLogo");
    cmd.arg("-NoExit");
    cmd.arg("-File");
    cmd.arg(&init_path);
    Ok(shim_dir)
}

fn build_nu_shim(cmd: &mut CommandBuilder, dirs: ShellInitDirs<'_>) -> Result<TempDir> {
    let shim_dir = create_secure_shim_dir()?;
    let home = resolve_home_dir(dirs.home_dir);
    let nu_dir = dirs
        .nu_config_dir
        .map(Path::to_path_buf)
        .or_else(|| default_nu_config_dir(home.as_deref()));

    let mut config_body = String::new();
    if let Some(ref dir) = nu_dir {
        let user_env = dir.join("env.nu");
        if user_env.is_file() {
            cmd.arg("--env-config");
            cmd.arg(&user_env);
        }
        let user_cfg = dir.join("config.nu");
        if user_cfg.is_file() {
            config_body.push_str(&format_nu_source_line(&user_cfg));
            config_body.push('\n');
        }
    }
    config_body.push_str(NU_SHIM_CONFIG);

    let cfg_path = shim_dir.path().join("config.nu");
    fs::write(&cfg_path, config_body).context("writing nushell config.nu shim")?;
    cmd.arg("--config");
    cmd.arg(&cfg_path);
    Ok(shim_dir)
}

const ZSH_SHIM_ZSHENV: &str = r#"# Vox zsh .zshenv launch shim
__vox_shim_dir="${ZDOTDIR}"
if [[ "${VOX_HAD_ZDOTDIR:-0}" == "1" && -n "${VOX_ORIG_ZDOTDIR:-}" ]]; then
  ZDOTDIR="${VOX_ORIG_ZDOTDIR}"
else
  unset ZDOTDIR
fi
if [[ -r "${ZDOTDIR:-$HOME}/.zshenv" ]]; then
  source "${ZDOTDIR:-$HOME}/.zshenv"
fi
if [[ -n "${ZDOTDIR:-}" ]]; then
  VOX_USER_ZDOTDIR="${ZDOTDIR}"
  VOX_HAD_USER_ZDOTDIR=1
else
  VOX_HAD_USER_ZDOTDIR=0
fi
ZDOTDIR="${__vox_shim_dir}"
unset __vox_shim_dir
"#;

const ZSH_SHIM_ZPROFILE: &str = r#"# Vox zsh .zprofile launch shim
if [[ -o login ]]; then
  if [[ "${VOX_HAD_USER_ZDOTDIR:-0}" == "1" && -n "${VOX_USER_ZDOTDIR:-}" ]]; then
    [[ -r "${VOX_USER_ZDOTDIR}/.zprofile" ]] && source "${VOX_USER_ZDOTDIR}/.zprofile"
  elif [[ -r "${HOME}/.zprofile" ]]; then
    source "${HOME}/.zprofile"
  fi
fi
"#;

const ZSH_SHIM_ZSHRC: &str = r#"# Vox zsh .zshrc launch shim
if [[ "${VOX_HAD_USER_ZDOTDIR:-0}" == "1" && -n "${VOX_USER_ZDOTDIR:-}" ]]; then
  export ZDOTDIR="${VOX_USER_ZDOTDIR}"
elif [[ "${VOX_HAD_ZDOTDIR:-0}" == "1" && -n "${VOX_ORIG_ZDOTDIR:-}" ]]; then
  export ZDOTDIR="${VOX_ORIG_ZDOTDIR}"
else
  unset ZDOTDIR
fi
unset VOX_ORIG_ZDOTDIR VOX_HAD_ZDOTDIR VOX_USER_ZDOTDIR VOX_HAD_USER_ZDOTDIR

unsetopt PROMPT_SP

if [[ -r "${ZDOTDIR:-$HOME}/.zshrc" ]]; then
  source "${ZDOTDIR:-$HOME}/.zshrc"
fi

unsetopt PROMPT_SP

__vox_urlencode_pwd() {
  emulate -L zsh
  setopt localoptions nomultibyte
  local LC_ALL=C str="$1" out="" c hex
  local -i i
  for (( i = 1; i <= ${#str}; i++ )); do
    c="${str[i]}"
    case "$c" in
      [-._~A-Za-z0-9/]) out+="$c" ;;
      *) printf -v hex '%%%02X' "'$c"; out+="$hex" ;;
    esac
  done
  REPLY="$out"
}

__vox_emit_cwd() {
  __vox_urlencode_pwd "$PWD"
  local host="${HOST:-$(hostname 2>/dev/null)}"
  printf '\e]7;file://%s%s\a' "$host" "$REPLY"
  local enc_pwd="$PWD"
  enc_pwd="${enc_pwd//\\/\\x5c}"
  enc_pwd="${enc_pwd//;/\\x3b}"
  enc_pwd="${enc_pwd//$'\r'/\\x0d}"
  enc_pwd="${enc_pwd//$'\n'/\\x0a}"
  enc_pwd="${enc_pwd//$'\a'/\\x07}"
  enc_pwd="${enc_pwd//$'\e'/\\x1b}"
  printf '\e]633;P;Cwd=%s\a' "$enc_pwd"
  local branch=""
  branch="$(command git symbolic-ref --quiet --short HEAD 2>/dev/null || true)"
  printf '\e]633;P;GitBranch=%s\a' "$branch"
}

__vox_pager_enter() {
  if [[ -z "${__VOX_PAGER_ACTIVE:-}" ]]; then
    __VOX_PAGER_ACTIVE=1
    [[ "${PAGER+x}" == "x" ]] && { __VOX_HAD_PAGER=1; __VOX_OLD_PAGER="$PAGER"; } || __VOX_HAD_PAGER=0
    [[ "${GIT_PAGER+x}" == "x" ]] && { __VOX_HAD_GIT_PAGER=1; __VOX_OLD_GIT_PAGER="$GIT_PAGER"; } || __VOX_HAD_GIT_PAGER=0
    [[ "${SYSTEMD_PAGER+x}" == "x" ]] && { __VOX_HAD_SYSTEMD_PAGER=1; __VOX_OLD_SYSTEMD_PAGER="$SYSTEMD_PAGER"; } || __VOX_HAD_SYSTEMD_PAGER=0
    [[ "${BAT_PAGER+x}" == "x" ]] && { __VOX_HAD_BAT_PAGER=1; __VOX_OLD_BAT_PAGER="$BAT_PAGER"; } || __VOX_HAD_BAT_PAGER=0
    [[ "${MANPAGER+x}" == "x" ]] && { __VOX_HAD_MANPAGER=1; __VOX_OLD_MANPAGER="$MANPAGER"; } || __VOX_HAD_MANPAGER=0
    [[ "${GH_PAGER+x}" == "x" ]] && { __VOX_HAD_GH_PAGER=1; __VOX_OLD_GH_PAGER="$GH_PAGER"; } || __VOX_HAD_GH_PAGER=0
  fi
  export PAGER=cat GIT_PAGER=cat SYSTEMD_PAGER=cat BAT_PAGER=cat MANPAGER=cat GH_PAGER=cat
}

__vox_pager_restore() {
  [[ -n "${__VOX_PAGER_ACTIVE:-}" ]] || return 0
  unset __VOX_PAGER_ACTIVE
  [[ "${__VOX_HAD_PAGER:-0}" == "1" ]] && export PAGER="${__VOX_OLD_PAGER}" || unset PAGER
  [[ "${__VOX_HAD_GIT_PAGER:-0}" == "1" ]] && export GIT_PAGER="${__VOX_OLD_GIT_PAGER}" || unset GIT_PAGER
  [[ "${__VOX_HAD_SYSTEMD_PAGER:-0}" == "1" ]] && export SYSTEMD_PAGER="${__VOX_OLD_SYSTEMD_PAGER}" || unset SYSTEMD_PAGER
  [[ "${__VOX_HAD_BAT_PAGER:-0}" == "1" ]] && export BAT_PAGER="${__VOX_OLD_BAT_PAGER}" || unset BAT_PAGER
  [[ "${__VOX_HAD_MANPAGER:-0}" == "1" ]] && export MANPAGER="${__VOX_OLD_MANPAGER}" || unset MANPAGER
  [[ "${__VOX_HAD_GH_PAGER:-0}" == "1" ]] && export GH_PAGER="${__VOX_OLD_GH_PAGER}" || unset GH_PAGER
  unset __VOX_HAD_PAGER __VOX_OLD_PAGER __VOX_HAD_GIT_PAGER __VOX_OLD_GIT_PAGER __VOX_HAD_SYSTEMD_PAGER __VOX_OLD_SYSTEMD_PAGER __VOX_HAD_BAT_PAGER __VOX_OLD_BAT_PAGER __VOX_HAD_MANPAGER __VOX_OLD_MANPAGER __VOX_HAD_GH_PAGER __VOX_OLD_GH_PAGER
}

__vox_preexec() {
  __VOX_IN_EXEC=1
  local cmd="$1"
  cmd="${cmd//\\/\\x5c}"
  cmd="${cmd//;/\\x3b}"
  cmd="${cmd//$'\n'/\\x0a}"
  printf '\e]633;E;%s\a\e]633;C\a' "$cmd"
}

__vox_precmd() {
  local ec=$?
  __vox_pager_restore
  if [[ -n "${__VOX_IN_EXEC:-}" ]]; then
    unset __VOX_IN_EXEC
    printf '\e]633;D;%d\a' "$ec"
  fi
  __vox_emit_cwd
}

__vox_prompt_wrap() {
  if [[ "${PROMPT:-}" != *"633;A"* ]]; then
    PROMPT="%{"$'\e]633;A\a'"%}${PROMPT:-%# }%{"$'\e]633;B\a'"%}"
  fi
}

autoload -Uz add-zsh-hook 2>/dev/null || true
add-zsh-hook preexec __vox_preexec
add-zsh-hook precmd __vox_precmd
add-zsh-hook precmd __vox_prompt_wrap
precmd_functions=(__vox_precmd ${precmd_functions:#__vox_precmd})

trap '__vox_ec=$?; if [[ -n "${__VOX_IN_EXEC:-}" ]]; then unset __VOX_IN_EXEC; printf "\e]633;D;%d\a" "$__vox_ec"; fi' EXIT

__vox_prompt_wrap
"#;

const BASH_SHIM_RC: &str = r#"# Vox bash --rcfile launch shim
if [ -r "${HOME}/.bashrc" ]; then
  . "${HOME}/.bashrc"
fi

__vox_urlencode_pwd() {
  local LC_ALL=C str="$1" out="" c hex i=0 len=${#1}
  while [ "$i" -lt "$len" ]; do
    c="${str:$i:1}"
    case "$c" in
      [-._~A-Za-z0-9/]) out="${out}${c}" ;;
      *) hex=$(printf '%%%02X' "'$c"); out="${out}${hex}" ;;
    esac
    i=$((i + 1))
  done
  __VOX_ENCODED_PWD="$out"
}

__vox_emit_cwd() {
  __vox_urlencode_pwd "$PWD"
  local host="${HOSTNAME:-$(hostname 2>/dev/null)}"
  printf '\e]7;file://%s%s\a' "$host" "$__VOX_ENCODED_PWD"
  local enc_pwd="$PWD"
  enc_pwd="${enc_pwd//\\/\\x5c}"
  enc_pwd="${enc_pwd//;/\\x3b}"
  enc_pwd="${enc_pwd//$'\r'/\\x0d}"
  enc_pwd="${enc_pwd//$'\n'/\\x0a}"
  enc_pwd="${enc_pwd//$'\a'/\\x07}"
  enc_pwd="${enc_pwd//$'\e'/\\x1b}"
  printf '\e]633;P;Cwd=%s\a' "$enc_pwd"
  local branch=""
  branch="$(command git symbolic-ref --quiet --short HEAD 2>/dev/null || true)"
  printf '\e]633;P;GitBranch=%s\a' "$branch"
}

if declare -p PROMPT_COMMAND 2>/dev/null | grep -q '^declare -a'; then
  __VOX_PC_IS_ARRAY=1
  __VOX_PREV_PROMPT_COMMANDS=("${PROMPT_COMMAND[@]}")
else
  __VOX_PC_IS_ARRAY=0
  __VOX_PREV_PROMPT_COMMAND="${PROMPT_COMMAND:-}"
fi
__VOX_PREV_DEBUG_TRAP="$(trap -p DEBUG | sed -e "s/^trap -- '//" -e "s/' DEBUG\$//")"

__vox_preexec() {
  local __vox_ec=$?
  if [ -n "${__VOX_AT_PROMPT:-}" ]; then
    case "${BASH_COMMAND:-}" in
      __vox_prompt*|__vox_on_exit*)
        if ! shopt -q extdebug 2>/dev/null; then return $__vox_ec; fi
        return 0
        ;;
    esac
    __VOX_AT_PROMPT=
    __VOX_IN_EXEC=1
    local cmd="${BASH_COMMAND}"
    cmd="${cmd//\\/\\x5c}"
    cmd="${cmd//;/\\x3b}"
    cmd="${cmd//$'\n'/\\x0a}"
    printf '\e]633;E;%s\a\e]633;C\a' "$cmd"
    if [ -n "${__VOX_PREV_DEBUG_TRAP:-}" ]; then
      eval "${__VOX_PREV_DEBUG_TRAP}"
    fi
    return 0
  fi
  if [ -n "${__VOX_PREV_DEBUG_TRAP:-}" ]; then
    eval "${__VOX_PREV_DEBUG_TRAP}"
  fi
  if ! shopt -q extdebug 2>/dev/null; then
    return $__vox_ec
  fi
}

__vox_pager_enter() {
  if [ -z "${__VOX_PAGER_ACTIVE:-}" ]; then
    __VOX_PAGER_ACTIVE=1
    [ "${PAGER+x}" = "x" ] && { __VOX_HAD_PAGER=1; __VOX_OLD_PAGER="$PAGER"; } || __VOX_HAD_PAGER=0
    [ "${GIT_PAGER+x}" = "x" ] && { __VOX_HAD_GIT_PAGER=1; __VOX_OLD_GIT_PAGER="$GIT_PAGER"; } || __VOX_HAD_GIT_PAGER=0
    [ "${SYSTEMD_PAGER+x}" = "x" ] && { __VOX_HAD_SYSTEMD_PAGER=1; __VOX_OLD_SYSTEMD_PAGER="$SYSTEMD_PAGER"; } || __VOX_HAD_SYSTEMD_PAGER=0
    [ "${BAT_PAGER+x}" = "x" ] && { __VOX_HAD_BAT_PAGER=1; __VOX_OLD_BAT_PAGER="$BAT_PAGER"; } || __VOX_HAD_BAT_PAGER=0
    [ "${MANPAGER+x}" = "x" ] && { __VOX_HAD_MANPAGER=1; __VOX_OLD_MANPAGER="$MANPAGER"; } || __VOX_HAD_MANPAGER=0
    [ "${GH_PAGER+x}" = "x" ] && { __VOX_HAD_GH_PAGER=1; __VOX_OLD_GH_PAGER="$GH_PAGER"; } || __VOX_HAD_GH_PAGER=0
  fi
  export PAGER=cat GIT_PAGER=cat SYSTEMD_PAGER=cat BAT_PAGER=cat MANPAGER=cat GH_PAGER=cat
}

__vox_pager_restore() {
  [ -n "${__VOX_PAGER_ACTIVE:-}" ] || return 0
  unset __VOX_PAGER_ACTIVE
  [ "${__VOX_HAD_PAGER:-0}" = "1" ] && export PAGER="${__VOX_OLD_PAGER}" || unset PAGER
  [ "${__VOX_HAD_GIT_PAGER:-0}" = "1" ] && export GIT_PAGER="${__VOX_OLD_GIT_PAGER}" || unset GIT_PAGER
  [ "${__VOX_HAD_SYSTEMD_PAGER:-0}" = "1" ] && export SYSTEMD_PAGER="${__VOX_OLD_SYSTEMD_PAGER}" || unset SYSTEMD_PAGER
  [ "${__VOX_HAD_BAT_PAGER:-0}" = "1" ] && export BAT_PAGER="${__VOX_OLD_BAT_PAGER}" || unset BAT_PAGER
  [ "${__VOX_HAD_MANPAGER:-0}" = "1" ] && export MANPAGER="${__VOX_OLD_MANPAGER}" || unset MANPAGER
  [ "${__VOX_HAD_GH_PAGER:-0}" = "1" ] && export GH_PAGER="${__VOX_OLD_GH_PAGER}" || unset GH_PAGER
  unset __VOX_HAD_PAGER __VOX_OLD_PAGER __VOX_HAD_GIT_PAGER __VOX_OLD_GIT_PAGER __VOX_HAD_SYSTEMD_PAGER __VOX_OLD_SYSTEMD_PAGER __VOX_HAD_BAT_PAGER __VOX_OLD_BAT_PAGER __VOX_HAD_MANPAGER __VOX_OLD_MANPAGER __VOX_HAD_GH_PAGER __VOX_OLD_GH_PAGER
}

__vox_prompt() {
  local ec=$?
  __vox_pager_restore
  __VOX_AT_PROMPT=
  if [ -n "${__VOX_IN_EXEC:-}" ]; then
    __VOX_IN_EXEC=
    printf '\e]633;D;%d\a' "$ec"
  fi
  if [ "${__VOX_PC_IS_ARRAY:-0}" = "1" ]; then
    local __vox_pc_item
    for __vox_pc_item in "${__VOX_PREV_PROMPT_COMMANDS[@]}"; do
      [ -n "$__vox_pc_item" ] && eval "$__vox_pc_item"
    done
  elif [ -n "${__VOX_PREV_PROMPT_COMMAND:-}" ]; then
    eval "${__VOX_PREV_PROMPT_COMMAND}"
  fi
  __vox_emit_cwd
  case "${PS1:-}" in
    *"633;A"*) ;;
    *) PS1="\[\e]633;A\a\]${PS1:-\s-\v\$ }\[\e]633;B\a\]" ;;
  esac
  __VOX_AT_PROMPT=1
}

__vox_on_exit() {
  local __vox_ec=$?
  if [ -n "${__VOX_IN_EXEC:-}" ]; then
    __VOX_IN_EXEC=
    printf '\e]633;D;%d\a' "$__vox_ec"
  fi
}

PS1="\[\e]633;A\a\]${PS1:-\s-\v\$ }\[\e]633;B\a\]"
PROMPT_COMMAND=__vox_prompt
trap '__vox_preexec' DEBUG
trap '__vox_on_exit' EXIT
"#;

const FISH_SHIM_INIT: &str = r#"# Vox fish --init-command launch shim
if not set -q __vox_osc633_loaded
  set -g __vox_osc633_loaded 1

  function __vox_emit_cwd
    set -l escaped_pwd (string escape --style=url -- $PWD | string replace -a '%2F' '/')
    set -l host (hostname 2>/dev/null)
    printf '\e]7;file://%s%s\a' "$host" "$escaped_pwd"
    set -l enc_pwd (string split \n -- "$PWD" | string replace -a '\\' '\\x5c' | string replace -a ';' '\\x3b' | string replace -a \r '\\x0d' | string replace -a \x07 '\\x07' | string replace -a \x1b '\\x1b' | string join '\\x0a')
    printf '\e]633;P;Cwd=%s\a' "$enc_pwd"
    set -l branch (command git symbolic-ref --quiet --short HEAD 2>/dev/null)
    printf '\e]633;P;GitBranch=%s\a' "$branch"
  end

  function __vox_fish_preexec --on-event fish_preexec
    set -g __vox_in_exec 1
    set -l cmd (string split \n -- "$argv[1]" | string replace -a '\\' '\\x5c' | string replace -a ';' '\\x3b' | string join '\\x0a')
    printf '\e]633;E;%s\a\e]633;C\a' "$cmd"
  end

  function __vox_pager_enter
    if not set -q __vox_pager_active
      set -g __vox_pager_active 1
      for v in PAGER GIT_PAGER SYSTEMD_PAGER BAT_PAGER MANPAGER GH_PAGER
        if set -q $v
          set -g __vox_had_$v 1
          set -g __vox_old_$v $$v
        else
          set -g __vox_had_$v 0
        end
      end
    end
    set -gx PAGER cat; set -gx GIT_PAGER cat; set -gx SYSTEMD_PAGER cat; set -gx BAT_PAGER cat; set -gx MANPAGER cat; set -gx GH_PAGER cat
  end

  function __vox_pager_restore
    if set -q __vox_pager_active
      set -e __vox_pager_active
      for v in PAGER GIT_PAGER SYSTEMD_PAGER BAT_PAGER MANPAGER GH_PAGER
        set -l had_var __vox_had_$v
        set -l old_var __vox_old_$v
        if test "$$had_var" = "1"
          set -gx $v $$old_var
        else
          set -e $v
        end
        set -e $had_var $old_var
      end
    end
  end

  function __vox_fish_postexec --on-event fish_postexec
    set -l ec $status
    __vox_pager_restore
    if set -q __vox_in_exec
      set -e __vox_in_exec
      printf '\e]633;D;%d\a' $ec
    end
  end

  function __vox_fish_exit --on-event fish_exit
    set -l ec $status
    if set -q __vox_in_exec
      set -e __vox_in_exec
      printf '\e]633;D;%d\a' $ec
    end
  end

  if functions --query fish_prompt
    functions --copy fish_prompt __vox_orig_fish_prompt
  end

  function fish_prompt
    __vox_emit_cwd
    printf '\e]633;A\a'
    if functions --query __vox_orig_fish_prompt
      __vox_orig_fish_prompt
    else
      printf '%s> ' (prompt_pwd)
    end
    printf '\e]633;B\a'
  end
end
"#;

const PWSH_SHIM_INIT: &str = r#"# Vox PowerShell -File launch shim
if (Test-Path $PROFILE) { . $PROFILE }

if (-not $global:__VoxOsc633Loaded) {
  $global:__VoxOsc633Loaded = $true
  $global:__VoxOrigPrompt = $function:prompt

  function global:__vox_pager_enter {
    if (-not $global:__VoxPagerActive) {
      $global:__VoxPagerActive = $true
      $global:__VoxSavedPagers = @{}
      foreach ($v in @('PAGER','GIT_PAGER','SYSTEMD_PAGER','BAT_PAGER','MANPAGER','GH_PAGER')) {
        $global:__VoxSavedPagers[$v] = [Environment]::GetEnvironmentVariable($v, 'Process')
      }
    }
    foreach ($v in @('PAGER','GIT_PAGER','SYSTEMD_PAGER','BAT_PAGER','MANPAGER','GH_PAGER')) {
      [Environment]::SetEnvironmentVariable($v, 'cat', 'Process')
    }
    if (-not $global:__VoxInExec) {
      $global:__VoxInExec = $true
      [Console]::Write("$([char]27)]633;C$([char]7)")
    }
  }

  function global:__vox_pager_restore {
    if ($global:__VoxPagerActive) {
      $global:__VoxPagerActive = $false
      foreach ($v in @('PAGER','GIT_PAGER','SYSTEMD_PAGER','BAT_PAGER','MANPAGER','GH_PAGER')) {
        [Environment]::SetEnvironmentVariable($v, $global:__VoxSavedPagers[$v], 'Process')
      }
      $global:__VoxSavedPagers = $null
    }
  }

  function global:prompt {
    $origOk = $?
    $origLast = $global:LASTEXITCODE
    __vox_pager_restore
    $esc = [char]27
    $bel = [char]7
    $out = ""
    if ($global:__VoxInExec) {
      $global:__VoxInExec = $false
      $code = if ($null -ne $origLast -and $origLast -ne 0) { $origLast } elseif ($origOk) { 0 } else { 1 }
      $out += "$esc]633;D;$code$bel"
    }
    $pwdPath = $executionContext.SessionState.Path.CurrentLocation.Path
    $uriPath = [System.Uri]::EscapeUriString(($pwdPath -replace '\\', '/'))
    if ($uriPath -notmatch '^/') { $uriPath = "/$uriPath" }
    $hostName = [System.Net.Dns]::GetHostName()
    $out += "$esc]7;file://$hostName$uriPath$bel"
    $encPwd = $pwdPath -replace '\\','\x5c' -replace ';','\x3b' -replace "`r",'\x0d' -replace "`n",'\x0a' -replace ([string][char]7),'\x07' -replace ([string][char]27),'\x1b'
    $out += "$esc]633;P;Cwd=$encPwd$bel"
    $branch = (git symbolic-ref --quiet --short HEAD 2>$null)
    if ($null -eq $branch) { $branch = "" }
    $out += "$esc]633;P;GitBranch=$branch$bel"
    $out += "$esc]633;A$bel"
    if ($null -ne $global:__VoxOrigPrompt) {
      $out += (& $global:__VoxOrigPrompt)
    } else {
      $out += "PS $pwdPath> "
    }
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
}
"#;

const NU_SHIM_CONFIG: &str = r#"# Vox Nushell --config launch shim
$env.__VOX_IN_EXEC = false
def --env __vox_pager_enter [] {
  if not ($env.__VOX_PAGER_ACTIVE? | default false) {
    $env.__VOX_PAGER_ACTIVE = true
    $env.__VOX_SAVED_PAGERS = {
      PAGER: ($env.PAGER? | default null), GIT_PAGER: ($env.GIT_PAGER? | default null),
      SYSTEMD_PAGER: ($env.SYSTEMD_PAGER? | default null), BAT_PAGER: ($env.BAT_PAGER? | default null),
      MANPAGER: ($env.MANPAGER? | default null), GH_PAGER: ($env.GH_PAGER? | default null)
    }
  }
  load-env { PAGER: "cat", GIT_PAGER: "cat", SYSTEMD_PAGER: "cat", BAT_PAGER: "cat", MANPAGER: "cat", GH_PAGER: "cat" }
}
def --env __vox_pager_restore [] {
  if ($env.__VOX_PAGER_ACTIVE? | default false) {
    $env.__VOX_PAGER_ACTIVE = false
    let s = ($env.__VOX_SAVED_PAGERS? | default {})
    if ($s.PAGER? == null) { hide-env -i PAGER } else { $env.PAGER = $s.PAGER }
    if ($s.GIT_PAGER? == null) { hide-env -i GIT_PAGER } else { $env.GIT_PAGER = $s.GIT_PAGER }
    if ($s.SYSTEMD_PAGER? == null) { hide-env -i SYSTEMD_PAGER } else { $env.SYSTEMD_PAGER = $s.SYSTEMD_PAGER }
    if ($s.BAT_PAGER? == null) { hide-env -i BAT_PAGER } else { $env.BAT_PAGER = $s.BAT_PAGER }
    if ($s.MANPAGER? == null) { hide-env -i MANPAGER } else { $env.MANPAGER = $s.MANPAGER }
    if ($s.GH_PAGER? == null) { hide-env -i GH_PAGER } else { $env.GH_PAGER = $s.GH_PAGER }
    hide-env -i __VOX_SAVED_PAGERS
  }
}
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
            __vox_pager_restore
            if ($env.__VOX_IN_EXEC? | default false) {
              $env.__VOX_IN_EXEC = false
              print -n ([(char -u 1b) "]633;D;" ($ec | into string) (char bel)] | str join)
            }
            let cwd = $env.PWD
            let uri_cwd = ($cwd | str replace -a '\' '/' | url encode)
            let slash_cwd = (if ($uri_cwd | str starts-with '%2F') { $uri_cwd | str replace -a '%2F' '/' } else { $"/($uri_cwd | str replace -a '%2F' '/' | str replace -a '%3A' ':')" })
            let enc_cwd = ($cwd | str replace -a '\' '\x5c' | str replace -a ';' '\x3b' | str replace -a "\r" '\x0d' | str replace -a "\n" '\x0a' | str replace -a (char bel) '\x07' | str replace -a (char -u 1b) '\x1b')
            let host = (sys host | get hostname | default "localhost")
            let branch = (do -i { ^git symbolic-ref --quiet --short HEAD } | complete | get stdout | str trim)
            print -n ([(char -u 1b) "]7;file://" $host $slash_cwd (char bel)] | str join)
            print -n ([(char -u 1b) "]633;P;Cwd=" $enc_cwd (char bel)] | str join)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_shells_generate_tempdir_shims_and_unknown_shells_fallback() {
        let fake_home = tempfile::tempdir().expect("fake home");
        let fake_nu_dir = tempfile::tempdir().expect("fake nu dir");

        for shell in ["zsh", "bash", "fish", "pwsh", "nu"] {
            let shim = prepare_shell_command_with_dirs(
                shell,
                Some(fake_home.path()),
                ShellInitDirs {
                    home_dir: Some(fake_home.path()),
                    nu_config_dir: Some(fake_nu_dir.path()),
                },
            )
            .unwrap_or_else(|e| panic!("shim failed for {shell}: {e}"));

            assert!(
                shim.osc633_capable,
                "{shell} should report osc633_capable == true"
            );
            let dir = shim
                .shim_dir
                .as_ref()
                .unwrap_or_else(|| panic!("{shell} must allocate a TempDir"));
            assert!(dir.path().exists(), "{shell} shim dir must exist on disk");

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = fs::metadata(dir.path()).unwrap().permissions().mode() & 0o777;
                assert_eq!(mode, 0o700, "{shell} TempDir must enforce 0700 mode");
            }
        }

        for unknown in ["sh", "cmd.exe"] {
            let shim = prepare_shell_command(unknown, None).unwrap();
            assert!(!shim.osc633_capable, "{unknown} must not claim osc633");
            assert!(shim.shim_dir.is_none(), "{unknown} must not create TempDir");
        }
    }

    #[test]
    fn zsh_shim_enforces_prompt_sp_exit_code_capture_and_osc7() {
        let fake_home = tempfile::tempdir().expect("fake home");
        let shim = prepare_shell_command_with_dirs(
            "/bin/zsh",
            None,
            ShellInitDirs {
                home_dir: Some(fake_home.path()),
                nu_config_dir: None,
            },
        )
        .unwrap();

        let dir = shim.shim_dir.expect("zsh tempdir");
        let zshenv = fs::read_to_string(dir.path().join(".zshenv")).expect(".zshenv");
        let zshrc = fs::read_to_string(dir.path().join(".zshrc")).expect(".zshrc");

        assert!(
            zshenv.contains(".zshenv"),
            ".zshenv must source user .zshenv"
        );
        assert!(
            zshrc.contains("unsetopt PROMPT_SP"),
            ".zshrc must disable macOS PROMPT_SP inverse % noise"
        );
        let user_rc_idx = zshrc.find(".zshrc").expect("sources user .zshrc");
        let hook_idx = zshrc.find("__vox_precmd()").expect("defines __vox_precmd");
        assert!(
            user_rc_idx < hook_idx,
            "user .zshrc must be sourced before Vox hooks"
        );
        assert!(
            zshrc.contains("__vox_precmd() {\n  local ec=$?"),
            "__vox_precmd must capture local ec=$? on its first line"
        );
        assert!(
            zshrc.contains("precmd_functions=(__vox_precmd ${precmd_functions:#__vox_precmd})"),
            "__vox_precmd must be forced to index 1 of precmd_functions"
        );
        assert!(
            zshrc.contains("EXIT"),
            ".zshrc must install EXIT trap for top-level exit N"
        );
        assert!(
            zshrc.contains("%{") && zshrc.contains("633;A") && zshrc.contains("633;B"),
            ".zshrc must wrap 633;A and 633;B inside %{{...%}}"
        );
        assert!(zshrc.contains("]7;file://"), ".zshrc must emit OSC 7");
        assert!(
            zshrc.contains("]633;P;Cwd="),
            ".zshrc must emit 633;P;Cwd property"
        );
        assert!(
            zshrc.contains("]633;P;GitBranch="),
            ".zshrc must emit 633;P;GitBranch property"
        );
    }

    #[test]
    fn bash_fish_and_pwsh_shims_source_user_configs_and_escape_quotes() {
        let fake_home = tempfile::tempdir().expect("fake home");

        let bash_shim = prepare_shell_command_with_dirs(
            "bash",
            None,
            ShellInitDirs {
                home_dir: Some(fake_home.path()),
                nu_config_dir: None,
            },
        )
        .unwrap();
        let bash_dir = bash_shim.shim_dir.expect("bash tempdir");
        let bashrc = fs::read_to_string(bash_dir.path().join("bashrc")).expect("bashrc");
        assert!(bashrc.contains(".bashrc"));
        assert!(bashrc.contains("local ec=$?"));
        assert!(bashrc.contains("\\[\\e]633;A\\a\\]"));
        assert!(bashrc.contains("]7;file://"));
        let bash_argv: Vec<String> = bash_shim
            .cmd
            .get_argv()
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert!(bash_argv.iter().any(|a| a == "--rcfile"));
        assert!(bash_argv.iter().any(|a| a == "-i"));

        let fish_shim = prepare_shell_command_with_dirs(
            "fish",
            None,
            ShellInitDirs {
                home_dir: Some(fake_home.path()),
                nu_config_dir: None,
            },
        )
        .unwrap();
        let fish_dir = fish_shim.shim_dir.expect("fish tempdir");
        let init_fish = fs::read_to_string(fish_dir.path().join("init.fish")).expect("init.fish");
        assert!(init_fish.contains("fish_preexec"));
        assert!(init_fish.contains("fish_postexec"));
        assert!(init_fish.contains("]7;file://"));
        assert!(
            init_fish.contains("string split \\n -- \"$argv[1]\"")
                && init_fish.contains("string join '\\\\x0a'"),
            "fish preexec must split and join multi-line commands with \\x0a"
        );
        assert!(
            init_fish.contains("$enc_pwd"),
            "fish shim must escape Cwd property payload"
        );
        assert_eq!(
            escape_fish_single_quoted("/tmp/o'hare\\dir/init.fish"),
            "/tmp/o\\'hare\\\\dir/init.fish"
        );

        let pwsh_shim = prepare_shell_command_with_dirs(
            "pwsh.exe",
            None,
            ShellInitDirs {
                home_dir: Some(fake_home.path()),
                nu_config_dir: None,
            },
        )
        .unwrap();
        let pwsh_dir = pwsh_shim.shim_dir.expect("pwsh tempdir");
        let init_ps1 = fs::read_to_string(pwsh_dir.path().join("init.ps1")).expect("init.ps1");
        assert!(init_ps1.contains("if (Test-Path $PROFILE) { . $PROFILE }"));
        let git_idx = init_ps1
            .find("git symbolic-ref")
            .expect("pwsh prompt runs git symbolic-ref");
        let orig_prompt_idx = init_ps1
            .find("& $global:__VoxOrigPrompt")
            .expect("pwsh prompt invokes __VoxOrigPrompt");
        let reset_ec_idx = init_ps1
            .find("$global:LASTEXITCODE = 0")
            .expect("pwsh prompt resets LASTEXITCODE");
        assert!(
            reset_ec_idx > git_idx && reset_ec_idx > orig_prompt_idx,
            "$global:LASTEXITCODE = 0 must execute AFTER git symbolic-ref and __VoxOrigPrompt"
        );
        let import_psrl_idx = init_ps1
            .find("Import-Module PSReadLine -ErrorAction SilentlyContinue")
            .expect("pwsh shim must explicitly import PSReadLine before checking Get-Module");
        let get_mod_idx = init_ps1
            .find("if (Get-Module -Name PSReadLine)")
            .expect("pwsh shim checks Get-Module PSReadLine");
        assert!(import_psrl_idx < get_mod_idx);
        assert!(
            init_ps1.contains("@('Enter', 'Ctrl+j')") && init_ps1.contains("-Chord $chord"),
            "pwsh shim must bind both Enter and Ctrl+j chords"
        );
        assert!(
            init_ps1.contains("Cwd=$encPwd"),
            "pwsh shim must escape Cwd property payload"
        );
        let pwsh_argv: Vec<String> = pwsh_shim
            .cmd
            .get_argv()
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert!(pwsh_argv.contains(&"-NoLogo".to_string()));
        assert!(pwsh_argv.contains(&"-NoExit".to_string()));
        assert!(pwsh_argv.contains(&"-File".to_string()));
    }

    #[test]
    fn nu_shim_splices_static_source_only_when_user_config_exists() {
        let fake_home = tempfile::tempdir().expect("fake home");
        let empty_nu_dir = tempfile::tempdir().expect("empty nu dir");

        // Case 1: No user config.nu -> no `source ` line emitted
        let shim_without = prepare_shell_command_with_dirs(
            "nu",
            None,
            ShellInitDirs {
                home_dir: Some(fake_home.path()),
                nu_config_dir: Some(empty_nu_dir.path()),
            },
        )
        .unwrap();
        let cfg_without = fs::read_to_string(
            shim_without
                .shim_dir
                .expect("nu tempdir")
                .path()
                .join("config.nu"),
        )
        .unwrap();
        assert!(
            !cfg_without.contains("\nsource ") && !cfg_without.starts_with("source "),
            "must not emit source when user config.nu is absent: {cfg_without}"
        );
        assert!(cfg_without.contains("upsert shell_integration.osc633 false"));
        assert!(cfg_without.contains("upsert shell_integration.osc7 false"));
        assert!(cfg_without.contains("pre_execution"));
        assert!(
            !cfg_without.contains("{|cmd|"),
            "nu pre_execution closure must take 0 args: {cfg_without}"
        );
        assert!(cfg_without.contains("let raw_cmd = (commandline)"));
        assert!(
            cfg_without.contains("$\"/($uri_cwd"),
            "nu slash_cwd interpolation must include $ sigil: {cfg_without}"
        );
        assert!(
            cfg_without.contains("\"]633;P;Cwd=\" $enc_cwd"),
            "nu shim must escape Cwd property payload: {cfg_without}"
        );
        assert!(
            cfg_without.contains("([\"] \"")
                || cfg_without.contains("([\"> \" (char -u 1b) \"]633;B\""),
            "nu PROMPT_INDICATOR must emit ]633;B after '> ': {cfg_without}"
        );
        assert!(cfg_without.contains("pre_prompt"));

        // Case 2: User config.nu + env.nu exist -> literal `source '...'` precedes `$env.config`
        let pop_nu_dir = tempfile::tempdir().expect("populated nu dir");
        let user_cfg = pop_nu_dir.path().join("config.nu");
        let user_env = pop_nu_dir.path().join("env.nu");
        fs::write(&user_cfg, "$env.MY_NU_TEST = 1\n").unwrap();
        fs::write(&user_env, "$env.MY_NU_ENV = 1\n").unwrap();

        let shim_with = prepare_shell_command_with_dirs(
            "nu",
            None,
            ShellInitDirs {
                home_dir: Some(fake_home.path()),
                nu_config_dir: Some(pop_nu_dir.path()),
            },
        )
        .unwrap();
        let argv_with: Vec<String> = shim_with
            .cmd
            .get_argv()
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert!(argv_with.contains(&"--config".to_string()));
        assert!(argv_with.contains(&"--env-config".to_string()));

        let cfg_with = fs::read_to_string(
            shim_with
                .shim_dir
                .expect("nu tempdir")
                .path()
                .join("config.nu"),
        )
        .unwrap();
        let src_pos = cfg_with
            .find("source '")
            .expect("must splice static literal source '...'");
        let hook_pos = cfg_with
            .find("$env.config")
            .expect("must register $env.config hooks");
        assert!(
            src_pos < hook_pos,
            "user config.nu source line must precede Vox hook registration"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn live_pty_tracer_zero_echo_resize_and_raii_cleanup() {
        let shell = if Path::new("/bin/zsh").exists() {
            "/bin/zsh"
        } else {
            "bash"
        };
        let (mut handle, mut rx) = crate::pty::spawn_pty(shell, 120, 32).expect("spawn_pty");
        assert!(handle.supports_osc633());
        let shim_path = handle.shim_path().expect("shim_path present").to_path_buf();
        assert!(
            shim_path.exists(),
            "TempDir must exist while handle is live"
        );

        // Collect initial prompt output up to `]633;B`
        let mut collected = Vec::new();
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        while tokio::time::Instant::now() < deadline {
            let Some(rem) = deadline.checked_duration_since(tokio::time::Instant::now()) else {
                break;
            };
            match tokio::time::timeout(rem, rx.recv()).await {
                Ok(Some(chunk)) => {
                    collected.extend_from_slice(&chunk);
                    let s = String::from_utf8_lossy(&collected);
                    if s.contains("]633;B") {
                        break;
                    }
                }
                Ok(None) | Err(_) => break,
            }
        }

        let startup = String::from_utf8_lossy(&collected);
        assert!(
            startup.contains("]633;A"),
            "initial prompt must contain OSC 633;A, got: {startup:?}"
        );
        assert!(
            startup.contains("]7;file://"),
            "initial prompt must contain OSC 7 file://, got: {startup:?}"
        );
        for forbidden in [
            "__vox_precmd",
            "__vox_preexec",
            "add-zsh-hook",
            "PROMPT_COMMAND",
        ] {
            assert!(
                !startup.contains(forbidden),
                "startup output leaked echoed hook source {forbidden:?}: {startup:?}"
            );
        }

        // Verify real MasterPty::resize + 0-dimension clamping
        handle.resize(132, 43).expect("resize 132x43");
        let sz = handle.get_size().expect("get_size");
        assert_eq!((sz.cols, sz.rows), (132, 43));

        handle.resize(0, 0).expect("resize 0x0 clamped");
        let clamped = handle.get_size().expect("get_size clamped");
        assert_eq!((clamped.cols, clamped.rows), (1, 1));

        // Drop handle and confirm TempDir is automatically unlinked via RAII
        handle.kill();
        drop(handle);
        assert!(
            !shim_path.exists(),
            "dropping PtyHandle must remove TempDir from disk"
        );
    }

    #[test]
    fn five_shells_define_non_subshell_pager_enter_and_restore() {
        let fake_home = tempfile::tempdir().expect("fake home");
        let fake_nu_dir = tempfile::tempdir().expect("fake nu dir");

        for (shell, file_name) in [
            ("zsh", ".zshrc"),
            ("bash", "bashrc"),
            ("fish", "init.fish"),
            ("pwsh", "init.ps1"),
            ("nu", "config.nu"),
        ] {
            let shim = prepare_shell_command_with_dirs(
                shell,
                None,
                ShellInitDirs {
                    home_dir: Some(fake_home.path()),
                    nu_config_dir: Some(fake_nu_dir.path()),
                },
            )
            .unwrap();
            let dir = shim.shim_dir.expect("shim tempdir");
            let content = fs::read_to_string(dir.path().join(file_name)).unwrap();
            assert!(
                content.contains("__vox_pager_enter"),
                "{shell} must define __vox_pager_enter"
            );
            assert!(
                content.contains("__vox_pager_restore"),
                "{shell} must define and invoke __vox_pager_restore"
            );
            for pager_var in [
                "PAGER",
                "GIT_PAGER",
                "SYSTEMD_PAGER",
                "BAT_PAGER",
                "MANPAGER",
                "GH_PAGER",
            ] {
                assert!(
                    content.contains(pager_var),
                    "{shell} pager guard must cover {pager_var}"
                );
            }
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn live_pty_pager_enter_preserves_cwd_env_and_restores_pagers() {
        let shell = if std::process::Command::new("zsh")
            .arg("--version")
            .output()
            .is_ok()
        {
            "zsh"
        } else {
            "bash"
        };
        let work_root = tempfile::tempdir().expect("work_root");
        let sub_dir = work_root.path().join("nested_agent_dir");
        fs::create_dir_all(&sub_dir).expect("create sub_dir");

        let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(256);
        let mut handle = crate::pty::spawn_pty_with_callback(
            shell,
            120,
            32,
            Some(work_root.path()),
            move |chunk| {
                let _ = tx.blocking_send(chunk.to_vec());
            },
            || {},
        )
        .expect("spawn_pty_with_callback");

        async fn recv_until(rx: &mut tokio::sync::mpsc::Receiver<Vec<u8>>, needle: &str) -> String {
            let mut buf = Vec::new();
            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
            while tokio::time::Instant::now() < deadline {
                let Some(rem) = deadline.checked_duration_since(tokio::time::Instant::now()) else {
                    break;
                };
                match tokio::time::timeout(rem, rx.recv()).await {
                    Ok(Some(chunk)) => {
                        buf.extend_from_slice(&chunk);
                        let s = String::from_utf8_lossy(&buf);
                        if s.contains(needle) {
                            return s.into_owned();
                        }
                    }
                    Ok(None) | Err(_) => break,
                }
            }
            String::from_utf8_lossy(&buf).into_owned()
        }

        let _ = recv_until(&mut rx, "]633;B").await;

        handle
            .write(b"export GIT_PAGER=custom_pager_orig\n")
            .expect("seed GIT_PAGER");
        let _ = recv_until(&mut rx, "]633;B").await;

        let agent_line = crate::agent_exec::format_agent_command_for_shell(
            handle.shell_name(),
            handle.supports_osc633(),
            "cd nested_agent_dir && export VOX_AGENT_ENV=persisted_42 && printf 'GP=%s\\n' \"$GIT_PAGER\"",
        );
        handle
            .write(format!("{agent_line}\n").as_bytes())
            .expect("agent command");
        let agent_out = recv_until(&mut rx, "]633;B").await;
        assert!(
            agent_out.contains("GP=cat"),
            "GIT_PAGER must be cat inside __vox_pager_enter turn, got: {agent_out:?}"
        );

        handle
            .write(b"printf 'AFTER=%s|%s|%s\\n' \"$GIT_PAGER\" \"$VOX_AGENT_ENV\" \"$(basename \"$PWD\")\"\n")
            .expect("verify restore and persistence");
        let restore_out = recv_until(&mut rx, "]633;B").await;
        assert!(
            restore_out.contains("AFTER=custom_pager_orig|persisted_42|nested_agent_dir"),
            "expected restored GIT_PAGER + persisted VOX_AGENT_ENV + nested_agent_dir cwd, got: {restore_out:?}"
        );

        handle.kill();
    }
}
