use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BudgetSignal {
    Normal,
    HighLoad { usage_ratio: f64 },
    Critical { usage_ratio: f64 },
    CostExceeded { cost_usd: f64, limit_usd: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionBudget {
    pub max_tokens: u64,
    pub used_tokens: u64,
    pub max_cost_usd: f64,
    pub used_cost_usd: f64,
    pub alert_threshold: f64,
}

impl Default for SessionBudget {
    fn default() -> Self {
        Self {
            max_tokens: 1_000_000,
            used_tokens: 0,
            max_cost_usd: 10.0,
            used_cost_usd: 0.0,
            alert_threshold: 0.8,
        }
    }
}

impl SessionBudget {
    pub fn signal(&self) -> BudgetSignal {
        if self.used_cost_usd >= self.max_cost_usd {
            return BudgetSignal::CostExceeded {
                cost_usd: self.used_cost_usd,
                limit_usd: self.max_cost_usd,
            };
        }
        let ratio = self.used_tokens as f64 / self.max_tokens as f64;
        if ratio >= 0.95 {
            BudgetSignal::Critical { usage_ratio: ratio }
        } else if ratio >= self.alert_threshold {
            BudgetSignal::HighLoad { usage_ratio: ratio }
        } else {
            BudgetSignal::Normal
        }
    }
}
