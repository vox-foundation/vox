//! `vox ludus` subcommands — profile, companions, quests, battles.

mod arena;
mod battle;
mod challenge;
mod collegium;
mod ctx;
mod db_util;
mod profile;
mod shop;

pub use ctx::LudusContext;

use anyhow::Result;
use owo_colors::OwoColorize;
use vox_ludus::{Companion, FreeAiClient, LudusProfile, companion::Mood, db, quest, sprite};

pub use arena::{arena_join, arena_leaderboard, arena_show};
pub use battle::{battle_start, battle_submit};
pub use challenge::{challenge_list, challenge_start, challenge_submit};
pub use collegium::{collegium_join, collegium_list, collegium_new, collegium_status};
pub use profile::{
    feedback_rate, mode_command, morning_digest, record_activity, record_cli_event_fire_and_forget,
    reward_claim, shield_use, status,
};
pub use shop::{shop_buy, shop_list};

/// Print a formatted terminal toast for gamification rewards and level-ups.
pub fn print_route_result(res: &vox_ludus::reward_policy::RouteResult) {
    if let Some(reward) = &res.reward {
        if reward.xp > 0 || reward.crystals > 0 || reward.lumens != 0 {
            let mut parts = Vec::new();
            if reward.xp > 0 {
                parts.push(format!("+{} XP", reward.xp).bright_yellow().to_string());
            }
            if reward.crystals > 0 {
                parts.push(format!("+{} 💎", reward.crystals).bright_cyan().to_string());
            }
            if reward.lumens > 0 {
                parts.push(format!("+{} ✦", reward.lumens).bright_magenta().to_string());
            } else if reward.lumens < 0 {
                parts.push(format!("{} ✦", reward.lumens).bright_red().to_string());
            }
            println!("  ✨ {} {}", "Reward:".dimmed(), parts.join(" | "));
        }
        if reward.grant_shield {
            println!(
                "  🛡️  {}",
                "SCUTUM ACTIVATED — Streak Shield earned!"
                    .bright_green()
                    .bold()
            );
        }
    }
    if let Some((lvl, title)) = &res.leveled_up {
        println!();
        println!(
            "{}",
            format!("  ⚡ LEVEL {}! You are now: {}  ⚡", lvl, title)
                .bright_yellow()
                .bold()
        );
        println!("     {}", "+50 Max Energy".dimmed());
        println!();
    }
}

/// Show details for a single companion by name.
pub async fn companion_show(name: &str) -> Result<()> {
    let db = db_util::get_db().await?;
    let user_id = vox_db::paths::local_user_id();
    let companions = db::list_companions(&db, &user_id).await?;
    let companion = match companions.into_iter().find(|c| c.name == name) {
        Some(c) => c,
        None => {
            println!("  Companion '{}' not found.", name);
            return Ok(());
        }
    };
    let sprite_text = companion
        .ascii_sprite
        .clone()
        .unwrap_or_else(|| sprite::generate_deterministic(&companion.name, companion.mood));
    println!("{}", "╔══════════════════════════════════╗".bright_cyan());
    println!("{}", "║       🐱 Companion Details      ║".bright_cyan());
    println!("{}", "╚══════════════════════════════════╝".bright_cyan());
    println!();
    println!(
        "  {} {} [{}]",
        companion.mood.emoji(),
        companion.name.bright_white().bold(),
        companion.language.bright_cyan(),
    );
    for line in sprite_text.lines() {
        println!("    {}", line.bright_green());
    }
    println!();
    println!("  ID: {}", companion.id.dimmed());
    println!(
        "  ❤️  {}/{}  ⚡ {}/{}  📊 {}%",
        companion.health,
        companion.max_health,
        companion.energy,
        companion.max_energy,
        companion.code_quality,
    );
    Ok(())
}

