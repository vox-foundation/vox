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

impl FlywheelConfig {
    pub fn load() -> Self {
        let path = std::path::Path::new("mens/config/flywheel.yaml");
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(config) = serde_yaml::from_str(&content) {
                return config;
            }
        }
        Self::default()
    }
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

pub fn evaluate_readiness(corpus_path: &std::path::Path) -> anyhow::Result<FlywheelSignal> {
    use std::io::BufRead;
    use xxhash_rust::xxh3::xxh3_64;
    
    let file = std::fs::File::open(corpus_path)?;
    let reader = std::io::BufReader::new(file);
    let mut count = 0;
    
    // Wave 3-03: Semantic Diversity Matrix
    // Uses AST hashes to ensure data novelty across mutations.
    let mut signatures = std::collections::HashSet::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() { continue; }
        count += 1;
        
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
            if let Some(resp) = v.get("response").and_then(|r| r.as_str()) {
                // If it's valid Vox, hash the AST structure to ignore comments/whitespace
                let tokens = vox_compiler::lexer::lex(resp);
                if let Ok(module) = vox_compiler::parser::parse(tokens) {
                    if let Ok(ser) = serde_json::to_vec(&module) {
                        signatures.insert(xxh3_64(&ser));
                    } else {
                        signatures.insert(xxh3_64(resp.as_bytes()));
                    }
                } else {
                    // Fallback to text hash for non-Vox lanes
                    signatures.insert(xxh3_64(resp.as_bytes()));
                }
            }
        }
    }

    let diversity = if count > 0 {
        signatures.len() as f64 / count as f64
    } else {
        0.0
    };

    let state = FlywheelState::new(FlywheelConfig::load());
    Ok(state.check(count, diversity))
}
