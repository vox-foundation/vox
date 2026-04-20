//! Ludus incremental schema ladder (V6+ feature tables).
//!
//! **V5 (gamification)** stays aligned with Arca: see
//! [`vox_db::schema::domains::gamification_coordination::SCHEMA_GAMIFICATION_ONLY`].

/// V5 gamification tables — Arca SSOT (`vox-db` manifest fragment).
pub const SCHEMA_V5: &str =
    vox_db::schema::domains::gamification_coordination::SCHEMA_GAMIFICATION_ONLY;

/// V6 migration SQL — multi-agent orchestration tables.
///
/// Adds tables for tracking agent sessions, events, costs,
/// inter-agent messages, and aggregated metrics.
pub const SCHEMA_V6: &str = "-- Redundant: agent_sessions, agent_events, cost_records, agent_metrics, a2a_messages moved to Arca baseline.";

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
    mode_label TEXT NOT NULL DEFAULT 'balanced',
    effective_multiplier REAL NOT NULL DEFAULT 1.0,
    awarded_xp INTEGER NOT NULL DEFAULT 0,
    awarded_crystals INTEGER NOT NULL DEFAULT 0,
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
ALTER TABLE gamify_profiles ADD COLUMN streak_days INTEGER NOT NULL DEFAULT 0;
ALTER TABLE gamify_profiles ADD COLUMN longest_streak INTEGER NOT NULL DEFAULT 0;
ALTER TABLE gamify_profiles ADD COLUMN streak_last_ts INTEGER NOT NULL DEFAULT 0;
ALTER TABLE gamify_profiles ADD COLUMN grace_available INTEGER NOT NULL DEFAULT 0;
ALTER TABLE gamify_profiles ADD COLUMN grace_used INTEGER NOT NULL DEFAULT 0;

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
    name TEXT NOT NULL,
    count INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (user_id, name)
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
ALTER TABLE gamify_policy_snapshots ADD COLUMN lumens INTEGER NOT NULL DEFAULT 0;
";

/// V14b — LUDUS: Karma Cleanup (Redundant if V14 is fresh, but safe)
pub const SCHEMA_V14B: &str = "
-- SQLite doesn't support DROP COLUMN on older versions, so we use dummy drop or just rely on new names.
-- Since this is pre-release, we can safely assume these were not yet in production.
";

