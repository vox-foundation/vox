//! Idempotent Ludus / gamification DDL alignment for Arca baseline databases.
//!
//! Baseline [`crate::schema::baseline_sql`] only includes core `gamify_*` tables; this cutover adds
//! extended Ludus tables and repairs historical naming drift (`gamify_collegiums`, `counter_name`).

use turso::Connection;

use crate::store::types::StoreError;

async fn table_exists(conn: &Connection, name: &str) -> Result<bool, StoreError> {
    let mut rows = conn
        .query(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1 LIMIT 1",
            (name,),
        )
        .await?;
    Ok(rows.next().await?.is_some())
}

async fn table_column_names(
    conn: &Connection,
    pragma_sql: &'static str,
) -> Result<Vec<String>, StoreError> {
    let mut rows = conn.query(pragma_sql, ()).await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        let name: String = row.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
        out.push(name);
    }
    Ok(out)
}

fn has_col(cols: &[String], name: &str) -> bool {
    cols.iter().any(|c| c == name)
}

/// Additive Ludus gamification schema + column renames (safe on existing Codex files).
pub async fn apply_ludus_gamify_cutover(conn: &Connection) -> Result<(), StoreError> {
    rename_collegiums_to_collegium(conn).await?;
    fix_gamify_counters_column(conn).await?;
    align_gamify_policy_snapshots_columns(conn).await?;
    align_gamify_profiles_columns(conn).await?;
    align_gamify_quests_columns(conn).await?;
    align_gamify_periodic_rewards_columns(conn).await?;
    align_gamify_companions_columns(conn).await?;

    let batch = r#"
CREATE TABLE IF NOT EXISTS gamify_achievements (
    id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    unlocked_at INTEGER NOT NULL,
    xp_rewarded INTEGER NOT NULL DEFAULT 0,
    crystals_rewarded INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id, user_id)
);
CREATE TABLE IF NOT EXISTS gamify_teaching_profiles (
    user_id TEXT PRIMARY KEY,
    stage TEXT NOT NULL DEFAULT 'onboarding',
    silenced INTEGER NOT NULL DEFAULT 0,
    mistake_counts TEXT NOT NULL DEFAULT '{}',
    cooldowns TEXT NOT NULL DEFAULT '{}',
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS gamify_policy_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    base_xp INTEGER NOT NULL DEFAULT 0,
    base_crystals INTEGER NOT NULL DEFAULT 0,
    mode_label TEXT NOT NULL DEFAULT 'balanced',
    effective_multiplier REAL NOT NULL DEFAULT 1.0,
    awarded_xp INTEGER NOT NULL DEFAULT 0,
    awarded_crystals INTEGER NOT NULL DEFAULT 0,
    streak_days INTEGER NOT NULL DEFAULT 0,
    grind_capped INTEGER NOT NULL DEFAULT 0,
    lumens INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_policy_snapshots_user ON gamify_policy_snapshots(user_id);
CREATE INDEX IF NOT EXISTS idx_policy_snapshots_event ON gamify_policy_snapshots(event_type);
CREATE TABLE IF NOT EXISTS gamify_ai_feedback (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    response_id TEXT NOT NULL,
    thumbs_up INTEGER NOT NULL,
    comment TEXT,
    tokens_generated INTEGER NOT NULL DEFAULT 0,
    example_code TEXT,
    contributed_to_corpus INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_gamify_ai_feedback_user ON gamify_ai_feedback(user_id);
CREATE TABLE IF NOT EXISTS gamify_periodic_rewards (
    reward_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    icon TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    xp_bonus INTEGER NOT NULL DEFAULT 0,
    crystal_bonus INTEGER NOT NULL DEFAULT 0,
    redeemed INTEGER NOT NULL DEFAULT 0,
    expires_at INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    unlock_condition TEXT DEFAULT '"WeeklyCheckIn"',
    PRIMARY KEY (reward_id, user_id)
);
CREATE INDEX IF NOT EXISTS idx_gamify_periodic_rewards_user ON gamify_periodic_rewards(user_id);
CREATE TABLE IF NOT EXISTS gamify_level_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    level INTEGER NOT NULL,
    title TEXT NOT NULL,
    xp_at_level INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);
CREATE INDEX IF NOT EXISTS idx_gamify_level_history_user ON gamify_level_history(user_id);
CREATE TABLE IF NOT EXISTS gamify_counters (
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    count INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (user_id, name)
);
CREATE TABLE IF NOT EXISTS gamify_collegium (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    leader_id TEXT,
    xp INTEGER NOT NULL DEFAULT 0,
    level INTEGER NOT NULL DEFAULT 1,
    lumens INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS gamify_collegium_members (
    collegium_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'member',
    joined_at INTEGER NOT NULL,
    PRIMARY KEY (collegium_id, user_id)
);
CREATE TABLE IF NOT EXISTS gamify_arena_events (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    event_type TEXT NOT NULL,
    start_ts INTEGER NOT NULL,
    end_ts INTEGER NOT NULL,
    target_xp INTEGER NOT NULL,
    current_xp INTEGER NOT NULL DEFAULT 0,
    target_lumens INTEGER NOT NULL DEFAULT 0,
    current_lumens INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'active'
);
CREATE TABLE IF NOT EXISTS gamify_arena_participants (
    event_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    xp_contributed INTEGER NOT NULL DEFAULT 0,
    lumens_contributed INTEGER NOT NULL DEFAULT 0,
    joined_at INTEGER NOT NULL,
    PRIMARY KEY (event_id, user_id)
);
CREATE TABLE IF NOT EXISTS gamify_daily_counters (
    user_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    day INTEGER NOT NULL,
    count INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (user_id, event_type, day)
);
CREATE TABLE IF NOT EXISTS gamify_event_config (
    event_type TEXT PRIMARY KEY,
    xp_override INTEGER,
    crystals_override INTEGER,
    enabled INTEGER NOT NULL DEFAULT 1,
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);
CREATE TABLE IF NOT EXISTS gamify_notifications (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    notification_type TEXT NOT NULL,
    title TEXT NOT NULL,
    message TEXT NOT NULL,
    read INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_gamify_notifications_user ON gamify_notifications(user_id);
CREATE TABLE IF NOT EXISTS gamify_hint_telemetry (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    action TEXT NOT NULL,
    reason TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_gamify_hint_telemetry_user ON gamify_hint_telemetry(user_id);
CREATE TABLE IF NOT EXISTS gamify_processed_events (
    user_id TEXT NOT NULL,
    dedupe_key TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (user_id, dedupe_key)
);
"#;
    conn.execute_batch(batch).await?;
    Ok(())
}

async fn rename_collegiums_to_collegium(conn: &Connection) -> Result<(), StoreError> {
    let plural = table_exists(conn, "gamify_collegiums").await?;
    let singular = table_exists(conn, "gamify_collegium").await?;
    if plural && !singular {
        conn.execute(
            "ALTER TABLE gamify_collegiums RENAME TO gamify_collegium",
            (),
        )
        .await?;
    }
    Ok(())
}

async fn fix_gamify_counters_column(conn: &Connection) -> Result<(), StoreError> {
    let cols = table_column_names(conn, "PRAGMA table_info(gamify_counters)").await?;
    if cols.is_empty() {
        return Ok(());
    }
    if has_col(&cols, "counter_name") && !has_col(&cols, "name") {
        conn.execute(
            "ALTER TABLE gamify_counters RENAME COLUMN counter_name TO name",
            (),
        )
        .await?;
    }
    Ok(())
}

async fn align_gamify_policy_snapshots_columns(conn: &Connection) -> Result<(), StoreError> {
    let cols = table_column_names(conn, "PRAGMA table_info(gamify_policy_snapshots)").await?;
    if cols.is_empty() {
        return Ok(());
    }
    if has_col(&cols, "mode") && !has_col(&cols, "mode_label") {
        conn.execute(
            "ALTER TABLE gamify_policy_snapshots RENAME COLUMN mode TO mode_label",
            (),
        )
        .await?;
    }
    if has_col(&cols, "xp_awarded") && !has_col(&cols, "awarded_xp") {
        conn.execute(
            "ALTER TABLE gamify_policy_snapshots RENAME COLUMN xp_awarded TO awarded_xp",
            (),
        )
        .await?;
    }
    if has_col(&cols, "crystals_awarded") && !has_col(&cols, "awarded_crystals") {
        conn.execute(
            "ALTER TABLE gamify_policy_snapshots RENAME COLUMN crystals_awarded TO awarded_crystals",
            (),
        )
        .await?;
    }
    if has_col(&cols, "lumens_awarded") && !has_col(&cols, "lumens") {
        conn.execute(
            "ALTER TABLE gamify_policy_snapshots RENAME COLUMN lumens_awarded TO lumens",
            (),
        )
        .await?;
    }
    Ok(())
}

async fn align_gamify_profiles_columns(conn: &Connection) -> Result<(), StoreError> {
    if table_column_names(conn, "PRAGMA table_info(gamify_profiles)")
        .await?
        .is_empty()
    {
        return Ok(());
    }
    let add = [
        ("total_xp_earned", "INTEGER NOT NULL DEFAULT 0"),
        ("prestige_level", "INTEGER NOT NULL DEFAULT 0"),
        ("lumens", "INTEGER NOT NULL DEFAULT 0"),
        ("generosity_lumens", "INTEGER NOT NULL DEFAULT 0"),
        ("streak_days", "INTEGER NOT NULL DEFAULT 0"),
        ("longest_streak", "INTEGER NOT NULL DEFAULT 0"),
        ("streak_last_ts", "INTEGER NOT NULL DEFAULT 0"),
        ("grace_available", "INTEGER NOT NULL DEFAULT 1"),
        ("grace_used", "INTEGER NOT NULL DEFAULT 0"),
        ("streak_shields", "INTEGER NOT NULL DEFAULT 0"),
    ];
    for (col, decl) in add {
        let cols = table_column_names(conn, "PRAGMA table_info(gamify_profiles)").await?;
        if !has_col(&cols, col) {
            let sql = format!("ALTER TABLE gamify_profiles ADD COLUMN {col} {decl}");
            conn.execute(&sql, ()).await?;
        }
    }
    Ok(())
}

async fn align_gamify_periodic_rewards_columns(conn: &Connection) -> Result<(), StoreError> {
    let cols = table_column_names(conn, "PRAGMA table_info(gamify_periodic_rewards)").await?;
    if cols.is_empty() {
        return Ok(());
    }
    if !has_col(&cols, "unlock_condition") {
        conn.execute(
            "ALTER TABLE gamify_periodic_rewards ADD COLUMN unlock_condition TEXT DEFAULT '\"WeeklyCheckIn\"'",
            (),
        )
        .await?;
    }
    Ok(())
}

async fn align_gamify_companions_columns(conn: &Connection) -> Result<(), StoreError> {
    let cols = table_column_names(conn, "PRAGMA table_info(gamify_companions)").await?;
    if cols.is_empty() {
        return Ok(());
    }
    if !has_col(&cols, "personality") {
        conn.execute(
            "ALTER TABLE gamify_companions ADD COLUMN personality TEXT NOT NULL DEFAULT '{}'",
            (),
        )
        .await?;
    }
    Ok(())
}

async fn align_gamify_quests_columns(conn: &Connection) -> Result<(), StoreError> {
    let cols = table_column_names(conn, "PRAGMA table_info(gamify_quests)").await?;
    if cols.is_empty() {
        return Ok(());
    }
    if !has_col(&cols, "hint") {
        conn.execute(
            "ALTER TABLE gamify_quests ADD COLUMN hint TEXT DEFAULT ''",
            (),
        )
        .await?;
    }
    if !has_col(&cols, "modifier") {
        conn.execute(
            "ALTER TABLE gamify_quests ADD COLUMN modifier TEXT DEFAULT 'none'",
            (),
        )
        .await?;
    }
    if !has_col(&cols, "status") {
        conn.execute(
            "ALTER TABLE gamify_quests ADD COLUMN status TEXT DEFAULT 'active'",
            (),
        )
        .await?;
    }
    Ok(())
}
