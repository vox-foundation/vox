//! Detector `vox/schema/inline-compile` — Schema/serde parse rebuilt inside hot functions.

use crate::rules::{DetectionRule, Finding, FindingConfidence, Language, Severity, SourceFile};

pub struct InlineSchemaCompileDetector {
    langs: Vec<Language>,
}

impl Default for InlineSchemaCompileDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl InlineSchemaCompileDetector {
    pub fn new() -> Self {
        Self {
            langs: vec![Language::Rust, Language::TypeScript],
        }
    }
}

impl DetectionRule for InlineSchemaCompileDetector {
    fn id(&self) -> &'static str {
        "vox/schema/inline-compile"
    }
    fn name(&self) -> &'static str {
        "Inline Schema Compile Detector"
    }
    fn description(&self) -> &'static str {
        "Flags Schema.decode/encode or serde_yaml/json schema parse inside non-module-scope bodies."
    }
    fn severity(&self) -> Severity {
        Severity::Info
    }
    fn languages(&self) -> &[Language] {
        &self.langs
    }

    fn detect(
        &self,
        file: &SourceFile,
        _rust_ctx: Option<&crate::analysis::RustFileContext>,
    ) -> Vec<Finding> {
        let mut out = Vec::new();
        let mut fn_depth = 0i32;
        for (idx, line) in file.lines.iter().enumerate() {
            let line_num = idx + 1;
            let trimmed = line.trim();
            if trimmed.starts_with("fn ")
                || trimmed.starts_with("pub fn ")
                || trimmed.starts_with("async fn ")
                || trimmed.starts_with("pub async fn ")
                || trimmed.contains("function ")
                || trimmed.contains("=> {")
            {
                fn_depth += 1;
            }
            if trimmed == "}" {
                fn_depth = (fn_depth - 1).max(0);
            }
            if fn_depth <= 0 {
                continue;
            }
            if trimmed.starts_with("//") {
                continue;
            }
            let hit = trimmed.contains("Schema.decode")
                || trimmed.contains("Schema.encode")
                || trimmed.contains("Schema.parser")
                || (trimmed.contains("serde_yaml::from_str") && trimmed.contains("schema"))
                || trimmed.contains("json_schema::");
            if hit {
                out.push(Finding {
                    rule_id: self.id().to_string(),
                    diagnostic_id: Some(
                        crate::diagnostics::catalog::INLINE_SCHEMA_COMPILE.to_string(),
                    ),
                    rule_name: self.name().to_string(),
                    severity: self.severity(),
                    file: file.path.clone(),
                    line: line_num,
                    column: 0,
                    message: "Schema compile/parse inside a function body; hoist to module scope when hot.".to_string(),
                    suggestion: Some(
                        "Move compiled schema/decoder to a lazy_static / OnceLock / module const.".to_string(),
                    ),
                    alternatives: Vec::new(),
                    rationale: Some("t3code no-inline-schema-compile.".to_string()),
                    context: file.context_around(line_num, 1),
                    confidence: Some(FindingConfidence::Low),
                    evidence: None,
                });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn flags_schema_decode_in_fn() {
        let d = InlineSchemaCompileDetector::new();
        let f = SourceFile::new(
            PathBuf::from("src/foo.ts"),
            "function run() {\n  return Schema.decode(S)(input);\n}\n".to_string(),
        );
        assert!(!d.detect(&f, None).is_empty());
    }
}
