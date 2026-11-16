//! Detector `vox/cargo/dep-hygiene`: enforces structural Cargo dependency hygiene in O(1)
//! per-manifest without spawning `cargo metadata`.
//!
//! Targets `Language::Unknown` exclusively and immediately exits unless `file.path` ends with
//! `Cargo.toml`, adding zero overhead to `.rs`, `.ts`, `.py`, `.gd`, or `.vox` scans.

use crate::diagnostics::catalog::CARGO_DEP_HYGIENE;
use crate::rules::{DetectionRule, Finding, FindingConfidence, Language, Severity, SourceFile};

/// Normalized forward-slash relative/full path helper.
fn norm_path(path: &std::path::Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Returns true for crates where `vox-repository` must stay severed in `[dependencies]`.
fn forbids_vox_repository_dep(norm: &str) -> bool {
    norm.ends_with("crates/vox-compiler/Cargo.toml")
        || norm.ends_with("crates/vox-db/Cargo.toml")
        || norm.ends_with("crates/vox-codegen/Cargo.toml")
        || norm.ends_with("crates/vox-doc-inventory/Cargo.toml")
        || norm.ends_with("crates/vox-corpus/Cargo.toml")
        || norm.ends_with("crates/vox-orchestrator-driver/Cargo.toml")
}

/// Returns true for pure `-types` leaf crates that must not pull heavy runtime/IO dependencies.
fn is_pure_types_crate(norm: &str) -> bool {
    norm.contains("crates/vox-") && norm.ends_with("-types/Cargo.toml")
}

const PURE_TYPES_FORBIDDEN_DEPS: &[&str] = &[
    "turso",
    "reqwest",
    "tokio",
    "axum",
    "walkdir",
    "vox-db",
    "vox-orchestrator",
    "vox-compiler",
];

const VOX_CLI_FORBIDDEN_DEPS: &[&str] = &[
    "vox-sql",
    "vox-scientia",
    "vox-research-shim",
    "vox-effort-audit",
    "vox-effort-route",
    "vox-oauth-pkce",
    "rmcp",
    "tokio-stream",
];

const VOX_COMPILER_FORBIDDEN_DEPS: &[&str] = &["flate2", "bzip2", "zstd"];
const VOX_CODEGEN_FORBIDDEN_DEPS: &[&str] = &["walkdir"];

/// Extracts the dependency crate name from a TOML dependency assignment line in `[dependencies]`.
fn dep_key(trimmed: &str) -> Option<&str> {
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('[') {
        return None;
    }
    let eq_idx = trimmed.find('=')?;
    let lhs = trimmed[..eq_idx].trim();
    // Support both `foo = ...` and `foo.workspace = true`
    let pkg = lhs.split('.').next()?.trim();
    if pkg.is_empty() { None } else { Some(pkg) }
}

/// Extracts the dependency crate name from a `[dependencies.<pkg>]` or
/// `[target.<cfg>.dependencies.<pkg>]` TOML sub-table header.
fn subtable_dep_key(header: &str) -> Option<&str> {
    let inner = header.strip_prefix('[')?.strip_suffix(']')?.trim();
    if let Some(rest) = inner.strip_prefix("dependencies.") {
        let pkg = rest
            .split('.')
            .next()?
            .trim()
            .trim_matches('"')
            .trim_matches('\'');
        return if pkg.is_empty() { None } else { Some(pkg) };
    }
    if inner.starts_with("target.")
        && let Some((_, after)) = inner.split_once(".dependencies.")
    {
        let pkg = after
            .split('.')
            .next()?
            .trim()
            .trim_matches('"')
            .trim_matches('\'');
        return if pkg.is_empty() { None } else { Some(pkg) };
    }
    None
}

/// Fast `Cargo.toml` structural dependency hygiene detector (`vox/cargo/dep-hygiene`).
pub struct CargoDepHygieneDetector;

impl Default for CargoDepHygieneDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl CargoDepHygieneDetector {
    pub fn new() -> Self {
        Self
    }

    fn make_finding(
        &self,
        file: &SourceFile,
        line: usize,
        message: String,
        suggestion: &str,
    ) -> Finding {
        Finding {
            rule_id: self.id().to_string(),
            diagnostic_id: Some(CARGO_DEP_HYGIENE.to_string()),
            rule_name: self.name().to_string(),
            severity: self.severity(),
            file: file.path.clone(),
            line,
            column: 0,
            message,
            suggestion: Some(suggestion.to_string()),
            alternatives: vec![],
            rationale: Some(
                "Keeping leaf crates decoupled and binary manifests free of zombie dependencies \
                 prevents incremental rebuild cascades."
                    .to_string(),
            ),
            context: file
                .lines
                .get(line.saturating_sub(1))
                .cloned()
                .unwrap_or_default(),
            confidence: Some(FindingConfidence::High),
            evidence: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn check_dep(
        &self,
        file: &SourceFile,
        norm: &str,
        line_no: usize,
        dep: &str,
        forbid_repo: bool,
        pure_types: bool,
        is_cli: bool,
        is_compiler: bool,
        is_codegen: bool,
        findings: &mut Vec<Finding>,
    ) {
        if forbid_repo && dep == "vox-repository" {
            findings.push(self.make_finding(
                file,
                line_no,
                format!("`vox-repository` forbidden in `[dependencies]` of `{norm}` — reintroduces 43-crate rebuild cascades."),
                "Inline the small discovery helper with `// vox:defactored-from vox-repository <date>` per AGENTS.md §Dependency Discipline.",
            ));
        }
        if pure_types && PURE_TYPES_FORBIDDEN_DEPS.contains(&dep) {
            findings.push(self.make_finding(
                file,
                line_no,
                format!("Pure `-types` crate `{norm}` must not depend on heavy runtime/IO crate `{dep}`."),
                "Keep `-types` crates pure data + serde only so L0/L1 consumers build without runtime edges.",
            ));
        }
        if is_cli && VOX_CLI_FORBIDDEN_DEPS.contains(&dep) {
            findings.push(self.make_finding(
                file,
                line_no,
                format!("Pruned zombie/heavy dependency `{dep}` reintroduced in `crates/vox-cli/Cargo.toml` `[dependencies]`."),
                "Move test-only deps to `[dev-dependencies]` or route via a companion crate/binary.",
            ));
        }
        if is_compiler && VOX_COMPILER_FORBIDDEN_DEPS.contains(&dep) {
            findings.push(self.make_finding(
                file,
                line_no,
                format!("Removed dependency `{dep}` reintroduced in `crates/vox-compiler/Cargo.toml` `[dependencies]`."),
                "Keep `vox-compiler` lean; avoid unused compression crates or `vox-repository`.",
            ));
        }
        if is_codegen && VOX_CODEGEN_FORBIDDEN_DEPS.contains(&dep) {
            findings.push(self.make_finding(
                file,
                line_no,
                format!("Removed dependency `{dep}` reintroduced in `crates/vox-codegen/Cargo.toml` `[dependencies]`."),
                "Keep `vox-codegen` decoupled from `walkdir` and `vox-repository`.",
            ));
        }
    }
}

impl DetectionRule for CargoDepHygieneDetector {
    fn id(&self) -> &'static str {
        CARGO_DEP_HYGIENE
    }

    fn name(&self) -> &'static str {
        "Cargo Dependency Hygiene Detector"
    }

    fn description(&self) -> &'static str {
        "Flags structural dependency regressions in Cargo.toml (leaf-core bypass, heavy deps in pure -types crates, pruned zombie deps in vox-cli/compiler/codegen, and missing dev-profile debug thinning)."
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn minimal_repro(&self) -> Option<&'static str> {
        Some(
            "# VIOLATION — crates/vox-compiler/Cargo.toml reintroduces vox-repository\n\
             [dependencies]\n\
             vox-repository = { workspace = true }\n\n\
             # FIX — keep pure discovery helpers Rule-3 defactored or in L0 leaf crates",
        )
    }

    fn languages(&self) -> &[Language] {
        // Cargo.toml maps to Language::Unknown; skipping all Rust/TS/Vox files saves scan time.
        &[Language::Unknown]
    }

    fn detect(
        &self,
        file: &SourceFile,
        _rust: Option<&crate::analysis::RustFileContext>,
    ) -> Vec<Finding> {
        let mut findings = Vec::new();
        if file.path.file_name().and_then(|n| n.to_str()) != Some("Cargo.toml") {
            return findings;
        }

        let norm = norm_path(&file.path);

        // Guard root workspace [profile.dev] unwind/debug settings against accidental removal.
        if file.content.contains("[workspace]") && file.content.contains("[profile.dev]") {
            if !file.content.contains("split-debuginfo = \"unpacked\"")
                || !file.content.contains("debug = 1")
            {
                findings.push(self.make_finding(
                    file,
                    1,
                    "Root Cargo.toml [profile.dev] must keep `split-debuginfo = \"unpacked\"` and `debug = 1` to avoid multi-GB rlibs and slow linking.".to_string(),
                    "Restore `split-debuginfo = \"unpacked\"` and `debug = 1` under `[profile.dev]`.",
                ));
            }
            return findings;
        }

        let forbid_repo = forbids_vox_repository_dep(&norm);
        let pure_types = is_pure_types_crate(&norm);
        let is_cli = norm.ends_with("crates/vox-cli/Cargo.toml");
        let is_compiler = norm.ends_with("crates/vox-compiler/Cargo.toml");
        let is_codegen = norm.ends_with("crates/vox-codegen/Cargo.toml");

        if !forbid_repo && !pure_types && !is_cli && !is_compiler && !is_codegen {
            return findings;
        }

        let mut in_normal_deps = false;
        for (idx, line) in file.lines.iter().enumerate() {
            let trimmed = line.trim();
            let line_no = idx + 1;
            if trimmed.starts_with('[') {
                let header = trimmed.split('#').next().unwrap_or("").trim();
                in_normal_deps = header == "[dependencies]"
                    || (header.starts_with("[target.") && header.ends_with(".dependencies]"));
                if !in_normal_deps && let Some(dep) = subtable_dep_key(header) {
                    self.check_dep(
                        file,
                        &norm,
                        line_no,
                        dep,
                        forbid_repo,
                        pure_types,
                        is_cli,
                        is_compiler,
                        is_codegen,
                        &mut findings,
                    );
                }
                continue;
            }
            if !in_normal_deps {
                continue;
            }
            let Some(dep) = dep_key(trimmed) else {
                continue;
            };
            self.check_dep(
                file,
                &norm,
                line_no,
                dep,
                forbid_repo,
                pure_types,
                is_cli,
                is_compiler,
                is_codegen,
                &mut findings,
            );
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn flags_vox_repository_reintroduced_in_compiler() {
        let sf = SourceFile::new(
            PathBuf::from("crates/vox-compiler/Cargo.toml"),
            "[package]\nname = \"vox-compiler\"\n[dependencies]\nvox-repository = { workspace = true }\n"
                .to_string(),
        );
        let findings = CargoDepHygieneDetector::new().detect(&sf, None);
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].diagnostic_id.as_deref(),
            Some(CARGO_DEP_HYGIENE)
        );
    }

    #[test]
    fn allows_test_only_dep_in_cli_dev_dependencies() {
        let sf = SourceFile::new(
            PathBuf::from("crates/vox-cli/Cargo.toml"),
            "[package]\nname = \"vox-cli\"\n[dependencies]\nserde = { workspace = true }\n[dev-dependencies]\nvox-effort-audit = { workspace = true }\n"
                .to_string(),
        );
        let findings = CargoDepHygieneDetector::new().detect(&sf, None);
        assert!(findings.is_empty());
    }

    #[test]
    fn flags_heavy_dep_in_pure_types_crate() {
        let sf = SourceFile::new(
            PathBuf::from("crates/vox-db-types/Cargo.toml"),
            "[package]\nname = \"vox-db-types\"\n[dependencies]\nturso = { workspace = true }\n"
                .to_string(),
        );
        let findings = CargoDepHygieneDetector::new().detect(&sf, None);
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn flags_subtable_and_commented_header_dependencies() {
        let commented = SourceFile::new(
            PathBuf::from("crates/vox-compiler/Cargo.toml"),
            "[package]\nname = \"vox-compiler\"\n[dependencies] # workspace core\nvox-repository = { workspace = true }\n"
                .to_string(),
        );
        assert_eq!(
            CargoDepHygieneDetector::new()
                .detect(&commented, None)
                .len(),
            1
        );

        let subtable = SourceFile::new(
            PathBuf::from("crates/vox-compiler/Cargo.toml"),
            "[package]\nname = \"vox-compiler\"\n[dependencies.vox-repository]\nworkspace = true\n[target.'cfg(unix)'.dependencies.flate2]\nworkspace = true\n"
                .to_string(),
        );
        assert_eq!(
            CargoDepHygieneDetector::new().detect(&subtable, None).len(),
            2
        );

        let types_subtable = SourceFile::new(
            PathBuf::from("crates/vox-db-types/Cargo.toml"),
            "[package]\nname = \"vox-db-types\"\n[target.'cfg(unix)'.dependencies.turso]\nworkspace = true\n"
                .to_string(),
        );
        assert_eq!(
            CargoDepHygieneDetector::new()
                .detect(&types_subtable, None)
                .len(),
            1
        );
    }
}
