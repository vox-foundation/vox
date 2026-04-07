//! `vox migrate` — incremental source migrations for web/React interop.
//!
//! Reporting uses the compiler AST lints (`lint_ast_declarations`) so diagnostics stay aligned with
//! [`vox_compiler::typeck::ast_decl_lints`].

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use regex::Regex;
use serde::Serialize;
use walkdir::WalkDir;

/// `@component fn` → `component` (one step toward Path C; may still need manual `view:` / return-type edits).
fn patch_legacy_component_fn_keyword(src: &str) -> Option<String> {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"@component\s+fn\b").expect("regex"));
    if !re.is_match(src) {
        return None;
    }
    Some(re.replace_all(src, "component").to_string())
}

/// `@hook fn` → `fn` (retired decorator surface; authors still add `view:` / hook body manually).
fn patch_legacy_hook_fn_keyword(src: &str) -> Option<String> {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"@hook\s+fn\b").expect("regex"));
    if !re.is_match(src) {
        return None;
    }
    Some(re.replace_all(src, "fn").to_string())
}

/// Parsed CLI for `vox migrate …`.
#[derive(Subcommand, Debug, Clone)]
pub enum MigrateCmd {
    /// Scan `.vox` files for React interop migration findings (Path C, retired decorators).
    Web(WebMigrateArgs),
}

#[derive(clap::Args, Debug, Clone)]
pub struct WebMigrateArgs {
    /// Root directory to scan (recursive).
    #[arg(default_value = ".")]
    pub path: PathBuf,
    /// Emit JSON instead of plain text.
    #[arg(long)]
    pub json: bool,
    /// Apply deterministic filesystem patches (`@component fn`, `@hook fn`, …).
    #[arg(long)]
    pub write: bool,
    /// Exit with failure if any migration findings (or parse errors) remain after the run (for CI).
    #[arg(long)]
    pub check: bool,
}

#[derive(Serialize)]
struct Finding {
    path: String,
    code: String,
    message: String,
    line: Option<u32>,
}

#[derive(Serialize)]
struct WebReport {
    files_scanned: usize,
    parse_errors: usize,
    files_written: usize,
    legacy_component_fn_replacements: usize,
    legacy_hook_fn_replacements: usize,
    findings: Vec<Finding>,
}

/// Run a `vox migrate` subcommand.
pub fn run(cmd: MigrateCmd) -> Result<()> {
    match cmd {
        MigrateCmd::Web(args) => run_web(args),
    }
}

fn run_web(args: WebMigrateArgs) -> Result<()> {
    let root = args.path.canonicalize().with_context(|| {
        format!(
            "canonicalize migrate root {}",
            args.path.display()
        )
    })?;

    let mut report = WebReport {
        files_scanned: 0,
        parse_errors: 0,
        files_written: 0,
        legacy_component_fn_replacements: 0,
        legacy_hook_fn_replacements: 0,
        findings: Vec::new(),
    };

    for entry in WalkDir::new(&root).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let p = entry.path();
        if p.extension().is_none_or(|e| e != "vox") {
            continue;
        }
        report.files_scanned += 1;
        if args.write {
            if let Some((comp_n, hook_n, new_src)) = maybe_patch_vox_file(p)? {
                report.files_written += 1;
                report.legacy_component_fn_replacements += comp_n;
                report.legacy_hook_fn_replacements += hook_n;
                scan_vox_source(p, &new_src, &mut report);
                continue;
            }
        }
        scan_vox_file(p, &mut report);
    }

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).context("serialize migrate report")?
        );
    } else {
        println!(
            "vox migrate web: scanned {} file(s), {} parse error(s), {} lint finding(s){}",
            report.files_scanned,
            report.parse_errors,
            report.findings.len(),
            if args.write {
                format!(
                    ", wrote {} file(s), {} @component and {} @hook keyword replacement(s)",
                    report.files_written,
                    report.legacy_component_fn_replacements,
                    report.legacy_hook_fn_replacements
                )
            } else {
                String::new()
            }
        );
        for f in &report.findings {
            let loc = f
                .line
                .map(|l| format!("{}:{}:", f.path, l))
                .unwrap_or_else(|| format!("{}:", f.path));
            println!("  {loc} [{}] {}", f.code, f.message);
        }
    }

    if args.check && (report.parse_errors > 0 || !report.findings.is_empty()) {
        bail!(
            "migrate web --check: {} parse error(s), {} finding(s)",
            report.parse_errors,
            report.findings.len()
        );
    }

    Ok(())
}

