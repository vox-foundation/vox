use crate::features::{ExtractedFeatures, LiteralContext};
use crate::rules::{DriftRule, WorkspaceContext, is_test_file};
use vox_code_audit::detectors::hardcoded_abs_dir::find_machine_local_dir;
use vox_code_audit::rules::{Finding, FindingConfidence, Language, Severity};

/// Flags hardcoded machine-local workstation home directories (`/Users/<user>`, `/home/<user>`,
/// `C:\Users\<user>`, `C:/Users/<user>`), ephemeral `.gemini/antigravity/brain/` paths, and
/// `/private/tmp/` directories across Rust, TypeScript/JS, and Vox AST string literals.
pub struct HardcodedAbsDirRule;

fn is_extended_test_path(p: &std::path::Path) -> bool {
    if is_test_file(p) {
        return true;
    }
    let s = p.to_string_lossy().replace('\\', "/");
    s.contains("/e2e/")
        || s.ends_with(".test.ts")
        || s.ends_with(".test.tsx")
        || s.ends_with(".spec.ts")
        || s.ends_with(".spec.tsx")
        || s.starts_with("e2e/")
}

impl DriftRule for HardcodedAbsDirRule {
    fn id(&self) -> &'static str {
        "drift/hardcoded-abs-dir"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn languages(&self) -> &[Language] {
        &[Language::Rust, Language::TypeScript, Language::Vox]
    }

    fn check(&self, features: &ExtractedFeatures, _ctx: &WorkspaceContext) -> Vec<Finding> {
        let file_is_test = is_extended_test_path(&features.file);
        let crate_name = features.crate_name.as_deref().unwrap_or("");

        features
            .string_literals
            .iter()
            .filter(|lit| !matches!(lit.ctx, LiteralContext::Doc))
            .filter(|lit| lit.value.contains('/') || lit.value.contains('\\'))
            .filter(|lit| !crate::extractor::is_allowed_at(features, self.id(), lit.loc.line))
            .filter_map(|lit| {
                let is_test_ctx = file_is_test || matches!(lit.ctx, LiteralContext::Test);
                let (_col, matched, kind) =
                    find_machine_local_dir(&lit.value, false, is_test_ctx)?;
                Some(Finding {
                    rule_id: self.id().to_string(),
                    rule_name: "Hardcoded Machine-Local Directory".into(),
                    severity: self.severity(),
                    file: features.file.clone(),
                    line: lit.loc.line,
                    column: lit.loc.col,
                    message: format!(
                        "{kind} (`{matched}`) — replace with a relative or env-resolved path"
                    ),
                    suggestion: Some(
                        "Use workspace-relative paths, `std::env::current_dir()`, `vox_config`, or an env variable."
                            .into(),
                    ),
                    context: format!("crate: {}", crate_name),
                    confidence: Some(FindingConfidence::High),
                    evidence: None,
                    diagnostic_id: Some(
                        vox_code_audit::diagnostics::catalog::FS_HARDCODED_ABS_DIR.into(),
                    ),
                    alternatives: vec![],
                    rationale: None,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::{ExtractedFeatures, LiteralContext, LiteralLoc, Loc};
    use std::path::PathBuf;

    fn ctx() -> WorkspaceContext {
        WorkspaceContext {
            workspace_version: "0.5.0".into(),
            workspace_root: PathBuf::from("."),
            layers: crate::layers_manifest::LayersManifest::default(),
        }
    }

    #[test]
    fn flags_machine_local_paths_across_rust_ts_and_vox() {
        let real = "realdev";
        for (path, lang, val) in [
            (
                "crates/vox-cli/src/lib.rs",
                Language::Rust,
                format!("/Users/{real}/dev/vox"),
            ),
            (
                "crates/vox-gui/ui/e2e/review/cv.spec.ts",
                Language::TypeScript,
                format!(r"C:\Users\{real}\dev\vox"),
            ),
            (
                "examples/app.vox",
                Language::Vox,
                format!("/home/{real}/project"),
            ),
        ] {
            let mut f = ExtractedFeatures::new(PathBuf::from(path), lang);
            f.string_literals.push(LiteralLoc {
                value: val,
                loc: Loc { line: 4, col: 8 },
                ctx: LiteralContext::Code,
            });
            let findings = HardcodedAbsDirRule.check(&f, &ctx());
            assert_eq!(findings.len(), 1, "expected 1 finding for {path}");
            assert_eq!(findings[0].rule_id, "drift/hardcoded-abs-dir");
            assert_eq!(findings[0].severity, Severity::Error);
        }
    }

    #[test]
    fn exempts_synthetic_users_in_test_context_only() {
        let real = "realdev";
        let mut f = ExtractedFeatures::new(
            PathBuf::from("crates/vox-config/src/paths.rs"),
            Language::Rust,
        );
        f.string_literals.push(LiteralLoc {
            value: "/home/alice/.vox".into(),
            loc: Loc { line: 10, col: 0 },
            ctx: LiteralContext::Test,
        });
        assert!(HardcodedAbsDirRule.check(&f, &ctx()).is_empty());

        // Real username inside a test context still errors.
        f.string_literals.push(LiteralLoc {
            value: format!("/Users/{real}/dev/vox"),
            loc: Loc { line: 20, col: 0 },
            ctx: LiteralContext::Test,
        });
        assert_eq!(HardcodedAbsDirRule.check(&f, &ctx()).len(), 1);
    }

    #[test]
    fn honors_drift_allow_comment() {
        let real = "realdev";
        let mut f =
            ExtractedFeatures::new(PathBuf::from("crates/vox-cli/src/lib.rs"), Language::Rust);
        f.string_literals.push(LiteralLoc {
            value: format!("/Users/{real}/dev/vox"),
            loc: Loc { line: 12, col: 0 },
            ctx: LiteralContext::Code,
        });
        let mut set = std::collections::HashSet::new();
        set.insert(12);
        f.allowed_lines.insert("hardcoded-abs-dir".into(), set);
        assert!(HardcodedAbsDirRule.check(&f, &ctx()).is_empty());
    }
}
