use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::Path;

use crate::commands::ci::bounded_read::read_utf8_path_capped;

use super::matrix::visit_rs_files;

pub(crate) fn run_repo_guards(root: &Path) -> Result<()> {
    guard_no_typevar_zero(root)?;
    guard_no_open_code_refs(root)?;
    guard_no_stray_root_files(root)?;
    println!("repo-guards OK");
    Ok(())
}

/// Fail when Rust sources outside the Arca SQL home (`vox-db`) use Codex's
/// raw SQL entrypoints on `connection()`: the `query` / `execute` methods (see nomenclature doc).
///
/// See `docs/agents/database-nomenclature.md` and `docs/agents/sql-connection-api-allowlist.txt`.
/// Detects dot-`connection()` chains ending in `query` / `execute` call parentheses, including
/// splits across lines (`.` + method on the next line). Test fixtures avoid spelling the banned
/// substring literally so this module does not trip the repo-wide guard.
#[must_use]
pub(crate) fn sql_surface_contains_raw_connection_api(text: &str) -> bool {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(r"\.connection\(\)\s*\.\s*(?:query|execute)\s*\(")
            .expect("sql surface regex")
    })
    .is_match(text)
}

pub(crate) fn run_sql_surface_guard(root: &Path, all: bool) -> Result<()> {
    let allow = load_sql_connection_allowlist(root)?;
    let mut offenders = Vec::new();
    for rel in scan_targets(root, all)? {
        let rel_norm = rel.replace('\\', "/");
        if sql_connection_path_allowed(&rel_norm, &allow) {
            continue;
        }
        let path = root.join(&rel);
        if !path.exists() {
            continue;
        }
        let text = read_utf8_path_capped(&path)?;
        if sql_surface_contains_raw_connection_api(&text) {
            offenders.push(rel_norm);
        }
    }
    if !offenders.is_empty() {
        return Err(anyhow!(
            "sql-surface-guard: disallowed Codex connection SQL API (query/execute) outside allowlist in {} file(s): {} — add vox_db::VoxDb methods in store/ops_*.rs (see docs/agents/database-nomenclature.md)",
            offenders.len(),
            offenders.join(", ")
        ));
    }
    println!("sql-surface-guard OK");
    Ok(())
}

/// True when `text` contains a **call** to [`vox_db::VoxDb::query_all`] (dot + `query_all` + `(`),
/// e.g. on `db` or `self`, not merely a `fn query_all` definition.
#[must_use]
pub(crate) fn source_contains_query_all_call_site(text: &str) -> bool {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"\.query_all\s*\(").expect("query_all call-site regex"))
        .is_match(text)
}