/// List all companions.
pub async fn companion_list() -> Result<()> {
    let db = db_util::get_db().await?;
    let user_id = vox_db::paths::local_user_id();
    let companions = db::list_companions(&db, &user_id).await?;

    println!("{}", "╔══════════════════════════════════╗".bright_cyan());
    println!("{}", "║       🐱 Your Companions        ║".bright_cyan());
    println!("{}", "╚══════════════════════════════════╝".bright_cyan());
    println!();

    if companions.is_empty() {
        println!("  You have no companions yet.");
    } else {
        for companion in companions {
            let sprite_text = companion
                .ascii_sprite
                .clone()
                .unwrap_or_else(|| sprite::generate_deterministic(&companion.name, companion.mood));
            println!(
                "  {} {} {} [{}]",
                companion.mood.emoji(),
                companion.name.bright_white().bold(),
                format!("({})", companion.language).dimmed(),
                companion.mood.bright_yellow(),
            );
            for line in sprite_text.lines() {
                println!("    {}", line.bright_green());
            }
            println!(
                "    ❤️  {}/{}  ⚡ {}/{}  📊 {}%",
                companion.health,
                companion.max_health,
                companion.energy,
                companion.max_energy,
                companion.code_quality,
            );
            println!();
        }
    }
    println!(
        "  Use {} to create a new companion",
        "vox ludus companion create --name <NAME> --code <FILE>".bright_green()
    );

    Ok(())
}

/// Create a new companion from a source file.
pub async fn companion_create(name: &str, code_file: &std::path::Path) -> Result<()> {
    let code = std::fs::read_to_string(code_file)?;

    let id = vox_runtime::builtins::vox_uuid();

    let user_id = vox_db::paths::local_user_id();
    let mut companion = Companion::new(&id, &user_id, name, "vox");
    companion.code_hash = Some(vox_runtime::builtins::vox_hash_fast(&code));
    companion.description = Some(format!("Created from {}", code_file.display()));

    // Generate sprite (try AI, fall back to deterministic)
    let client = FreeAiClient::auto_discover().await;
    let sprite_text = sprite::generate_ai_sprite(&client, name, "vox", Mood::Neutral).await;
    companion.ascii_sprite = Some(sprite_text.clone());

    let db_conn = db_util::get_db().await?;
    db::upsert_companion(&db_conn, &companion).await?;

    // Increment Quests
    let mut profile = match db::get_profile(&db_conn, &user_id).await? {
        Some(p) => p,
        None => LudusProfile::new_default(&user_id),
    };
    let mut quests = db::list_quests(&db_conn, &user_id).await?;
    for q in &mut quests {
        if q.quest_type == quest::QuestType::Create && q.increment(1) {
            println!(
                "  {} Quest Completed: {}",
                "🌟".bright_yellow(),
                q.description.bright_white()
            );
            profile.add_xp(q.xp_reward);
            profile.add_crystals(q.crystal_reward);
        }
    }
    db::upsert_profile(&db_conn, &profile).await?;
    for q in &quests {
        db::upsert_quest(&db_conn, q).await?;
    }

    println!("{}", "✨ Companion created!".bright_green().bold());
    println!();
    println!(
        "  {} {} [{}]",
        companion.mood.emoji(),
        companion.name.bright_white().bold(),
        companion.language.bright_cyan(),
    );
    for line in sprite_text.lines() {
        println!("    {}", line.bright_green());
    }
    println!();
    println!("  ID: {}", companion.id.dimmed());
    println!(
        "  Code quality: {}%",
        companion.code_quality.to_string().bright_yellow()
    );

    Ok(())
}

/// List daily quests.
pub async fn quest_list() -> Result<()> {
    let db = db_util::get_db().await?;

    // Generate deterministic daily quests based on the day
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let day_seed = now / 86_400;

    let user_id = vox_db::paths::local_user_id();
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

/// Generate quests: workspace scanner (TODOs/FIXMEs) + daily archetype templates.
pub async fn quest_generate() -> Result<()> {
    let db = db_util::get_db().await?;
    let user_id = vox_db::paths::local_user_id();

    println!("{}", "╔══════════════════════════════════╗".bright_yellow());
    println!("{}", "║   📋 Generating Quests...       ║".bright_yellow());
    println!("{}", "╚══════════════════════════════════╝".bright_yellow());
    println!();

    // 1. Workspace-scan dynamic quests (from TODOs/FIXMEs)
    let workspace_root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let existing_quests = db::list_quests(&db, &user_id).await.unwrap_or_default();
    let active_count = existing_quests
        .iter()
        .filter(|q| !q.completed && !q.is_expired())
        .count();

    let dynamic = vox_ludus::quest_engine::generate_dynamic_quests(
        &user_id,
        &workspace_root,
        active_count,
        5,
    );

    for dq in &dynamic {
        db::upsert_quest(&db, &dq.quest).await?;
        let src = dq
            .source_issue
            .as_ref()
            .map(|i| format!(" ({}:{})", i.file_path.display(), i.line))
            .unwrap_or_default();
        println!(
            "  ⚔️  {} {}{}",
            dq.quest.quest_type.emoji(),
            dq.quest.description.bright_white(),
            src.dimmed(),
        );
        if let Some(hint) = &dq.hint {
            println!("     💡 {}", hint.dimmed());
        }
    }

    // 2. Daily archetype fallback if no dynamic quests
    if dynamic.is_empty() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let day_seed = now / 86_400;
        let quests = quest::generate_daily_quests(&user_id, day_seed);
        for q in &quests {
            db::upsert_quest(&db, q).await?;
        }
        println!("  (No workspace issues found — using daily archetype quests)");
    }

    println!(
        "  Use {} to view and track them.",
        "vox ludus quest list".bright_green()
    );

    Ok(())
}

