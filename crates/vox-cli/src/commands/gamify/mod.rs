//! Ludus helpers (profile, companions, quests, battles), invoked via **`vox ludus`** when `extras-ludus` is enabled.
//!
//! Wired as real modules (no `include!` shards). CLI entry may be added later; functions are used by tests and tooling.

#![allow(dead_code)]

mod activity;
mod battle;
mod companions;
mod quests;
mod render;

pub use activity::{record_activity, status};
pub use battle::{battle_start, battle_submit};
pub use companions::{companion_create, companion_interact, companion_list};
pub use quests::quest_list;