/// Returns `(@component fixes, @hook fixes, new_source)` when anything changed.
fn maybe_patch_vox_file(path: &Path) -> Result<Option<(usize, usize, String)>> {
    let src =
        std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    static RE_COMP: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    static RE_HOOK: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let re_comp = RE_COMP.get_or_init(|| Regex::new(r"@component\s+fn\b").expect("regex"));
    let re_hook = RE_HOOK.get_or_init(|| Regex::new(r"@hook\s+fn\b").expect("regex"));
    let comp_before = re_comp.find_iter(&src).count();
    let hook_before = re_hook.find_iter(&src).count();
    if comp_before == 0 && hook_before == 0 {
        return Ok(None);
    }
    let mut fixed = src;
    if let Some(s) = patch_legacy_component_fn_keyword(&fixed) {
        fixed = s;
    }
    if let Some(s) = patch_legacy_hook_fn_keyword(&fixed) {
        fixed = s;
    }
    let comp_after = re_comp.find_iter(&fixed).count();
    let hook_after = re_hook.find_iter(&fixed).count();
    let comp_n = comp_before.saturating_sub(comp_after);
    let hook_n = hook_before.saturating_sub(hook_after);
    if comp_n == 0 && hook_n == 0 {
        return Ok(None);
    }
    std::fs::write(path, &fixed).with_context(|| format!("write patched {}", path.display()))?;
    Ok(Some((comp_n, hook_n, fixed)))
}

fn scan_vox_source(path: &Path, src: &str, report: &mut WebReport) {
    let rel = path.display().to_string();
    let tokens = vox_compiler::lexer::cursor::lex(src);
    let module = match vox_compiler::parser::parse(tokens) {
        Ok(m) => m,
        Err(errs) => {
            report.parse_errors += 1;
            for e in errs {
                let off = e.span.start;
                report.findings.push(Finding {
                    path: rel.clone(),
                    code: "migrate.parse".into(),
                    message: e.message,
                    line: line_for_offset(src, off),
                });
            }
            return;
        }
    };

    let diags = vox_compiler::typeck::typecheck_module(&module, src);
    for d in diags {
        let code = d.code.clone().unwrap_or_else(|| "lint.unknown".into());
        if !is_migration_lint(&code) {
            continue;
        }
        let line = line_for_offset(src, d.span.start);
        report.findings.push(Finding {
            path: rel.clone(),
            code,
            message: d.message,
            line,
        });
    }
}

fn scan_vox_file(path: &Path, report: &mut WebReport) {
    let rel = path.display().to_string();
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            report.findings.push(Finding {
                path: rel,
                code: "migrate.io".into(),
                message: format!("read failed: {e}"),
                line: None,
            });
            return;
        }
    };
    scan_vox_source(path, &src, report);
}

fn is_migration_lint(code: &str) -> bool {
    matches!(
        code,
        "lint.legacy_component_fn"
            | "lint.retired_context"
            | "lint.retired_hook_fn"
            | "lint.retired_provider_fn"
            | "lint.retired_page_decl"
            | "lint.component_react_hook"
    ) || code.starts_with("lint.retired_")
}

fn line_for_offset(src: &str, byte_off: usize) -> Option<u32> {
    if byte_off > src.len() {
        return None;
    }
    let prefix = &src[..byte_off];
    Some(prefix.bytes().filter(|&b| b == b'\n').count() as u32 + 1)
}

#[cfg(test)]
mod report_shape_tests {
    use super::WebReport;

    #[test]
    fn migrate_web_report_json_includes_write_counters() {
        let r = WebReport {
            files_scanned: 1,
            parse_errors: 0,
            files_written: 0,
            legacy_component_fn_replacements: 0,
            legacy_hook_fn_replacements: 0,
            findings: vec![],
        };
        let v = serde_json::to_value(&r).expect("serde");
        assert!(v.get("files_written").is_some());
        assert!(v.get("legacy_component_fn_replacements").is_some());
        assert!(v.get("legacy_hook_fn_replacements").is_some());
        let mut keys: Vec<_> = v
            .as_object()
            .expect("object")
            .keys()
            .cloned()
            .collect();
        keys.sort();
        assert_eq!(
            keys,
            vec![
                "files_scanned",
                "files_written",
                "findings",
                "legacy_component_fn_replacements",
                "legacy_hook_fn_replacements",
                "parse_errors",
            ],
            "JSON must expose a stable key set for migrate --json consumers"
        );
    }
}
