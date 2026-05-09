//! Compaction strategy selector based on context utilization (D7).
//!
//! Maps context utilization to one of the three existing [`CompactionStrategy`]
//! variants without duplicating compaction logic.
//! All logic is pure: no async, no I/O.

use serde::{Deserialize, Serialize};

use crate::compaction::CompactionStrategy;

/// Thresholds for compaction strategy selection. Defaults mirror contract defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionTriggerConfig {
    /// Utilization < this → Conservative (keep most context).
    pub conservative_below: f64,
    /// Utilization < this → Balanced (trim moderately).
    pub balanced_below: f64,
    // utilization >= balanced_below → Aggressive (trim hard)
}

impl Default for CompactionTriggerConfig {
    fn default() -> Self {
        Self {
            conservative_below: 0.60,
            balanced_below: 0.85,
        }
    }
}

/// Pure compaction strategy trigger.
pub struct CompactionTrigger {
    config: CompactionTriggerConfig,
}

impl CompactionTrigger {
    pub fn new(config: CompactionTriggerConfig) -> Self {
        Self { config }
    }

    /// Select a [`CompactionStrategy`] for the given context utilization ratio `[0, 1]`.
    ///
    /// - utilization < `conservative_below` → [`CompactionStrategy::Conservative`]
    /// - utilization < `balanced_below` → [`CompactionStrategy::Balanced`]
    /// - utilization ≥ `balanced_below` → [`CompactionStrategy::Aggressive`]
    #[must_use]
    #[inline]
    pub fn select(&self, utilization: f64) -> CompactionStrategy {
        let u = utilization.clamp(0.0, 1.0);
        if u < self.config.conservative_below {
            CompactionStrategy::Conservative
        } else if u < self.config.balanced_below {
            CompactionStrategy::Balanced
        } else {
            CompactionStrategy::Aggressive
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trigger() -> CompactionTrigger {
        CompactionTrigger::new(CompactionTriggerConfig::default())
    }

    #[test]
    fn low_utilization_is_conservative() {
        let t = trigger();
        assert_eq!(t.select(0.30), CompactionStrategy::Conservative);
    }

    #[test]
    fn at_conservative_boundary_is_balanced() {
        let t = trigger();
        assert_eq!(t.select(0.60), CompactionStrategy::Balanced);
    }

    #[test]
    fn mid_utilization_is_balanced() {
        let t = trigger();
        assert_eq!(t.select(0.70), CompactionStrategy::Balanced);
    }

    #[test]
    fn at_balanced_boundary_is_aggressive() {
        let t = trigger();
        assert_eq!(t.select(0.85), CompactionStrategy::Aggressive);
    }

    #[test]
    fn full_utilization_is_aggressive() {
        let t = trigger();
        assert_eq!(t.select(1.0), CompactionStrategy::Aggressive);
    }

    #[test]
    fn out_of_range_clamped() {
        let t = trigger();
        assert_eq!(t.select(-0.5), CompactionStrategy::Conservative);
        assert_eq!(t.select(1.5), CompactionStrategy::Aggressive);
    }
}
