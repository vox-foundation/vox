//! Battery Unit (`1 BU = $0.001`) Pre-Flight Gate & 3-Tier Transactional Cost Ledger (`T3-BATTERY-LEDGER-01..02`).
//!
//! Replaces the legacy unreferenced 27-line `AtomicU64` `UsageTracker` stub with `BatteryAndCostLedger`,
//! adapted from:
//! - `guitaripod/omnichat` (`src/lib/battery-pricing.ts`, `src/lib/usage-tracking.ts`: `1 BU = $0.001 USD`)
//! - `TGlide/thom-chat` & `vibheksoni/t3router` 3-tier cost provenance (`ProviderReportedUsd`, `CatalogRateEstimated`, `FreeTier`)

use crate::models::{CostProvenanceTier, TurnCostDeltaReceipt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Conversion factor: `1` Battery Unit (`BU`) equals `$0.001` USD (`1_000 BU = $1.00 USD`).
pub const USD_PER_BATTERY_UNIT: f64 = 0.001;

/// Convert a USD amount into integer Battery Units (`ceil(usd / 0.001)`).
#[must_use]
pub fn usd_to_battery_units(cost_usd: f64) -> u64 {
    if cost_usd <= 0.0 || !cost_usd.is_finite() {
        return 0;
    }
    (cost_usd / USD_PER_BATTERY_UNIT).ceil() as u64
}

/// Convert Battery Units (`BU`) into USD (`bu * 0.001`).
#[must_use]
pub fn battery_units_to_usd(battery_units: u64) -> f64 {
    (battery_units as f64) * USD_PER_BATTERY_UNIT
}

/// Returns `true` if `provider_or_model` executes locally at zero marginal API cost
/// (`Ollama`, `VoxLocal`, `Mens`, `local`) and bypasses the pre-flight Battery Unit balance gate.
#[must_use]
pub fn is_zero_cost_local_model(provider_or_model: &str) -> bool {
    let lower = provider_or_model.to_ascii_lowercase();
    lower.contains("ollama")
        || lower.contains("voxlocal")
        || lower.contains("vox-local")
        || lower.contains("mens")
        || lower.starts_with("local/")
}

/// Outcome of the pre-flight Battery Unit balance gate (`402 Payment Required` / `InsufficientBalance` equivalent).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PreflightBudgetDecision {
    /// Local / MENS / Ollama zero-cost model; pre-flight balance check bypassed.
    ZeroCostLocalBypass,
    /// Sufficient Battery Units available (`remaining_bu >= estimated_bu`).
    Allowed {
        remaining_bu: u64,
        estimated_bu: u64,
    },
    /// Insufficient Battery Units (`remaining_bu < estimated_bu`).
    InsufficientBalance { remaining_bu: u64, required_bu: u64 },
}

/// Immutable audit record written on Step 3 of the 4-step post-turn usage transaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatteryTransaction {
    /// Turn or request identifier.
    pub turn_id: String,
    /// Model identifier.
    pub model_id: String,
    /// Prompt tokens consumed.
    pub prompt_tokens: u32,
    /// Completion tokens generated.
    pub completion_tokens: u32,
    /// Battery units debited (`ceil(cost_usd / 0.001)`).
    pub debited_bu: u64,
    /// Balance remaining after debit.
    pub balance_after_bu: u64,
    /// 3-tier cost provenance receipt.
    pub cost_receipt: TurnCostDeltaReceipt,
}

/// Aggregated daily usage summary updated on Step 4 of the post-turn usage transaction.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct DailyUsageSummary {
    /// Calendar day key (`YYYY-MM-DD`).
    pub day_key: String,
    /// Total turns recorded on this day.
    pub total_turns: u64,
    /// Total prompt tokens recorded on this day.
    pub prompt_tokens: u64,
    /// Total completion tokens recorded on this day.
    pub completion_tokens: u64,
    /// Total Battery Units consumed on this day.
    pub total_bu: u64,
    /// Total USD cost consumed on this day.
    pub total_cost_usd: f64,
}

/// 4-step transactional Battery Unit and 3-tier cost ledger (`BatteryAndCostLedger`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryAndCostLedger {
    remaining_balance_bu: u64,
    transactions: Vec<BatteryTransaction>,
    daily_summaries: HashMap<String, DailyUsageSummary>,
    pub four_hour_spent_bu: u64,
    pub four_hour_limit_bu: u64,
    pub four_hour_reset_ms: u64,
    pub monthly_spent_bu: u64,
    pub monthly_limit_bu: u64,
    pub total_cache_read_tokens: u64,
}

impl Default for BatteryAndCostLedger {
    fn default() -> Self {
        Self::new(10_000) // Default 10,000 BU ($10.00 USD) local workspace budget
    }
}