/// Show the user leaderboard.
pub async fn leaderboard_show(metric: &str, limit: usize) -> Result<()> {
    let db = db_util::get_db().await?;
    let metric_lower = metric.to_lowercase();
    let entries = if metric_lower == "lumens" || metric_lower == "karma" {
        db::lumens_leaderboard(&db, limit as i64).await?
    } else {
        db::leaderboard(&db, limit as i64).await?
    };

    println!("{}", "╔══════════════════════════════════╗".bright_yellow());
    println!("║       🏆 {} Leaderboard       ║", metric.to_uppercase());
    println!("{}", "╚══════════════════════════════════╝".bright_yellow());
    println!();

    if entries.is_empty() {
        println!("  No entries found.");
    } else {
        println!(
            "  {:>3} {:<20} {:>10}",
            "Rk",
            "User".bright_white(),
            metric.to_uppercase().bright_white()
        );
        println!("  {}", "─".repeat(35).dimmed());

        for (i, entry) in entries.iter().enumerate() {
            let rank = i + 1;
            let medal = match rank {
                1 => "🥇".to_string(),
                2 => "🥈".to_string(),
                3 => "🥉".to_string(),
                _ => rank.to_string(),
            };
            println!(
                "  {:>3} {:<20} {:>10}",
                medal,
                entry.user_id.bright_white().bold(),
                entry.score.to_string().bright_cyan()
            );
        }
    }

    Ok(())
}

/// List pending notifications.
pub async fn notify_list() -> Result<()> {
    let db = db_util::get_db().await?;
    let user_id = vox_db::paths::local_user_id();
    let notifications = db::list_unread_notifications(&db, &user_id, 10).await?;

    println!("{}", "╔══════════════════════════════════╗".bright_cyan());
    println!("{}", "║       🔔 Notifications          ║".bright_cyan());
    println!("{}", "╚══════════════════════════════════╝".bright_cyan());
    println!();

    if notifications.is_empty() {
        println!("  You have no pending notifications.");
    } else {
        for n in notifications {
            let icon = match n.notification_type {
                vox_ludus::notifications::NotificationType::LevelUp => "🆙",
                vox_ludus::notifications::NotificationType::AchievementUnlocked => "🏆",
                vox_ludus::notifications::NotificationType::QuestCompleted => "📋",
                vox_ludus::notifications::NotificationType::ChallengeCompleted => "⚔️",
                vox_ludus::notifications::NotificationType::BattleWon => "🎉",
                vox_ludus::notifications::NotificationType::BattleLost => "💔",
                vox_ludus::notifications::NotificationType::ArenaJoined => "🏟️",
                vox_ludus::notifications::NotificationType::CollegiumJoined => "🏫",
                vox_ludus::notifications::NotificationType::CompanionCreated => "🐣",
                vox_ludus::notifications::NotificationType::ItemPurchased => "🛍️",
                vox_ludus::notifications::NotificationType::FeedbackReceived => "💬",
                _ => "ℹ️",
            };
            println!(
                "  {} {}  {}",
                icon,
                vox_ludus::util::format_unix_time(n.created_at).dimmed(),
                if n.read {
                    n.message.dimmed().to_string()
                } else {
                    n.message.bright_white().to_string()
                }
            );
        }
        db::mark_all_notifications_read(&db, &user_id).await?;
    }

    Ok(())
}

