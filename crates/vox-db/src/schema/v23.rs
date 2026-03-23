/// V23: **personality** column for `gamify_companions`.
pub const SCHEMA_V23: &str = r#"
ALTER TABLE gamify_companions ADD COLUMN personality TEXT NOT NULL DEFAULT 'focused';
ALTER TABLE gamify_quests ADD COLUMN hint TEXT;
ALTER TABLE gamify_quests ADD COLUMN modifier TEXT;
ALTER TABLE gamify_quests ADD COLUMN status TEXT NOT NULL DEFAULT 'active';
"#;
