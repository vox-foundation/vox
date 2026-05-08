use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::rules::Severity;
use crate::review::types::{ReviewCategory, ReviewFinding};

/// Represents a GitHub pull request comment (review or issue).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubComment {
    pub id: u64,
    pub body: String,
    pub path: Option<String>,
    pub line: Option<usize>,
    pub user: GithubUser,
    pub html_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubUser {
    pub login: String,
}

/// Ingests CodeRabbit (or any GitHub) comments and converts them to [`ReviewFinding`]s.
pub fn ingest_coderabbit_comments(comments: &[GithubComment]) -> Vec<ReviewFinding> {
    comments
        .iter()
        .filter(|c| is_coderabbit_nitpick(c))
        .map(|c| {
            let category = infer_category_from_body(&c.body);
            ReviewFinding {
                category,
                severity: Severity::Warning, // CodeRabbit nitpicks are usually warnings
                file: PathBuf::from(c.path.as_deref().unwrap_or(".")),
                line: c.line.unwrap_or(0),
                message: clean_comment_body(&c.body),
                suggestion: extract_suggestion(&c.body),
                confidence: 90, // AI review is usually high confidence but advisory
            }
        })
        .collect()
}

fn is_coderabbit_nitpick(c: &GithubComment) -> bool {
    c.user.login.contains("coderabbit") || c.body.contains("CodeRabbit")
}

fn infer_category_from_body(body: &str) -> ReviewCategory {
    let lower = body.to_lowercase();
    if lower.contains("security") || lower.contains("vuln") {
        ReviewCategory::Security
    } else if lower.contains("performance") || lower.contains("slow") {
        ReviewCategory::Performance
    } else if lower.contains("style") || lower.contains("naming") {
        ReviewCategory::Style
    } else if lower.contains("logic") || lower.contains("bug") {
        ReviewCategory::Logic
    } else {
        ReviewCategory::Style // Default for nitpicks
    }
}

fn clean_comment_body(body: &str) -> String {
    // Strip CodeRabbit branding/noise if present
    body.lines()
        .filter(|line| !line.contains("CodeRabbit") && !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn extract_suggestion(body: &str) -> Option<String> {
    // Look for fence blocks (```)
    if let Some(start) = body.find("```") {
        let rest = &body[start + 3..];
        if let Some(end) = rest.find("```") {
            let content = &rest[..end];
            // Skip the language identifier if present (e.g. ```rust\n)
            if let Some(nl) = content.find('\n') {
                return Some(content[nl + 1..].to_string());
            }
            return Some(content.to_string());
        }
    }
    None
}
