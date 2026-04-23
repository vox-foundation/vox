//! Complexity tracking for Socrates

use crate::policy_types::ComplexityBand;
use serde::{Deserialize, Serialize};

/// Socrates-driven complexity evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocratesComplexityJudge;

impl SocratesComplexityJudge {
    /// Classify the query into a discrete routing band.
    pub fn estimate_complexity(goal: &str, router_hint: Option<u8>) -> ComplexityBand {
        let score = router_hint.unwrap_or(5).clamp(1, 10);

        let is_multi_hop =
            goal.contains("compare") || goal.contains("synthesize") || goal.contains("across");

        if score >= 8 || is_multi_hop {
            ComplexityBand::MultiHop
        } else if score >= 6 {
            ComplexityBand::Complex
        } else if score >= 4 {
            ComplexityBand::Moderate
        } else {
            ComplexityBand::Simple
        }
    }
}