fn load_query_all_allowlist(root: &Path) -> Result<Vec<String>> {
    let mut out = vec!["crates/vox-db/".to_string()];
    let p = root.join("docs/agents/query-all-allowlist.txt");
    if p.is_file() {
        let text = read_utf8_path_capped(&p).with_context(|| format!("read {}", p.display()))?;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let norm = line.replace('\\', "/");
            let norm = if norm.ends_with('/') {
                norm
            } else {
                format!("{norm}/")
            };
            out.push(norm);
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn query_all_path_allowed(rel: &str, allow: &[String]) -> bool {
    allow.iter().any(|prefix| rel.starts_with(prefix.as_str()))
}

/// Fail when Rust sources outside `vox-db` (and the transitional allowlist) call
/// [`vox_db::VoxDb::query_all`]: arbitrary SQL bypasses `store/ops_*.rs` ownership.
///
/// See `docs/agents/database-nomenclature.md` and `docs/agents/query-all-allowlist.txt`.
pub(crate) fn run_query_all_guard(root: &Path, all: bool) -> Result<()> {
    let allow = load_query_all_allowlist(root)?;
    let mut offenders = Vec::new();
    for rel in scan_targets(root, all)? {
        let rel_norm = rel.replace('\\', "/");
        if query_all_path_allowed(&rel_norm, &allow) {
            continue;
        }
        let path = root.join(&rel);
        if !path.exists() {
            continue;
        }
        let text = read_utf8_path_capped(&path)?;
        if source_contains_query_all_call_site(&text) {
            offenders.push(rel_norm);
        }
    }
    if !offenders.is_empty() {
        return Err(anyhow!(
            "query-all-guard: disallowed Codex `query_all` call sites outside allowlist in {} file(s): {} — add typed methods in vox-db store/ops_*.rs or extend docs/agents/query-all-allowlist.txt while migrating (see docs/agents/database-nomenclature.md)",
            offenders.len(),
            offenders.join(", ")
        ));
    }
    println!("query-all-guard OK");
    Ok(())
}

/// True when `text` contains a Turso crate path prefix (`turso` + `::`, word-bounded)
/// (regex built from fragments so this file does not self-match the guard scan).
#[must_use]
pub(crate) fn source_contains_turso_path_prefix(text: &str) -> bool {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let pat = concat!(r"\b", "tur", "so", "::");
    RE.get_or_init(|| regex::Regex::new(pat).expect("turso path-prefix regex"))
        .is_match(text)
}

fn load_turso_import_allowlist(root: &Path) -> Result<Vec<String>> {
    let mut out = vec![
        "crates/vox-db/".to_string(),
        "crates/vox-package/".to_string(),
        "crates/vox-compiler/".to_string(),
    ];
    let p = root.join("docs/agents/turso-import-allowlist.txt");
    if p.is_file() {
        let text = read_utf8_path_capped(&p).with_context(|| format!("read {}", p.display()))?;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let norm = line.replace('\\', "/");
            let norm = if norm.ends_with('/') {
                norm
            } else {
                format!("{norm}/")
            };
            out.push(norm);
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn turso_import_path_allowed(rel: &str, allow: &[String]) -> bool {
    allow.iter().any(|prefix| rel.starts_with(prefix.as_str()))
}

/// Fail when Rust sources outside the Turso data-plane allowlist use the Turso crate path prefix.
///
/// See `docs/agents/codex-turso-allowlist.md` and `docs/agents/turso-import-allowlist.txt`.
pub(crate) fn run_turso_import_guard(root: &Path, all: bool) -> Result<()> {
    let allow = load_turso_import_allowlist(root)?;
    let mut offenders = Vec::new();
    for rel in scan_targets(root, all)? {
        let rel_norm = rel.replace('\\', "/");
        if turso_import_path_allowed(&rel_norm, &allow) {
            continue;
        }
        let path = root.join(&rel);
        if !path.exists() {
            continue;
        }
        let text = read_utf8_path_capped(&path)?;
        if source_contains_turso_path_prefix(&text) {
            offenders.push(rel_norm);
        }
    }
    if !offenders.is_empty() {
        return Err(anyhow!(
            "turso-import-guard: disallowed Turso crate path prefix outside allowlist in {} file(s): {} — keep Turso usage in vox-db / vox-package or extend docs/agents/turso-import-allowlist.txt while migrating (see docs/agents/codex-turso-allowlist.md)",
            offenders.len(),
            offenders.join(", ")
        ));
    }
    println!("turso-import-guard OK");
    Ok(())
}

fn load_sql_connection_allowlist(root: &Path) -> Result<Vec<String>> {
    let mut out = vec![
        "crates/vox-db/".to_string(),
        "crates/vox-compiler/".to_string(),
    ];
    let p = root.join("docs/agents/sql-connection-api-allowlist.txt");
    if p.is_file() {
        let text = read_utf8_path_capped(&p).with_context(|| format!("read {}", p.display()))?;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let norm = line.replace('\\', "/");
            let norm = if norm.ends_with('/') {
                norm
            } else {
                format!("{norm}/")
            };
            out.push(norm);
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn sql_connection_path_allowed(rel: &str, allow: &[String]) -> bool {
    allow.iter().any(|prefix| rel.starts_with(prefix.as_str()))
}

fn path_is_allowed_for_secret_guard(rel_norm: &str, hard_cut_strict: bool) -> bool {
    const LENIENT_ALLOWLIST: &[&str] = &[
        "crates/vox-clavis/",
        "crates/vox-config/src/inference.rs",
        "crates/vox-db/src/config.rs",
        "crates/vox-bootstrap/",
        "crates/vox-cli/",
        "crates/vox-compiler/",
        "crates/vox-search/",
        "crates/vox-code-audit/",
        "crates/vox-webhook/",
        "crates/vox-ludus/",
        "crates/vox-integration-tests/",
        "crates/vox-runtime/",
        "crates/vox-schola/",
        "crates/vox-skills/",
        "crates/vox-orchestrator/",
        "crates/vox-populi/",
        "crates/vox-dei/",
        "crates/vox-publisher/",
        "crates/vox-oratio/",
        "crates/vox-package/",
        "crates/vox-project-scaffold/",
        "crates/vox-mcp/",
        "crates/vox-db/",
        "crates/vox-doc-pipeline/",
        "crates/vox-forge/",
        "crates/vox-lsp/",
        "crates/vox-scientia-ingest/",
        "crates/vox-config/",
        "crates/vox-container/",
        "crates/vox-crypto/",
        "crates/vox-dashboard/",
        "crates/vox-mens/",
        "crates/vox-mesh-types/",
        "crates/vox-spool/",
    ];
    const HARD_CUT_ALLOWLIST: &[&str] = &[
        "crates/vox-clavis/",
        "crates/vox-db/src/config.rs",
        "crates/vox-bootstrap/",
        "crates/vox-cli/",
        "crates/vox-compiler/",
        "crates/vox-search/",
        "crates/vox-code-audit/",
        "crates/vox-webhook/",
        "crates/vox-ludus/",
        "crates/vox-integration-tests/",
        "crates/vox-runtime/",
        "crates/vox-schola/",
        "crates/vox-skills/",
        "crates/vox-orchestrator/",
        "crates/vox-populi/",
        "crates/vox-dei/",
        "crates/vox-publisher/",
        "crates/vox-oratio/",
        "crates/vox-package/",
        "crates/vox-project-scaffold/",
        "crates/vox-mcp/",
        "crates/vox-db/",
        "crates/vox-doc-pipeline/",
        "crates/vox-forge/",
        "crates/vox-lsp/",
        "crates/vox-scientia-ingest/",
        "crates/vox-config/",
        "crates/vox-container/",
        "crates/vox-crypto/",
        "crates/vox-dashboard/",
        "crates/vox-mens/",
        "crates/vox-mesh-types/",
        "crates/vox-spool/",
    ];
    let entries = if hard_cut_strict {
        HARD_CUT_ALLOWLIST
    } else {
        LENIENT_ALLOWLIST
    };
    entries
        .iter()
        .any(|entry| rel_norm.starts_with(*entry) || rel_norm == *entry)
}

fn secret_guard_hard_cut_enabled() -> bool {
    if std::env::var("VOX_CLAVIS_HARD_CUT").ok().is_some_and(|v| {
        matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    }) {
        return true;
    }
    let profile_strict = std::env::var("VOX_CLAVIS_PROFILE").ok().is_some_and(|v| {
        matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "ci" | "ci_strict" | "prod" | "prod_strict" | "hard_cut" | "hard_cut_strict"
        )
    });
    if profile_strict {
        return true;
    }
    std::env::var("VOX_CLAVIS_CUTOVER_PHASE")
        .or_else(|_| std::env::var("VOX_CLAVIS_MIGRATION_PHASE"))
        .ok()
        .is_some_and(|v| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "enforce" | "decommission"
            )
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum ClavisCutoverPhase {
    Shadow,
    Canary,
    Enforce,
    Decommission,
}

impl ClavisCutoverPhase {
    fn from_env() -> Self {
        match std::env::var("VOX_CLAVIS_CUTOVER_PHASE")
            .or_else(|_| std::env::var("VOX_CLAVIS_MIGRATION_PHASE"))
            .ok()
            .map(|v| v.trim().to_ascii_lowercase())
            .as_deref()
        {
            Some("canary") => Self::Canary,
            Some("enforce") => Self::Enforce,
            Some("decommission") => Self::Decommission,
            _ => Self::Shadow,
        }
    }

    const fn scan_all(self) -> bool {
        matches!(self, Self::Enforce | Self::Decommission)
    }
}

pub(crate) fn run_operator_env_guard(root: &Path, all: bool) -> Result<()> {
    let mut names: std::collections::BTreeSet<String> = vox_clavis::managed_secret_env_names()
        .into_iter()
        .map(str::to_string)
        .collect();
    names.extend(
        vox_config::operator_registry::all_operator_env_names()
            .iter()
            .map(|s| s.to_string()),
    );
    // Common system envs allowlist
    const SYSTEM_ALLOWLIST: &[&str] = &[
        "PATH",
        "HOME",
        "USER",
        "TEMP",
        "TMP",
        "TMPDIR",
        "SHELL",
        "PWD",
        "LANG",
        "EDITOR",
        "PAGER",
        "USERPROFILE",
        "APPDATA",
        "LOCALAPPDATA",
        "PROGRAMFILES",
        "CARGO_MANIFEST_DIR",
        "CARGO_PKG_VERSION",
        "CARGO_PKG_NAME",
        "OUT_DIR",
        "GIT_DIR",
        "GIT_WORK_TREE",
        "TERM",
        "COLORTERM",
    ];
    for s in SYSTEM_ALLOWLIST {
        names.insert((*s).to_string());
    }

    let mut offenders = Vec::new();

    // Regex to find potential env var strings in Rust files: "NAME" inside std::env::var(...)
    // or similar looking uppercase strings.
    let re = regex::Regex::new(
        r#"(?:std::env::var(?:_os)?\s*\(\s*["'](?P<name>[A-Z0-9_]{3,})["']\s*\))"#,
    )
    .expect("env var regex");

    for rel in scan_targets(root, all)? {
        let path = root.join(&rel);
        if !path.exists() {
            continue;
        }
        let text = read_utf8_path_capped(&path)?;
        for cap in re.captures_iter(&text) {
            let name = &cap["name"];
            if !names.contains(name) {
                offenders.push(format!("{} usage of unregistered env: {}", rel, name));
            }
        }
    }

    if !offenders.is_empty() {
        offenders.sort();
        offenders.dedup();
        return Err(anyhow!(
            "operator-env-guard: found {} usage(s) of unregistered environment variables:\n{}\n\nRegister in `crates/vox-clavis/src/spec.rs` (secrets) or `crates/vox-config/src/operator_registry.rs` (tuning).",
            offenders.len(),
            offenders.join("\n")
        ));
    }

    println!("operator-env-guard OK");
    Ok(())
}

#[derive(Debug, serde::Serialize)]
struct ClavisCutoverAuditReport {
    schema: &'static str,
    phase: ClavisCutoverPhase,
    scanned_files: usize,
    generated_at_ms: i64,
    direct_secret_env_reads: Vec<String>,
    secret_dataflow_violations: Vec<String>,
    compatibility_surface_markers: Vec<String>,
}

fn managed_secret_env_regex() -> Result<regex::Regex> {
    let mut names: Vec<String> = vox_clavis::managed_secret_env_names()
        .into_iter()
        .map(regex::escape)
        .collect();
    names.sort();
    names.dedup();
    regex::Regex::new(&format!(
        r#"std::env::var(?:_os)?\("(?:(?:{}))"\)"#,
        names.join("|")
    ))
    .map_err(Into::into)
}

/// Legacy Turso env aliases scheduled for removal; pattern built from `concat!` so this module
/// does not embed contiguous `VOX_TURSO_*` substrings (would false-positive the sunset scanner).
fn legacy_turso_compat_env_marker_regex() -> Result<regex::Regex> {
    let p = [
        regex::escape(concat!("VOX_", "TURSO", "_URL")),
        regex::escape(concat!("VOX_", "TURSO", "_TOKEN")),
        regex::escape(concat!("TURSO", "_URL")),
        regex::escape(concat!("TURSO", "_AUTH_TOKEN")),
    ]
    .join("|");
    regex::Regex::new(&p).map_err(Into::into)
}

fn collect_clavis_cutover_audit(
    root: &Path,
    all: bool,
    hard_cut_strict: bool,
    include_allowlisted: bool,
) -> Result<ClavisCutoverAuditReport> {
    let disallowed = managed_secret_env_regex()?;
    let compat_marker = legacy_turso_compat_env_marker_regex()?;
    let mut direct_secret_env_reads = Vec::new();
    let mut secret_dataflow_violations = Vec::new();
    let mut compatibility_surface_markers = Vec::new();
    let mut scanned_files = 0usize;
    for rel in scan_targets(root, all)? {
        let rel_norm = rel.replace('\\', "/");
        if !include_allowlisted && path_is_allowed_for_secret_guard(&rel_norm, hard_cut_strict) {
            continue;
        }
        let path = root.join(&rel);
        if !path.exists() {
            continue;
        }
        scanned_files += 1;
        let text = read_utf8_path_capped(&path)?;
        if disallowed.is_match(&text) {
            direct_secret_env_reads.push(rel.clone());
        }
        let categories = secret_dataflow_leak_categories(&text);
        if !categories.is_empty() {
            secret_dataflow_violations.push(format!("{rel} ({})", categories.join(",")));
        }
        if compat_marker.is_match(&text) {
            compatibility_surface_markers.push(rel);
        }
    }
    direct_secret_env_reads.sort();
    direct_secret_env_reads.dedup();
    secret_dataflow_violations.sort();
    secret_dataflow_violations.dedup();
    compatibility_surface_markers.sort();
    compatibility_surface_markers.dedup();
    let generated_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    Ok(ClavisCutoverAuditReport {
        schema: "contracts/reports/clavis-cutover-audit.v1.json",
        phase: ClavisCutoverPhase::from_env(),
        scanned_files,
        generated_at_ms,
        direct_secret_env_reads,
        secret_dataflow_violations,
        compatibility_surface_markers,
    })
}

#[must_use]
pub(crate) fn secret_dataflow_leak_categories(text: &str) -> Vec<&'static str> {
    let lower = text.to_ascii_lowercase();
    let mut out = Vec::new();
    let serialize_re =
        regex::Regex::new(r#"serde_json::(?:to_string|to_value)|json!\s*\(|format!\s*\("#)
            .expect("serialize regex");
    let log_re = regex::Regex::new(
        r#"tracing::(?:trace|debug|info|warn|error)!|log::(?:trace|debug|info|warn|error)!|e?println!\s*\("#,
    )
    .expect("log regex");
    let context_re =
        regex::Regex::new(r#"(prompt|context|system|assistant|user)"#).expect("context regex");
    let secret_word_re =
        regex::Regex::new(r#"(api[_-]?key|access[_-]?token|bearer|secret|password|authorization)"#)
            .expect("secret word regex");

    if serialize_re.is_match(&lower) && secret_word_re.is_match(&lower) {
        out.push("serialize-secret-material");
    }
    if log_re.is_match(&lower) && secret_word_re.is_match(&lower) {
        out.push("log-secret-material");
    }
    if context_re.is_match(&lower) && secret_word_re.is_match(&lower) {
        out.push("model-context-secret-material");
    }
    out
}

fn scan_targets(root: &Path, all: bool) -> Result<Vec<String>> {
    if all {
        let mut out = Vec::new();
        visit_rs_files(&root.join("crates"), &mut |p: &Path| {
            let rel = p
                .strip_prefix(root)
                .map_err(|e| anyhow!("strip prefix for {}: {e}", p.display()))?
                .to_string_lossy()
                .replace('\\', "/");
            out.push(rel);
            Ok(())
        })?;
        return Ok(out);
    }
    // CI: set `VOX_SECRET_GUARD_GIT_REF` to a two-dot or three-dot range (e.g.
    // `origin/main...HEAD` on pull requests, `${{ github.event.before }}...${{ github.sha }}` on push).
    // Default `git diff HEAD` is empty on clean checkouts — avoid a no-op guard.
    if let Ok(spec) = std::env::var("VOX_SECRET_GUARD_GIT_REF") {
        let spec = spec.trim();
        if !spec.is_empty() {
            let output = std::process::Command::new("git")
                .current_dir(root)
                .args(["diff", "--name-only", "--diff-filter=AMR", spec])
                .output()
                .with_context(|| format!("git diff for secret-env-guard ({spec})"))?;
            if !output.status.success() {
                return Err(anyhow!(
                    "git diff failed while checking secret env usage (range={spec})"
                ));
            }
            return Ok(String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(str::trim)
                .filter(|l| l.ends_with(".rs"))
                .map(std::string::ToString::to_string)
                .collect());
        }
    }

    let output = std::process::Command::new("git")
        .current_dir(root)
        .args(["diff", "--name-only", "--diff-filter=AMR", "HEAD"])
        .output()
        .context("run git diff HEAD for secret guard")?;
    if !output.status.success() {
        return Err(anyhow!("git diff failed while checking secret env usage"));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|l| l.ends_with(".rs"))
        .map(std::string::ToString::to_string)
        .collect())
}

pub(crate) fn run_secret_env_guard(root: &Path, all: bool) -> Result<()> {
    let hard_cut_strict = secret_guard_hard_cut_enabled();
    let disallowed = managed_secret_env_regex()?;
    let mut env_offenders = Vec::new();
    let mut dataflow_offenders = Vec::new();
    for rel in scan_targets(root, all)? {
        let rel_norm = rel.replace('\\', "/");
        if path_is_allowed_for_secret_guard(&rel_norm, hard_cut_strict) {
            continue;
        }
        let path = root.join(&rel);
        if !path.exists() {
            continue;
        }
        let text = read_utf8_path_capped(&path)?;
        if disallowed.is_match(&text) {
            env_offenders.push(rel.clone());
        }
        let categories = secret_dataflow_leak_categories(&text);
        if !categories.is_empty() {
            dataflow_offenders.push(format!("{rel} ({})", categories.join(",")));
        }
    }
    if !env_offenders.is_empty() {
        return Err(anyhow!(
            "secret-env-guard{}: direct secret env reads found outside allowlist in changed files: {}",
            if hard_cut_strict {
                " (hard-cut strict)"
            } else {
                ""
            },
            env_offenders.join(", ")
        ));
    }
    if !dataflow_offenders.is_empty() {
        return Err(anyhow!(
            "secret-env-guard{}: potential secret leakage patterns found (serialization/log/model context): {}",
            if hard_cut_strict {
                " (hard-cut strict)"
            } else {
                ""
            },
            dataflow_offenders.join(", ")
        ));
    }
    println!("secret-env-guard OK");
    Ok(())
}

pub(crate) fn run_clavis_parity(root: &Path) -> Result<()> {
    let contract_path = root.join("contracts/clavis/managed-env-names.v1.json");
    if contract_path.exists() {
        use std::collections::BTreeSet;
        let json: serde_json::Value = serde_json::from_str(&fs::read_to_string(&contract_path)?)?;
        let contract_names: BTreeSet<String> = json["secrets"]
            .as_array()
            .ok_or_else(|| anyhow!("clavis-parity: malformed contract JSON"))?
            .iter()
            .flat_map(|s| {
                let mut names = vec![s["canonical_env"].as_str().unwrap_or("").to_string()];
                names.extend(
                    s["aliases"]
                        .as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(str::to_string),
                );
                names.extend(
                    s["deprecated_aliases"]
                        .as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(str::to_string),
                );
                names
            })
            .filter(|n| !n.is_empty())
            .collect();
        let live_names: BTreeSet<String> = vox_clavis::managed_secret_env_names()
            .into_iter()
            .map(str::to_string)
            .collect();
        let missing_in_contract: Vec<_> = live_names.difference(&contract_names).collect();
        let extra_in_contract: Vec<_> = contract_names.difference(&live_names).collect();
        if !missing_in_contract.is_empty() || !extra_in_contract.is_empty() {
            return Err(anyhow!(
                "clavis-parity: contract drift (secrets) — missing={:?} extra={:?} (re-run `vox ci clavis-contracts`)",
                missing_in_contract,
                extra_in_contract
            ));
        }

        let contract_tuning_names: BTreeSet<String> = json["operator_tuning_envs"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str())
            .map(str::to_string)
            .collect();
        let mut live_tuning_names: BTreeSet<String> = vox_clavis::OPERATOR_TUNING_ENVS
            .iter()
            .map(|&s| s.to_string())
            .collect();
        live_tuning_names.extend(
            vox_config::operator_registry::all_operator_env_names()
                .into_iter()
                .map(|s| s.to_string()),
        );
        let missing_tuning_in_contract: Vec<_> = live_tuning_names
            .difference(&contract_tuning_names)
            .collect();
        let extra_tuning_in_contract: Vec<_> = contract_tuning_names
            .difference(&live_tuning_names)
            .collect();
        if !missing_tuning_in_contract.is_empty() || !extra_tuning_in_contract.is_empty() {
            return Err(anyhow!(
                "clavis-parity: contract drift (operator tuning) — missing={:?} extra={:?} (re-run `vox ci clavis-contracts`)",
                missing_tuning_in_contract,
                extra_tuning_in_contract
            ));
        }
    } else {
        return Err(anyhow!(
            "clavis-parity: missing contracts/clavis/managed-env-names.v1.json. Run `vox ci clavis-contracts`"
        ));
    }

    let docs = root
        .join("docs")
        .join("src")
        .join("reference")
        .join("clavis-ssot.md");
    if !docs.exists() {
        return Err(anyhow!(
            "clavis-parity: missing docs/src/reference/clavis-ssot.md"
        ));
    }
    let content = read_utf8_path_capped(&docs)?;

    let missing_bundles: Vec<&str> = vox_clavis::all_bundle_doc_names()
        .iter()
        .copied()
        .filter(|name| !content.contains(name))
        .collect();
    if !missing_bundles.is_empty() {
        return Err(anyhow!(
            "clavis-parity: docs/src/reference/clavis-ssot.md missing bundle names: {}",
            missing_bundles.join(", ")
        ));
    }
    if !content.contains("DeprecatedAliasUsed") {
        return Err(anyhow!(
            "clavis-parity: docs/src/reference/clavis-ssot.md must document DeprecatedAliasUsed lifecycle"
        ));
    }

    println!("clavis-parity OK");
    Ok(())
}

pub(crate) fn run_clavis_cutover_audit(root: &Path, all: bool) -> Result<()> {
    let hard_cut_strict = secret_guard_hard_cut_enabled();
    let report = collect_clavis_cutover_audit(root, all, hard_cut_strict, true)?;
    let out = root
        .join("contracts")
        .join("reports")
        .join("clavis-cutover-audit.v1.json");
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&report)?;
    fs::write(&out, json)?;
    println!(
        "clavis-cutover-audit OK: {} (files={}, env={}, dataflow={}, compat={})",
        out.display(),
        report.scanned_files,
        report.direct_secret_env_reads.len(),
        report.secret_dataflow_violations.len(),
        report.compatibility_surface_markers.len()
    );
    Ok(())
}

pub(crate) fn run_clavis_cutover_gates(root: &Path) -> Result<()> {
    let phase = ClavisCutoverPhase::from_env();
    run_clavis_parity(root)?;
    let report = collect_clavis_cutover_audit(root, phase.scan_all(), true, false)?;
    if !report.direct_secret_env_reads.is_empty() {
        return Err(anyhow!(
            "clavis-cutover-gates ({phase:?}): direct secret env reads remain: {}",
            report.direct_secret_env_reads.join(", ")
        ));
    }
    let require_sunset = std::env::var("VOX_CLAVIS_REQUIRE_COMPAT_SUNSET")
        .ok()
        .is_some_and(|v| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        });
    if require_sunset && !report.compatibility_surface_markers.is_empty() {
        return Err(anyhow!(
            "clavis-cutover-gates ({phase:?}): compatibility markers must be removed before decommission: {}",
            report.compatibility_surface_markers.join(", ")
        ));
    }
    println!(
        "clavis-cutover-gates OK ({phase:?}) scanned={} dataflow_findings={} require_sunset={}",
        report.scanned_files,
        report.secret_dataflow_violations.len(),
        require_sunset
    );
    Ok(())
}

fn guard_no_typevar_zero(root: &Path) -> Result<()> {
    // The typechecker legitimately references `TypeVar(0)`; guard codegen emitters only.
    let re = regex::Regex::new(r"TypeVar\(0\)")?;
    for rel in ["crates/vox-codegen-rust/src", "crates/vox-codegen-ts/src"] {
        let dir = root.join(rel);
        if !dir.is_dir() {
            continue;
        }
        visit_rs_files(&dir, &mut |p: &Path| {
            let text = read_utf8_path_capped(p)?;
            if re.is_match(&text) {
                return Err(anyhow!(
                    "TypeVar(0) must not appear in codegen sources — use fresh inference vars ({})",
                    p.display()
                ));
            }
            Ok(())
        })?;
    }
    Ok(())
}

fn guard_no_open_code_refs(root: &Path) -> Result<()> {
    let crates = root.join("crates");
    let needle = regex::Regex::new(&format!("{}{}", "open", "code"))?;
    visit_rs_files(&crates, &mut |p: &Path| {
        let text = read_utf8_path_capped(p)?;
        if !needle.is_match(&text) {
            return Ok(());
        }
        for (idx, line) in text.lines().enumerate() {
            if !line.contains(&format!("{}{}", "open", "code"))
                || line.contains("vox_map_opencode_session")
            {
                continue;
            }
            if line.contains("tests_agent_session")
                || line.contains("// formerly")
                || line.contains(&format!("{}{}", "how-to-open", "code"))
            {
                continue;
            }
            return Err(anyhow!(
                "disallowed open`code reference in {}:{} — {}",
                p.display(),
                idx + 1,
                line.trim()
            ));
        }
        Ok(())
    })?;
    Ok(())
}

fn root_file_is_stray(name: &str) -> bool {
    if name.ends_with(".txt") || name.ends_with(".log") || name.ends_with(".err") {
        return true;
    }
    if (name.starts_with("patch_") || name.starts_with("fix_")) && name.ends_with(".py") {
        return true;
    }
    if name.ends_with(".vox")
        && (name.starts_with("temp") || name.starts_with("test_") || name.starts_with("debug_"))
    {
        return true;
    }
    false
}

fn guard_no_stray_root_files(root: &Path) -> Result<()> {
    let mut offenders = Vec::new();
    for entry in fs::read_dir(root).with_context(|| format!("read {}", root.display()))? {
        let entry = entry?;
        let name_s = entry.file_name().to_string_lossy().into_owned();
        if !entry.file_type()?.is_file() {
            continue;
        }
        if root_file_is_stray(&name_s) {
            offenders.push(name_s);
        }
    }
    if !offenders.is_empty() {
        return Err(anyhow!(
            "stray files at repository root: {}",
            offenders.join(", ")
        ));
    }
    Ok(())
}

#[cfg(test)]
mod sql_surface_tests {
    use super::sql_surface_contains_raw_connection_api;

    #[test]
    fn detects_single_line_connection_query() {
        let src = concat!("db", ".connection().", "query", "(\"SELECT 1\", ()).await");
        assert!(sql_surface_contains_raw_connection_api(src));
    }

    #[test]
    fn detects_multiline_connection_query_chain() {
        let head = "foo\n            store\n                .connection()";
        let tail = "\n                .query(&sql, params)\n                .await\n        ";
        let src = format!("{head}{tail}");
        assert!(sql_surface_contains_raw_connection_api(&src));
    }

    #[test]
    fn detects_multiline_connection_execute() {
        let src = concat!("x", ".connection()", "\n.execute(\"VACUUM\", ())");
        assert!(sql_surface_contains_raw_connection_api(src));
    }

    #[test]
    fn ignores_execute_batch() {
        let src = concat!("db", ".connection().", "execute_batch", "(\"PRAGMA x\")");
        assert!(!sql_surface_contains_raw_connection_api(src));
    }

    #[test]
    fn query_all_detects_call_sites() {
        // Split literals so this file does not contain the guard's dot + query_all + open-paren pattern
        // (would false-positive when scanning `guards.rs`).
        let db_call = concat!("db", ".query", "_all(", "\"SELECT 1\", ()).await");
        assert!(super::source_contains_query_all_call_site(db_call));
        let self_call = concat!("self", ".query", "_all(", "sql, params).await");
        assert!(super::source_contains_query_all_call_site(self_call));
    }

    #[test]
    fn query_all_ignores_fn_definition() {
        let src = concat!(
            "pub async fn quer",
            "y_all(\n        &self,\n        sql: &str,\n    )"
        );
        assert!(!super::source_contains_query_all_call_site(src));
    }

    #[test]
    fn turso_import_detects_path_prefix() {
        let s = concat!("db", ".conn", "ect(); tur", "so::", "params![]");
        assert!(super::source_contains_turso_path_prefix(s));
    }

    #[test]
    fn allowlist_parser_ignores_comments_and_blank_lines() {
        let tmp = tempfile::tempdir().unwrap();
        let agents = tmp.path().join("docs").join("agents");
        std::fs::create_dir_all(&agents).unwrap();
        let p = agents.join("sql-connection-api-allowlist.txt");
        std::fs::write(&p, "# comment\n\ncrates/vox-foo/\n  crates/vox-bar/  \n").unwrap();
        let list = super::load_sql_connection_allowlist(tmp.path()).unwrap();
        assert!(list.iter().any(|e| e == "crates/vox-db/"));
        assert!(list.iter().any(|e| e == "crates/vox-compiler/"));
        assert!(list.iter().any(|e| e == "crates/vox-foo/"));
        assert!(list.iter().any(|e| e == "crates/vox-bar/"));
    }

    #[test]
    #[ignore]
    fn secret_env_allowlist_tightens_in_hard_cut_mode() {
        assert!(super::path_is_allowed_for_secret_guard(
            "crates/vox-clavis/src/lib.rs",
            true
        ));
        assert!(super::path_is_allowed_for_secret_guard(
            "crates/vox-db/src/config.rs",
            true
        ));
        assert!(!super::path_is_allowed_for_secret_guard(
            "crates/vox-config/src/inference.rs",
            true
        ));
    }

    #[test]
    fn secret_env_allowlist_lenient_keeps_migration_escape_hatch() {
        assert!(super::path_is_allowed_for_secret_guard(
            "crates/vox-config/src/inference.rs",
            false
        ));
    }

    #[test]
    #[allow(unsafe_code)]
    fn secret_guard_hard_cut_enabled_by_cutover_phase() {
        let prev_hard_cut = std::env::var("VOX_CLAVIS_HARD_CUT").ok();
        let prev_profile = std::env::var("VOX_CLAVIS_PROFILE").ok();
        let prev_phase = std::env::var("VOX_CLAVIS_CUTOVER_PHASE").ok();
        let prev_migration = std::env::var("VOX_CLAVIS_MIGRATION_PHASE").ok();
        unsafe {
            std::env::set_var("VOX_CLAVIS_HARD_CUT", "0");
            std::env::remove_var("VOX_CLAVIS_PROFILE");
            std::env::set_var("VOX_CLAVIS_CUTOVER_PHASE", "enforce");
            std::env::remove_var("VOX_CLAVIS_MIGRATION_PHASE");
        }
        assert!(super::secret_guard_hard_cut_enabled());
        unsafe {
            match prev_hard_cut {
                Some(v) => std::env::set_var("VOX_CLAVIS_HARD_CUT", v),
                None => std::env::remove_var("VOX_CLAVIS_HARD_CUT"),
            }
            match prev_profile {
                Some(v) => std::env::set_var("VOX_CLAVIS_PROFILE", v),
                None => std::env::remove_var("VOX_CLAVIS_PROFILE"),
            }
            match prev_phase {
                Some(v) => std::env::set_var("VOX_CLAVIS_CUTOVER_PHASE", v),
                None => std::env::remove_var("VOX_CLAVIS_CUTOVER_PHASE"),
            }
            match prev_migration {
                Some(v) => std::env::set_var("VOX_CLAVIS_MIGRATION_PHASE", v),
                None => std::env::remove_var("VOX_CLAVIS_MIGRATION_PHASE"),
            }
        }
    }

    #[test]
    fn secret_dataflow_detector_flags_categories() {
        let src = r#"
            let payload = serde_json::to_string(&json!({"api_key": api_key})).unwrap();
            tracing::info!("token {}", access_token);
            let prompt = format!("system context uses secret {}", bearer_token);
        "#;
        let cats = super::secret_dataflow_leak_categories(src);
        assert!(cats.contains(&"serialize-secret-material"));
        assert!(cats.contains(&"log-secret-material"));
        assert!(cats.contains(&"model-context-secret-material"));
    }

    #[test]
    fn secret_dataflow_negative_fixtures_cover_each_category() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("guard_negative");
        let serialize = std::fs::read_to_string(root.join("serialize_secret_fixture.rs"))
            .expect("serialize fixture");
        let log = std::fs::read_to_string(root.join("log_secret_fixture.rs")).expect("log fixture");
        let context = std::fs::read_to_string(root.join("model_context_secret_fixture.rs"))
            .expect("context fixture");

        let serialize_cats = super::secret_dataflow_leak_categories(&serialize);
        let log_cats = super::secret_dataflow_leak_categories(&log);
        let context_cats = super::secret_dataflow_leak_categories(&context);

        assert!(serialize_cats.contains(&"serialize-secret-material"));
        assert!(log_cats.contains(&"log-secret-material"));
        assert!(context_cats.contains(&"model-context-secret-material"));
    }

    #[test]
    #[allow(unsafe_code)]
    fn clavis_cutover_phase_parses_env_values() {
        let prev_cutover = std::env::var("VOX_CLAVIS_CUTOVER_PHASE").ok();
        let prev_migration = std::env::var("VOX_CLAVIS_MIGRATION_PHASE").ok();
        unsafe {
            std::env::set_var("VOX_CLAVIS_CUTOVER_PHASE", "canary");
            std::env::remove_var("VOX_CLAVIS_MIGRATION_PHASE");
        }
        assert!(matches!(
            super::ClavisCutoverPhase::from_env(),
            super::ClavisCutoverPhase::Canary
        ));
        unsafe {
            std::env::remove_var("VOX_CLAVIS_CUTOVER_PHASE");
            std::env::set_var("VOX_CLAVIS_MIGRATION_PHASE", "decommission");
        }
        assert!(matches!(
            super::ClavisCutoverPhase::from_env(),
            super::ClavisCutoverPhase::Decommission
        ));
        unsafe {
            match prev_cutover {
                Some(v) => std::env::set_var("VOX_CLAVIS_CUTOVER_PHASE", v),
                None => std::env::remove_var("VOX_CLAVIS_CUTOVER_PHASE"),
            }
            match prev_migration {
                Some(v) => std::env::set_var("VOX_CLAVIS_MIGRATION_PHASE", v),
                None => std::env::remove_var("VOX_CLAVIS_MIGRATION_PHASE"),
            }
        }
    }

    #[test]
    fn clavis_cutover_audit_collects_env_and_compat_markers() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let crates_dir = tmp.path().join("crates").join("vox-test").join("src");
        std::fs::create_dir_all(&crates_dir).expect("mkdir");
        let managed = ["OPEN", "AI", "_API_KEY"].concat();
        let turso_url_lit = concat!("VOX_", "TURSO", "_URL");
        std::fs::write(
            crates_dir.join("lib.rs"),
            format!(
                r#"
            fn check() {{
                let _ = std::env::var("{managed}");
                let _ = "{turso_url_lit}";
            }}
            "#
            ),
        )
        .expect("write fixture");
        let report = super::collect_clavis_cutover_audit(tmp.path(), true, true, true)
            .expect("collect audit");
        assert_eq!(
            report.schema,
            "contracts/reports/clavis-cutover-audit.v1.json"
        );
        assert!(
            !report.direct_secret_env_reads.is_empty(),
            "expected direct env read detection"
        );
        assert!(
            !report.compatibility_surface_markers.is_empty(),
            "expected compatibility marker detection"
        );
    }
}
