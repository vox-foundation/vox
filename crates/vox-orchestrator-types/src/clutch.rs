use serde::{Deserialize, Serialize};

use crate::routing_values::CostPreference;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum QualityLevel {
    Flash,
    #[default]
    Balanced,
    Premium,
}

impl QualityLevel {
    /// Map quality level to a cost preference for model selection.
    ///
    /// Free-by-default policy: `Flash` and `Balanced` both resolve to `Economy`
    /// so that the two most common quality tiers prefer free/cheap models.
    /// Only `Premium` opts in to `Performance` (paid-model-preferred) routing.
    #[must_use]
    pub fn to_cost_preference(self) -> CostPreference {
        match self {
            Self::Flash | Self::Balanced => CostPreference::Economy,
            Self::Premium => CostPreference::Performance,
        }
    }
}

/// Budget-gate aggressiveness selected by the clutch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BudgetAggressiveness {
    Aggressive,
    Default,
    Relaxed,
}

impl BudgetAggressiveness {
    /// `(downgrade_fraction, halt_fraction)` for the orchestrator budget gate.
    #[must_use]
    pub fn thresholds(self) -> (f64, f64) {
        match self {
            Self::Aggressive => (0.70, 0.90),
            Self::Default => (0.80, 0.95),
            Self::Relaxed => (0.90, 1.0),
        }
    }
}

/// Resolved control knobs for one clutch detent. Pure data — no I/O.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedClutch {
    pub quality: QualityLevel,
    pub cost_preference: CostPreference,
    pub axes: (u8, u8, u8),
    pub force_free_pool: bool,
    pub always_delegate_free: bool,
    pub delegate_free_when_simple: bool,
    pub budget_gate: BudgetAggressiveness,
}

/// User-facing "how much gas" control. Single SSOT for the four detents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ClutchProfile {
    Free,
    #[default]
    Efficiency,
    Balanced,
    Genius,
}

impl ClutchProfile {
    /// Parse a GUI-supplied label into a profile. Case-insensitive; matches
    /// `lib/driveConsole.ts` `ClutchId` exactly (`free`|`efficiency`|`balanced`|`genius`).
    /// Unknown labels return `None`.
    #[must_use]
    pub fn from_label(label: &str) -> Option<Self> {
        match label.trim().to_ascii_lowercase().as_str() {
            "free" => Some(Self::Free),
            "efficiency" => Some(Self::Efficiency),
            "balanced" => Some(Self::Balanced),
            "genius" => Some(Self::Genius),
            _ => None,
        }
    }

    #[must_use]
    pub fn resolve(self) -> ResolvedClutch {
        match self {
            Self::Free => ResolvedClutch {
                quality: QualityLevel::Flash,
                cost_preference: CostPreference::Economy,
                axes: (70, 15, 15),
                force_free_pool: true,
                always_delegate_free: true,
                delegate_free_when_simple: true,
                budget_gate: BudgetAggressiveness::Aggressive,
            },
            Self::Efficiency => ResolvedClutch {
                quality: QualityLevel::Flash,
                cost_preference: CostPreference::Economy,
                axes: (70, 15, 15),
                force_free_pool: false,
                always_delegate_free: false,
                delegate_free_when_simple: true,
                budget_gate: BudgetAggressiveness::Default,
            },
            Self::Balanced => ResolvedClutch {
                quality: QualityLevel::Balanced,
                cost_preference: CostPreference::Economy,
                axes: (33, 33, 34),
                force_free_pool: false,
                always_delegate_free: false,
                delegate_free_when_simple: false,
                budget_gate: BudgetAggressiveness::Default,
            },
            Self::Genius => ResolvedClutch {
                quality: QualityLevel::Premium,
                cost_preference: CostPreference::Performance,
                axes: (15, 15, 70),
                force_free_pool: false,
                always_delegate_free: false,
                delegate_free_when_simple: false,
                budget_gate: BudgetAggressiveness::Relaxed,
            },
        }
    }
}
