//! Integration tests for `CodeStore` gamification CRUD (`ops_ludus`).

use turso::params;
use vox_db::VoxDb;

/// Gamification DDL not included in the vox-pm baseline or coordination schema.
///
/// Tables already created by `open_memory()`:
/// - `a2a_messages` (coordination schema)
/// - `agent_oplog` (coordination schema)
/// - `distributed_locks` (coordination schema)
/// - `mesh_heartbeats` (coordination schema)
/// - `actor_state` (v21)
///
/// Tables below are from `vox-ludus/src/schema.rs` and must be applied
/// manually in tests targeting `vox-pm` CRUD methods.
const GAMIFY_DDL: &str = "
CREATE TABLE IF NOT EXISTS gamify_profiles (
    user_id TEXT PRIMARY KEY,
    level INTEGER NOT NULL DEFAULT 1,
    xp INTEGER NOT NULL DEFAULT 0,
    crystals INTEGER NOT NULL DEFAULT 100,
    energy INTEGER NOT NULL DEFAULT 100,
    max_energy INTEGER NOT NULL DEFAULT 100,
    last_energy_regen INTEGER NOT NULL DEFAULT 0,
    last_active INTEGER NOT NULL DEFAULT 0,
    streak_days INTEGER NOT NULL DEFAULT 0,
    longest_streak INTEGER NOT NULL DEFAULT 0,
    streak_last_ts INTEGER NOT NULL DEFAULT 0,
    grace_available INTEGER NOT NULL DEFAULT 0,
    grace_used INTEGER NOT NULL DEFAULT 0,
    total_xp_earned INTEGER NOT NULL DEFAULT 0,
    prestige_level INTEGER NOT NULL DEFAULT 0,
    lumens INTEGER NOT NULL DEFAULT 0,
    generosity_lumens INTEGER NOT NULL DEFAULT 0,
    streak_shields INTEGER NOT NULL DEFAULT 0,
    trust_tier INTEGER DEFAULT 0,
    reward_suppressed INTEGER NOT NULL DEFAULT 0,
    suppressed_until_ts INTEGER NOT NULL DEFAULT 0,
    suppression_reason TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS gamify_achievements (
    id TEXT NOT NULL, user_id TEXT NOT NULL, unlocked_at INTEGER NOT NULL,
    xp_rewarded INTEGER NOT NULL DEFAULT 0, crystals_rewarded INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id, user_id)
);
CREATE TABLE IF NOT EXISTS gamify_level_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT, user_id TEXT NOT NULL, level INTEGER NOT NULL,
    title TEXT NOT NULL, xp_at_level INTEGER NOT NULL DEFAULT 0, created_at INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS gamify_companions (
    id TEXT PRIMARY KEY, user_id TEXT NOT NULL, name TEXT NOT NULL, description TEXT,
    code_hash TEXT, language TEXT NOT NULL DEFAULT 'vox', ascii_sprite TEXT, mood TEXT NOT NULL DEFAULT 'neutral',
    health INTEGER NOT NULL DEFAULT 100, max_health INTEGER NOT NULL DEFAULT 100,
    energy INTEGER NOT NULL DEFAULT 100, max_energy INTEGER NOT NULL DEFAULT 100,
    code_quality INTEGER NOT NULL DEFAULT 50, last_active INTEGER NOT NULL DEFAULT 0,
    personality TEXT NOT NULL DEFAULT 'focused',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS gamify_quests (
    id TEXT PRIMARY KEY, user_id TEXT NOT NULL, quest_type TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT '', description TEXT NOT NULL, xp_reward INTEGER NOT NULL DEFAULT 0,
    crystal_reward INTEGER NOT NULL DEFAULT 0, target INTEGER NOT NULL DEFAULT 1,
    progress INTEGER NOT NULL DEFAULT 0, status TEXT NOT NULL DEFAULT 'active',
    expires_at INTEGER NOT NULL DEFAULT 0, completed INTEGER NOT NULL DEFAULT 0,
    hint TEXT, modifier TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS gamify_battles (
    id TEXT PRIMARY KEY, user_id TEXT NOT NULL, companion_id TEXT NOT NULL,
    bug_type TEXT NOT NULL, bug_description TEXT NOT NULL, bug_code TEXT, submitted_code TEXT,
    success INTEGER NOT NULL DEFAULT 0, crystals_earned INTEGER NOT NULL DEFAULT 0,
    xp_earned INTEGER NOT NULL DEFAULT 0, duration_secs INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS agent_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT, agent_id TEXT NOT NULL, event_type TEXT NOT NULL,
    payload_json TEXT, cli_version TEXT, timestamp TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS gamify_hint_telemetry (
    user_id TEXT NOT NULL, kind TEXT NOT NULL, action TEXT NOT NULL, reason TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS gamify_processed_events (
    user_id TEXT NOT NULL, dedupe_key TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (user_id, dedupe_key)
);
CREATE TABLE IF NOT EXISTS gamify_notifications (
    id TEXT PRIMARY KEY, user_id TEXT NOT NULL, notification_type TEXT NOT NULL,
    title TEXT NOT NULL, message TEXT NOT NULL, read INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT 0, expires_at INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS cost_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT, agent_id TEXT NOT NULL, session_id TEXT,
    provider TEXT NOT NULL, model TEXT, input_tokens INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0, cost_usd REAL NOT NULL DEFAULT 0.0,
    timestamp TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS agent_sessions (
    id TEXT PRIMARY KEY, agent_id TEXT NOT NULL, agent_name TEXT,
    started_at TEXT NOT NULL DEFAULT (datetime('now')), ended_at TEXT,
    status TEXT NOT NULL DEFAULT 'active', task_snapshot TEXT, context_summary TEXT
);
CREATE TABLE IF NOT EXISTS agent_metrics (
    agent_id TEXT NOT NULL, metric_name TEXT NOT NULL, metric_value REAL NOT NULL DEFAULT 0.0,
    period TEXT NOT NULL DEFAULT 'session', timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (agent_id, metric_name, period)
);
CREATE TABLE IF NOT EXISTS gamify_counters (
    user_id TEXT NOT NULL, name TEXT NOT NULL, count INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (user_id, name)
);
CREATE TABLE IF NOT EXISTS gamify_daily_counters (
    user_id TEXT NOT NULL, event_type TEXT NOT NULL, day INTEGER NOT NULL,
    count INTEGER NOT NULL DEFAULT 0, PRIMARY KEY (user_id, event_type, day)
);
CREATE TABLE IF NOT EXISTS gamify_event_config (
    event_type TEXT PRIMARY KEY, xp_override INTEGER, crystals_override INTEGER,
    enabled INTEGER NOT NULL DEFAULT 1, updated_at INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS gamify_collegium (
    id TEXT PRIMARY KEY, name TEXT NOT NULL, lumens INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS gamify_collegium_members (
    collegium_id TEXT NOT NULL, user_id TEXT NOT NULL, role TEXT NOT NULL DEFAULT 'member',
    joined_at INTEGER NOT NULL, PRIMARY KEY (collegium_id, user_id)
);
CREATE TABLE IF NOT EXISTS gamify_policy_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT, user_id TEXT NOT NULL, event_type TEXT NOT NULL,
    base_xp INTEGER NOT NULL DEFAULT 0, base_crystals INTEGER NOT NULL DEFAULT 0,
    mode_label TEXT NOT NULL DEFAULT 'balanced', effective_multiplier REAL NOT NULL DEFAULT 1.0,
    awarded_xp INTEGER NOT NULL DEFAULT 0, awarded_crystals INTEGER NOT NULL DEFAULT 0,
    streak_days INTEGER NOT NULL DEFAULT 0, grind_capped INTEGER NOT NULL DEFAULT 0,
    lumens INTEGER NOT NULL DEFAULT 0, metadata TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS agent_locks (
    path TEXT NOT NULL, agent_id TEXT NOT NULL, repository_id TEXT NOT NULL DEFAULT '',
    acquired_at TEXT NOT NULL DEFAULT (datetime('now')), PRIMARY KEY (path)
);
CREATE TABLE IF NOT EXISTS agent_heartbeats (
    agent_id TEXT NOT NULL, repository_id TEXT NOT NULL DEFAULT '', status TEXT NOT NULL DEFAULT 'idle',
    last_seen TEXT NOT NULL DEFAULT (datetime('now')), PRIMARY KEY (agent_id, repository_id)
);
";

async fn open() -> VoxDb {
    let store: VoxDb = VoxDb::open_memory().await.expect("in-memory CodeStore");
    for tbl in &[
        "gamify_profiles",
        "gamify_achievements",
        "gamify_level_history",
        "gamify_companions",
        "gamify_quests",
        "gamify_battles",
        "agent_events",
        "cost_records",
        "agent_sessions",
        "agent_metrics",
        "gamify_counters",
        "gamify_daily_counters",
        "gamify_event_config",
        "gamify_collegium",
        "gamify_collegium_members",
        "gamify_policy_snapshots",
        "agent_locks",
        "agent_heartbeats",
    ] {
        store
            .connection()
            .execute(&format!("DROP TABLE IF EXISTS {}", tbl), ())
            .await
            .expect("drop tbl");
    }

    for stmt in GAMIFY_DDL.split(';') {
        let s = stmt.trim();
        if !s.is_empty() {
            store.connection().execute(s, ()).await.expect("gamify DDL");
        }
    }
    store
}

#[tokio::test]
async fn profile_upsert_and_load() {
    let store: VoxDb = open().await;
    store
        .upsert_gamify_profile(
            "user-1", 3, 250, 10, 100, 100, 0, 0, 5, 5, 0, 2, 0, 250, 0, 50, 0, 0, 0, 0, 0,
        )
        .await
        .expect("upsert profile");
    let row = store
        .get_gamify_profile_raw("user-1")
        .await
        .expect("get profile");
    let vals = row.expect("profile exists");
    assert_eq!(vals[0], 3, "level");
    assert_eq!(vals[1], 250, "xp");
    assert_eq!(vals[2], 10, "crystals");
}

#[tokio::test]
async fn achievement_unlock_idempotent() {
    let store: VoxDb = open().await;
    let inserted = store
        .unlock_gamify_achievement("u1", "ach1", 1000, 50, 10)
        .await
        .unwrap();
    assert!(inserted, "first unlock should be true");
    let again = store
        .unlock_gamify_achievement("u1", "ach1", 1010, 50, 10)
        .await
        .unwrap();
    assert!(!again, "second unlock should be false (idempotent)");
    let list = store.list_gamify_achievements("u1").await.unwrap();
    assert_eq!(list.len(), 1);
}

#[tokio::test]
async fn quest_lifecycle() {
    let store: VoxDb = open().await;
    store
        .upsert_gamify_quest(
            "q1",
            "u1",
            "create",
            "Do something",
            100,
            5,
            3,
            0,
            "active",
            0,
            false,
        )
        .await
        .unwrap();
    let quests = store.list_gamify_quests("u1").await.unwrap();
    assert_eq!(quests.len(), 1);
    store
        .update_gamify_quest_status("q1", "u1", "completed", true)
        .await
        .unwrap();
    let list_after = store.list_gamify_quests("u1").await.unwrap();
    // In our current list implementation, we don't have the status filter.
    // Just verify it still exists or has correct data if needed.
    assert_eq!(list_after.len(), 1);

    store.delete_gamify_quest("q1").await.unwrap();
    let all = store.list_gamify_quests("u1").await.unwrap();
    assert!(all.is_empty(), "quest deleted");
}

#[tokio::test]
async fn battle_crud() {
    let store: VoxDb = open().await;
    store
        .insert_gamify_battle(
            "b1", "u1", "comp-1", "type-a", "desc", None, None, false, 0, 0, 30, 1000,
        )
        .await
        .unwrap();
    let battles = store.list_gamify_battles("u1", 10).await.unwrap();
    assert_eq!(battles.len(), 1);
    store
        .update_gamify_battle("b1", Some("fixed_code"), true, 5, 50, 25)
        .await
        .unwrap();
    let b = store
        .get_gamify_battle("b1")
        .await
        .unwrap()
        .expect("battle exists");
    assert_eq!(b[7].as_deref(), Some("1"), "success=true→1");
}

#[tokio::test]
async fn counters_increment() {
    let store: VoxDb = open().await;
    let v1 = store
        .increment_gamify_counter("u1", "xp_calls")
        .await
        .unwrap();
    assert_eq!(v1, 1);
    let v2 = store
        .increment_gamify_counter("u1", "xp_calls")
        .await
        .unwrap();
    assert_eq!(v2, 2);
    store
        .set_gamify_counter("u1", "xp_calls", 100)
        .await
        .unwrap();
    let v3 = store.get_gamify_counter("u1", "xp_calls").await.unwrap();
    assert_eq!(v3, 100);
}

#[tokio::test]
async fn daily_counter_increment() {
    let store: VoxDb = open().await;
    let day = 20260323i64;
    let v1 = store
        .increment_gamify_daily_counter("u1", "build_success", day)
        .await
        .unwrap();
    assert_eq!(v1, 1);
    let v2 = store
        .get_gamify_daily_counter("u1", "build_success", day)
        .await
        .unwrap();
    assert_eq!(v2, 1);
}

#[tokio::test]
async fn cost_records() {
    let store: VoxDb = open().await;
    store
        .insert_gamify_cost_record(
            "agent-1",
            Some("sess-1"),
            "google",
            Some("gemini-flash"),
            100,
            50,
            0.001,
        )
        .await
        .unwrap();
    let total = store.get_gamify_agent_cost_usd("agent-1").await.unwrap();
    assert!(total > 0.0, "cost > 0");
    let records = store.list_gamify_cost_records("agent-1", 10).await.unwrap();
    assert_eq!(records.len(), 1);
}

#[tokio::test]
async fn a2a_send_poll_acknowledge() {
    let store: VoxDb = open().await;
    store
        .send_a2a_message(
            "uuid-1", "agent-1", "agent-2", "progress", "50% done", 1, None, "repo-abc",
        )
        .await
        .unwrap();
    let inbox = store.poll_a2a_inbox("agent-2", "repo-abc").await.unwrap();
    assert_eq!(inbox.len(), 1, "one message in inbox");
    store
        .acknowledge_a2a_message_by_uuid("uuid-1")
        .await
        .unwrap();
    let inbox2 = store.poll_a2a_inbox("agent-2", "repo-abc").await.unwrap();
    assert!(inbox2.is_empty(), "acknowledged message not returned");

    // Artificially age the message to the year 2020 to ensure it is much older than 0 days
    store
        .connection()
        .execute(
            "UPDATE a2a_messages SET created_at = '2020-01-01 00:00:00'",
            (),
        )
        .await
        .unwrap();

    let _pruned = store.prune_a2a_messages(0).await.unwrap();
    // Verify by querying instead of relying on `execute()` affected row count which can be flaky in libsql memory DB
    let mut rows: turso::Rows = store
        .connection()
        .query("SELECT COUNT(*) FROM a2a_messages WHERE acknowledged=1", ())
        .await
        .unwrap();
    let count = rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap();
    assert_eq!(count, 0, "acknowledged message pruned");
}

#[tokio::test]
async fn a2a_claim_prevents_duplicate_poll_by_second_consumer() {
    let store: VoxDb = open().await;
    store
        .send_a2a_message(
            "uuid-dup", "agent-1", "agent-2", "ping", "{}", 1, None, "repo-x",
        )
        .await
        .unwrap();
    let first = store
        .poll_a2a_inbox_claimed("agent-2", "repo-x", "consumer-a", 8, 300_000)
        .await
        .unwrap();
    assert_eq!(first.len(), 1);
    let second = store
        .poll_a2a_inbox_claimed("agent-2", "repo-x", "consumer-b", 8, 300_000)
        .await
        .unwrap();
    assert!(
        second.is_empty(),
        "active claim must hide the row from other consumers"
    );
}

#[tokio::test]
async fn a2a_expired_claim_allows_handoff() {
    let store: VoxDb = open().await;
    store
        .send_a2a_message(
            "uuid-expiry",
            "agent-1",
            "agent-2",
            "ping",
            "{}",
            1,
            None,
            "repo-y",
        )
        .await
        .unwrap();
    let _ = store
        .poll_a2a_inbox_claimed("agent-2", "repo-y", "consumer-a", 8, -60_000)
        .await
        .unwrap();
    let handoff = store
        .poll_a2a_inbox_claimed("agent-2", "repo-y", "consumer-b", 8, 300_000)
        .await
        .unwrap();
    assert_eq!(handoff.len(), 1);
    assert_eq!(handoff[0].message_uuid, "uuid-expiry");
}

#[tokio::test]
async fn oplog_append_and_list() {
    let store: VoxDb = open().await;
    store
        .append_oplog_entry(
            "agent-1",
            "OP-000001",
            "{\"FileEdit\":{\"paths\":[\"a.rs\"]}}",
            "edit a.rs",
            None,
            None,
            None,
            1000,
            "repo-abc",
        )
        .await
        .unwrap();
    let entries = store
        .list_oplog_entries(Some("agent-1"), "repo-abc", 10)
        .await
        .unwrap();
    assert_eq!(entries.len(), 1);
    store.set_oplog_undone("OP-000001", true).await.unwrap();
    let entries2 = store
        .list_oplog_entries(None, "repo-abc", 10)
        .await
        .unwrap();
    assert_eq!(entries2[0][8].as_deref(), Some("1"), "undone=1");
}

#[tokio::test]
async fn actor_state_crud() {
    let store: VoxDb = open().await;
    store
        .save_actor_state("my_key", "{\"x\":42}")
        .await
        .unwrap();
    let loaded = store.load_actor_state("my_key").await.unwrap();
    assert_eq!(loaded.as_deref(), Some("{\"x\":42}"));
    store.delete_actor_state("my_key").await.unwrap();
    let gone = store.load_actor_state("my_key").await.unwrap();
    assert!(gone.is_none());
}

#[tokio::test]
async fn orchestration_lineage_append_and_list() {
    let store: VoxDb = open().await;
    let repo = "repo-lineage-1";
    store
        .append_orchestration_lineage_event(
            repo,
            "task_submitted",
            99,
            Some(7),
            Some("sess-a"),
            None,
            None,
            None,
            Some(r#"{"x":1}"#),
        )
        .await
        .unwrap();
    let rows = store
        .list_orchestration_lineage_for_task(repo, 99, 10)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1.as_str(), "task_submitted");
}

#[tokio::test]
async fn orchestration_lineage_prune_respects_cutoff() {
    let store: VoxDb = open().await;
    let repo = "repo-prune-L";
    store
        .append_orchestration_lineage_event(
            repo,
            "task_submitted",
            1,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();
    let pruned = store
        .prune_orchestration_lineage_older_than_ms(0, 50)
        .await
        .unwrap();
    assert_eq!(pruned, 0, "positive created_at_ms must not be < 0");
    store
        .connection()
        .execute(
            "UPDATE orchestration_lineage_events SET created_at_ms = 100 WHERE repository_id = ?1",
            params![repo],
        )
        .await
        .unwrap();
    let pruned2 = store
        .prune_orchestration_lineage_older_than_ms(500, 10)
        .await
        .unwrap();
    assert_eq!(pruned2, 1);
}

#[tokio::test]
async fn agent_metrics_upsert() {
    let store: VoxDb = open().await;
    store
        .upsert_gamify_agent_metric("agent-1", "tokens_in", 1500.0, "daily")
        .await
        .unwrap();
    store
        .upsert_gamify_agent_metric("agent-1", "tokens_out", 700.0, "daily")
        .await
        .unwrap();
    let metrics = store
        .get_gamify_agent_metrics("agent-1", "daily")
        .await
        .unwrap();
    assert_eq!(metrics.len(), 2);
}

#[tokio::test]
async fn gamify_periodic_condition_queries() {
    let store: VoxDb = open().await;
    store
        .upsert_gamify_profile(
            "u_periodic",
            1,
            0,
            0,
            100,
            100,
            0,
            0,
            5,
            0,
            0,
            1,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        )
        .await
        .unwrap();
    let last = store
        .gamify_periodic_profile_last_active("u_periodic")
        .await
        .unwrap();
    assert!(last.is_some());

    assert_eq!(
        store
            .gamify_periodic_daily_quests_completed_today_count("u_periodic")
            .await
            .unwrap(),
        0
    );

    store
        .connection()
        .execute(
            "INSERT INTO gamify_quests (id, user_id, quest_type, title, description, status) VALUES ('q1', 'u_periodic', 'daily', '', '', 'completed')",
            (),
        )
        .await
        .unwrap();
    assert_eq!(
        store
            .gamify_periodic_daily_quests_completed_today_count("u_periodic")
            .await
            .unwrap(),
        1
    );

    store
        .connection()
        .execute(
            "INSERT INTO gamify_achievements (id, user_id, unlocked_at) VALUES ('ach1', 'u_periodic', 0)",
            (),
        )
        .await
        .unwrap();
    assert!(
        store
            .gamify_periodic_has_achievement("u_periodic", "ach1")
            .await
            .unwrap()
    );

    assert_eq!(
        store
            .gamify_periodic_profile_streak_days("u_periodic")
            .await
            .unwrap(),
        Some(5)
    );

    store
        .insert_gamify_policy_snapshot(
            "u_periodic",
            "doc_item",
            0,
            0,
            "balanced",
            1.0,
            0,
            0,
            0,
            false,
            0,
            None,
        )
        .await
        .unwrap();
    assert!(
        store
            .gamify_periodic_doc_item_count_this_month("u_periodic")
            .await
            .unwrap()
            >= 1
    );

    assert!(
        store
            .gamify_periodic_has_completed_quest("u_periodic", "q1")
            .await
            .unwrap()
    );

    assert!(
        store
            .gamify_periodic_perfect_week_completed_count("u_periodic")
            .await
            .unwrap()
            >= 1
    );
}
