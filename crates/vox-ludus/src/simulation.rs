//! Monte Carlo simulation suite for bug battles.
//!
//! Enables automated balance validation by running thousands of simulated
//! battles across all bug types and archetypes. Funnels results into
//! persistent artifacts (JSONL/Markdown) per Zero-STDOUT architecture.

use crate::battle::BugType;
use crate::combat::{CombatState, CombatResult, BugEnemy};
use crate::ability::{Archetype, default_abilities};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Result metrics for a single simulation batch.
#[derive(Debug, Serialize, Deserialize)]
pub struct SimulationReport {
    /// Number of battles simulated.
    pub iterations: u32,
    /// Overall win percentage (0.0 - 1.0).
    pub win_rate: f64,
    /// Average turns per battle.
    pub avg_turns: f64,
    /// Detailed breakdown by bug type.
    pub results_by_type: HashMap<String, TypeStats>,
}

/// Statistics for a specific bug type.
#[derive(Debug, Serialize, Deserialize)]
pub struct TypeStats {
    /// Total wins against this type.
    pub wins: u32,
    /// Total losses against this type.
    pub losses: u32,
    /// Total battles against this type.
    pub total: u32,
    /// Average turns to resolve.
    pub avg_turns: f64,
}

/// Run a Monte Carlo battle sweep to validate combat balance.
///
/// This does NOT touch the database; it runs purely in-memory simulations
/// using the `CombatState` engine.
pub fn run_monte_carlo_battle_sweep(iterations: u32) -> SimulationReport {
    let mut total_wins = 0;
    let mut total_turns = 0;
    let mut type_stats: HashMap<BugType, (u32, u32, u32)> = HashMap::new();

    let bug_types = [
        BugType::Syntax,
        BugType::Logic,
        BugType::Performance,
        BugType::Security,
    ];
    
    // We simulate using a Centurion with all abilities unlocked to test "peak" balance.
    let mut abilities = default_abilities(Archetype::Centurion);
    for a in &mut abilities {
        a.unlocked = true;
    }

    for i in 0..iterations {
        let bug_type = bug_types[(i as usize) % bug_types.len()];
        // Create a bug enemy for the simulation.
        let enemy = BugEnemy::new(
            bug_type,
            "A synthetic bug for Monte Carlo sweep."
        );

        let mut state = CombatState::new(
            format!("sim-{}", i),
            enemy,
            100, 100, // companion hp
            100, 100, // companion energy
            abilities.clone(),
        );

        // Simple Heuristic AI:
        // 1. If health < 30% and heal is available, heal.
        // 2. Use the most expensive available damage ability.
        // 3. Otherwise, use basic strike.
        while state.result == CombatResult::Ongoing && state.turn < 100 {
            let hp_pct = state.companion_hp as f64 / state.companion_max_hp as f64;
            
            let action = if hp_pct < 0.3 {
                state.abilities.iter()
                    .find(|a| a.damage < 0 && state.companion_energy >= a.energy_cost && state.cooldowns.get(&a.id).copied().unwrap_or(0) == 0)
                    .map(|a| a.id.clone())
            } else {
                state.abilities.iter()
                    .filter(|a| a.damage > 0 && state.companion_energy >= a.energy_cost && state.cooldowns.get(&a.id).copied().unwrap_or(0) == 0)
                    .max_by_key(|a| a.damage)
                    .map(|a| a.id.clone())
            };

            if let Some(id) = action {
                let _ = state.use_ability(&id);
            } else {
                // Skip turn to recharge energy and tick cooldowns
                state.skip_turn();
            }
        }

        let entry = type_stats.entry(bug_type).or_insert((0, 0, 0));
        if state.result == CombatResult::Victory {
            total_wins += 1;
            entry.0 += 1;
        } else {
            entry.1 += 1;
        }
        total_turns += state.turn;
        entry.2 += state.turn;
    }

    let mut results_by_type = HashMap::new();
    for (t, (wins, losses, turns)) in type_stats {
        let total = wins + losses;
        results_by_type.insert(t.as_str().to_string(), TypeStats {
            wins,
            losses,
            total,
            avg_turns: turns as f64 / total as f64,
        });
    }

    SimulationReport {
        iterations,
        win_rate: total_wins as f64 / iterations as f64,
        avg_turns: total_turns as f64 / iterations as f64,
        results_by_type,
    }
}
