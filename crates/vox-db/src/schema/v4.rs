/// V4: package yank flag (soft-delete published versions).
pub const SCHEMA_V4: &str = "
ALTER TABLE packages ADD COLUMN yanked INTEGER NOT NULL DEFAULT 0;
";
