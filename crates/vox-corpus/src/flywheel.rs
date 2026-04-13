use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlywheelConfig {
    /// Minimum new dogfood records before triggering a corpus refresh.
    pub sample_floor: usize,
    /// Must exceed this diversity score before triggering a training run.
    pub min_ast_diversity: f64,
    /// Maximum hours between forced check-ins.
    pub max_interval_hours: u64,
    /// Enable automatic training trigger (vs. emit signal only).
    pub auto_train: bool,
}

impl Default for FlywheelConfig {
    fn default() -> Self {
        Self {
            sample_floor: 500,
            min_ast_diversity: 0.40,
            max_interval_hours: 168,
            auto_train: false,
        }
    }
}

pub enum FlywheelSignal {
    Pending { new_samples: usize },
    Ready { ast_diversity: f64 },
    Triggered,
    Idle,
}

pub struct FlywheelState {
    pub config: FlywheelConfig,
    pub last_run_at_ms: i64,
    pub accumulated_samples: usize,
}

impl FlywheelState {
    pub fn new(config: FlywheelConfig) -> Self {
        Self {
            config,
            last_run_at_ms: 0,
            accumulated_samples: 0,
        }
    }

    pub fn check(&self, current_samples: usize, current_diversity: f64) -> FlywheelSignal {
        if current_samples < self.config.sample_floor {
            return FlywheelSignal::Pending {
                new_samples: current_samples,
            };
        }

        if current_diversity < self.config.min_ast_diversity {
            return FlywheelSignal::Idle; // Diversity gate failed
        }

        FlywheelSignal::Ready {
            ast_diversity: current_diversity,
        }
    }
}
