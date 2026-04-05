//! PowerShell AST extraction + exec-policy enforcement for **`vox shell check`**.
//!
//! This is the **authoritative** path for validating host/agent PowerShell command strings against
//! [`DEFAULT_POLICY_REL`]. It is **not** wired into `vox shell repl` passthrough; REPL remains a
//! minimal dev utility (see `mod.rs`).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::OnceLock;

use anyhow::{Context, Result, anyhow};
use regex::Regex;
use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::commands::ci::bounded_read::read_utf8_path_capped;

fn url_host_regex() -> &'static Regex {
    static URL_HOST: OnceLock<Regex> = OnceLock::new();
    URL_HOST.get_or_init(|| Regex::new(r"(?i)https?://([^/?#'\s]+)").expect("URL_HOST regex"))
}

/// Repo-relative default policy path.
pub const DEFAULT_POLICY_REL: &str = "contracts/terminal/exec-policy.v1.yaml";
const SCHEMA_REL: &str = "contracts/terminal/exec-policy.v1.schema.json";
const EXTRACT_SCRIPT_REL: &str = "contracts/terminal/pwsh_extract_command_asts.ps1";

#[derive(Debug, Deserialize)]
struct PwshExtraction {
    #[serde(default)]
    parse_errors: Vec<PwshParseError>,
    #[serde(default)]
    commands: Vec<PwshCommand>,
    #[serde(default)]
    string_literals: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PwshParseError {
    message: String,
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct PwshCommand {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    parameters: Vec<String>,
}

/// Terminal execution policy (mirrors `exec-policy.v1.schema.json`).
#[derive(Debug, Clone, Deserialize)]
pub struct ExecPolicyV1 {
    pub version: u32,
    pub allowed_cmdlets: Vec<String>,
    pub allowed_binaries: Vec<String>,
    #[serde(default)]
    pub blocked_parameters: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub network_fetch_commands: Vec<String>,
    #[serde(default)]
    pub network_fetch_domains: Vec<String>,
}

fn repo_root() -> PathBuf {
    vox_repository::resolve_repo_root_for_ci()
}

fn resolve_pwsh() -> Result<PathBuf> {
    which::which("pwsh")
        .or_else(|_| which::which("powershell"))
        .map_err(|_| {
            anyhow!(
                "`pwsh` (or `powershell`) not found on PATH; terminal AST check requires PowerShell"
            )
        })
}

fn policy_path(explicit: Option<&Path>) -> PathBuf {
    explicit
        .map(Path::to_path_buf)
        .unwrap_or_else(|| repo_root().join(DEFAULT_POLICY_REL))
}

pub fn validate_policy_yaml_against_schema(
    repo_root: &Path,
    policy_yaml: &Path,
) -> Result<ExecPolicyV1> {
    let schema_path = repo_root.join(SCHEMA_REL);
    let schema_src = read_utf8_path_capped(&schema_path)
        .with_context(|| format!("read {}", schema_path.display()))?;
    let policy_src = read_utf8_path_capped(policy_yaml)
        .with_context(|| format!("read {}", policy_yaml.display()))?;
    let schema_val: JsonValue = serde_json::from_str(&schema_src)
        .with_context(|| format!("parse {}", schema_path.display()))?;
    let instance: JsonValue =
        serde_yaml::from_str(&policy_src).context("parse exec policy YAML as JSON value")?;
    let validator = vox_jsonschema_util::compile_validator(&schema_val, schema_path.display())
        .context("compile exec-policy.v1.schema.json")?;
    vox_jsonschema_util::validate(
        &instance,
        &validator,
        format!("{} vs {}", policy_yaml.display(), schema_path.display()),
    )
    .map_err(|e| anyhow!("{e:#}"))?;

    serde_yaml::from_str::<ExecPolicyV1>(&policy_src).context("deserialize exec policy")
}

fn run_pwsh_extract(repo_root: &Path, payload: &str) -> Result<PwshExtraction> {
    let script = repo_root.join(EXTRACT_SCRIPT_REL);
    if !script.is_file() {
        return Err(anyhow!("missing extractor script {}", script.display()));
    }
    let pwsh = resolve_pwsh()?;
    let output = std::process::Command::new(&pwsh)
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(&script)
        .env("VOX_SHELL_CHECK_PAYLOAD", payload)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("spawn {}", pwsh.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "pwsh extractor failed ({}): {}",
            output.status,
            stderr.trim()
        ));
    }
    let stdout = String::from_utf8(output.stdout).context("pwsh stdout utf-8")?;
    serde_json::from_str::<PwshExtraction>(&stdout).context("parse pwsh JSON output")
}

