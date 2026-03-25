use anyhow::Result;
use owo_colors::OwoColorize;
use turso::params;
use vox_ludus::{battle::Battle, db, quest, LudusProfile};

use super::activity::get_db;

/// Start a bug battle.
pub async fn battle_start(companion_name: &str) -> Result<()> {
    let db_conn = get_db().await?;

    let user_id = vox_db::paths::local_user_id();

    let companions = db::list_companions(&db_conn, &user_id).await?;
    let companion = match companions.into_iter().find(|c| c.name == companion_name) {
        Some(c) => c,
        None => {
            println!(
                "  ❌ Companion '{}' not found!",
                companion_name.bright_yellow()
            );
            return Ok(());
        }
    };

    // Scan for real bugs!
    use vox_toestub::rules::Severity;
    use vox_toestub::{ToestubConfig, ToestubEngine};

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

    let battle = Battle::from_finding(
        format!(
            "battle-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        ),
        &user_id,
        companion.id.clone(),
        &finding.rule_id,
        finding.message.clone(),
        Some(String::new()), // Removed code_context usage
    );

    db::insert_battle(&db_conn, &battle).await?;

    println!("{}", "╔══════════════════════════════════╗".bright_red());
    println!("{}", "║       ⚔️  Bug Battle!            ║".bright_red());
    println!("{}", "╚══════════════════════════════════╝".bright_red());
    println!();
    println!(
        "  🐱 {} vs {} {}",
        companion_name.bright_white().bold(),
        battle.bug_type.emoji(),
        battle.bug_type.display().bright_red(),
    );
    println!();
    println!("  {}", battle.bug_description.bright_yellow());
    if let Some(ref code) = battle.bug_code {
        println!();
        println!(
            "  Buggy code location: {}:{}",
            finding.file.display(),
            finding.line
        );
        println!("  Context:");
        for line in code.lines().take(5) {
            println!("    {}", line.bright_red());
        }
    }
    println!();
    println!(
        "  Rewards: 💎{} ⭐{}",
        battle.bug_type.crystal_reward().to_string().bright_yellow(),
        battle.bug_type.xp_reward().to_string().bright_cyan(),
    );
    println!();
    println!(
        "  Submit your fix with: {}",
        format!(
            "vox gamify battle submit --companion {} --code <FILE>",
            companion_name
        )
        .bright_green()
    );

    Ok(())
}

/// Submit code to win a bug battle.
pub async fn battle_submit(companion_name: &str, code_file: &std::path::Path) -> Result<()> {
    let db_conn = get_db().await?;
    let code = std::fs::read_to_string(code_file)?;

    let user_id = vox_db::paths::local_user_id();
    let mut profile = match db::get_profile(&db_conn, &user_id).await? {
        Some(p) => p,
        None => LudusProfile::new_default(&user_id),
    };

    let companions = db::list_companions(&db_conn, &user_id).await?;
    let mut companion = match companions.into_iter().find(|c| c.name == companion_name) {
        Some(c) => c,
        None => {
            println!(
                "  ❌ Companion '{}' not found!",
                companion_name.to_string().bright_yellow()
            );
            return Ok(());
        }
    };

    // Find the most recent unfinished battle for this companion
    let battles = db::list_battles(&db_conn, &user_id, 20).await?;
    let mut battle = match battles
        .into_iter()
        .find(|b| b.companion_id == companion.id && !b.success)
    {
        Some(b) => b,
        None => {
            println!(
                "  ❌ No active battle found for {}. Start one with {}!",
                companion_name.bright_white().bold(),
                "vox gamify battle start".bright_green()
            );
            return Ok(());
        }
    };

    println!("{}", "╔══════════════════════════════════╗".bright_green());
    println!("{}", "║      🏆  Battle Result!         ║".bright_green());
    println!("{}", "╚══════════════════════════════════╝".bright_green());
    println!();

    if !companion.spend_battle_energy() {
        println!(
            "  {} {} is too tired to battle! Try resting them.",
            "💤".bright_blue(),
            companion.name.bright_white().bold()
        );
        return Ok(());
    }

    // In a real implementation we would type check the submitted code
    let is_success = !code.contains("todo!()") && !code.is_empty();

    if is_success {
        battle.record_result(true, 60); // Simulated 60s
        battle.submitted_code = Some(code);

        let leveled_up = profile.add_xp(battle.xp_earned);
        profile.add_crystals(battle.crystals_earned);

        // Increment Quests
        let mut quests = db::list_quests(&db_conn, &user_id).await?;
        for q in &mut quests {
            if q.quest_type == quest::QuestType::Battle && q.increment(1) {
                println!(
                    "  {} Quest Completed: {}",
                    "🌟".bright_yellow(),
                    q.description.bright_white()
                );
                profile.add_xp(q.xp_reward);
                profile.add_crystals(q.crystal_reward);
            }
        }

        // Persist everything
        db::upsert_profile(&db_conn, &profile).await?;
        db::upsert_companion(&db_conn, &companion).await?;

        // Update battle record
        db_conn.connection().execute(
            "UPDATE gamify_battles SET success = 1, crystals_earned = ?1, xp_earned = ?2, submitted_code = ?3 WHERE id = ?4",
            params![battle.crystals_earned as i64, battle.xp_earned as i64, battle.submitted_code.clone(), battle.id.clone()]
        ).await?;

        for q in &quests {
            db::upsert_quest(&db_conn, q).await?;
        }

        println!(
            "  🎉 You and {} slayed the bug!",
            companion.name.bright_white().bold()
        );
        println!();
        println!("  +{} XP", battle.xp_earned.to_string().bright_cyan());
        println!(
            "  +{} Crystals",
            battle.crystals_earned.to_string().bright_cyan()
        );

        if leveled_up {
            println!(
                "  ⭐ {} You are now level {}",
                "Level Up!".bright_yellow(),
                profile.level.to_string().bright_white()
            );
        }
    } else {
        println!("  ❌ Oh no! The bug fought back.");
        db::upsert_companion(&db_conn, &companion).await?;
        println!("  Make sure your code works and has no `todo!()` remaining.");
    }

    Ok(())
}
