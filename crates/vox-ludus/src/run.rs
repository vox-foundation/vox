//! Orchestration API for Ludus commands — battle, companion interact, etc.
//!
//! Callers (e.g. vox-cli) parse args and call these functions; they return
//! outcome structs for the CLI to format and display.

use anyhow::Result;
use std::path::PathBuf;
use vox_db::Codex;

use crate::battle::Battle;
use crate::companion::Companion;
use crate::db;
use crate::profile::LudusProfile;
use crate::quest;

/// Minimal finding info for starting a battle (from TOESTUB or similar).
#[derive(Debug, Clone)]
pub struct BattleFinding {
    /// Rule identifier, e.g. `"stub/todo"`.
    pub rule_id: String,
    /// Human-readable description.
    pub message: String,
    /// File where the finding was detected.
    pub file_path: PathBuf,
    /// 1-indexed line number.
    pub line: usize,
    /// Optional code context.
    pub context: Option<String>,
}

/// Outcome of starting a battle.
#[derive(Debug)]
pub struct BattleStartOutcome {
    /// The created battle.
    pub battle: Battle,
    /// The companion used.
    pub companion: Companion,
    /// Companion name for display.
    pub companion_name: String,
}

/// Outcome of submitting a battle.
#[derive(Debug)]
pub struct BattleSubmitOutcome {
    /// Whether the fix was accepted.
    pub success: bool,
    /// The battle after update.
    pub battle: Battle,
    /// The companion after update.
    pub companion: Companion,
    /// The profile after update.
    pub profile: LudusProfile,
    /// Whether the user leveled up.
    pub leveled_up: bool,
    /// Description of a completed quest, if any.
    pub quest_completed: Option<String>,
}

/// Result of attempting to submit a battle.
#[derive(Debug)]
pub enum BattleSubmitResult {
    /// Companion has no battle energy.
    Tired,
    /// No companion or battle found.
    NotFound,
    /// Submission processed.
    Outcome(Box<BattleSubmitOutcome>),
}

/// Start a bug battle from a finding. Caller (CLI) runs TOESTUB scan and passes the first finding.
pub async fn run_battle_start(
    db: &Codex,
    user_id: &str,
    companion_name: &str,
    finding: &BattleFinding,
) -> Result<Option<BattleStartOutcome>> {
    let companions = db::list_companions(db, user_id).await?;
    let companion = match companions.into_iter().find(|c| c.name == companion_name) {
        Some(c) => c,
        None => return Ok(None),
    };

    let battle_id = format!(
        "battle-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );

    let battle = Battle::from_finding(
        battle_id,
        user_id,
        &companion.id,
        &finding.rule_id,
        finding.message.clone(),
        finding.context.clone(),
    );

    db::insert_battle(db, &battle).await?;

    Ok(Some(BattleStartOutcome {
        battle,
        companion: companion.clone(),
        companion_name: companion_name.to_string(),
    }))
}

/// Submit code to resolve a battle. Caller validates the code and passes `is_success`.
pub async fn run_battle_submit(
    db: &Codex,
    user_id: &str,
    companion_name: &str,
    code: String,
    is_success: bool,
) -> Result<BattleSubmitResult> {
    let mut profile = match db::get_profile(db, user_id).await? {
        Some(p) => p,
        None => LudusProfile::new_default(user_id),
    };

    let companions = db::list_companions(db, user_id).await?;
    let mut companion = match companions.into_iter().find(|c| c.name == companion_name) {
        Some(c) => c,
        None => return Ok(BattleSubmitResult::NotFound),
    };

    let battles = db::list_battles(db, user_id, 20).await?;
    let mut battle = match battles
        .into_iter()
        .find(|b| b.companion_id == companion.id && !b.success)
    {
        Some(b) => b,
        None => return Ok(BattleSubmitResult::NotFound),
    };

    if !companion.spend_battle_energy() {
        return Ok(BattleSubmitResult::Tired);
    }

    let mut quest_completed: Option<String> = None;

    if is_success {
        battle.record_result(true, 60);
        battle.submitted_code = Some(code);

        let leveled_up = profile.add_xp(battle.xp_earned);
        profile.add_crystals(battle.crystals_earned);

        let mut quests = db::list_quests(db, user_id).await?;
        for q in &mut quests {
            if q.quest_type == quest::QuestType::Battle && q.increment(1) {
                quest_completed = Some(q.description.clone());
                profile.add_xp(q.xp_reward);
                profile.add_crystals(q.crystal_reward);
            }
        }

        db::upsert_profile(db, &profile).await?;
        db::upsert_companion(db, &companion).await?;
        db::update_battle(db, &battle).await?;

        for q in &quests {
            db::upsert_quest(db, q).await?;
        }

        Ok(BattleSubmitResult::Outcome(Box::new(BattleSubmitOutcome {
            success: true,
            battle,
            companion,
            profile,
            leveled_up,
            quest_completed,
        })))
    } else {
        db::upsert_companion(db, &companion).await?;
        Ok(BattleSubmitResult::Outcome(Box::new(BattleSubmitOutcome {
            success: false,
            battle,
            companion,
            profile,
            leveled_up: false,
            quest_completed: None,
        })))
    }
}

/// Run a Monte Carlo battle simulation sweep and write results to file.
pub fn run_monte_carlo_battle_sweep(
    iterations: u32,
    output_dir: PathBuf,
) -> Result<crate::simulation::SimulationReport> {
    let report = crate::simulation::run_monte_carlo_battle_sweep(iterations);

    // Ensure output directory exists
    if !output_dir.exists() {
        std::fs::create_dir_all(&output_dir)?;
    }

    // Write JSONL (atomic artifact)
    let jsonl_path = output_dir.join("telemetry.jsonl");
    let jsonl_content = serde_json::to_string(&report)?;
    std::fs::write(&jsonl_path, jsonl_content)?;

    // Write Markdown Summary (atomic artifact)
    let summary_path = output_dir.join("summary.md");
    let mut md = String::new();
    md.push_str("# Monte Carlo Battle Sweep Results\n\n");
    md.push_str(&format!("- **Iterations**: {}\n", report.iterations));
    md.push_str(&format!(
        "- **Overall Win Rate**: {:.2}%\n",
        report.win_rate * 100.0
    ));
    md.push_str(&format!("- **Average Turns**: {:.2}\n\n", report.avg_turns));
    md.push_str("| Bug Type | Wins | Losses | Total | Avg Turns |\n");
    md.push_str("|----------|------|--------|-------|-----------|\n");

    // Sort keys for deterministic output
    let mut keys: Vec<_> = report.results_by_type.keys().collect();
    keys.sort();

    for t in keys {
        let stats = &report.results_by_type[t];
        md.push_str(&format!(
            "| {} | {} | {} | {} | {:.2} |\n",
            t, stats.wins, stats.losses, stats.total, stats.avg_turns
        ));
    }

    std::fs::write(&summary_path, md)?;

    Ok(report)
}
