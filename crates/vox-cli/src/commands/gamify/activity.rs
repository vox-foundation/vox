use anyhow::Result;
use owo_colors::OwoColorize;
use vox_db::VoxDb;
use vox_ludus::{FreeAiClient, LudusProfile, db};

pub(crate) async fn get_db() -> Result<VoxDb> {
    VoxDb::connect_default()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to local gamification DB: {}", e))
}

/// Record a daily activity action and display a subtle message if a streak/level changes.
pub async fn record_activity() -> Result<()> {
    // Fail silently if DB is not available
    let db = match get_db().await {
        Ok(db) => db,
        Err(_) => return Ok(()),
    };

    let user_id = vox_db::paths::local_user_id();
    let mut profile = match db::get_profile(&db, &user_id).await.unwrap_or(None) {
        Some(p) => p,
        None => {
            let p = LudusProfile::new_default(&user_id);
            let _ = db::upsert_profile(&db, &p).await;
            p
        }
    };

    let old_level = profile.level;
    let result = profile.record_daily_activity();

    let _ = db::upsert_profile(&db, &profile).await;

    use vox_ludus::streak::StreakResult;
    match result {
        StreakResult::Continued { streak, bonus_xp } if streak > 1 => {
            println!(
                "  🔥 {} {} ({} XP)",
                "Streak continued:".bright_yellow(),
                streak,
                bonus_xp.to_string().bright_cyan()
            );
        }
        StreakResult::SavedByGrace { streak, bonus_xp } => {
            println!(
                "  🛡️ {} {} ({} XP)",
                "Streak saved by grace period:".bright_green(),
                streak,
                bonus_xp.to_string().bright_cyan()
            );
        }
        StreakResult::BrokenReset { previous } if previous > 1 => {
            println!(
                "  💔 {} (was {} days)",
                "Streak broken".bright_red(),
                previous
            );
        }
        _ => {}
    }

    if profile.level > old_level {
        println!(
            "  🏆 {} You reached {}!",
            "LEVEL UP!".bright_magenta().bold(),
            format!("Level {}", profile.level).bright_yellow()
        );
    }

    Ok(())
}

/// Display gamification status (profile overview).
pub async fn status() -> Result<()> {
    let db = get_db().await?;
    let user_id = vox_db::paths::local_user_id();
    let mut profile = match db::get_profile(&db, &user_id).await? {
        Some(p) => p,
        None => {
            let p = LudusProfile::new_default(&user_id);
            db::upsert_profile(&db, &p).await?;
            p
        }
    };

    profile.regen_energy();
    db::upsert_profile(&db, &profile).await?;

    println!("{}", "╔══════════════════════════════════╗".bright_purple());
    println!("{}", "║     ⚡ Vox Gamification ⚡       ║".bright_purple());
    println!("{}", "╚══════════════════════════════════╝".bright_purple());
    println!();
    println!(
        "  🏅 Level {}  •  {} XP to next level",
        profile.level.to_string().bright_yellow(),
        profile.xp_to_next_level().to_string().bright_cyan(),
    );
    println!(
        "  💎 {} crystals  •  ⚡ {}/{} energy",
        profile.crystals.to_string().bright_yellow(),
        profile.energy.to_string().bright_green(),
        profile.max_energy,
    );
    println!();

    // Show AI provider status
    let client = FreeAiClient::auto_discover().await;
    println!("  🤖 AI providers:");
    for provider in client.providers() {
        println!("    • {}", provider.name().bright_blue());
    }
    println!();
    println!(
        "  Use {} to see your companions",
        "vox gamify companion list".bright_green()
    );
    println!(
        "  Use {} to see daily quests",
        "vox gamify quest list".bright_green()
    );

    Ok(())
}