impl BatteryAndCostLedger {
    /// Create a ledger initialized with `initial_balance_bu` Battery Units (`1 BU = $0.001`).
    #[must_use]
    pub fn new(initial_balance_bu: u64) -> Self {
        Self {
            remaining_balance_bu: initial_balance_bu,
            transactions: Vec::new(),
            daily_summaries: HashMap::new(),
            four_hour_spent_bu: 0,
            four_hour_limit_bu: 1_000,
            four_hour_reset_ms: 4 * 3_600 * 1_000,
            monthly_spent_bu: 0,
            monthly_limit_bu: initial_balance_bu.max(10_000),
            total_cache_read_tokens: 0,
        }
    }

    /// Current remaining balance in Battery Units (`BU`).
    #[must_use]
    pub fn remaining_balance_bu(&self) -> u64 {
        self.remaining_balance_bu
    }

    /// Total Battery Units spent across all recorded turns.
    #[must_use]
    pub fn total_spent_bu(&self) -> u64 {
        self.transactions.iter().map(|t| t.debited_bu).sum()
    }

    /// Total USD cost across all recorded turns in micro-dollars (`1 USD = 1_000_000`).
    #[must_use]
    pub fn total_spent_usd_micros(&self) -> u64 {
        let usd: f64 = self
            .transactions
            .iter()
            .map(|t| t.cost_receipt.cost_usd_delta)
            .sum();
        (usd * 1_000_000.0).round() as u64
    }

    /// Number of zero-cost local turns recorded.
    #[must_use]
    pub fn local_turns_recorded(&self) -> u64 {
        self.transactions
            .iter()
            .filter(|t| t.cost_receipt.tier == CostProvenanceTier::LocalZeroCost)
            .count() as u64
    }

    /// Rolling 4-hour burst quota utilization percentage (`0.0..=100.0`).
    #[must_use]
    pub fn four_hour_pct(&self) -> f64 {
        if self.four_hour_limit_bu == 0 {
            return 0.0;
        }
        ((self.four_hour_spent_bu as f64 / self.four_hour_limit_bu as f64) * 100.0)
            .clamp(0.0, 100.0)
    }

    /// Monthly quota utilization percentage (`0.0..=100.0`).
    #[must_use]
    pub fn monthly_pct(&self) -> f64 {
        if self.monthly_limit_bu == 0 {
            return 0.0;
        }
        ((self.monthly_spent_bu as f64 / self.monthly_limit_bu as f64) * 100.0).clamp(0.0, 100.0)
    }

    /// Pre-flight balance check before dispatching a turn.
    /// Zero-cost local models (`Ollama`, `VoxLocal`, `Mens`) bypass the check immediately.
    #[must_use]
    pub fn check_preflight_balance(
        &self,
        model_id: &str,
        estimated_cost_usd: f64,
    ) -> PreflightBudgetDecision {
        if is_zero_cost_local_model(model_id) {
            return PreflightBudgetDecision::ZeroCostLocalBypass;
        }
        let estimated_bu = usd_to_battery_units(estimated_cost_usd).max(1);
        if self.remaining_balance_bu < estimated_bu {
            PreflightBudgetDecision::InsufficientBalance {
                remaining_bu: self.remaining_balance_bu,
                required_bu: estimated_bu,
            }
        } else {
            PreflightBudgetDecision::Allowed {
                remaining_bu: self.remaining_balance_bu,
                estimated_bu,
            }
        }
    }

    /// Execute the 4-step post-turn usage reconciliation:
    /// 1. Compute `TurnCostDeltaReceipt` across the 3 provenance tiers.
    /// 2. Decrement `remaining_balance_bu` (saturating at `0` for zero-cost local models).
    /// 3. Append a `BatteryTransaction` audit record.
    /// 4. Upsert the `DailyUsageSummary` for `day_key`.
    pub fn record_turn_usage(
        &mut self,
        turn_id: impl Into<String>,
        day_key: impl Into<String>,
        model_id: &str,
        prompt_tokens: u32,
        completion_tokens: u32,
        provider_reported_cost_usd: Option<f64>,
        catalog_cost_per_1k_input: f64,
        catalog_cost_per_1k_output: f64,
    ) -> BatteryTransaction {
        let is_free = is_zero_cost_local_model(model_id);
        let (tier, cost_usd_delta) = if is_free {
            (CostProvenanceTier::LocalZeroCost, 0.0)
        } else if let Some(reported) = provider_reported_cost_usd.filter(|&c| c > 0.0) {
            (CostProvenanceTier::PlatformCredits, reported)
        } else {
            let est = ((prompt_tokens as f64) / 1_000.0) * catalog_cost_per_1k_input
                + ((completion_tokens as f64) / 1_000.0) * catalog_cost_per_1k_output;
            (CostProvenanceTier::Byok, est)
        };

        let debited_bu = if is_free {
            0
        } else {
            usd_to_battery_units(cost_usd_delta)
        };
        self.remaining_balance_bu = self.remaining_balance_bu.saturating_sub(debited_bu);
        self.four_hour_spent_bu = self.four_hour_spent_bu.saturating_add(debited_bu);
        self.monthly_spent_bu = self.monthly_spent_bu.saturating_add(debited_bu);

        let cost_receipt = TurnCostDeltaReceipt {
            model_id: model_id.to_string(),
            tier,
            cost_usd_delta,
            credits_deducted: Some(debited_bu as f64),
            credits_remaining: Some(self.remaining_balance_bu as f64),
            prompt_tokens,
            completion_tokens,
            cache_read_tokens: 0,
        };

        let tx = BatteryTransaction {
            turn_id: turn_id.into(),
            model_id: model_id.to_string(),
            prompt_tokens,
            completion_tokens,
            debited_bu,
            balance_after_bu: self.remaining_balance_bu,
            cost_receipt: cost_receipt.clone(),
        };
        self.transactions.push(tx.clone());

        let day = day_key.into();
        let summary =
            self.daily_summaries
                .entry(day.clone())
                .or_insert_with(|| DailyUsageSummary {
                    day_key: day,
                    ..Default::default()
                });
        summary.total_turns += 1;
        summary.prompt_tokens += u64::from(prompt_tokens);
        summary.completion_tokens += u64::from(completion_tokens);
        summary.total_bu += debited_bu;
        summary.total_cost_usd += cost_receipt.cost_usd_delta;

        tx
    }

