//! Bug battle commands — thin wrappers over vox_ludus orchestration.

use anyhow::Result;
use owo_colors::OwoColorize;
use vox_ludus::{BattleFinding, run_battle_start, run_battle_submit};
use vox_toestub::rules::Severity;
use vox_toestub::{ToestubConfig, ToestubEngine};

use crate::commands::ci::bounded_read::read_utf8_path_capped;

use super::db_util;

/// Start a bug battle.
pub async fn battle_start(companion_name: &str) -> Result<()> {
    let db = db_util::get_db().await?;
    let user_id = vox_db::paths::local_user_id();

    println!("  {} Searching for bugs in the rift...", "🔍".bright_blue());

    let config = ToestubConfig {
        roots: vec![std::env::current_dir()?],
        min_severity: Severity::Warning,
        excludes: vec![".git".to_string(), "target".to_string()],
        ..Default::default()
    };

    let engine = ToestubEngine::new(config);
    let (scan_result, _) = engine.run_and_report();

    let finding = match scan_result.findings.first() {
        Some(f) => f,
        None => {
            println!("  ✨ No bugs found! Your codebase is too strong today.");
            return Ok(());
        }
    };

    let battle_finding = BattleFinding {
        rule_id: finding.rule_id.clone(),
        message: finding.message.clone(),
        file_path: finding.file.clone(),
        line: finding.line,
        context: if finding.context.is_empty() {
            None
        } else {
            Some(finding.context.clone())
        },
    };

    let outcome = match run_battle_start(&db, &user_id, companion_name, &battle_finding).await? {
        Some(o) => o,
        None => {
            println!(
                "  ❌ Companion '{}' not found!",
                companion_name.bright_yellow()
            );
            return Ok(());
        }
    };

    println!("{}", "╔══════════════════════════════════╗".bright_red());
    println!("{}", "║      ⚔️  Bug Battle!            ║".bright_red());
    println!("{}", "╚══════════════════════════════════╝".bright_red());
    println!();
    println!(
        "  🐱 {} vs {} {}",
        outcome.companion_name.bright_white().bold(),
        outcome.battle.bug_type.emoji(),
        outcome.battle.bug_type.display().bright_red(),
    );
    println!();
    println!("  {}", outcome.battle.bug_description.bright_yellow());
    if let Some(ref code) = outcome.battle.bug_code {
        println!();
        println!(
            "  Buggy code location: {}:{}",
            battle_finding.file_path.display(),
            battle_finding.line
        );
        println!("  Context:");
        for line in code.lines().take(5) {
            println!("    {}", line.bright_red());
        }
    }
    println!();
    println!(
        "  Rewards: 💎{} ⭐{}",
        outcome
            .battle
            .bug_type
            .crystal_reward()
            .to_string()
            .bright_yellow(),
        outcome
            .battle
            .bug_type
            .xp_reward()
            .to_string()
            .bright_cyan(),
    );
    println!();
    println!(
        "  Submit your fix with: {}",
        format!(
            "vox ludus battle submit --companion {} --code <FILE>",
            companion_name
        )
        .bright_green()
    );

    Ok(())
}

/// Submit code to win a bug battle.
pub async fn battle_submit(companion_name: &str, code_file: &std::path::Path) -> Result<()> {
    let db = db_util::get_db().await?;
    let code = read_utf8_path_capped(code_file)?;
    let user_id = vox_db::paths::local_user_id();

    let is_success = !code.contains("todo!()") && !code.is_empty(); // toestub-ignore(stub)

    let result = run_battle_submit(&db, &user_id, companion_name, code, is_success).await?;

    match result {
        vox_ludus::BattleSubmitResult::Tired => {
            println!(
                "  {} {} is too tired to battle! Try resting them.",
                "💤".bright_blue(),
                companion_name.bright_white().bold()
            );
            return Ok(());
        }
        vox_ludus::BattleSubmitResult::NotFound => {
            println!(
                "  ❌ No active battle found for {}. Start one with {}!",
                companion_name.bright_white().bold(),
                "vox ludus battle start".bright_green()
            );
            return Ok(());
        }
        vox_ludus::BattleSubmitResult::Outcome(o) => {
            println!("{}", "╔══════════════════════════════════╗".bright_green());
            println!("{}", "║      🏆  Battle Result!         ║".bright_green());
            println!("{}", "╚══════════════════════════════════╝".bright_green());
            println!();

            if o.success {
                println!(
                    "  🎉 You and {} slayed the bug!",
                    o.companion.name.bright_white().bold()
                );
                println!();
                println!("  +{} XP", o.battle.xp_earned.to_string().bright_cyan());
                println!(
                    "  +{} Crystals",
                    o.battle.crystals_earned.to_string().bright_cyan()
                );

                if let Some(ref q) = o.quest_completed {
                    println!(
                        "  {} Quest Completed: {}",
                        "🌟".bright_yellow(),
                        q.bright_white()
                    );
                }

                if o.leveled_up {
                    println!(
                        "  ⭐ {} You are now level {}",
                        "Level Up!".bright_yellow(),
                        o.profile.level.to_string().bright_white()
                    );
                }
            } else {
                println!("  ❌ Oh no! The bug fought back.");
                println!("  Make sure your code works and has no `todo!()` remaining."); // toestub-ignore(stub)
            }
        }
    }

    Ok(())
}
