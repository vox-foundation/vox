//! Developer Fatigue and Cognitive Pacing monitor.
//!
//! Tracks IDE context switches and session length to monitor cognitive load.
//! Generates FatigueEvents when human limits are approached, enabling the Vox Orchestrator
//! to protect the developer by shifting to AI-heavy boilerplate mode or mandating breaks.

use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct FatigueEvent {
    /// Timestamp of the event
    pub timestamp_ms: u64,
    /// Human readable reason (e.g., "Context thrashing detected")
    pub trigger_reason: String,
    /// Mandated rest period returned by the Socrates engine
    pub required_rest_ms: u64,
}

/// Monitors developer cognitive pacing via heuristic session data.
#[derive(Debug, Clone)]
pub struct FatigueMonitor {
    /// Number of distinct file/context switches in a short rolling window
    pub recent_context_switches: u32,
    /// The timestamp of the last captured IDE event
    pub last_interaction_ms: u64,
    /// Length of the current unbroken session
    pub session_start_ms: u64,
}

impl Default for FatigueMonitor {
    fn default() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        Self {
            recent_context_switches: 0,
            last_interaction_ms: now,
            session_start_ms: now,
        }
    }
}

impl FatigueMonitor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records an IDE context switch (e.g. standard file focus change)
    pub fn record_context_switch(&mut self, timestamp_ms: u64) {
        // Simple decay: if gap is > 5 mins, reset the rolling window
        if timestamp_ms.saturating_sub(self.last_interaction_ms) > 300_000 {
            self.recent_context_switches = 0;
        }

        self.recent_context_switches += 1;
        self.last_interaction_ms = timestamp_ms;
    }

    /// Evaluate if the developer has hit a cognitive pacing threshold.
    /// Returns a FatigueEvent if the developer requires intervention.
    pub fn evaluate_fatigue(&self, attention_spent_ratio: f64) -> Option<FatigueEvent> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let session_duration_ms = now.saturating_sub(self.session_start_ms);

        // Thresholds derived from `04_mental_health_and_fatigue_lake.md`
        let hours_worked = session_duration_ms as f64 / 3_600_000.0;

        if attention_spent_ratio >= 1.0 {
            Some(FatigueEvent {
                timestamp_ms: now,
                trigger_reason: "Attention Budget completely exhausted".to_string(),
                required_rest_ms: 15 * 60 * 1000, // 15 mins
            })
        } else if self.recent_context_switches > 25 {
            Some(FatigueEvent {
                timestamp_ms: now,
                trigger_reason: "High cognitive thrashing (Exceeds context switch threshold)"
                    .to_string(),
                required_rest_ms: 5 * 60 * 1000,
            })
        } else if hours_worked > 4.0 {
            Some(FatigueEvent {
                timestamp_ms: now,
                trigger_reason: "Continuous working flow exceeds 4 hours; risk of burnout"
                    .to_string(),
                required_rest_ms: 30 * 60 * 1000,
            })
        } else {
            None
        }
    }
}
