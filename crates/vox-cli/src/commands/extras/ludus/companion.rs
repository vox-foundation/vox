//! Companion CRUD and interaction commands.

use anyhow::Result;
use owo_colors::OwoColorize;
use vox_ludus::{Companion, FreeAiClient, companion::Mood, db, quest, sprite};

use crate::commands::ci::bounded_read::read_utf8_path_capped;

use super::db_util;

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
    let code = read_utf8_path_capped(code_file)?;

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
        None => vox_ludus::LudusProfile::new_default(&user_id),
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