    /// Record prompt-cache read tokens (`total_cache_read_tokens`) for the latest turn.
    pub fn add_cache_read_tokens(&mut self, cache_read_tokens: u32) {
        self.total_cache_read_tokens = self
            .total_cache_read_tokens
            .saturating_add(u64::from(cache_read_tokens));
        if let Some(last) = self.transactions.last_mut() {
            last.cost_receipt.cache_read_tokens = cache_read_tokens;
        }
    }

    /// Immutable view of recorded transactions.
    #[must_use]
    pub fn transactions(&self) -> &[BatteryTransaction] {
        &self.transactions
    }

    /// Lookup a daily usage summary by `YYYY-MM-DD`.
    #[must_use]
    pub fn daily_summary(&self, day_key: &str) -> Option<&DailyUsageSummary> {
        self.daily_summaries.get(day_key)
    }
}

/// Atomic counter facade retained for lock-free hot-path metrics alongside `BatteryAndCostLedger`.
#[derive(Debug, Default)]
pub struct UsageTracker {
    pub input_tokens: AtomicU64,
    pub output_tokens: AtomicU64,
    pub total_cost_usd_micros: AtomicU64,
}

impl UsageTracker {
    pub fn record(&self, input: u32, output: u32, cost_usd: f64) {
        self.input_tokens.fetch_add(input as u64, Ordering::Relaxed);
        self.output_tokens
            .fetch_add(output as u64, Ordering::Relaxed);
        let micros = (cost_usd * 1_000_000.0) as u64;
        self.total_cost_usd_micros
            .fetch_add(micros, Ordering::Relaxed);
    }

    pub fn total_tokens(&self) -> u64 {
        self.input_tokens.load(Ordering::Relaxed) + self.output_tokens.load(Ordering::Relaxed)
    }

    pub fn total_cost_usd(&self) -> f64 {
        self.total_cost_usd_micros.load(Ordering::Relaxed) as f64 / 1_000_000.0
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BatteryAndCostLedger, PreflightBudgetDecision, battery_units_to_usd, usd_to_battery_units,
    };
    use crate::models::CostProvenanceTier;

    #[test]
    fn battery_unit_conversion_and_preflight_gate_bypass_local_models() {
        assert_eq!(usd_to_battery_units(0.001), 1);
        assert_eq!(usd_to_battery_units(0.0025), 3);
        assert!((battery_units_to_usd(10) - 0.01).abs() < 1e-9);

        let mut ledger = BatteryAndCostLedger::new(2);
        // Zero-cost local MENS / Ollama models always bypass pre-flight balance checks
        assert_eq!(
            ledger.check_preflight_balance("mens-qwen-coder-7b", 0.05),
            PreflightBudgetDecision::ZeroCostLocalBypass
        );
        // Paid model requiring 5 BU when balance is 2 BU is rejected
        assert_eq!(
            ledger.check_preflight_balance("anthropic/claude-sonnet-4-5", 0.005),
            PreflightBudgetDecision::InsufficientBalance {
                remaining_bu: 2,
                required_bu: 5,
            }
        );

        // Record a 2 BU ($0.002) turn and verify all 4 ledger steps
        let tx = ledger.record_turn_usage(
            "turn-1",
            "2026-10-25",
            "anthropic/claude-sonnet-4-5",
            500,
            250,
            Some(0.002),
            0.003,
            0.015,
        );
        assert_eq!(tx.cost_receipt.tier, CostProvenanceTier::PlatformCredits);
        assert_eq!(tx.debited_bu, 2);
        assert_eq!(ledger.remaining_balance_bu(), 0);
        let daily = ledger.daily_summary("2026-10-25").expect("daily summary");
        assert_eq!(daily.total_turns, 1);
        assert_eq!(daily.total_bu, 2);
    }
}