/// Clear all notifications.
pub async fn notify_clear() -> Result<()> {
    let db = db_util::get_db().await?;
    let user_id = vox_db::paths::local_user_id();
    db::mark_all_notifications_read(&db, &user_id).await?;
    println!("  ✅ Notifications cleared.");
    Ok(())
}

/// Show a contextual hint.
pub async fn hint_show(context: Option<&str>) -> Result<()> {
    let _db = db_util::get_db().await?;
    // In a real implementation, we'd load the teaching engine and get a hint
    // For now, let's show a generic one or a context-specific one
    let hint = match context {
        Some("build") => "Try using `vox check` before `vox build` to catch errors faster.",
        Some("battle") => "Companions with higher 'Code Quality' deal more damage in battles.",
        _ => "You can adopt multiple companions, but only one can join you in a battle.",
    };

    println!("  💡 {} {}", "Pro Tip:".bright_yellow().bold(), hint);
    Ok(())
}

/// List all glyphs and achievements.
pub async fn glyph_list(unlocked_only: bool) -> Result<()> {
    let db = db_util::get_db().await?;
    let user_id = vox_db::paths::local_user_id();
    let tracker = vox_ludus::achievement::AchievementTracker::new();
    let unlocked = db::list_unlocked_achievements(&db, &user_id).await?;

    println!("{}", "╔══════════════════════════════════╗".bright_white());
    println!("{}", "║       ⭐ Glyphs & Achievements  ║".bright_white());
    println!("{}", "╚══════════════════════════════════╝".bright_white());
    println!();

    for ach in tracker.all_achievements() {
        let is_unlocked = unlocked.iter().any(|(id, _)| id == &ach.id.0);
        if unlocked_only && !is_unlocked {
            continue;
        }

        let icon = if is_unlocked { "✨" } else { "🔒" };
        let name = if is_unlocked {
            ach.name.bright_white().bold().to_string()
        } else {
            ach.name.dimmed().to_string()
        };

        println!(
            "  {} {} - {}",
            icon,
            name,
            ach.description.italic().dimmed()
        );
        if is_unlocked {
            println!(
                "     ⭐ {}  💎 {}",
                ach.xp_reward.to_string().bright_cyan(),
                ach.crystal_reward.to_string().bright_yellow()
            );
        }
        println!();
    }

    Ok(())
}

/// List project-specific Lex Pack rules.
pub async fn pack_list() -> Result<()> {
    // Try to load Lex Pack from project root
    let root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let pack_path = root.join(".vox/ludus/lex-pack.toml");

    println!(
        "{}",
        "╔══════════════════════════════════╗".bright_magenta()
    );
    println!("{}", "║       📦 Lex Pack Rules         ║".bright_magenta());
    println!(
        "{}",
        "╚══════════════════════════════════╝".bright_magenta()
    );
    println!();

    if pack_path.exists() {
        match vox_ludus::lex_pack::load_lex_pack(&pack_path) {
            Ok(pack) => {
                println!(
                    "  {} v{}",
                    pack.name.bright_white().bold(),
                    pack.version.bright_cyan()
                );
                println!(
                    "  {}",
                    pack.description.as_deref().unwrap_or("").italic().dimmed()
                );
                println!();

                if !pack.glyphs.is_empty() {
                    println!("  Custom Glyphs:");
                    for g in pack.glyphs {
                        println!(
                            "    {} {} [{}]",
                            g.icon,
                            g.name.bright_white(),
                            g.trigger_event.dimmed()
                        );
                    }
                }

                if !pack.lumens_weights.is_empty() {
                    println!("\n  Lumen Weights:");
                    for lw in pack.lumens_weights {
                        println!(
                            "    {:<20} {:>+4} ✦",
                            lw.event_type.dimmed(),
                            lw.lumens_delta.to_string().bright_yellow()
                        );
                    }
                }
            }
            Err(e) => {
                println!("  ❌ Error loading Lex Pack: {}", e);
            }
        }
    } else {
        println!("  No active Lex Pack found for this project.");
        println!(
            "  Run {} to create one.",
            "vox ludus pack init".bright_green()
        );
    }

    Ok(())
}

