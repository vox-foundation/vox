use serde::{Deserialize, Serialize};

/// Category of the issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CritiqueCategory {
    Safety,
    Architecture,
    Completeness,
    Complexity,
    VoxCompliance,
    Dependencies,
}

/// A single structured finding from the reviewer LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CritiqueNote {
    /// Which step_id or section does this note concern?
    /// Use None for global/cross-cutting concerns.
    pub target_step_id: Option<usize>,
    /// Category of the issue.
    pub category: CritiqueCategory,
    /// Severity: "blocker", "major", "minor".
    pub severity: String,
    /// Concise description of the problem (1-3 sentences).
    pub problem: String,
    /// Specific, actionable suggestion for fixing it.
    pub suggestion: String,
}

/// The full output of a reviewer LLM call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewResult {
    /// Overall assessment: "approved", "needs_revision", "rejected".
    pub verdict: String,
    /// A short human-readable summary of the review.
    pub summary: String,
    /// Structured findings. Empty means no issues found.
    pub notes: Vec<CritiqueNote>,
    /// Confidence score 0.0–1.0 for the plan as submitted.
    pub confidence: f32,
}

pub fn parse_review_result(raw: &str) -> Result<ReviewResult, serde_json::Error> {
    // Strip fences
    let trimmed = raw.trim();
    let json_text = if trimmed.starts_with("```json") {
        trimmed.strip_prefix("```json").unwrap_or(trimmed).strip_suffix("```").unwrap_or(trimmed).trim()
    } else {
        trimmed
    };
    serde_json::from_str(json_text)
}
