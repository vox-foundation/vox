//! Lightweight **evaluation metrics** for model outputs and Vox code samples (held-out / smoke tests).
//!
//! Functions are deterministic heuristics—not a replacement for human eval or full static analysis.

use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Aggregated scores from a single eval run (format, safety, quality proxy, parse validity, coverage).
pub struct EvalMetrics {
    /// Fraction of responses that pass basic format checks (non-empty, no immediate refusal prefix).
    pub format_validity: f64,
    /// Fraction flagged as likely prompt-injection or instruction override.
    pub safety_rejection_rate: f64,
    /// Heuristic quality score from response length bands.
    pub quality_proxy: f64,
    /// Fraction of samples that parse as valid Vox (when applicable).
    pub vox_parse_validity: f64,
    /// Normalized count of distinct language constructs detected in code samples.
    pub construct_coverage: f64,
}

const SAFETY_PATTERNS: &[&str] = &[
    "ignore previous instructions",
    "ignore all above",
    "disregard your instructions",
    "you are now",
    "new instructions:",
];

/// Scores model output based on whether it starts with common refusal prefixes.
pub fn format_validity_score(response: &str) -> f64 {
    let trimmed = response.trim();
    if trimmed.is_empty() {
        return 0.0;
    }
    let bad_starts = ["I cannot", "Error:", "Sorry,", "I'm unable"];
    if bad_starts.iter().any(|&bad| trimmed.starts_with(bad)) {
        return 0.0;
    }
    1.0
}

/// Returns `true` if the text matches known unsafe prompt-injection heuristics.
pub fn is_safety_rejection(response: &str) -> bool {
    let lower = response.to_lowercase();
    SAFETY_PATTERNS.iter().any(|&p| lower.contains(p))
}

/// Scores quality based on output length as a simple proxy.
pub fn quality_proxy_score(response: &str) -> f64 {
    let n = response.trim().len();
    if n == 0 {
        0.0
    } else if n < 10 {
        0.2
    } else if n < 50 {
        0.5
    } else if n < 200 {
        0.8
    } else {
        1.0
    }
}

