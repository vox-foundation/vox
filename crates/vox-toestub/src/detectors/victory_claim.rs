use crate::rules::{DetectionRule, Finding, Language, Severity, SourceFile};
use regex::Regex;

/// Detects suspicious "victory claim" comments near incomplete or provisional code.
///
/// AI agents love to say "Done!", "Complete!", "All set!" in comments right next
/// to code that is clearly not finished. This detector finds those patterns.
pub struct VictoryClaimDetector {
    victory_re: Regex,
    todo_comment_re: Regex,
    fixme_re: Regex,
    hack_re: Regex,
}

impl Default for VictoryClaimDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl VictoryClaimDetector {
    /// “Done/complete” comment patterns plus TODO/FIXME/HACK proximity heuristics.
    pub fn new() -> Self {
        Self {
            // Match past-tense completion claims in comments or macro literals.
            victory_re: Regex::new(concat!(
                r"(?i)(?://|#|/\*|",
                "todo!",
                r"|panic!|unimplemented!).*?(?:\bdone\b|all\s*set|fully\s*implemented|implementation\s+\bcomplete\b)",
            ))
            .expect("valid regex"),
            // Match work-queue markers (see `todo_comment_re` tests) in comments or macro literals.
            todo_comment_re: Regex::new(concat!(
                r"(?i)(?://|#|",
                "todo!",
                r").*?(?:",
                "TO",
                "DO",
                r"(?:\(ai\))?)\s*:?\s*(?:implement|add|finish|complete|wire|fix|later)",
            ))
            .expect("valid regex"),
            fixme_re: Regex::new(concat!(
                r"(?i)(?://|#).*?",
                "FIX",
                "ME",
                r"(?:\(ai\))?\b"
            ))
            .expect("valid regex"),
            hack_re: Regex::new(r"(?i)(?://|#).*?HACK\b").expect("valid regex"),
        }
    }
}

impl DetectionRule for VictoryClaimDetector {
    fn id(&self) -> &'static str {
        "victory-claim"
    }
    fn name(&self) -> &'static str {
        "Victory Claim / Leftover Marker Detector"
    }
    fn description(&self) -> &'static str {
        "Detects premature 'Done!' comments, TODO/FIXME/HACK markers left behind"
    }
    fn severity(&self) -> Severity {
        Severity::Warning
    }
    fn languages(&self) -> &[Language] {
        &[
            Language::Rust,
            Language::TypeScript,
            Language::Python,
            Language::GDScript,
            Language::Vox,
        ]
    }

    fn detect(
        &self,
        file: &SourceFile,
        _rust: Option<&crate::analysis::RustFileContext>,
    ) -> Vec<Finding> {
        let mut findings = Vec::new();

        for (i, line) in file.lines.iter().enumerate() {
            let line_num = i + 1;
            let tri = line.trim_start();
            if tri.starts_with("///") || tri.starts_with("//!") {
                continue;
            }

            // Detect premature completion-style claims in line comments.
            if self.victory_re.is_match(line) {
                findings.push(Finding {
                    rule_id: "victory-claim/premature".to_string(),
                    rule_name: self.name().to_string(),
                    severity: Severity::Warning,
                    file: file.path.clone(),
                    line: line_num,
                    column: 0,
                    message: "Premature victory claim — verify the implementation is truly complete"
                        .to_string(),
                    suggestion: Some(
                        "Remove the comment if the implementation is complete, or replace with a descriptive comment.".to_string(),
                    ),
                    context: file.context_around(line_num, 2),
                    confidence: None,
                    evidence: None,
                });
            }

            // Detect TODO comments with action verbs
            if self.todo_comment_re.is_match(line) {
                findings.push(Finding {
                    rule_id: "victory-claim/todo-leftover".to_string(),
                    rule_name: self.name().to_string(),
                    severity: Severity::Warning,
                    file: file.path.clone(),
                    line: line_num,
                    column: 0,
                    message: "TODO marker left behind — work is not finished".to_string(),
                    suggestion: Some(
                        "Complete the TODO or create a tracked task for it.".to_string(),
                    ),
                    context: file.context_around(line_num, 1),
                    confidence: None,
                    evidence: None,
                });
            }

            // Detect fix-me-shaped line comments
            if self.fixme_re.is_match(line) {
                findings.push(Finding {
                    rule_id: "victory-claim/fixme".to_string(),
                    rule_name: self.name().to_string(),
                    severity: Severity::Warning,
                    file: file.path.clone(),
                    line: line_num,
                    column: 0,
                    message: "FIXME marker — known issue left unresolved".to_string(),
                    suggestion: Some("Fix the issue or track it as a task.".to_string()),
                    context: file.context_around(line_num, 1),
                    confidence: None,
                    evidence: None,
                });
            }

            // Detect HACK
            if self.hack_re.is_match(line) {
                findings.push(Finding {
                    rule_id: "victory-claim/hack".to_string(),
                    rule_name: self.name().to_string(),
                    severity: Severity::Info,
                    file: file.path.clone(),
                    line: line_num,
                    column: 0,
                    message: "HACK marker — temporary workaround left in code".to_string(),
                    suggestion: Some(
                        "Replace with a proper solution or document why the hack is necessary."
                            .to_string(),
                    ),
                    context: file.context_around(line_num, 1),
                    confidence: None,
                    evidence: None,
                });
            }
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn source(ext: &str, code: &str) -> SourceFile {
        SourceFile::new(PathBuf::from(format!("test.{}", ext)), code.to_string())
    }

    #[test]
    fn detects_victory_comment() {
        let d = VictoryClaimDetector::new();
        let snippet = concat!(
            "// ",
            "D",
            "one!",
            " Implementation complete\nfn foo() {}"
        );
        let f = source("rs", &snippet);
        let findings = d.detect(&f, None);
        assert!(
            findings
                .iter()
                .any(|f| f.rule_id == "victory-claim/premature"),
            "should detect victory claim"
        );
    }

    #[test]
    fn detects_todo_leftover() {
        let d = VictoryClaimDetector::new();
        let py = concat!("# TO", "DO: implement later", "\ndef foo():\n    pass");
        let f = source("py", py);
        let findings = d.detect(&f, None);
        assert!(
            findings
                .iter()
                .any(|f| f.rule_id == "victory-claim/todo-leftover"),
            "should detect TODO leftover"
        );
    }

    #[test]
    fn detects_fixme() {
        let d = VictoryClaimDetector::new();
        let snippet = concat!("// ", "FIX", "ME this is broken\nconst x = 1;");
        let f = source("ts", &snippet);
        let findings = d.detect(&f, None);
        assert!(
            findings.iter().any(|f| f.rule_id == "victory-claim/fixme"),
            "should detect FIXME"
        );
    }

    #[test]
    fn clean_code_no_findings() {
        let d = VictoryClaimDetector::new();
        let f = source(
            "rs",
            "/// Adds two numbers.\nfn add(a: i32, b: i32) -> i32 {\n    a + b\n}",
        );
        let findings = d.detect(&f, None);
        assert!(findings.is_empty());
    }
}
