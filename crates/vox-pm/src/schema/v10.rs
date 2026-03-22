/// V10: Agent reliability scores for Socrates-influenced routing and calibration.
pub const SCHEMA_V10: &str = "
CREATE TABLE IF NOT EXISTS agent_reliability (
  agent_id INTEGER NOT NULL PRIMARY KEY,
  reliability REAL NOT NULL DEFAULT 0.5,
  updated_at_ms INTEGER NOT NULL DEFAULT 0
);
";
