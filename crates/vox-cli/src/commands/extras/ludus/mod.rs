//! `vox ludus` subcommands — profile, companions, quests, battles.

mod arena;
mod battle;
mod challenge;
mod collegium;
mod companion;
mod ctx;
mod db_util;
#[cfg(feature = "ludus-hud")]
mod hud;
mod pack;
mod profile;
mod progress;
mod quests_notifications;
mod shop;

pub use ctx::LudusContext;

use owo_colors::OwoColorize;

pub use arena::{arena_join, arena_leaderboard, arena_show};
pub use battle::{battle_start, battle_submit};
pub use challenge::{challenge_list, challenge_start, challenge_submit};
pub use collegium::{collegium_join, collegium_list, collegium_new, collegium_status};
pub use companion::{
    companion_create, companion_interact, companion_interact_str, companion_list, companion_show,
};
#[cfg(feature = "ludus-hud")]
pub use hud::run as ludus_hud_run;
pub use pack::{pack_init, pack_list};
pub use profile::{
    audit_show, digest_weekly, disable_ludus, enable_ludus, feedback_rate, metrics_show,
    mode_command, morning_digest, profile_merge_from_default, record_activity,
    record_cli_event_fire_and_forget, reward_claim, session_digest, shield_use, status,
};
pub use progress::render_progress_bar;
pub use quests_notifications::{
    glyph_list, hint_show, leaderboard_show, notify_clear, notify_list, quest_generate, quest_list,
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
