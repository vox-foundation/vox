//! Registry of all built-in detection rules.

/// Flags use of APIs or patterns marked deprecated in project policy.
pub mod deprecated_usage;
/// Heuristic duplicate / near-duplicate logic across files.
pub mod dry_violation;
/// Functions or handlers with empty or trivial bodies (`pass`-style placeholders).
pub mod empty_body;
/// `lib.rs` / module files with too many top-level definitions or oversized type dumps.
pub mod file_organization;
/// Single files that exceed line or method-count thresholds (“god object” smell).
pub mod god_object;
/// CR / CRLF in source files vs LF policy (`vox ci line-endings` parity).
pub mod line_endings;
/// Suspicious literals (large ints, long strings) that should be named constants.
pub mod magic_value;
/// Scaling risks: blocking I/O in async, unbounded reads, SQL/HTTP heuristics.
pub mod scaling;
/// Optional JSON-schema cross-check when a schema path is configured.
pub mod schema_compliance;
/// High-entropy strings and common secret patterns (keys, tokens).
pub mod secrets;
/// Directory sprawl: too many files per folder or banned generic filenames (`utils.rs`, …).
pub mod sprawl;
/// String constants where an enum or ADT would be clearer.
pub mod stringly_typed_enum;
/// `TODO` / `unimplemented!` / obvious stub markers left in shipped code.
pub mod stub;
/// References to symbols that are not defined or imported in the current compilation unit.
pub mod unresolved_ref;
/// Modules declared but never imported or wired into the build graph.
pub mod unwired_module;
/// Heuristic `.unwrap()` in Rust (informational nudge).
pub mod unwrap_call;
/// Premature “done” comments or victory language without matching tests or implementation.
pub mod victory_claim;

use crate::rules::DetectionRule;

/// Returns all built-in detectors.
pub fn all_rules(schema_path: Option<std::path::PathBuf>) -> Vec<Box<dyn DetectionRule>> {
    vec![
        Box::new(stub::StubDetector::new()),
        Box::new(empty_body::EmptyBodyDetector::new()),
        Box::new(magic_value::MagicValueDetector::new()),
        Box::new(victory_claim::VictoryClaimDetector::new()),
        Box::new(unwired_module::UnwiredModuleDetector::new()),
        Box::new(dry_violation::DryViolationDetector::new()),
        Box::new(unresolved_ref::UnresolvedRefDetector::new()),
        Box::new(deprecated_usage::DeprecatedUsageDetector::new()),
        Box::new(secrets::SecretDetector::new()),
        Box::new(god_object::GodObjectDetector::default()),
        Box::new(sprawl::SprawlDetector::default()),
        Box::new(schema_compliance::SchemaComplianceDetector::new(
            schema_path,
        )),
        Box::new(file_organization::FileOrganizationDetector::default()),
        Box::new(stringly_typed_enum::StringlyTypedEnumDetector::new()),
        Box::new(unwrap_call::UnwrapCallDetector::new()),
        Box::new(line_endings::LineEndingDetector::new()),
        Box::new(scaling::ScalingSurfacesDetector::new()),
    ]
}

/// Returns the number of built-in rules.
pub fn rule_count() -> usize {
    17
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_rules_instantiate() {
        let rules = all_rules(None);
        assert_eq!(rules.len(), rule_count());
        // Every rule must have a non-empty ID and name
        for rule in &rules {
            println!("Rule ID: {}", rule.id());
            assert!(!rule.id().is_empty(), "rule ID must not be empty");
            assert!(!rule.name().is_empty(), "rule name must not be empty");
            assert!(
                !rule.languages().is_empty(),
                "rule must support at least one language"
            );
        }
    }
    #[test]
    fn god_object_detector_catches_large_files() {
        use crate::rules::SourceFile;
        use std::path::PathBuf;
        let content = "fn main() {}\n".repeat(600);
        let file = SourceFile::new(PathBuf::from("large.rs"), content);
        let detector = god_object::GodObjectDetector::default();
        let findings = detector.detect(&file);
        assert!(!findings.is_empty());
        assert!(findings[0].message.contains("too large"));
    }

    #[test]
    fn god_object_detector_ignores_blank_only_padding_lines() {
        use crate::rules::SourceFile;
        use std::path::PathBuf;
        let mut content = String::new();
        for _ in 0..600 {
            content.push('\n');
        }
        content.push_str("fn main() {}\n");
        let file = SourceFile::new(PathBuf::from("padded.rs"), content);
        let detector = god_object::GodObjectDetector::default();
        let findings = detector.detect(&file);
        let size_findings: Vec<_> = findings
            .iter()
            .filter(|f| f.message.contains("non-blank lines"))
            .collect();
        assert!(
            size_findings.is_empty(),
            "blank padding should not count toward god-object size"
        );
    }

    #[test]
    fn sprawl_detector_catches_forbidden_names() {
        use crate::rules::SourceFile;
        use std::path::PathBuf;
        let file = SourceFile::new(PathBuf::from("utils.rs"), "fn helper() {}".to_string());
        let detector = sprawl::SprawlDetector::default();
        let findings = detector.detect(&file);
        assert!(!findings.is_empty());
        assert!(findings[0].message.contains("forbidden"));
    }

    #[test]
    fn organization_detector_catches_bloated_lib() {
        use crate::rules::SourceFile;
        use std::path::PathBuf;
        let content =
            "pub struct A; pub struct B; pub struct C; pub struct D;".replace("; ", ";\n");
        let file = SourceFile::new(PathBuf::from("src/lib.rs"), content);
        let detector = file_organization::FileOrganizationDetector::default();
        let findings = detector.detect(&file);
        assert!(!findings.is_empty());
        assert!(
            findings[0]
                .message
                .contains("lib.rs contains 4 definitions")
        );
    }
}
