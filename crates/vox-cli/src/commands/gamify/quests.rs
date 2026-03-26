use anyhow::Result;
use owo_colors::OwoColorize;
use vox_ludus::{db, quest};

use super::activity::get_db;
use super::render::render_progress_bar;

/// List daily quests.
pub async fn quest_list() -> Result<()> {
    let db = get_db().await?;

    // Generate deterministic daily quests based on the day
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let day_seed = now / 86_400;

    let user_id = vox_ludus::db::canonical_user_id();
    let mut quests = db::list_quests(&db, &user_id).await?;

    // Filter out expired quests or if none exist for today, generate new ones
    if quests.is_empty() || quests.iter().all(|q| q.is_expired()) {
        quests = quest::generate_daily_quests(&user_id, day_seed);
        for q in &quests {
            db::upsert_quest(&db, q).await?;
        }
    }

    println!("{}", "╔══════════════════════════════════╗".bright_yellow());
    println!("{}", "║       📋 Daily Quests           ║".bright_yellow());
    println!("{}", "╚══════════════════════════════════╝".bright_yellow());
    println!();

    for q in &quests {
        let progress_bar = render_progress_bar(q.progress_pct(), 20);
        let status_icon = if q.completed { "✅" } else { "⬜" };

        println!(
            "  {} {} {}",
            status_icon,
            q.quest_type.emoji(),
            q.description.bright_white(),
        );
        println!(
            "    {} {}/{}  💎{} ⭐{}",
            progress_bar,
            q.progress,
            q.target,
            q.crystal_reward.to_string().bright_yellow(),
            q.xp_reward.to_string().bright_cyan(),
        );
        println!("    💡 {}", q.hint.dimmed());
        println!();
    }

    Ok(())
}
