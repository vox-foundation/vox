//! Combo chain detector — detects multi-event sequences within a time window
//! and fires bonus reward events when a named combo is completed.
//!
//! ## Design
//! - Combos are defined as ordered sequences of event types that must all fire
//!   within `window_secs` of the *first* event in the chain.
//! - State is persisted in `gamify_daily_counters` using synthetic counter keys
//!   so progress survives terminal restarts within the same day.
//! - When a combo completes, a bonus event is emitted through `route_event`.
//!
//! ## Predefined combos
//! | Name              | Events (in order)                              | Window | Bonus event        |
//! |-------------------|------------------------------------------------|--------|--------------------|
//! | Virtus Trifecta   | lint_clean → test_pass → snapshot_captured     | 5 min  | virtus_trifecta    |
//! | Exterminatus      | bug_fix × 3                                    | 1 hr   | exterminatus       |
//! | Iron Will         | test_fail → test_fail → test_pass              | 30 min | iron_will_recovery |
//! | Scribe's Fury     | doc_added × 5                                  | 1 hr   | scribes_fury       |

use anyhow::Result;
use vox_db::Codex;

// ── Combo definitions ─────────────────────────────────────────────────────────

/// A single combo definition.
pub struct ComboDefinition {
    /// Unique identifier for this combo.
    pub id: &'static str,
    /// Human-readable name shown to the player.
    pub display_name: &'static str,
    /// Ordered event sequence. Repeated entries mean "N times".
    pub sequence: &'static [&'static str],
    /// How many seconds the whole chain must complete within.
    pub window_secs: i64,
    /// The reward event slug fired on completion.
    pub bonus_event: &'static str,
}

/// All registered combos. Add new entries here; no other code needs changing.
pub static COMBOS: &[ComboDefinition] = &[
    ComboDefinition {
        id: "virtus_trifecta",
        display_name: "Virtus Trifecta",
        sequence: &["lint_clean", "test_pass", "snapshot_captured"],
        window_secs: 300, // 5 minutes
        bonus_event: "virtus_trifecta",
    },
    ComboDefinition {
        id: "exterminatus",
        display_name: "Exterminatus",
        sequence: &["bug_fix", "bug_fix", "bug_fix"],
        window_secs: 3_600, // 1 hour
        bonus_event: "exterminatus",
    },
    ComboDefinition {
        id: "iron_will_recovery",
        display_name: "Iron Will",
        sequence: &["test_fail", "test_fail", "test_pass"],
        window_secs: 1_800, // 30 minutes
        bonus_event: "iron_will_recovery",
    },
    ComboDefinition {
        id: "scribes_fury",
        display_name: "Scribe's Fury",
        sequence: &[
            "doc_added",
            "doc_added",
            "doc_added",
            "doc_added",
            "doc_added",
        ],
        window_secs: 3_600, // 1 hour
        bonus_event: "scribes_fury",
    },
];

// ── Bonus reward table ────────────────────────────────────────────────────────
// These are registered in reward_policy::base_reward in reward_policy.rs.
// Listed here for documentation only.
//
// "virtus_trifecta"    => XP: 500, crystals: 100, lumens: 20
// "exterminatus"       => XP: 200, crystals: 200 (crystal bonus)
// "iron_will_recovery" => XP: 300, crystals: 60, lumens: 10
// "scribes_fury"       => XP: 400, crystals: 80, lumens: 15

// ── Persistent state keys ─────────────────────────────────────────────────────

/// Counter key for storing combo step progress: `combo:{id}:step`
fn step_key(combo_id: &str) -> String {
    format!("combo:{combo_id}:step")
}

/// Counter key for storing the Unix timestamp when the combo chain started.
fn start_ts_key(combo_id: &str) -> String {
    format!("combo:{combo_id}:start_ts")
}

// ── Core detection logic ──────────────────────────────────────────────────────

/// Process an incoming event against all registered combos.
///
/// For each combo whose next expected event matches `event_type`:
/// - advance the step counter,
/// - check if the window has already expired (reset if so),
/// - if all steps satisfied → return the combo's bonus event.
///
/// Returns a vec of bonus event slugs to fire (usually 0 or 1).
/// The caller (event router) must fire each returned slug through `route_event`.
pub async fn process_event(
    db: &Codex,
    user_id: &str,
    event_type: &str,
) -> Result<Vec<&'static str>> {
    let now = crate::util::now_unix();
    let mut bonuses = Vec::new();

    for combo in COMBOS {
        let step_k = step_key(combo.id);
        let ts_k = start_ts_key(combo.id);

        // Current progress for this combo
        let initial_step = crate::db_ext::get_daily_counter(db, user_id, &step_k)
            .await
            .unwrap_or(0);
        let start_ts = crate::db_ext::get_daily_counter(db, user_id, &ts_k)
            .await
            .unwrap_or(0);

        // Check window expiry (if chain started but window passed → reset)
        let mut step = initial_step;
        if step > 0 && start_ts > 0 && (now - start_ts) > combo.window_secs {
            // Window expired — reset to zero silently
            reset_combo(db, user_id, combo.id).await;
            step = 0;
            // Do not advance — the current event might start a new chain below
        }

        // Is the current event the next expected step?
        let next_expected = combo.sequence.get(step as usize).copied();
        if next_expected != Some(event_type) {
            continue;
        }

        // Record start timestamp on first step
        if step == 0 {
            let _ = set_counter_exact(db, user_id, &ts_k, now).await;
        }

        // Advance step
        let new_step = crate::db_ext::increment_daily_counter(db, user_id, &step_k)
            .await
            .unwrap_or(step + 1);

        if new_step as usize >= combo.sequence.len() {
            // Combo complete!
            bonuses.push(combo.bonus_event);
            reset_combo(db, user_id, combo.id).await;
            tracing::info!(
                "[ludus] Combo '{}' completed by user {}",
                combo.display_name,
                user_id
            );
        }
    }

    Ok(bonuses)
}

/// Reset a combo chain (step + start_ts back to 0).
async fn reset_combo(db: &Codex, user_id: &str, combo_id: &str) {
    let _ = set_counter_exact(db, user_id, &step_key(combo_id), 0).await;
    let _ = set_counter_exact(db, user_id, &start_ts_key(combo_id), 0).await;
}

/// Overwrite a daily counter to an exact value (used for resets and timestamps).
async fn set_counter_exact(db: &Codex, user_id: &str, key: &str, value: i64) -> Result<()> {
    let day = crate::util::now_unix() / 86_400;
    db.connection()
        .execute(
            "INSERT INTO gamify_daily_counters (user_id, event_type, day, count)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (user_id, event_type, day)
         DO UPDATE SET count = excluded.count",
            turso::params![user_id, key, day, value],
        )
        .await?;
    Ok(())
}