/// V15 — LUDUS: Collegium (Teams)
pub const SCHEMA_V15: &str = "
CREATE TABLE IF NOT EXISTS gamify_collegium (
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

/// V19 — LUDUS: Identity Federation (GitHub OAuth Device Flow)
pub const SCHEMA_V19: &str = "
CREATE TABLE IF NOT EXISTS vox_identities (
    vox_user_id   TEXT NOT NULL,
    provider      TEXT NOT NULL,
    provider_id   TEXT NOT NULL,
    provider_login TEXT,
    access_token_ref TEXT,
    linked_at     INTEGER NOT NULL,
    PRIMARY KEY (vox_user_id, provider)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_vox_identities_provider
    ON vox_identities(provider, provider_id);
";

/// V20 — LUDUS: Recognition metadata for policy snapshots
pub const SCHEMA_V20: &str = "
ALTER TABLE gamify_policy_snapshots ADD COLUMN metadata TEXT;
";

/// V21 — LUDUS: Trust tier for profiles
pub const SCHEMA_V21: &str = "
ALTER TABLE gamify_profiles ADD COLUMN trust_tier INTEGER DEFAULT 0;
";

/// V22 — LUDUS: Suppression flags for profiles
pub const SCHEMA_V22: &str = "
ALTER TABLE gamify_profiles ADD COLUMN reward_suppressed INTEGER NOT NULL DEFAULT 0;
ALTER TABLE gamify_profiles ADD COLUMN suppressed_until_ts INTEGER NOT NULL DEFAULT 0;
ALTER TABLE gamify_profiles ADD COLUMN suppression_reason TEXT;
";

/// V23 — LUDUS: Dispute table
pub const SCHEMA_V23: &str = "
CREATE TABLE IF NOT EXISTS gamify_disputes (
    id TEXT PRIMARY KEY,
    accused_user_id TEXT NOT NULL,
    accuser_user_id TEXT NOT NULL,
    github_event_id TEXT,
    snapshot_id INTEGER,
    evidence_json TEXT NOT NULL,
    malice_score REAL NOT NULL DEFAULT 0.0,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at INTEGER NOT NULL,
    resolved_at INTEGER,
    appeal_deadline_ts INTEGER NOT NULL,
    penalty_applied INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_gamify_disputes_accused ON gamify_disputes(accused_user_id);
CREATE INDEX IF NOT EXISTS idx_gamify_disputes_status ON gamify_disputes(status);
";

/// V24 — LUDUS: Dispute votes table
pub const SCHEMA_V24: &str = "
CREATE TABLE IF NOT EXISTS gamify_dispute_votes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    dispute_id TEXT NOT NULL REFERENCES gamify_disputes(id),
    juror_user_id TEXT NOT NULL,
    verdict TEXT NOT NULL,
    rationale TEXT,
    cast_at INTEGER NOT NULL,
    UNIQUE(dispute_id, juror_user_id)
);
CREATE INDEX IF NOT EXISTS idx_dispute_votes_dispute ON gamify_dispute_votes(dispute_id);
CREATE INDEX IF NOT EXISTS idx_dispute_votes_juror ON gamify_dispute_votes(juror_user_id);
";

/// V25 — LUDUS: Juror pool assignment table
pub const SCHEMA_V25: &str = "
CREATE TABLE IF NOT EXISTS gamify_dispute_jury (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    dispute_id TEXT NOT NULL REFERENCES gamify_disputes(id),
    juror_user_id TEXT NOT NULL,
    assigned_at INTEGER NOT NULL,
    notified INTEGER NOT NULL DEFAULT 0,
    UNIQUE(dispute_id, juror_user_id)
);
";

/// V26 — LUDUS: Companion personality column
pub const SCHEMA_V26: &str = "
ALTER TABLE gamify_companions ADD COLUMN personality TEXT NOT NULL DEFAULT 'focused';
";

/// V27 — LUDUS: Missing telemetry and dedupe tables
pub const SCHEMA_V27: &str = "
CREATE TABLE IF NOT EXISTS gamify_hint_telemetry (
    user_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    action TEXT NOT NULL,
    reason TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS gamify_processed_events (
    user_id TEXT NOT NULL,
    dedupe_key TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (user_id, dedupe_key)
);
CREATE TABLE IF NOT EXISTS gamify_notifications (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    notification_type TEXT NOT NULL,
    title TEXT NOT NULL,
    message TEXT NOT NULL,
    read INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT 0,
    expires_at INTEGER NOT NULL DEFAULT 0
);
";

/// All active LUDUS schema migrations.
pub const ALL_MIGRATIONS: &[(&str, &str)] = &[
    ("v6", SCHEMA_V6),
    ("v7", SCHEMA_V7),
    ("v8", SCHEMA_V8),
    ("v9", SCHEMA_V9),
    ("v10", SCHEMA_V10),
    ("v11", SCHEMA_V11),
    ("v14", SCHEMA_V14),
    ("v14b", SCHEMA_V14B),
    ("v15", SCHEMA_V15),
    ("v16", SCHEMA_V16),
    ("v17", SCHEMA_V17),
    ("v18", SCHEMA_V18),
    ("v19", SCHEMA_V19),
    ("v20", SCHEMA_V20),
    ("v21", SCHEMA_V21),
    ("v22", SCHEMA_V22),
    ("v23", SCHEMA_V23),
    ("v24", SCHEMA_V24),
    ("v25", SCHEMA_V25),
    ("v26", SCHEMA_V26),
    ("v27", SCHEMA_V27),
];

