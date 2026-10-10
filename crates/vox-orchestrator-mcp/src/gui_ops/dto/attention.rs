//! Attention / settings wire DTOs (plan 07 Task 9). Field names are part of the Axis UI
//! contract — do NOT rename.
use serde::{Deserialize, Serialize};

/// Recorded LLM spend (actuals) vs the configured budget caps, for the Runtime settings
/// surface. Spend comes from the single SSOT aggregate (`VoxDb::llm_spend_summary`); the
/// caps come from the server's `VoxConfig` — same sources the rest of the system routes and
/// charges on.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmSpendDto {
    pub session_usd: f64,
    pub day_usd: f64,
    pub total_usd: f64,
    pub daily_budget_usd: f64,
    pub per_session_budget_usd: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spend_dto_round_trips_camel_case() {
        let d = LlmSpendDto {
            session_usd: 0.03,
            day_usd: 0.1,
            total_usd: 1.2,
            daily_budget_usd: 5.0,
            per_session_budget_usd: 1.0,
        };
        let j = serde_json::to_string(&d).expect("serialize");
        assert!(j.contains("\"sessionUsd\":0.03"), "{j}");
        assert!(j.contains("\"dailyBudgetUsd\":5.0"), "{j}");
        assert_eq!(serde_json::from_str::<LlmSpendDto>(&j).unwrap(), d);
    }
}