/// Initialize a new Lex Pack.
pub async fn pack_init(template: &str) -> Result<()> {
    let root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let ludus_dir = root.join(".vox/ludus");
    if !ludus_dir.exists() {
        std::fs::create_dir_all(&ludus_dir)?;
    }

    let pack_path = ludus_dir.join("lex-pack.toml");
    if pack_path.exists() {
        println!("  ❌ Lex Pack already exists in this project.");
        return Ok(());
    }

    let toml_content = match template {
        "core" => {
            r#"[pack]
id = "project-core"
name = "Core Rules"
description = "Project-specific rewards and quality gates"
version = "0.1.0"

[[glyphs]]
id = "test-commander"
name = "Test Commander"
description = "Write 10 new passing tests in one day"
icon = "🎖️"
trigger_event = "test_pass"
trigger_count = 10
xp_reward = 100

[[lumens_weights]]
event_type = "toestub_clean"
lumens_delta = 5
"#
        }
        _ => anyhow::bail!("Unknown template '{}'", template),
    };

    std::fs::write(&pack_path, toml_content)?;
    println!(
        "  ✅ {} initialized at {}",
        "Lex Pack".bright_green(),
        pack_path.display().dimmed()
    );

    Ok(())
}

fn parse_interaction(s: &str) -> Option<vox_ludus::Interaction> {
    match s.to_lowercase().as_str() {
        "feed" => Some(vox_ludus::Interaction::Feed),
        "play" => Some(vox_ludus::Interaction::Play),
        "rest" => Some(vox_ludus::Interaction::Rest),
        "train" => Some(vox_ludus::Interaction::TaskAssigned),
        _ => None,
    }
}

/// Interact with a companion (CLI entry: parses interaction string).
pub async fn companion_interact_str(name: &str, interaction: &str) -> Result<()> {
    let i = parse_interaction(interaction).ok_or_else(|| {
        anyhow::anyhow!(
            "Unknown interaction '{}'. Use: feed, play, rest, train",
            interaction
        )
    })?;
    companion_interact(name, i).await
}

/// Interact with a companion.
pub async fn companion_interact(name: &str, interaction: vox_ludus::Interaction) -> Result<()> {
    let db_conn = db_util::get_db().await?;
    let user_id = vox_db::paths::local_user_id();
    let companions = db::list_companions(&db_conn, &user_id).await?;

    let mut companion = match companions.into_iter().find(|c| c.name == name) {
        Some(c) => c,
        None => {
            println!(
                "  ❌ Companion '{}' not found!",
                name.to_string().bright_yellow()
            );
            return Ok(());
        }
    };

    println!("{}", "╔══════════════════════════════════╗".bright_cyan());
    println!("{}", "║      🐾  Interaction!           ║".bright_cyan());
    println!("{}", "╚══════════════════════════════════╝".bright_cyan());
    println!();
    println!(
        "  Interacting with {}...",
        companion.name.bright_white().bold()
    );

    companion.interact(interaction);

    // Regenerate sprite based on new mood if needed
    let client = FreeAiClient::auto_discover().await;
    let sprite_text = sprite::generate_ai_sprite(
        &client,
        &companion.name,
        &companion.language,
        companion.mood,
    )
    .await;
    companion.ascii_sprite = Some(sprite_text.clone());

    db::upsert_companion(&db_conn, &companion).await?;

    match interaction {
        vox_ludus::Interaction::Feed => println!("  🍔 You fed {}!", companion.name),
        vox_ludus::Interaction::Play => println!("  🎾 You played with {}!", companion.name),
        vox_ludus::Interaction::Rest => println!("  💤 {} took a rest.", companion.name),
        _ => println!("  ⚙️ System event triggered for {}.", companion.name),
    }

    println!();
    for line in sprite_text.lines() {
        println!("    {}", line.bright_green());
    }

    println!();
    println!(
        "    {}  {}/{}  ⚡ {}/{}  [{}]",
        companion.mood.emoji(),
        companion.health,
        companion.max_health,
        companion.energy,
        companion.max_energy,
        companion.mood.bright_yellow(),
    );

    Ok(())
}

/// Render a progress bar.
pub fn render_progress_bar(pct: f64, width: usize) -> String {
    let filled = (pct * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!(
        "[{}{}]",
        "█".repeat(filled).bright_green(),
        "░".repeat(empty).dimmed(),
    )
}
