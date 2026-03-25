//! V5 database schema: Gamification tables.
//!
//! Extends the existing vox-arca schema with tables for player profiles,
//! companions, quests, and battles.

/// V5 migration SQL — gamification tables.
///
/// All tables reference `users(id)` from V3, but use TEXT foreign keys
/// rather than enforcing FK constraints (user may be a local-only profile).
pub const SCHEMA_V5: &str = "
-- ── Gamification: Player Profiles ────────────────────────

CREATE TABLE IF NOT EXISTS gamify_profiles (
    user_id TEXT PRIMARY KEY,
    level INTEGER NOT NULL DEFAULT 1,
    xp INTEGER NOT NULL DEFAULT 0,
    crystals INTEGER NOT NULL DEFAULT 100,
    energy INTEGER NOT NULL DEFAULT 100,
    max_energy INTEGER NOT NULL DEFAULT 100,
    last_energy_regen TEXT NOT NULL DEFAULT (datetime('now')),
    last_active TEXT NOT NULL DEFAULT (datetime('now')),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ── Gamification: Companions ─────────────────────────────

CREATE TABLE IF NOT EXISTS gamify_companions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    code_hash TEXT,
    language TEXT NOT NULL DEFAULT 'vox',
    ascii_sprite TEXT,
    mood TEXT NOT NULL DEFAULT 'neutral',
    health INTEGER NOT NULL DEFAULT 100,
    max_health INTEGER NOT NULL DEFAULT 100,
    energy INTEGER NOT NULL DEFAULT 100,
    max_energy INTEGER NOT NULL DEFAULT 100,
    code_quality INTEGER NOT NULL DEFAULT 50,
    last_active TEXT NOT NULL DEFAULT (datetime('now')),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_gamify_companions_user ON gamify_companions(user_id);

-- ── Gamification: Daily Quests ───────────────────────────

CREATE TABLE IF NOT EXISTS gamify_quests (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    quest_type TEXT NOT NULL,
    description TEXT NOT NULL,
    target INTEGER NOT NULL DEFAULT 1,
    progress INTEGER NOT NULL DEFAULT 0,
    crystal_reward INTEGER NOT NULL DEFAULT 10,
    xp_reward INTEGER NOT NULL DEFAULT 15,
    completed INTEGER NOT NULL DEFAULT 0,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_gamify_quests_user ON gamify_quests(user_id);
CREATE INDEX IF NOT EXISTS idx_gamify_quests_expires ON gamify_quests(expires_at);

-- ── Gamification: Bug Battles ────────────────────────────

CREATE TABLE IF NOT EXISTS gamify_battles (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    companion_id TEXT NOT NULL,
    bug_type TEXT NOT NULL,
    bug_description TEXT NOT NULL,
    bug_code TEXT,
    submitted_code TEXT,
    success INTEGER NOT NULL DEFAULT 0,
    crystals_earned INTEGER NOT NULL DEFAULT 0,
    xp_earned INTEGER NOT NULL DEFAULT 0,
    duration_secs INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_gamify_battles_user ON gamify_battles(user_id);
CREATE INDEX IF NOT EXISTS idx_gamify_battles_companion ON gamify_battles(companion_id);
";

/// V6 migration SQL — multi-agent orchestration tables.
///
/// Adds tables for tracking agent sessions, events, costs,
/// inter-agent messages, and aggregated metrics.
pub const SCHEMA_V6: &str = "
-- ── Multi-Agent: Sessions ────────────────────────────────

CREATE TABLE IF NOT EXISTS agent_sessions (
    id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL,
    agent_name TEXT,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    task_snapshot TEXT,
    context_summary TEXT
);

CREATE INDEX IF NOT EXISTS idx_agent_sessions_agent ON agent_sessions(agent_id);
CREATE INDEX IF NOT EXISTS idx_agent_sessions_status ON agent_sessions(status);

-- ── Multi-Agent: Events (timeline) ───────────────────────

CREATE TABLE IF NOT EXISTS agent_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    payload_json TEXT,
    cli_version TEXT,
    timestamp TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_agent_events_agent ON agent_events(agent_id);
CREATE INDEX IF NOT EXISTS idx_agent_events_type ON agent_events(event_type);
CREATE INDEX IF NOT EXISTS idx_agent_events_ts ON agent_events(timestamp);

-- ── Multi-Agent: Cost Records ────────────────────────────

CREATE TABLE IF NOT EXISTS cost_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    session_id TEXT,
    provider TEXT NOT NULL,
    model TEXT,
    input_tokens INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    cost_usd REAL NOT NULL DEFAULT 0.0,
    timestamp TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_cost_records_agent ON cost_records(agent_id);
CREATE INDEX IF NOT EXISTS idx_cost_records_session ON cost_records(session_id);
CREATE INDEX IF NOT EXISTS idx_cost_records_ts ON cost_records(timestamp);

-- ── Multi-Agent: A2A Messages ────────────────────────────

CREATE TABLE IF NOT EXISTS a2a_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sender TEXT NOT NULL,
    receiver TEXT,
    msg_type TEXT NOT NULL,
    payload TEXT,
    correlation_id TEXT,
    acknowledged INTEGER NOT NULL DEFAULT 0,
    timestamp TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_a2a_messages_sender ON a2a_messages(sender);
CREATE INDEX IF NOT EXISTS idx_a2a_messages_receiver ON a2a_messages(receiver);
CREATE INDEX IF NOT EXISTS idx_a2a_messages_type ON a2a_messages(msg_type);

-- ── Multi-Agent: Aggregated Metrics ──────────────────────

CREATE TABLE IF NOT EXISTS agent_metrics (
    agent_id TEXT NOT NULL,
    metric_name TEXT NOT NULL,
    metric_value REAL NOT NULL DEFAULT 0.0,
    period TEXT NOT NULL DEFAULT 'session',
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (agent_id, metric_name, period)
);
";

/// V7 migration SQL — teaching state and reward policy snapshots.
pub const SCHEMA_V7: &str = "
-- ── Teaching: Per-user profile ───────────────────────────

CREATE TABLE IF NOT EXISTS gamify_teaching_profiles (
    user_id TEXT PRIMARY KEY,
    stage TEXT NOT NULL DEFAULT 'onboarding',
    silenced INTEGER NOT NULL DEFAULT 0,
    mistake_counts TEXT NOT NULL DEFAULT '{}',
    cooldowns TEXT NOT NULL DEFAULT '{}',
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Quest status: already included in gamify_quests at creation time (see SCHEMA_V5).

-- ── Reward policy: diagnostic snapshots ──────────────────

CREATE TABLE IF NOT EXISTS gamify_policy_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    base_xp INTEGER NOT NULL DEFAULT 0,
    base_crystals INTEGER NOT NULL DEFAULT 0,
    mode TEXT NOT NULL DEFAULT 'balanced',
    effective_multiplier REAL NOT NULL DEFAULT 1.0,
    xp_awarded INTEGER NOT NULL DEFAULT 0,
    crystals_awarded INTEGER NOT NULL DEFAULT 0,
    streak_days INTEGER NOT NULL DEFAULT 0,
    grind_capped INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_policy_snapshots_user ON gamify_policy_snapshots(user_id);
CREATE INDEX IF NOT EXISTS idx_policy_snapshots_event ON gamify_policy_snapshots(event_type);
";

/// V8 — Gamification Wave 3: prestige columns, AI feedback, and periodic rewards.
pub const SCHEMA_V8: &str = "
ALTER TABLE gamify_profiles ADD COLUMN total_xp_earned INTEGER NOT NULL DEFAULT 0;
ALTER TABLE gamify_profiles ADD COLUMN prestige_level INTEGER NOT NULL DEFAULT 0;

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
";
/// V9 — Quest Enhancement: hint, modifier, and status columns.
pub const SCHEMA_V9: &str = "
ALTER TABLE gamify_quests ADD COLUMN hint TEXT DEFAULT '';
ALTER TABLE gamify_quests ADD COLUMN modifier TEXT DEFAULT 'none';
ALTER TABLE gamify_quests ADD COLUMN status TEXT DEFAULT 'active';
";
/// V10 — Counter persistence for achievements.
pub const SCHEMA_V10: &str = "
CREATE TABLE IF NOT EXISTS gamify_counters (
    user_id TEXT NOT NULL,
    counter_name TEXT NOT NULL,
    count INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (user_id, counter_name)
);
";

/// V11 — Periodic Rewards Unlock Condition
pub const SCHEMA_V11: &str = "
ALTER TABLE gamify_periodic_rewards ADD COLUMN unlock_condition TEXT DEFAULT '\"WeeklyCheckIn\"';
";

// NOTE: V12 and V13 are reserved or were merged into V14 during development.
// They are explicitly skipped in ALL_MIGRATIONS to maintain sequence.

/// V14 — LUDUS: Lumens System
pub const SCHEMA_V14: &str = "
ALTER TABLE gamify_profiles ADD COLUMN lumens INTEGER NOT NULL DEFAULT 0;
ALTER TABLE gamify_profiles ADD COLUMN generosity_lumens INTEGER NOT NULL DEFAULT 0;
ALTER TABLE gamify_policy_snapshots ADD COLUMN lumens_awarded INTEGER NOT NULL DEFAULT 0;
";

/// V14b — LUDUS: Karma Cleanup (Redundant if V14 is fresh, but safe)
pub const SCHEMA_V14B: &str = "
-- SQLite doesn't support DROP COLUMN on older versions, so we use dummy drop or just rely on new names.
-- Since this is pre-release, we can safely assume these were not yet in production.
";

/// V15 — LUDUS: Collegium (Teams)
pub const SCHEMA_V15: &str = "
CREATE TABLE IF NOT EXISTS gamify_collegiums (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    leader_id TEXT NOT NULL,
    xp INTEGER NOT NULL DEFAULT 0,
    level INTEGER NOT NULL DEFAULT 1,
    lumens INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS gamify_collegium_members (
    collegium_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'member',
    joined_at INTEGER NOT NULL,
    PRIMARY KEY (collegium_id, user_id)
);
";

/// V16 — LUDUS: Arena (Community Events)
pub const SCHEMA_V16: &str = "
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
";

/// V17 — LUDUS: Streak Shields
pub const SCHEMA_V17: &str = "
ALTER TABLE gamify_profiles ADD COLUMN streak_shields INTEGER NOT NULL DEFAULT 0;
";

/// V18 — LUDUS: Daily counters (grind persistence) and per-event reward overrides.
pub const SCHEMA_V18: &str = "
-- Per-user, per-event-type, per-day counter for grind cap persistence.
CREATE TABLE IF NOT EXISTS gamify_daily_counters (
    user_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    day INTEGER NOT NULL,  -- unix day number (unix_epoch_secs / 86400)
    count INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (user_id, event_type, day)
);

-- Admin/user-level overrides for per-event XP and crystals.
CREATE TABLE IF NOT EXISTS gamify_event_config (
    event_type TEXT PRIMARY KEY,
    xp_override INTEGER,
    crystals_override INTEGER,
    enabled INTEGER NOT NULL DEFAULT 1,
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);
";

/// All active LUDUS schema migrations.
pub const ALL_MIGRATIONS: &[(&str, &str)] = &[
    ("v14", SCHEMA_V14),
    ("v14b", SCHEMA_V14B),
    ("v15", SCHEMA_V15),
    ("v16", SCHEMA_V16),
    ("v17", SCHEMA_V17),
    ("v18", SCHEMA_V18),
];
