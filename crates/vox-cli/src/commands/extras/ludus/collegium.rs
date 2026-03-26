use crate::commands::extras::ludus::db_util;
use anyhow::Result;
use owo_colors::OwoColorize;
use vox_ludus::db as ludus_db;

/// Create a new collegium (team).
pub async fn collegium_new(name: &str, description: Option<&str>) -> Result<()> {
    let codex = db_util::get_db().await?;
    let user_id = vox_ludus::db::canonical_user_id();
    let id = name.to_lowercase().replace(' ', "-");

    ludus_db::create_collegium(&codex, &id, name, description, &user_id).await?;

    println!("{}", "╔══════════════════════════════════╗".bright_cyan());
    println!("{}", "║       🏛️  New Collegium Created!  ║".bright_cyan());
    println!("{}", "╚══════════════════════════════════╝".bright_cyan());
    println!();
    println!("  Name: {}", name.bright_white().bold());
    println!("  ID:   {}", id.bright_cyan());
    if let Some(desc) = description {
        println!("  Description: {}", desc.dimmed());
    }
    println!();
    println!("  Creator: {}", user_id.bright_cyan());
    println!("  You have been added as the 'Pontifex' (Leader).");

    let event_json = serde_json::json!({
        "type": "collegium_created",
        "collegium_id": id,
    });
    let res = vox_ludus::event_router::route_event(&codex, &user_id, &event_json).await?;
    crate::commands::extras::ludus::print_route_result(&res);

    Ok(())
}

/// List all collegiums.
pub async fn collegium_list() -> Result<()> {
    let codex = db_util::get_db().await?;
    let collegiums = ludus_db::list_collegiums(&codex).await?;

    println!("{}", "╔══════════════════════════════════╗".bright_cyan());
    println!("{}", "║       🏛️  Active Collegiums      ║".bright_cyan());
    println!("{}", "╚══════════════════════════════════╝".bright_cyan());
    println!();

    if collegiums.is_empty() {
        println!("  No active collegiums found. Start one with `vox ludus collegium new`!");
    } else {
        println!(
            "  ✦ {:<20} {:>10} {:>10} ✦",
            "Collegium (ID)", "Lumens", "Members"
        );
        println!("  {}", "─".repeat(45).dimmed());

        for (i, (id, name, lumens, members)) in collegiums.iter().enumerate() {
            let rank = i + 1;
            let medal = match rank {
                1 => "🥇".to_string(),
                2 => "🥈".to_string(),
                3 => "🥉".to_string(),
                _ => rank.to_string(),
            };
            println!(
                "  {:>2} {:<20} {:>11} {:>10}",
                medal,
                format!("{} ({})", name, id).bright_white(),
                lumens.to_string().bright_yellow(),
                members.to_string().bright_cyan(),
            );
        }
    }

    println!();
    println!(
        "  Join with {} to become a member.",
        "vox ludus collegium join --id <ID>".bright_green()
    );

    Ok(())
}

/// Join a collegium.
pub async fn collegium_join(id: &str) -> Result<()> {
    let codex = db_util::get_db().await?;
    let user_id = vox_ludus::db::canonical_user_id();

    ludus_db::join_collegium(&codex, id, &user_id, "legionnaire").await?;

    println!(
        "🏛️  You have joined collegium: {}",
        id.bright_white().bold()
    );
    println!("  Welcome to the ranks, legionnaire!");
    Ok(())
}

/// Show status of a collegium.
pub async fn collegium_status(id: Option<&str>) -> Result<()> {
    let codex = db_util::get_db().await?;
    let user_id = vox_ludus::db::canonical_user_id();

    let collegium = if let Some(cid) = id {
        ludus_db::get_collegium(&codex, cid).await?
    } else {
        ludus_db::get_user_collegium(&codex, &user_id).await?
    };

    println!("{}", "╔══════════════════════════════════╗".bright_cyan());
    println!("{}", "║       🏛️  Collegium Status       ║".bright_cyan());
    println!("{}", "╚══════════════════════════════════╝".bright_cyan());
    println!();

    match collegium {
        Some((id, name, lumens, members)) => {
            println!(
                "  Status for: {} ({})",
                name.bright_white().bold(),
                id.dimmed()
            );
            println!(
                "  Collective Lumens: {} ✦",
                lumens.to_string().bright_yellow()
            );
            println!("  Active Members:    {}", members.to_string().bright_cyan());

            // Progress to next milestone
            let current_lumens = lumens.max(0);
            let next_milestone = ((current_lumens / 1000) + 1) * 1000;
            let remaining = next_milestone - current_lumens;
            println!(
                "  Next milestone:    {} ✦ (+{} to go)",
                next_milestone.to_string().bright_white(),
                remaining.to_string().bright_yellow()
            );
        }
        None => {
            println!("  ❌ No collegium found. Join one or specify with --id.");
        }
    }

    Ok(())
}
