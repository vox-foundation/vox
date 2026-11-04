use crate::diagnostics::catalog;
use crate::rules::{DetectionRule, Finding, FindingConfidence, Language, Severity, SourceFile};
use regex::Regex;

/// Enforces the workspace All-TursoDB persistence doctrine (`vox/db/non-turso-persistence`).
///
/// See `docs/src/architecture/turso-indexing-and-persistence-audit-research-2026.md` and
/// `contracts/db/data-storage-policy.v1.yaml`:
/// 1. Forbids introducing rival embedded/ORM database crates (`sled`, `redb`, `rocksdb`,
///    `lmdb`, `duckdb`, `diesel`, `sea-orm`, `libsqlite3-sys`) in `Cargo.toml` or `.rs` imports.
/// 2. Forbids C-SQLite-only `USING fts5` virtual table DDL in new runtime code (Turso uses
///    composite B-tree indexes + `vox-search` Tantivy derived indexes).
pub struct DbPersistencePolicyDetector {
    cargo_banned_db_re: Regex,
    rust_banned_import_re: Regex,
    fts5_ddl_re: Regex,
}

impl Default for DbPersistencePolicyDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl DbPersistencePolicyDetector {
    pub fn new() -> Self {
        Self {
            cargo_banned_db_re: Regex::new(
                r#"^\s*"?(sled|redb|rocksdb|lmdb|duckdb|diesel|sea-orm|libsqlite3-sys)"?\s*="#,
            )
            .expect("valid cargo_banned_db_re"),
            rust_banned_import_re: Regex::new(
                r"\buse\s+(sled|redb|rocksdb|lmdb|duckdb|diesel|sea_orm)::",
            )
            .expect("valid rust_banned_import_re"),
            fts5_ddl_re: Regex::new(r"(?i)CREATE\s+VIRTUAL\s+TABLE\s+[^;]*USING\s+fts5\b")
                .expect("valid fts5_ddl_re"),
        }
    }

    fn is_fts5_legacy_allowlisted(path_str: &str) -> bool {
        let norm = path_str.replace('\\', "/");
        norm.ends_with("crates/vox-db/src/schema_extensions.rs")
            || norm.contains("/tests/")
            || norm.ends_with("_test.rs")
            || norm.ends_with("turso_fts_probe.rs")
            || norm.contains("crates/vox-code-audit/")
    }
}