fn normalize_invocation_name(name: &str) -> String {
    let name = name.trim();
    if name.is_empty() {
        return String::new();
    }
    // Path-like: use file stem (foo.cmd, bar.exe, baz.ps1)
    if name.contains('/') || name.contains('\\') {
        return Path::new(name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(name)
            .to_string();
    }
    name.to_string()
}

fn allowed_cmdlets_set(policy: &ExecPolicyV1) -> HashSet<String> {
    policy
        .allowed_cmdlets
        .iter()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

fn allowed_binaries_set(policy: &ExecPolicyV1) -> HashSet<String> {
    policy
        .allowed_binaries
        .iter()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

fn invocation_allowed(invoke_key: &str, cmdlets: &HashSet<String>, bins: &HashSet<String>) -> bool {
    let k = invoke_key.trim().to_ascii_lowercase();
    if k.is_empty() {
        return false;
    }
    cmdlets.contains(&k) || bins.contains(&k)
}

fn blocked_params_for_command(
    command_name: Option<&str>,
    blocked: &HashMap<String, Vec<String>>,
) -> HashSet<String> {
    let mut out: HashSet<String> = HashSet::new();
    if let Some(glob) = blocked.get("*") {
        for p in glob {
            out.insert(p.trim().to_ascii_lowercase());
        }
    }
    if let Some(name) = command_name {
        let n = name.trim();
        for (k, v) in blocked {
            if k == "*" {
                continue;
            }
            if k.trim().eq_ignore_ascii_case(n) {
                for p in v {
                    out.insert(p.trim().to_ascii_lowercase());
                }
            }
        }
    }
    out
}

fn network_fetch_set(policy: &ExecPolicyV1) -> HashSet<String> {
    policy
        .network_fetch_commands
        .iter()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

fn domain_allowlist(policy: &ExecPolicyV1) -> HashSet<String> {
    policy
        .network_fetch_domains
        .iter()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

fn check_blocked_parameters(
    command_name: Option<&str>,
    parameters: &[String],
    policy: &ExecPolicyV1,
) -> Result<()> {
    let blocked = blocked_params_for_command(command_name, &policy.blocked_parameters);
    if blocked.is_empty() {
        return Ok(());
    }
    for p in parameters {
        let pn = p.trim().trim_start_matches('-').to_ascii_lowercase();
        if blocked.contains(&pn) {
            return Err(anyhow!(
                "blocked parameter `-{}` on command {:?} (see exec-policy blocked_parameters)",
                p.trim().trim_start_matches('-'),
                command_name.unwrap_or("")
            ));
        }
    }
    Ok(())
}

fn check_network_urls(
    fetch_commands: &HashSet<String>,
    domains: &HashSet<String>,
    extraction: &PwshExtraction,
) -> Result<()> {
    if fetch_commands.is_empty() {
        return Ok(());
    }
    for cmd in &extraction.commands {
        let Some(name) = cmd.name.as_deref() else {
            continue;
        };
        let nkey = normalize_invocation_name(name).to_ascii_lowercase();
        if !fetch_commands.contains(&nkey) {
            continue;
        }
        for lit in &extraction.string_literals {
            for cap in url_host_regex().captures_iter(lit) {
                let Some(host_part) = cap.get(1).map(|m| m.as_str()) else {
                    continue;
                };
                let host = host_part
                    .split_once(':')
                    .map(|(h, _)| h)
                    .unwrap_or(host_part)
                    .trim_end_matches('.')
                    .to_ascii_lowercase();
                if domains.is_empty() {
                    return Err(anyhow!(
                        "network_fetch_commands includes `{name}` but network_fetch_domains is empty; URL in string literal is denied (host hint: {host})"
                    ));
                }
                if !domains.contains(&host) {
                    return Err(anyhow!(
                        "network URL host `{host}` is not in network_fetch_domains (command `{name}`)"
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Run policy check on a PowerShell source string (blocking).
pub fn run_check(payload: &str, policy_file: Option<&Path>) -> Result<()> {
    let root = repo_root();
    let policy_path = policy_path(policy_file);
    let policy = validate_policy_yaml_against_schema(&root, &policy_path)?;
    if policy.version != 1 {
        return Err(anyhow!(
            "unsupported exec policy version {}",
            policy.version
        ));
    }

    let extraction = run_pwsh_extract(&root, payload)?;

    if !extraction.parse_errors.is_empty() {
        let msgs: Vec<_> = extraction
            .parse_errors
            .iter()
            .map(|e| format!("{} ({})", e.message, e.text))
            .collect();
        return Err(anyhow!("PowerShell parse error(s): {}", msgs.join("; ")));
    }

    let cmdlets = allowed_cmdlets_set(&policy);
    let bins = allowed_binaries_set(&policy);
    let net_cmds = network_fetch_set(&policy);
    let net_domains = domain_allowlist(&policy);

    for cmd in &extraction.commands {
        check_blocked_parameters(cmd.name.as_deref(), &cmd.parameters, &policy)?;

        let Some(raw_name) = cmd.name.as_deref() else {
            return Err(anyhow!(
                "command with no resolvable name (dynamic invocation not allowed by exec policy)"
            ));
        };
        if raw_name.trim().is_empty() {
            return Err(anyhow!(
                "empty command name after parse (dynamic invocation not allowed)"
            ));
        }

        let key = normalize_invocation_name(raw_name);
        if !invocation_allowed(&key, &cmdlets, &bins) {
            return Err(anyhow!(
                "command {:?} is not in allowed_cmdlets or allowed_binaries (normalized key: {})",
                raw_name,
                key
            ));
        }
    }

    check_network_urls(&net_cmds, &net_domains, &extraction)?;

    Ok(())
}

/// CI helper: validate policy file against JSON Schema only (no pwsh).
pub fn validate_policy_file(repo_root: &Path, policy_yaml: &Path) -> Result<ExecPolicyV1> {
    validate_policy_yaml_against_schema(repo_root, policy_yaml)
}

#[cfg(test)]
mod shell_policy_tests {
    use super::*;

    fn repo_root() -> PathBuf {
        vox_repository::resolve_repo_root_for_ci()
    }

    fn pwsh_on_path() -> bool {
        which::which("pwsh").is_ok() || which::which("powershell").is_ok()
    }

    #[test]
    fn default_policy_yaml_validates_against_schema() {
        let root = repo_root();
        let policy = root.join(DEFAULT_POLICY_REL);
        assert!(
            policy.is_file(),
            "missing {} (run from repo root)",
            policy.display()
        );
        let loaded = validate_policy_yaml_against_schema(&root, &policy);
        assert!(loaded.is_ok(), "{:?}", loaded.err());
    }

    #[test]
    fn shell_check_rejects_unknown_command_when_pwsh_available() {
        if !pwsh_on_path() {
            return;
        }
        let root = repo_root();
        let policy = root.join(DEFAULT_POLICY_REL);
        let err = run_check("Totally-Fake-Cmdlet-VoxPolicyTest", Some(policy.as_path()));
        assert!(
            err.is_err(),
            "expected policy rejection, got {:?}",
            err.ok()
        );
    }

    #[test]
    fn shell_check_rejects_blocked_recurse_when_pwsh_available() {
        if !pwsh_on_path() {
            return;
        }
        let root = repo_root();
        let policy = root.join(DEFAULT_POLICY_REL);
        let err = run_check("Get-ChildItem -Recurse", Some(policy.as_path()));
        assert!(
            err.is_err(),
            "expected blocked -Recurse, got {:?}",
            err.ok()
        );
    }

    #[test]
    fn shell_check_reports_parse_errors_when_pwsh_available() {
        if !pwsh_on_path() {
            return;
        }
        let root = repo_root();
        let policy = root.join(DEFAULT_POLICY_REL);
        let err = run_check("{", Some(policy.as_path()));
        let msg = err.expect_err("invalid PowerShell should not pass");
        let s = format!("{msg:#}");
        assert!(
            s.to_ascii_lowercase().contains("parse"),
            "expected parse error in message: {s}"
        );
    }
}
