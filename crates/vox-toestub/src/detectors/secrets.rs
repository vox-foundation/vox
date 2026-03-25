use crate::rules::{DetectionRule, Finding, Language, Severity, SourceFile};
use regex::Regex;

/// Detects hardcoded secrets, API keys, and credentials.
pub struct SecretDetector {
    generic_secret: Regex,
    aws_key: Regex,
    jwt_token: Regex,
    supported_langs: Vec<Language>,
}

impl Default for SecretDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretDetector {
    /// Builds generic secret, AWS key, and JWT heuristics for all supported [`Language`]s.
    pub fn new() -> Self {
        Self {
            generic_secret: Regex::new(r#"(?i)(password|passwd|secret|api[_-]?key|access[_-]?token|auth[_-]?token|bearer)\s*[:=]\s*["'][^"']{8,}["']"#)
                .expect("valid regex"),
            aws_key: Regex::new(r#"\bAKIA[0-9A-Z]{16}\b"#).expect("valid regex"),
            jwt_token: Regex::new(r#"\beyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9\.[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+"#)
                .expect("valid regex"),
            supported_langs: vec![
                Language::Rust,
                Language::TypeScript,
                Language::Python,
                Language::GDScript,
                Language::Vox,
            ],
        }
    }

    fn make_finding(
        &self,
        file: &SourceFile,
        line_num: usize,
        message: String,
        severity: Severity,
    ) -> Finding {
        Finding {
            rule_id: self.id().to_string(),
            rule_name: self.name().to_string(),
            severity,
            file: file.path.clone(),
            line: line_num,
            column: 0,
            message,
            suggestion: Some(
                "Use environment variables or a secret manager instead of hardcoding credentials."
                    .into(),
            ),
            context: file.context_around(line_num, 1),
        }
    }

    fn check_line(&self, file: &SourceFile, line: &str, line_num: usize) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Skip comments
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with('*') {
            return findings;
        }

        // Skip common test/example/placeholder patterns
        let upper = line.to_uppercase();
        if upper.contains("EXAMPLE")
            || upper.contains("PLACEHOLDER")
            || upper.contains("YOUR_")
            || upper.contains("ENV_VAR")
            || line.contains("std::env::var")
            || line.contains("env::var")
            || line.contains("dotenv")
            || line.contains("process.env.")
            || line.contains("os.environ")
        {
            return findings;
        }

        if let Some(m) = self.aws_key.find(line) {
            let key = m.as_str();
            if !Self::aws_key_is_synthetic_placeholder(key) {
                findings.push(self.make_finding(
                    file,
                    line_num,
                    "Potential AWS Access Key ID detected.".to_string(),
                    Severity::Critical,
                ));
            }
        }

        if self.generic_secret.is_match(line) {
            findings.push(self.make_finding(
                file,
                line_num,
                "Potential hardcoded secret or API key detected.".to_string(),
                Severity::Error,
            ));
        }

        if self.jwt_token.is_match(line) {
            findings.push(self.make_finding(
                file,
                line_num,
                "Potential hardcoded JWT token detected.".to_string(),
                Severity::Error,
            ));
        }

        findings
    }

    /// Test/doc keys like `AKIAZZZZZZZZZZZZZZ` are intentionally repetitive; treat as non-secret.
    fn aws_key_is_synthetic_placeholder(key: &str) -> bool {
        let Some(suffix) = key.strip_prefix("AKIA") else {
            return false;
        };
        if suffix.len() != 16 {
            return false;
        }
        let mut chars = suffix.chars();
        let Some(first) = chars.next() else {
            return false;
        };
        suffix.chars().all(|c| c == first)
    }
}

impl DetectionRule for SecretDetector {
    fn detect(&self, file: &SourceFile) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (i, line) in file.lines.iter().enumerate() {
            findings.extend(self.check_line(file, line, i + 1));
        }
        findings
    }

    fn id(&self) -> &'static str {
        "security/hardcoded-secret"
    }

    fn name(&self) -> &'static str {
        "Hardcoded Secret Detector"
    }

    fn description(&self) -> &'static str {
        "Detects potential API keys, passwords, and other credentials in source code."
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn languages(&self) -> &[Language] {
        &self.supported_langs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn source(lang: &str, code: &str) -> SourceFile {
        SourceFile::new(PathBuf::from(format!("test.{}", lang)), code.to_string())
    }

    #[test]
    fn detects_aws_key() {
        let d = SecretDetector::new();
        // Split so repo-wide scan of secrets.rs does not contain a contiguous AKIA+16 match.
        let rs = ["let key = \"AKIA", "1234567890ABCDEF\";"].concat();
        let f = source("rs", &rs);
        let findings = d.detect(&f);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("AWS"));
    }

    #[test]
    fn detects_generic_password() {
        let d = SecretDetector::new();
        // Split so the Rust source line does not match the generic-secret regex (repo-wide scan).
        let py = ["DB_PASSWORD = 'super", "-secret-pass-123'"].concat();
        let f = source("py", &py);
        let findings = d.detect(&f);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("hardcoded secret"));
    }

    #[test]
    fn ignores_example_key() {
        let d = SecretDetector::new();
        // The word EXAMPLE in an AWS key is a common doc pattern — skip it
        let f = source("rs", r#"let k = "AKIAIOSFODNN7EXAMPLE";"#);
        let findings = d.detect(&f);
        assert!(findings.is_empty(), "example key should be excluded");
    }

    #[test]
    fn ignores_uniform_synthetic_aws_key() {
        let d = SecretDetector::new();
        let f = source("rs", r#"let key = "AKIAZZZZZZZZZZZZZZ";"#);
        assert!(
            d.detect(&f).is_empty(),
            "uniform synthetic AWS keys are treated as fixtures"
        );
    }

    #[test]
    fn ignores_env_var_reads() {
        let d = SecretDetector::new();
        let f = source("rs", r#"let key = std::env::var("API_KEY").unwrap();"#);
        let findings = d.detect(&f);
        assert!(findings.is_empty(), "env var reads should not be flagged");
    }

    #[test]
    fn ignores_comment_lines() {
        let d = SecretDetector::new();
        let rs = ["// password: \"super", "-secret-123\""].concat();
        let f = source("rs", &rs);
        let findings = d.detect(&f);
        assert!(findings.is_empty(), "comment lines should not be flagged");
    }
}