impl DetectionRule for DbPersistencePolicyDetector {
    fn id(&self) -> &'static str {
        "db/non-turso-persistence"
    }

    fn name(&self) -> &'static str {
        "All-TursoDB Persistence Policy Detector"
    }

    fn description(&self) -> &'static str {
        "Detects non-Turso database crates (sled, redb, rocksdb, duckdb, diesel, sea-orm) and C-SQLite-only USING fts5 DDL."
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn languages(&self) -> &[Language] {
        &[Language::Rust, Language::Vox]
    }

    fn diagnostic_id(&self) -> Option<&'static str> {
        Some(catalog::DB_NON_TURSO_PERSISTENCE)
    }

    fn explain(&self) -> &'static str {
        "Vox enforces an All-TursoDB persistence policy (tursodatabase/turso via `vox-db`).\n\
         Do not introduce rival embedded database engines (`sled`, `redb`, `rocksdb`, `duckdb`, \
         `diesel`, `sea-orm`) or C-SQLite-only `USING fts5` virtual tables. Add tables to \
         `crates/vox-db/src/schema/domains/*.rs` with composite/covering B-tree indexes and \
         expose typed methods in `crates/vox-db/src/store/ops_*.rs`."
    }

    fn detect(
        &self,
        file: &SourceFile,
        _rust_ctx: Option<&crate::analysis::RustFileContext>,
    ) -> Vec<Finding> {
        let mut findings = Vec::new();
        let path_str = file.path.to_string_lossy();
        let is_cargo_toml = file
            .path
            .file_name()
            .and_then(|s| s.to_str())
            .is_some_and(|n| n == "Cargo.toml");

        for (i, line) in file.lines.iter().enumerate() {
            let line_num = i + 1;
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with('#') {
                continue;
            }

            if is_cargo_toml {
                if let Some(caps) = self.cargo_banned_db_re.captures(line) {
                    let dep = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
                    findings.push(Finding {
                        rule_id: self.id().to_string(),
                        rule_name: self.name().to_string(),
                        severity: self.severity(),
                        file: file.path.clone(),
                        line: line_num,
                        column: 1,
                        message: format!(
                            "Non-Turso database dependency `{dep}` is forbidden by All-TursoDB policy — use `vox-db` (`turso`)."
                        ),
                        suggestion: Some(
                            "Store relational, KV, and vector state in `vox-db` (`turso`).".to_string(),
                        ),
                        context: trimmed.to_string(),
                        confidence: Some(FindingConfidence::High),
                        evidence: None,
                        diagnostic_id: self.diagnostic_id().map(str::to_string),
                        rationale: None,
                        alternatives: Vec::new(),
                    });
                }
                continue;
            }

            if file.language == Language::Rust {
                if let Some(caps) = self.rust_banned_import_re.captures(line) {
                    let krate = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
                    findings.push(Finding {
                        rule_id: self.id().to_string(),
                        rule_name: self.name().to_string(),
                        severity: self.severity(),
                        file: file.path.clone(),
                        line: line_num,
                        column: 1,
                        message: format!(
                            "Non-Turso database import `{krate}::` is forbidden by All-TursoDB policy — use `vox-db` (`turso`)."
                        ),
                        suggestion: Some(
                            "Add a table in `crates/vox-db/src/schema/domains/` and typed methods in `crates/vox-db/src/store/ops_*.rs`.".to_string(),
                        ),
                        context: trimmed.to_string(),
                        confidence: Some(FindingConfidence::High),
                        evidence: None,
                        diagnostic_id: self.diagnostic_id().map(str::to_string),
                        rationale: None,
                        alternatives: Vec::new(),
                    });
                }

                if !Self::is_fts5_legacy_allowlisted(&path_str) && self.fts5_ddl_re.is_match(line) {
                    findings.push(Finding {
                        rule_id: self.id().to_string(),
                        rule_name: self.name().to_string(),
                        severity: Severity::Warning,
                        file: file.path.clone(),
                        line: line_num,
                        column: 1,
                        message: "C-SQLite `USING fts5` virtual tables are not supported by Turso — use `vox-db` B-tree indexes or `vox-search` derived Tantivy lexical indexing.".to_string(),
                        suggestion: Some(
                            "Remove `USING fts5` DDL and index via `vox-db` B-trees or `vox-search::library::lexical`.".to_string(),
                        ),
                        context: trimmed.to_string(),
                        confidence: Some(FindingConfidence::High),
                        evidence: None,
                        diagnostic_id: self.diagnostic_id().map(str::to_string),
                        rationale: None,
                        alternatives: Vec::new(),
                    });
                }
            }
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn flags_banned_db_in_cargo_toml() {
        let detector = DbPersistencePolicyDetector::new();
        let file = SourceFile::new(
            PathBuf::from("crates/foo/Cargo.toml"),
            "[dependencies]\nredb = \"2.0\"\n".to_string(),
        );
        let findings = detector.detect(&file, None);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("redb"));
    }

    #[test]
    fn flags_banned_db_import_in_rust() {
        let detector = DbPersistencePolicyDetector::new();
        let file = SourceFile::new(
            PathBuf::from("crates/foo/src/lib.rs"),
            "use sled::Db;\n".to_string(),
        );
        let findings = detector.detect(&file, None);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("sled::"));
    }

    #[test]
    fn flags_fts5_virtual_table_outside_allowlist() {
        let detector = DbPersistencePolicyDetector::new();
        let file = SourceFile::new(
            PathBuf::from("crates/foo/src/lib.rs"),
            "let sql = \"CREATE VIRTUAL TABLE docs_fts USING fts5(body);\";\n".to_string(),
        );
        let findings = detector.detect(&file, None);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("USING fts5"));
    }
}
