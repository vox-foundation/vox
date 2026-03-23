//! Arca SQL: Ludus gamification system.
pub const SCHEMA_GAMIFICATION: &str = "
CREATE TABLE IF NOT EXISTS gamify_profiles (
    user_id TEXT PRIMARY KEY,
    level INTEGER NOT NULL DEFAULT 1,
    xp INTEGER NOT NULL DEFAULT 0,
    crystals INTEGER NOT NULL DEFAULT 100,
    energy INTEGER NOT NULL DEFAULT 100,
    max_energy INTEGER NOT NULL DEFAULT 100,
    last_energy_regen INTEGER NOT NULL DEFAULT 0,
    last_active INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

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
    last_active INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

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

CREATE INDEX IF NOT EXISTS idx_gamify_companions_user ON gamify_companions(user_id);
CREATE INDEX IF NOT EXISTS idx_gamify_quests_user ON gamify_quests(user_id);
CREATE INDEX IF NOT EXISTS idx_gamify_quests_expires ON gamify_quests(expires_at);
CREATE INDEX IF NOT EXISTS idx_gamify_battles_user ON gamify_battles(user_id);
CREATE INDEX IF NOT EXISTS idx_gamify_battles_companion ON gamify_battles(companion_id);
";