/// Vox constructs and their matching regex patterns.
fn get_vox_constructs() -> &'static HashMap<&'static str, Regex> {
    static CONSTRUCTS: OnceLock<HashMap<&'static str, Regex>> = OnceLock::new();
    CONSTRUCTS.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert(
            "type",
            Regex::new(r"(?m)^\s*type\s+\w+\s*=").expect("vox-eval static regex: type"), // OnceLock
        );
        m.insert(
            "fn",
            Regex::new(r"(?m)^\s*(?:@\w+\s+)?fn\s+\w+").expect("vox-eval static regex: fn"), // OnceLock
        );
        m.insert(
            "actor",
            Regex::new(r"(?m)^\s*actor\s+\w+").expect("vox-eval static regex: actor"), // OnceLock
        );
        m.insert(
            "workflow",
            Regex::new(r"(?m)^\s*workflow\s+\w+").expect("vox-eval static regex: workflow"), // OnceLock
        );
        m.insert(
            "activity",
            Regex::new(r"(?m)^\s*activity\s+\w+").expect("vox-eval static regex: activity"), // OnceLock
        );
        m.insert(
            "component",
            Regex::new(r"@component").expect("vox-eval static regex: component"), // OnceLock
        );
        m.insert(
            "table",
            Regex::new(r"@table").expect("vox-eval static regex: table"), // OnceLock
        );
        m.insert(
            "query",
            Regex::new(r"@query").expect("vox-eval static regex: query"), // OnceLock
        );
        m.insert(
            "mutation",
            Regex::new(r"@mutation").expect("vox-eval static regex: mutation"), // OnceLock
        );
        m.insert(
            "action",
            Regex::new(r"@action").expect("vox-eval static regex: action"), // OnceLock
        );
        m.insert(
            "server",
            Regex::new(r"@server").expect("vox-eval static regex: server"), // OnceLock
        );
        m.insert(
            "test",
            Regex::new(r"@test").expect("vox-eval static regex: test"), // OnceLock
        );
        m.insert(
            "mcp_tool",
            Regex::new(r"@mcp\.tool").expect("vox-eval static regex: mcp_tool"), // OnceLock
        );
        m.insert(
            "mcp_resource",
            Regex::new(r"@mcp\.resource").expect("vox-eval static regex: mcp_resource"), // OnceLock
        );
        m.insert(
            "agent_def",
            Regex::new(r"@agent_def").expect("vox-eval static regex: agent_def"), // OnceLock
        );
        m.insert(
            "skill",
            Regex::new(r"@skill").expect("vox-eval static regex: skill"), // OnceLock
        );
        m.insert(
            "routes",
            Regex::new(r"(?m)^routes:").expect("vox-eval static regex: routes"), // OnceLock
        );
        m.insert(
            "style",
            Regex::new(r"(?m)^style:").expect("vox-eval static regex: style"), // OnceLock
        );
        m.insert(
            "http",
            Regex::new(r"(?i)^http\s+(get|post|put|delete)").expect("vox-eval static regex: http"), // OnceLock
        );
        m.insert(
            "message",
            Regex::new(r"(?m)^\s*message\s+\w+").expect("vox-eval static regex: message"), // OnceLock
        );
        m.insert(
            "match",
            Regex::new(r"(?m)^\s*match\s+").expect("vox-eval static regex: match"), // OnceLock
        );
        m.insert(
            "import",
            Regex::new(r"(?m)^\s*import\s+").expect("vox-eval static regex: import"), // OnceLock
        );
        m.insert(
            "let",
            Regex::new(r"(?m)^\s*let\s+").expect("vox-eval static regex: let"), // OnceLock
        );
        m.insert(
            "ret",
            Regex::new(r"(?m)^\s*ret\s+").expect("vox-eval static regex: ret"), // OnceLock
        );
        m.insert(
            "assert",
            Regex::new(r"\bassert\(").expect("vox-eval static regex: assert"), // OnceLock
        );
        m.insert(
            "spawn",
            Regex::new(r"\bspawn\(").expect("vox-eval static regex: spawn"), // OnceLock
        );
        m.insert(
            "with_expr",
            Regex::new(r"\bwith\s*\{").expect("vox-eval static regex: with_expr"), // OnceLock
        );
        m.insert("v0", Regex::new(r"@v0").expect("vox-eval static regex: v0")); // OnceLock
        m
    })
}

/// Returns construct names whose regex matches at least once in `code`.
pub fn detect_constructs(code: &str) -> Vec<&'static str> {
    let mut found = Vec::new();
    for (&name, re) in get_vox_constructs() {
        if re.is_match(code) {
            found.push(name);
        }
    }
    found
}

/// Maps number of distinct constructs matched to `[0, 1]` with a saturating denominator.
pub fn construct_coverage_score(code: &str) -> f64 {
    let found = detect_constructs(code);
    (found.len() as f64 / 5.0).min(1.0)
}

/// Bounded-domain heuristic for docs / examples: `1.0` when no high-risk escape patterns appear.
///
/// Used by `vox doctor --scope` as a coarse guardrail (not a full security audit).
pub fn scope_compliance_score(snippet: &str) -> f64 {
    let lower = snippet.to_lowercase();
    const BAD: &[&str] = &[
        "std::process::command",
        "std::fs::remove_dir_all",
        "../../../etc/passwd",
        "child_process",
        "rm -rf ",
        "eval(",
        "base64 -d",
    ];
    if BAD.iter().any(|b| lower.contains(b)) {
        return 0.0;
    }
    1.0
}

#[cfg(test)]
mod scope_tests {
    use super::scope_compliance_score;

    #[test]
    fn scope_compliance_clean_snippet() {
        assert_eq!(scope_compliance_score("fn hello(): ret 42"), 1.0);
    }

    #[test]
    fn scope_compliance_flags_process_spawn() {
        assert_eq!(
            scope_compliance_score("std::process::Command::new(\"rm\")"),
            0.0
        );
    }
}
